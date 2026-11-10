//! URL detection, `/api/message/parselink` resolution, safe OpenGraph HTML fallback, and link message sending.

use crate::protocol::wire::{
    chat_send_host, next_client_message_id, obj, opt_str, require_login, strip_group_prefix,
    unwrap_data, SendMessageReceipt, WireData,
};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use super::send::{send_group_message_with_client_id, send_message_with_client_id};
use crate::protocol::registry;
use crate::protocol::session::Session;

#[derive(Debug, Clone, Default, Serialize, Deserialize, specta::Type)]
pub struct LinkMetadata {
    pub url: String,
    pub title: String,
    pub desc: String,
    pub thumb: String,
    pub src: String,
}

/// Extract the first `https://` or `http://` URL from a message text string.
pub fn extract_first_url(text: &str) -> Option<String> {
    let trimmed = text.trim();
    let pos = trimmed
        .find("https://")
        .or_else(|| trimmed.find("http://"))?;
    let after = &trimmed[pos..];
    let end = after.find(char::is_whitespace).unwrap_or(after.len());
    let url = after[..end].trim_end_matches([')', ']', ',', ';']);
    (url.len() >= 10).then(|| url.to_string())
}

fn unescape_html_entities(s: &str) -> String {
    s.replace("&amp;", "&")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&#x27;", "'")
        .replace("&apos;", "'")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&nbsp;", " ")
        .trim()
        .to_string()
}

fn extract_meta_attr(tag: &str, attr_name: &str) -> Option<String> {
    let lower = tag.to_ascii_lowercase();
    for pat in [
        format!("{attr_name}=\""),
        format!("{attr_name}='"),
        format!("{attr_name} = \""),
    ] {
        if let Some(idx) = lower.find(&pat) {
            let quote_char = pat.chars().last().unwrap_or('"');
            if let Some(rest) = tag.get(idx + pat.len()..) {
                if let Some(end) = rest.find(quote_char) {
                    if let Some(raw_val) = rest.get(..end) {
                        let val = unescape_html_entities(raw_val);
                        if !val.is_empty() {
                            return Some(val);
                        }
                    }
                }
            }
        }
    }
    None
}

fn is_binary_document_or_media_url(url: &str) -> bool {
    let without_query = url.split(['?', '#']).next().unwrap_or(url);
    let lower = without_query.to_ascii_lowercase();
    const BIN_EXTS: &[&str] = &[
        ".docx", ".doc", ".xlsx", ".xls", ".pptx", ".ppt", ".pdf", ".zip", ".rar", ".7z", ".tar",
        ".gz", ".xz", ".apk", ".ipa", ".dmg", ".exe", ".msi", ".bin", ".crypt", ".db", ".sqlite",
        ".png", ".jpg", ".jpeg", ".gif", ".webp", ".jxl", ".mp4", ".mov", ".mkv", ".mp3", ".m4a",
        ".wav", ".aac", ".ogg",
    ];
    BIN_EXTS.iter().any(|ext| lower.ends_with(ext))
        || lower.contains("trans-bin.zaloapp.com")
        || lower.contains("/dl/file/")
}

fn looks_like_binary_payload(bytes: &[u8]) -> bool {
    if bytes.starts_with(b"PK\x03\x04")
        || bytes.starts_with(b"%PDF-")
        || bytes.starts_with(b"\x89PNG")
        || bytes.starts_with(b"\xFF\xD8\xFF")
        || bytes.starts_with(b"GIF8")
        || bytes.starts_with(b"Rar!")
        || bytes.starts_with(b"7z\xBC\xAF")
        || bytes.starts_with(b"\x1F\x8B")
        || bytes.starts_with(b"\xFD7zXZ\0")
        || bytes.starts_with(b"SQLite format 3\0")
    {
        return true;
    }
    let head = &bytes[..bytes.len().min(512)];
    !head.starts_with(b"\xFF\xFE") && !head.starts_with(b"\xFE\xFF") && head.contains(&0)
}

fn parse_opengraph_html_bytes(url: &str, raw_bytes: &[u8]) -> Option<LinkMetadata> {
    if raw_bytes.is_empty() || looks_like_binary_payload(raw_bytes) {
        return None;
    }
    let take_len = raw_bytes.len().min(131_072);
    let html = String::from_utf8_lossy(&raw_bytes[..take_len]);

    let mut og_title = String::new();
    let mut og_desc = String::new();
    let mut og_thumb = String::new();
    let mut og_site = String::new();

    let mut cursor = html.as_ref();
    while let Some(start) = cursor.find('<') {
        let after = &cursor[start..];
        let Some(end) = after.find('>') else {
            break;
        };
        let tag = &after[..=end];
        cursor = &after[end + 1..];

        let tag_bytes = tag.as_bytes();
        let is_meta_tag = tag_bytes
            .get(1..5)
            .is_some_and(|b| b.eq_ignore_ascii_case(b"meta"))
            && tag_bytes
                .get(5)
                .is_some_and(|b| b.is_ascii_whitespace() || *b == b'/');
        let is_title_tag = tag_bytes
            .get(1..6)
            .is_some_and(|b| b.eq_ignore_ascii_case(b"title"))
            && tag_bytes
                .get(6)
                .is_some_and(|b| b.is_ascii_whitespace() || *b == b'>');

        if is_meta_tag {
            let key = extract_meta_attr(tag, "property")
                .or_else(|| extract_meta_attr(tag, "name"))
                .unwrap_or_default()
                .to_ascii_lowercase();
            let Some(content) = extract_meta_attr(tag, "content") else {
                continue;
            };
            match key.as_str() {
                "og:title" | "twitter:title" | "lark:url:video_title" if og_title.is_empty() => {
                    og_title = content;
                }
                "og:description" | "twitter:description" | "description" if og_desc.is_empty() => {
                    og_desc = content;
                }
                "og:image" | "twitter:image" | "lark:url:video_cover_image_url"
                    if og_thumb.is_empty() && content.starts_with("http") =>
                {
                    og_thumb = content;
                }
                "og:site_name" if og_site.is_empty() => {
                    og_site = content;
                }
                _ => {}
            }
        } else if og_title.is_empty() && is_title_tag {
            if let Some(close) = cursor.find("</") {
                if let Some(raw_slice) = cursor.get(..close) {
                    let raw_t = unescape_html_entities(raw_slice);
                    if !raw_t.is_empty() {
                        og_title = raw_t;
                    }
                }
            }
        }
    }

    let domain = url
        .split("://")
        .nth(1)
        .and_then(|r| r.split('/').next())
        .unwrap_or("Liên kết")
        .to_string();
    Some(LinkMetadata {
        url: url.to_string(),
        title: og_title,
        desc: og_desc,
        thumb: og_thumb,
        src: if !og_site.is_empty() { og_site } else { domain },
    })
}

/// Scrape OpenGraph / Twitter Card / HTML `<meta>` tags directly from an external webpage URL.
pub async fn scrape_opengraph_metadata(url: &str) -> Option<LinkMetadata> {
    if is_binary_document_or_media_url(url) {
        return None;
    }
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_millis(3800))
        .redirect(reqwest::redirect::Policy::limited(5))
        .build()
        .ok()?;
    let resp = client
        .get(url)
        .header(
            "User-Agent",
            "Mozilla/5.0 (compatible; Discordbot/2.0; +https://discordapp.com)",
        )
        .header("Accept-Language", "vi-VN,vi;q=0.9,en-US;q=0.8")
        .send()
        .await
        .ok()?;
    if !resp.status().is_success() {
        return None;
    }
    if let Some(ct) = resp
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
    {
        let ct_lower = ct.to_ascii_lowercase();
        if !ct_lower.contains("text/html") && !ct_lower.contains("application/xhtml") {
            return None;
        }
    }
    let bytes = resp.bytes().await.ok()?;
    parse_opengraph_html_bytes(url, &bytes)
}

fn is_placeholder_zalo_thumb(thumb: &str) -> bool {
    thumb.is_empty()
        || thumb.contains("feed_thumb_link")
        || thumb.contains("im_parselink")
        || thumb.contains("aHR0cHM6Ly9yZXMtemFsby56YWRuLnZuL3VwbG9hZC9tZWRpYS8yMDE5LzEwLzE1L2ZlZWRfdGh1bWJfbGlua19fMV9fMTU3MTEzMzEyMjc3OF85Mzc4MC5wbmc=")
}

/// `data` object of `/api/message/parselink` (cmd `11285`).
///
/// # Wire Response Example
/// ```json
/// {
///   "title": "Zalo",
///   "desc": "Ứng dụng nhắn tin",
///   "thumb": "https://thumb.example/1",
///   "src": "zalo.me"
/// }
/// ```
///
/// # Core Transformation
/// Fills [`LinkMetadata`]; values equal to the URL itself and placeholder thumbs are rejected by
/// the caller rather than here.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct LinkMetaWireData {
    #[serde(rename = "title")]
    pub title: Option<Value>,
    #[serde(rename = "desc")]
    pub desc: Option<Value>,
    #[serde(rename = "thumb")]
    pub thumb: Option<Value>,
    #[serde(rename = "src")]
    pub src: Option<Value>,
}

impl LinkMetaWireData {
    pub fn title(&self) -> Option<&str> {
        opt_str(self.title.as_ref()).map(str::trim)
    }

    /// Trimmed page description.
    pub fn desc(&self) -> Option<&str> {
        opt_str(self.desc.as_ref()).map(str::trim)
    }

    /// Trimmed thumbnail URL.
    pub fn thumb(&self) -> Option<&str> {
        opt_str(self.thumb.as_ref()).map(str::trim)
    }

    /// Trimmed source host.
    pub fn src(&self) -> Option<&str> {
        opt_str(self.src.as_ref()).map(str::trim)
    }
}

/// Parse rich link metadata via `/api/message/parselink` (`cmd 11285`) with OpenGraph HTML fallback.
///
/// # Errors
/// Infallible in normal flow; returns `Ok(LinkMetadata)` falling back to the URL's domain title.
pub async fn parse_link_metadata(s: &Session, raw_url: &str) -> Result<LinkMetadata> {
    let url = raw_url.trim().to_string();
    let domain = url
        .split("://")
        .nth(1)
        .and_then(|r| r.split('/').next())
        .unwrap_or("Liên kết")
        .to_string();
    let mut meta = LinkMetadata {
        url: url.clone(),
        title: String::new(),
        desc: String::new(),
        thumb: String::new(),
        src: domain.clone(),
    };

    if s.login.is_some() {
        let body = obj(&[
            ("link", json!(url)),
            ("version", json!(1)),
            ("imei", json!(s.imei)),
        ]);
        if let Ok(dec) = s.api_get(registry::message::PARSE_LINK, Some(&body)).await {
            let d = LinkMetaWireData::from_value(unwrap_data(&dec));
            if let Some(t) = d.title() {
                if !t.is_empty() && t != url {
                    meta.title = t.to_string();
                }
            }
            if let Some(ds) = d.desc() {
                if !ds.is_empty() && ds != url {
                    meta.desc = ds.to_string();
                }
            }
            if let Some(th) = d.thumb() {
                if !is_placeholder_zalo_thumb(th) {
                    meta.thumb = th.to_string();
                }
            }
            if let Some(sr) = d.src() {
                if !sr.is_empty() {
                    meta.src = sr.to_string();
                }
            }
        }
    }

    if meta.title.is_empty() || meta.thumb.is_empty() || meta.desc.is_empty() {
        if let Some(og) = scrape_opengraph_metadata(&url).await {
            if meta.title.is_empty() && !og.title.is_empty() {
                meta.title = og.title;
            }
            if meta.desc.is_empty() && !og.desc.is_empty() {
                meta.desc = og.desc;
            }
            if meta.thumb.is_empty() && !og.thumb.is_empty() {
                meta.thumb = og.thumb;
            }
            if (meta.src.is_empty() || meta.src == domain) && !og.src.is_empty() {
                meta.src = og.src;
            }
        }
    }

    if meta.title.is_empty() {
        meta.title = domain;
    }
    Ok(meta)
}

/// Send a rich link message (`cmd 11286` 1-1 / `cmd 11193` Group), falling back to plain text on failure.
///
/// # Errors
/// Returns an error if unauthenticated or both link send and text fallback fail.
pub async fn send_link_message(
    s: &Session,
    to_id: &str,
    text: &str,
    meta: &LinkMetadata,
    is_group: bool,
) -> Result<SendMessageReceipt> {
    send_link_message_with_client_id(s, to_id, text, meta, is_group, next_client_message_id()).await
}

/// Send a rich link message with a caller-supplied monotonic `clientId`.
///
/// # Errors
/// Returns an error if unauthenticated or both link send and text fallback fail.
pub async fn send_link_message_with_client_id(
    s: &Session,
    to_id: &str,
    text: &str,
    meta: &LinkMetadata,
    is_group: bool,
    now: u64,
) -> Result<SendMessageReceipt> {
    require_login(s)?;
    let msg_str = if text.trim().is_empty() {
        meta.url.as_str()
    } else {
        text.trim()
    };
    let title_str = if meta.title.trim().is_empty() {
        meta.url.as_str()
    } else {
        meta.title.trim()
    };
    let media_json = json!({
        "type": 0,
        "count": 0,
        "mediaTitle": "",
        "artist": "",
        "streamUrl": "",
        "stream_icon": ""
    })
    .to_string();

    let res: Result<Value> = async {
        if is_group {
            let clean_grid = strip_group_prefix(to_id);
            let body = obj(&[
                ("grid", json!(clean_grid)),
                ("msg", json!(msg_str)),
                ("href", json!(meta.url)),
                ("src", json!(meta.src)),
                ("title", json!(title_str)),
                ("desc", json!(meta.desc)),
                ("thumb", json!(meta.thumb)),
                ("type", json!(0)),
                ("media", json!(media_json)),
                ("clientId", json!(now)),
                ("mentionInfo", json!("[]")),
                ("language", json!("vi")),
                ("ttl", json!(0)),
                ("imei", json!(s.imei)),
            ]);
            let params = s.encoded_params(&body)?;
            let url = format!(
                "{}/api/group/sendlink?{}",
                s.domains.group,
                s.common_params()
            );
            let resp = s.post_form(&url, format!("params={params}")).await?;
            let dec = s.maybe_decrypt(resp, s.session_key_bytes()).await;
            if dec.get("error_code").and_then(Value::as_i64).unwrap_or(0) != 0 {
                anyhow::bail!("sendlink group error");
            }
            Ok(dec)
        } else {
            let host = chat_send_host(s);
            let body = obj(&[
                ("toId", json!(to_id)),
                ("msg", json!(msg_str)),
                ("href", json!(meta.url)),
                ("src", json!(meta.src)),
                ("title", json!(title_str)),
                ("desc", json!(meta.desc)),
                ("thumb", json!(meta.thumb)),
                ("type", json!(0)),
                ("media", json!(media_json)),
                ("clientId", json!(now)),
                ("mentionInfo", json!("")),
                ("language", json!("vi")),
                ("ttl", json!(0)),
                ("imei", json!(s.imei)),
            ]);
            let params = s.encoded_params(&body)?;
            let url = format!("{host}/api/message/link?{}", s.common_params());
            let resp = s.post_form(&url, format!("params={params}")).await?;
            let dec = s.maybe_decrypt(resp, s.session_key_bytes()).await;
            if dec.get("error_code").and_then(Value::as_i64).unwrap_or(0) != 0 {
                anyhow::bail!("sendlink error");
            }
            Ok(dec)
        }
    }
    .await;

    match res {
        Ok(v) => Ok(SendMessageReceipt::from_wire(&v)),
        Err(_) => {
            if is_group {
                send_group_message_with_client_id(s, to_id, msg_str, now).await
            } else {
                send_message_with_client_id(s, to_id, msg_str, now).await
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_opengraph_html_bytes_handles_docx_zip_and_multibyte_tags_without_panic() {
        let malformed_html = b"<\xff\xfe[\n\xff!0p>_rels/.rels><\xff\xfe!title>Xin ch\xc3\xa0o</title><meta property=\"og:title\" content=\"B\xc3\xa1o c\xc3\xa1o Q4\">";
        let parsed = parse_opengraph_html_bytes("https://example.com/view", malformed_html)
            .expect("must parse without char-boundary panic");
        assert_eq!(parsed.title, "Báo cáo Q4");

        let docx_zip = b"PK\x03\x04\x14\x00\x06\x00<\xff\xfe[\n_rels/.rels>";
        assert!(parse_opengraph_html_bytes("https://f.zalo.me/doc", docx_zip).is_none());
        assert!(is_binary_document_or_media_url(
            "https://f10-zpc.zdn.vn/dl/file/bao_cao.docx?token=123"
        ));
    }
}
