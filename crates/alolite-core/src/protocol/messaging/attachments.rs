//! Chunked photo/video/file attachment upload, `file_done` async waiter registry, and media URL/filename helpers.

use std::collections::HashMap;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{LazyLock, Mutex};
use std::time::{Duration, Instant};

use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::sync::oneshot;

use super::threads::ChatMessage;
use crate::crypto::{md5_hex, md5_hex_bytes};
use crate::protocol::session::{Session, SessionError};
use crate::protocol::wire::{
    domain_or, error_text, field_str, next_client_message_id, obj, opt_str, opt_url, require_login,
    strip_group_prefix, WireData,
};

const FILE_DONE_TIMEOUT: Duration = Duration::from_secs(90);
const FILE_DONE_EARLY_TTL: Duration = Duration::from_secs(120);

#[derive(Default)]
struct FileDoneRegistry {
    waiters: HashMap<String, oneshot::Sender<String>>,
    early: HashMap<String, (String, Instant)>,
}

static FILE_DONE_REGISTRY: LazyLock<Mutex<FileDoneRegistry>> =
    LazyLock::new(|| Mutex::new(FileDoneRegistry::default()));

static FILE_DONE_LISTENER_ACTIVE: AtomicBool = AtomicBool::new(false);

/// Format a byte count into a human-readable size string (e.g. `"1.4 MB"`, `"320 KB"`).
pub fn format_file_size(bytes: u64) -> String {
    if bytes >= 1024 * 1024 {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    } else if bytes >= 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{bytes} B")
    }
}

/// Convert Zalo `.jxl` URLs (`/jxl/` ... `.jxl?jxlstatus=1`) to standard JPEG URLs (`/jpg/` ... `.jpg`).
pub fn jxl_to_jpg_url(url: &str) -> String {
    if !url.contains("/jxl/") || !url.contains(".jxl") {
        return url.to_string();
    }
    let without_flag = url
        .replace("?jxlstatus=1&", "?")
        .replace("&jxlstatus=1", "")
        .replace("?jxlstatus=1", "");
    if without_flag.contains("e2esession=") {
        return without_flag;
    }
    without_flag
        .replace("/jxl/", "/jpg/")
        .replace(".jxl", ".jpg")
}

/// Check whether `url` points to a Zalo photo CDN asset (`photo-stal`, `photo-zmp`, `zpcloud.zdn.vn`, etc., `#72`).
pub fn is_zalo_photo_cdn_url(url: &str) -> bool {
    let u = url.trim().to_ascii_lowercase();
    if (!u.starts_with("http://") && !u.starts_with("https://")) || u.contains("/renewmedia") {
        return false;
    }
    let path_part = u.split('?').next().unwrap_or(&u);
    if [
        ".doc", ".docx", ".pdf", ".xls", ".xlsx", ".ppt", ".pptx", ".zip", ".rar", ".7z", ".txt",
        ".csv", ".mp4", ".mov", ".mp3", ".m4a", ".exe", ".dmg", ".apk",
    ]
    .iter()
    .any(|ext| path_part.ends_with(ext))
    {
        return false;
    }
    u.contains("photo-stal")
        || u.contains("photo-zmp")
        || u.contains("photo-aka")
        || (u.contains("zdn.vn")
            && (path_part.contains("/jpg/")
                || path_part.contains("/png/")
                || path_part.contains("/webp/")
                || path_part.contains("/gif/")
                || path_part.ends_with(".jpg")
                || path_part.ends_with(".jpeg")
                || path_part.ends_with(".png")
                || path_part.ends_with(".webp")
                || path_part.ends_with(".gif")))
}

/// Sanitize and resolve the original sender filename for a Zalo file attachment (`#74`).
pub fn resolve_attachment_file_name(title: Option<&str>, text: &str, url: &str) -> String {
    let is_usable_name = |s: &str| -> bool {
        let t = s.trim();
        !t.is_empty()
            && !t.starts_with("http://")
            && !t.starts_with("https://")
            && !t.eq_ignore_ascii_case("renewmedia")
            && !t.starts_with("renewMedia.")
            && !t.starts_with("renewmedia.")
            && !matches!(
                t,
                "[Tập tin]" | "[Chia sẻ]" | "[Hình ảnh]" | "[Video]" | "[Tin nhắn]"
            )
    };

    fn strip_prefix_tag(s: &str) -> &str {
        let t = s.trim();
        t.strip_prefix("[Tập tin]")
            .or_else(|| t.strip_prefix("[Chia sẻ]"))
            .map(str::trim)
            .unwrap_or(t)
    }

    let raw_candidate = title
        .map(strip_prefix_tag)
        .filter(|s| is_usable_name(s))
        .or_else(|| {
            let cleaned = strip_prefix_tag(text);
            is_usable_name(cleaned).then_some(cleaned)
        })
        .unwrap_or_else(|| {
            let path_seg = url
                .split('?')
                .next()
                .unwrap_or(url)
                .rsplit('/')
                .next()
                .unwrap_or("tap_tin_zalo");
            if path_seg.is_empty() || path_seg.eq_ignore_ascii_case("renewmedia") {
                "tap_tin_zalo"
            } else {
                path_seg
            }
        });

    let sanitized: String = raw_candidate
        .chars()
        .map(|c| {
            if matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|') || c.is_control() {
                '_'
            } else {
                c
            }
        })
        .collect();
    let trimmed = sanitized.trim().trim_matches('.');
    if trimmed.is_empty() {
        "tap_tin_zalo".to_string()
    } else {
        trimmed.to_string()
    }
}

/// Non-empty, trimmed string from an optional wire field.
fn non_empty_string(v: Option<&Value>) -> Option<String> {
    opt_str(v)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

/// Lower-case file extension without the leading dot, when the field is present.
fn normalized_ext(v: Option<&Value>) -> Option<String> {
    opt_str(v)
        .map(|s| s.trim().trim_start_matches('.').to_string())
        .filter(|s| !s.is_empty())
}

/// Link/media descriptor carried by the `content`/`params` of a link message.
///
/// # Wire Response Example
/// ```json
/// {
///   "href": "https://example.com/report.pdf",
///   "thumb": "https://thumb.example/1",
///   "fileName": "report",
///   "description": "Tài liệu họp",
///   "fileExt": "pdf",
///   "params": "{\"fileExt\":\"pdf\",\"fileSize\":1234,\"src\":\"Tập tin Zalo\"}"
/// }
/// ```
///
/// # Core Transformation
/// Feeds the `(url, thumb, title, src, desc)` tuple of `extract_link_info`; `params` is a
/// JSON-encoded **string** on the wire and is decoded separately into [`LinkParamsWireData`].
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct LinkInfoWireData {
    #[serde(rename = "href")]
    pub href: Option<Value>,
    #[serde(rename = "thumb")]
    pub thumb: Option<Value>,
    #[serde(rename = "hd")]
    pub hd: Option<Value>,
    #[serde(rename = "fileName")]
    pub file_name: Option<Value>,
    #[serde(rename = "name")]
    pub name: Option<Value>,
    #[serde(rename = "title")]
    pub title: Option<Value>,
    #[serde(rename = "description")]
    pub description: Option<Value>,
    #[serde(rename = "caption")]
    pub caption: Option<Value>,
    #[serde(rename = "fileExt")]
    pub file_ext: Option<Value>,
    #[serde(rename = "params")]
    pub params: Option<Value>,
}

impl LinkInfoWireData {
    /// Decode the nested `params` JSON string, if it parses.
    pub fn parsed_params(&self) -> Option<LinkParamsWireData> {
        serde_json::from_str::<LinkParamsWireData>(self.params.as_ref()?.as_str()?.trim()).ok()
    }
}

/// `params` sub-object of a link descriptor (sent as a JSON-encoded string).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct LinkParamsWireData {
    #[serde(rename = "fileName")]
    pub file_name: Option<Value>,
    #[serde(rename = "mediaTitle")]
    pub media_title: Option<Value>,
    #[serde(rename = "fileExt")]
    pub file_ext: Option<Value>,
    #[serde(rename = "fileSize")]
    pub file_size: Option<Value>,
    #[serde(rename = "checksum")]
    pub checksum: Option<Value>,
    #[serde(rename = "src")]
    pub src: Option<Value>,
}

/// Extract rich link/media card metadata (`link_url`, `link_title`, `link_desc`, `link_thumb`, `link_src`)
/// from a Zalo `chat.recommended` or rich-object `content`.
pub(super) fn extract_link_info(
    v: &Value,
) -> (
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
) {
    match v {
        Value::String(s) => {
            let trimmed = s.trim();
            if trimmed.starts_with('{') && trimmed.ends_with('}') {
                if let Ok(parsed) = serde_json::from_str::<Value>(trimmed) {
                    return extract_link_info(&parsed);
                }
            }
            (None, None, None, None, None)
        }
        Value::Object(_) => {
            let wire = LinkInfoWireData::from_value(v);
            let Some(url) = opt_url(wire.href.as_ref()) else {
                return (None, None, None, None, None);
            };
            let thumb = opt_url(wire.thumb.as_ref().or(wire.hd.as_ref()));
            let mut title = non_empty_string(
                wire.file_name
                    .as_ref()
                    .or(wire.name.as_ref())
                    .or(wire.title.as_ref()),
            );
            let desc = non_empty_string(wire.description.as_ref().or(wire.caption.as_ref()));
            let mut src = None;
            let mut file_ext = normalized_ext(wire.file_ext.as_ref());
            let mut has_file_params =
                wire.file_name.is_some() || url.to_ascii_lowercase().contains("/renewmedia");

            if let Some(params) = wire.parsed_params() {
                if title.is_none() {
                    title =
                        non_empty_string(params.file_name.as_ref().or(params.media_title.as_ref()));
                }
                if file_ext.is_none() {
                    file_ext = normalized_ext(params.file_ext.as_ref());
                }
                if params.file_ext.is_some()
                    || params.file_size.is_some()
                    || params.checksum.is_some()
                {
                    has_file_params = true;
                }
                src = non_empty_string(params.src.as_ref());
            }
            if let (Some(t), Some(ext)) = (title.as_mut(), file_ext.as_ref()) {
                let suffix = format!(".{ext}");
                if !t
                    .to_ascii_lowercase()
                    .ends_with(&suffix.to_ascii_lowercase())
                    && !t.starts_with("http://")
                    && !t.starts_with("https://")
                {
                    t.push_str(&suffix);
                }
            }
            if src.is_none() && has_file_params {
                src = Some("Tập tin Zalo".to_string());
            }
            if src.is_none() && is_zalo_photo_cdn_url(&url) {
                src = Some("Hình ảnh Zalo".to_string());
            }
            if src.is_none() {
                src = url
                    .split("://")
                    .nth(1)
                    .and_then(|rest| rest.split('/').next())
                    .filter(|h| !h.is_empty())
                    .map(str::to_string);
            }
            (Some(url), title, desc, thumb, src)
        }
        _ => (None, None, None, None, None),
    }
}

fn file_id_key(v: &Value) -> Option<String> {
    match v {
        Value::String(s) if !s.trim().is_empty() => Some(s.trim().to_string()),
        Value::Number(n) => Some(n.to_string()),
        _ => None,
    }
}

/// `act_type: "file_done"` control item: the file id plus the resolved media URL.
///
/// # Wire Response Example
/// ```json
/// { "act_type": "file_done", "fileId": 1234567, "data": { "url": "https://f.zdn.vn/a.pdf" } }
/// ```
///
/// # Core Transformation
/// Produces the `(fileId, url)` pairs that resolve pending async uploads; `fileId` is looked up
/// on the control item first and on the decoded `data` object second.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct FileDoneControlWireData {
    #[serde(rename = "fileId")]
    pub file_id: Option<Value>,
    #[serde(rename = "url")]
    pub url: Option<Value>,
    #[serde(rename = "fileUrl")]
    pub file_url: Option<Value>,
}

impl FileDoneControlWireData {
    /// Raw `fileId` as sent (string or number).
    pub fn file_id_value(&self) -> Option<&Value> {
        self.file_id.as_ref()
    }

    /// Resolved media URL (`url` > `fileUrl`).
    pub fn url(&self) -> Option<String> {
        field_str(self.url.as_ref()).or_else(|| field_str(self.file_url.as_ref()))
    }
}

/// `data` object of a photo upload (`/api/message/photo` family).
///
/// # Wire Response Example
/// ```json
/// {
///   "photoId": "1234567890123456789",
///   "normalUrl": "https://photo.example/normal",
///   "hdUrl": "https://photo.example/hd",
///   "thumbUrl": "https://photo.example/thumb"
/// }
/// ```
///
/// # Core Transformation
/// `hdUrl` and `thumbUrl` fall back to the normal URL so an outgoing image message always has a
/// renderable target.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct PhotoUploadWireData {
    #[serde(rename = "photoId")]
    pub photo_id: Option<Value>,
    #[serde(rename = "clientFileId")]
    pub client_file_id: Option<Value>,
    #[serde(rename = "normalUrl")]
    pub normal_url: Option<Value>,
    #[serde(rename = "hdUrl")]
    pub hd_url: Option<Value>,
    #[serde(rename = "oriUrl")]
    pub ori_url: Option<Value>,
    #[serde(rename = "url")]
    pub url: Option<Value>,
    #[serde(rename = "thumbUrl")]
    pub thumb_url: Option<Value>,
}

impl PhotoUploadWireData {
    /// Server photo id (`photoId` > `clientFileId`).
    pub fn photo_id(&self) -> Option<String> {
        field_str(self.photo_id.as_ref()).or_else(|| field_str(self.client_file_id.as_ref()))
    }

    /// Primary media URL (`normalUrl` > `hdUrl` > `oriUrl` > `url`).
    pub fn normal_url(&self) -> String {
        field_str(self.normal_url.as_ref())
            .or_else(|| field_str(self.hd_url.as_ref()))
            .or_else(|| field_str(self.ori_url.as_ref()))
            .or_else(|| field_str(self.url.as_ref()))
            .unwrap_or_default()
    }

    /// High-definition URL (`hdUrl` > `normalUrl` > `oriUrl`).
    pub fn hd_url(&self) -> Option<String> {
        field_str(self.hd_url.as_ref())
            .or_else(|| field_str(self.normal_url.as_ref()))
            .or_else(|| field_str(self.ori_url.as_ref()))
    }

    /// Thumbnail URL (`thumbUrl` > `normalUrl` > `hdUrl`).
    pub fn thumb_url(&self) -> Option<String> {
        field_str(self.thumb_url.as_ref())
            .or_else(|| field_str(self.normal_url.as_ref()))
            .or_else(|| field_str(self.hd_url.as_ref()))
    }
}

/// `data` object of a chunked file upload (`/api/message/asyncfile/upload`).
///
/// # Wire Response Example
/// ```json
/// { "fileId": "1234567890123456789", "fileUrl": "https://file.example/x", "normalUrl": "https://file.example/x" }
/// ```
///
/// # Core Transformation
/// Supplies the `fileId` / `fileUrl` pair used by the final `asyncfile/msg` call; a `-1` id means
/// the chunk was accepted but the file is not ready yet.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct UploadChunkWireData {
    #[serde(rename = "fileId")]
    pub file_id: Option<Value>,
    #[serde(rename = "clientFileId")]
    pub client_file_id: Option<Value>,
    #[serde(rename = "fileUrl")]
    pub file_url: Option<Value>,
    #[serde(rename = "url")]
    pub url: Option<Value>,
    #[serde(rename = "normalUrl")]
    pub normal_url: Option<Value>,
}

impl UploadChunkWireData {
    /// Uploaded file id (`fileId` > `clientFileId`), dropping the `-1` placeholder.
    pub fn file_id(&self) -> Option<String> {
        field_str(self.file_id.as_ref())
            .or_else(|| field_str(self.client_file_id.as_ref()))
            .filter(|id| id != "-1")
    }

    /// Uploaded file URL (`fileUrl` > `url` > `normalUrl`).
    pub fn file_url(&self) -> String {
        field_str(self.file_url.as_ref())
            .or_else(|| field_str(self.url.as_ref()))
            .or_else(|| field_str(self.normal_url.as_ref()))
            .unwrap_or_default()
    }
}

/// `data` object of the outbound send endpoints (`sharefile_full`, `photo_original/send`, …).
///
/// # Wire Response Example
/// ```json
/// { "msgId": "1731215400000_123456789", "fileUrl": "https://f.zdn.vn/a.pdf" }
/// ```
///
/// # Core Transformation
/// Yields the outgoing message id (falling back to `out_<clientId>`) and, for share-file sends,
/// the final media URL.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct SendAckWireData {
    #[serde(rename = "msgId")]
    pub msg_id: Option<Value>,
    #[serde(rename = "fileUrl")]
    pub file_url: Option<Value>,
    #[serde(rename = "url")]
    pub url: Option<Value>,
    #[serde(rename = "href")]
    pub href: Option<Value>,
}

impl SendAckWireData {
    /// Server message id (`msgId`).
    pub fn msg_id(&self) -> Option<String> {
        field_str(self.msg_id.as_ref())
    }

    /// Share-file media URL (`fileUrl` > `url` > `href`).
    pub fn file_url(&self) -> Option<String> {
        field_str(self.file_url.as_ref())
            .or_else(|| field_str(self.url.as_ref()))
            .or_else(|| field_str(self.href.as_ref()))
    }
}

/// Parse `act_type: "file_done"` items (`{fileId, data: {url}}`) into `(fileId, fileUrl)` pairs.
pub fn extract_file_done_controls(data_node: &Value) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for c in crate::protocol::wire::iter_control_items(data_node) {
        if c.get("act_type").and_then(Value::as_str) != Some("file_done") {
            continue;
        }
        let outer = FileDoneControlWireData::from_value(c);
        let d = crate::protocol::wire::parse_control_data(c.get("data"));
        let inner = FileDoneControlWireData::from_value(&d);
        let Some(file_id) = outer
            .file_id_value()
            .or_else(|| inner.file_id_value())
            .and_then(file_id_key)
        else {
            continue;
        };
        let Some(url) = inner.url() else {
            continue;
        };
        out.push((file_id, url));
    }
    out
}

/// Resolve pending async file uploads from a decrypted control payload (`cmd 601` or `lp_control`).
pub fn dispatch_file_done_controls(data_node: &Value) {
    FILE_DONE_LISTENER_ACTIVE.store(true, Ordering::Relaxed);
    let events = extract_file_done_controls(data_node);
    if events.is_empty() {
        return;
    }
    let Ok(mut reg) = FILE_DONE_REGISTRY.lock() else {
        crate::logger::log_error("attachments::file_done", "registry mutex poisoned");
        return;
    };
    reg.early
        .retain(|_, (_, at)| at.elapsed() < FILE_DONE_EARLY_TTL);
    for (file_id, url) in events {
        match reg.waiters.remove(&file_id) {
            Some(tx) => {
                let _ = tx.send(url);
            }
            None => {
                reg.early.insert(file_id, (url, Instant::now()));
            }
        }
    }
}

fn register_file_done_waiter(file_id: &str) -> oneshot::Receiver<String> {
    let (tx, rx) = oneshot::channel();
    match FILE_DONE_REGISTRY.lock() {
        Ok(mut reg) => match reg.early.remove(file_id) {
            Some((url, _)) => {
                let _ = tx.send(url);
            }
            None => {
                reg.waiters.insert(file_id.to_string(), tx);
            }
        },
        Err(_) => crate::logger::log_error("attachments::file_done", "registry mutex poisoned"),
    }
    rx
}

async fn wait_for_file_done(file_id: &str, timeout: Duration) -> Option<String> {
    let rx = register_file_done_waiter(file_id);
    let res = tokio::time::timeout(timeout, rx).await;
    if let Ok(mut reg) = FILE_DONE_REGISTRY.lock() {
        reg.waiters.remove(file_id);
    }
    res.ok().and_then(Result::ok)
}

/// Upload and send a photo, video, or general file attachment to a 1-1 or group conversation.
///
/// # Errors
/// Returns an error if unauthenticated, `file_path` is unreadable/empty, or upload/send fails.
pub async fn send_attachment(
    s: &Session,
    to_id: &str,
    is_group: bool,
    file_path: &Path,
    caption: &str,
) -> Result<ChatMessage> {
    require_login(s)?;
    let file_bytes = std::fs::read(file_path)?;
    if file_bytes.is_empty() {
        anyhow::bail!("Tập tin rỗng (0 byte)");
    }
    let file_name = file_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("attachment.bin")
        .to_string();
    let ext = file_path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();

    let is_image = matches!(
        ext.as_str(),
        "png" | "jpg" | "jpeg" | "webp" | "gif" | "bmp"
    );
    let is_video = matches!(ext.as_str(), "mp4" | "mov" | "mkv" | "webm" | "avi");

    let file_host = domain_or(&s.domains.file, crate::endpoints::FILE_FALLBACK_HOST);
    let clean_target = if is_group {
        strip_group_prefix(to_id)
    } else {
        to_id
    };
    let client_id = next_client_message_id();
    let total_size = file_bytes.len() as u64;
    let chunk_size: usize = 1_048_576;
    let total_chunks = file_bytes.len().div_ceil(chunk_size).max(1);
    let target_type = if is_group { 11 } else { 2 };

    if is_image {
        let (width, height) = image::load_from_memory(&file_bytes)
            .map(|img| (img.width(), img.height()))
            .unwrap_or((800, 600));
        let mime = match ext.as_str() {
            "png" => "image/png",
            "webp" => "image/webp",
            "gif" => "image/gif",
            "bmp" => "image/bmp",
            _ => "image/jpeg",
        };
        let upload_ep = if is_group {
            "/api/group/photo_original/upload"
        } else {
            "/api/message/photo_original/upload"
        };

        let mut last_up_dec = Value::Null;
        for (idx, chunk) in file_bytes.chunks(chunk_size).enumerate() {
            let chunk_id = idx + 1;
            let mut m = serde_json::Map::new();
            m.insert("totalChunk".into(), json!(total_chunks));
            m.insert("fileName".into(), json!(file_name));
            m.insert("clientId".into(), json!(client_id));
            m.insert("totalSize".into(), json!(total_size));
            m.insert("imei".into(), json!(s.imei));
            m.insert("isE2EE".into(), json!(0));
            m.insert("jxl".into(), json!(0));
            m.insert("chunkId".into(), json!(chunk_id));
            if is_group {
                m.insert("grid".into(), json!(clean_target));
            } else {
                m.insert("toid".into(), json!(clean_target));
            }
            let params = s.encoded_params(&Value::Object(m))?;
            let url = format!(
                "{file_host}{upload_ep}?{}&params={params}&type={target_type}",
                s.common_params()
            );
            let up_raw = s
                .post_multipart(&url, "chunkContent", &file_name, mime, chunk)
                .await?;
            last_up_dec = s.maybe_decrypt(up_raw, s.session_key_bytes()).await;
        }

        let photo =
            PhotoUploadWireData::from_value(last_up_dec.get("data").unwrap_or(&last_up_dec));
        let photo_id = photo.photo_id();
        let normal_url = photo.normal_url();
        let hd_url = photo.hd_url().unwrap_or_else(|| normal_url.clone());
        let thumb_url = photo.thumb_url().unwrap_or_else(|| normal_url.clone());

        let send_resp = if let Some(pid) = photo_id.filter(|_| !normal_url.is_empty()) {
            let send_ep = if is_group {
                "/api/group/photo_original/send"
            } else {
                "/api/message/photo_original/send"
            };
            let pid_val = pid
                .parse::<u64>()
                .map(|n| json!(n))
                .unwrap_or_else(|_| json!(pid));
            let mut send_obj = serde_json::Map::new();
            send_obj.insert("photoId".into(), pid_val);
            send_obj.insert("clientId".into(), json!(client_id));
            send_obj.insert("desc".into(), json!(caption));
            send_obj.insert("width".into(), json!(width));
            send_obj.insert("height".into(), json!(height));
            send_obj.insert("rawUrl".into(), json!(normal_url));
            send_obj.insert("hdUrl".into(), json!(hd_url));
            send_obj.insert("thumbUrl".into(), json!(thumb_url));
            send_obj.insert("oriUrl".into(), json!(normal_url));
            send_obj.insert("normalUrl".into(), json!(normal_url));
            send_obj.insert("hdSize".into(), json!(total_size));
            send_obj.insert("zsource".into(), json!(-1));
            send_obj.insert("jcp".into(), json!("{\"convertible\":true}"));
            send_obj.insert("imei".into(), json!(s.imei));
            if is_group {
                send_obj.insert("grid".into(), json!(clean_target));
                send_obj.insert("visibility".into(), json!(0));
            } else {
                send_obj.insert("toid".into(), json!(clean_target));
            }
            let params = s.encoded_params(&Value::Object(send_obj))?;
            let url = format!("{file_host}{send_ep}?{}", s.common_params());
            let r = s.post_form(&url, format!("params={params}")).await?;
            s.maybe_decrypt(r, s.session_key_bytes()).await
        } else {
            let single_ep = if is_group {
                "/api/group/sendpt"
            } else {
                "/api/message/photo"
            };
            let mut body_obj = serde_json::Map::new();
            body_obj.insert("clientId".into(), json!(client_id));
            body_obj.insert("desc".into(), json!(caption));
            body_obj.insert("width".into(), json!(width));
            body_obj.insert("height".into(), json!(height));
            body_obj.insert("imei".into(), json!(s.imei));
            if is_group {
                body_obj.insert("grid".into(), json!(clean_target));
                body_obj.insert("visibility".into(), json!(0));
            } else {
                body_obj.insert("toid".into(), json!(clean_target));
            }
            let params = s.encoded_params(&Value::Object(body_obj))?;
            let url = format!(
                "{file_host}{single_ep}?{}&params={params}&type={target_type}",
                s.common_params()
            );
            let r = s
                .post_multipart(&url, "fileContent", &file_name, mime, &file_bytes)
                .await?;
            s.maybe_decrypt(r, s.session_key_bytes()).await
        };

        let send_env = crate::protocol::wire::ErrorEnvelopeWireData::parse(&send_resp);
        if let Some(code) = send_env.code_opt() {
            if code != 0 {
                return Err(SessionError::new(code, error_text(send_env.message())).into());
            }
        }

        let final_thumb = if !thumb_url.is_empty() {
            thumb_url
        } else {
            format!("local://{}", file_path.display())
        };
        let final_url = if !hd_url.is_empty() {
            hd_url
        } else {
            file_path.display().to_string()
        };
        let _ = std::fs::create_dir_all("stickers_cache");
        let cache_key = format!("thumb_{}", md5_hex(&final_thumb));
        let _ = std::fs::write(format!("stickers_cache/{cache_key}.png"), &file_bytes);

        let msg_id = send_resp
            .get("data")
            .and_then(|d| SendAckWireData::from_value(d).msg_id())
            .unwrap_or_else(|| format!("out_{client_id}"));
        let display_text = if !caption.trim().is_empty() {
            caption.trim().to_string()
        } else {
            format!("[Hình ảnh] {file_name}")
        };
        let cm = ChatMessage {
            id: msg_id,
            client_message_id: client_id.to_string(),
            sender: "Tôi".into(),
            text: display_text,
            time: crate::ui::clock(),
            is_outgoing: true,
            ts: client_id as i64,
            link_url: Some(final_url),
            link_title: Some(format!(
                "{file_name} ({width}×{height} · {})",
                format_file_size(total_size)
            )),
            link_desc: if caption.trim().is_empty() {
                None
            } else {
                Some(caption.trim().to_string())
            },
            link_thumb: Some(final_thumb),
            link_src: Some("Hình ảnh Zalo".into()),
            ..Default::default()
        };
        return Ok(cm);
    }

    let mime = match ext.as_str() {
        "mp4" => "video/mp4",
        "mov" => "video/quicktime",
        "mkv" => "video/x-matroska",
        "webm" => "video/webm",
        "mp3" => "audio/mpeg",
        "wav" => "audio/wav",
        "pdf" => "application/pdf",
        "zip" => "application/zip",
        "txt" | "md" | "log" => "text/plain",
        _ => "application/octet-stream",
    };
    let checksum = md5_hex_bytes(&file_bytes);
    let upload_ep = if is_group {
        "/api/group/asyncfile/upload"
    } else {
        "/api/message/asyncfile/upload"
    };

    let mut last_up_dec = Value::Null;
    for (idx, chunk) in file_bytes.chunks(chunk_size).enumerate() {
        let chunk_id = idx + 1;
        let mut m = serde_json::Map::new();
        m.insert("totalChunk".into(), json!(total_chunks));
        m.insert("fileName".into(), json!(file_name));
        m.insert("clientId".into(), json!(client_id));
        m.insert("totalSize".into(), json!(total_size));
        m.insert("imei".into(), json!(s.imei));
        m.insert("isE2EE".into(), json!(0));
        m.insert("chunkId".into(), json!(chunk_id));
        if is_group {
            m.insert("grid".into(), json!(clean_target));
        } else {
            m.insert("toid".into(), json!(clean_target));
        }
        let params = s.encoded_params(&Value::Object(m))?;
        let url = format!(
            "{file_host}{upload_ep}?{}&params={params}&type={target_type}",
            s.common_params()
        );
        let up_raw = s
            .post_multipart(&url, "chunkContent", &file_name, mime, chunk)
            .await
            .map_err(|e| anyhow::anyhow!("tải lên phần {chunk_id}/{total_chunks} thất bại: {e}"))?;
        last_up_dec = s.maybe_decrypt(up_raw, s.session_key_bytes()).await;
        let env = crate::protocol::wire::ErrorEnvelopeWireData::parse(&last_up_dec);
        if let Some(code) = env.code_opt() {
            if code != 0 {
                return Err(SessionError::new(code, error_text(env.message())).into());
            }
        }
    }

    let upload = UploadChunkWireData::from_value(last_up_dec.get("data").unwrap_or(&last_up_dec));
    let file_id = upload.file_id();
    let mut file_url = upload.file_url();
    if file_url.is_empty() && FILE_DONE_LISTENER_ACTIVE.load(Ordering::Relaxed) {
        if let Some(fid) = file_id.as_deref() {
            match wait_for_file_done(fid, FILE_DONE_TIMEOUT).await {
                Some(url) => file_url = url,
                None => crate::logger::log_error(
                    "attachments::send_attachment",
                    &format!("no file_done event for fileId {fid} within {FILE_DONE_TIMEOUT:?}"),
                ),
            }
        }
    }

    let mut send_resp = if let Some(fid) = file_id {
        let msg_ep = if is_group {
            "/api/group/asyncfile/msg"
        } else {
            "/api/message/asyncfile/msg"
        };
        let fid_val = fid
            .parse::<u64>()
            .map(|n| json!(n))
            .unwrap_or_else(|_| json!(fid));
        let mut msg_obj = serde_json::Map::new();
        msg_obj.insert("fileId".into(), fid_val);
        if !file_url.is_empty() {
            msg_obj.insert("fileUrl".into(), json!(file_url));
        }
        msg_obj.insert("checksum".into(), json!(checksum));
        msg_obj.insert("checksumSha".into(), json!(""));
        msg_obj.insert("extension".into(), json!(ext));
        msg_obj.insert("totalSize".into(), json!(total_size));
        msg_obj.insert("fileName".into(), json!(file_name));
        msg_obj.insert("clientId".into(), json!(client_id));
        msg_obj.insert("fType".into(), json!(if is_video { 2 } else { 1 }));
        msg_obj.insert("fileCount".into(), json!(0));
        msg_obj.insert("fdata".into(), json!("{}"));
        msg_obj.insert("imei".into(), json!(s.imei));
        if is_group {
            msg_obj.insert("grid".into(), json!(clean_target));
        } else {
            msg_obj.insert("toid".into(), json!(clean_target));
        }
        let params = s.encoded_params(&Value::Object(msg_obj))?;
        let url = format!("{file_host}{msg_ep}?{}", s.common_params());
        let r = s.post_form(&url, format!("params={params}")).await?;
        s.maybe_decrypt(r, s.session_key_bytes()).await
    } else {
        json!({ "error_code": -1 })
    };

    if crate::protocol::wire::ErrorEnvelopeWireData::parse(&send_resp)
        .code_opt()
        .unwrap_or(-1)
        != 0
    {
        let share_ep = if is_group {
            "/api/group/sharefile_full"
        } else {
            "/api/message/sharefile_full"
        };
        let body = if is_group {
            obj(&[
                ("grid", json!(clean_target)),
                ("clientId", json!(client_id)),
            ])
        } else {
            obj(&[
                ("toid", json!(clean_target)),
                ("clientId", json!(client_id)),
            ])
        };
        let params = s.encoded_params(&body)?;
        let url = format!(
            "{file_host}{share_ep}?{}&params={params}&type={target_type}",
            s.common_params()
        );
        let r = s
            .post_multipart(&url, "fileContent", &file_name, mime, &file_bytes)
            .await?;
        send_resp = s.maybe_decrypt(r, s.session_key_bytes()).await;
    }

    let send_env = crate::protocol::wire::ErrorEnvelopeWireData::parse(&send_resp);
    if let Some(code) = send_env.code_opt() {
        if code != 0 {
            return Err(SessionError::new(code, error_text(send_env.message())).into());
        }
    }

    let final_url = if !file_url.is_empty() {
        file_url
    } else {
        send_resp
            .get("data")
            .and_then(|d| SendAckWireData::from_value(d).file_url())
            .unwrap_or_default()
    };
    let msg_id = send_resp
        .get("data")
        .and_then(|d| SendAckWireData::from_value(d).msg_id())
        .unwrap_or_else(|| format!("out_{client_id}"));
    let kind_label = if is_video { "Video" } else { "Tập tin" };
    let display_text = if !caption.trim().is_empty() {
        caption.trim().to_string()
    } else {
        format!("[{kind_label}] {file_name}")
    };
    let cm = ChatMessage {
        id: msg_id,
        client_message_id: client_id.to_string(),
        sender: "Tôi".into(),
        text: display_text,
        time: crate::ui::clock(),
        is_outgoing: true,
        ts: client_id as i64,
        link_url: if final_url.is_empty() {
            None
        } else {
            Some(final_url)
        },
        link_title: Some(format!("{file_name} ({})", format_file_size(total_size))),
        link_desc: if caption.trim().is_empty() {
            None
        } else {
            Some(caption.trim().to_string())
        },
        link_src: Some(format!("{kind_label} Zalo")),
        ..Default::default()
    };
    Ok(cm)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_file_done_controls_parses_object_and_string_data() {
        let payload = json!({
            "controls": [
                { "content": { "act_type": "file_done", "fileId": 1234567, "data": { "url": "https://f.zdn.vn/a.pdf" } } },
                { "content": { "act_type": "file_done", "fileId": "89", "data": "{\"url\":\"https://f.zdn.vn/b.mp4\"}" } },
                { "content": { "act_type": "voip", "fileId": 1, "data": { "url": "https://x" } } },
                { "content": { "act_type": "file_done", "fileId": 2, "data": {} } }
            ]
        });
        assert_eq!(
            extract_file_done_controls(&payload),
            vec![
                ("1234567".to_string(), "https://f.zdn.vn/a.pdf".to_string()),
                ("89".to_string(), "https://f.zdn.vn/b.mp4".to_string()),
            ]
        );
    }

    #[tokio::test]
    async fn file_done_resolves_waiters_registered_before_and_after_the_event() {
        let ev = |id: &str, url: &str| json!({ "controls": [{ "content": { "act_type": "file_done", "fileId": id, "data": { "url": url } } }] });

        let waiter = tokio::spawn(async {
            wait_for_file_done("test_fd_after", Duration::from_secs(5)).await
        });
        tokio::time::sleep(Duration::from_millis(50)).await;
        dispatch_file_done_controls(&ev("test_fd_after", "https://f/after"));
        assert_eq!(waiter.await.unwrap().as_deref(), Some("https://f/after"));

        dispatch_file_done_controls(&ev("test_fd_early", "https://f/early"));
        assert_eq!(
            wait_for_file_done("test_fd_early", Duration::from_secs(5))
                .await
                .as_deref(),
            Some("https://f/early")
        );

        assert_eq!(
            wait_for_file_done("test_fd_none", Duration::from_millis(20)).await,
            None
        );
        assert!(!FILE_DONE_REGISTRY
            .lock()
            .unwrap()
            .waiters
            .contains_key("test_fd_none"));
    }
}
