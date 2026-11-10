//! Wire message parser for long-polling and history payloads.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::clock::message_time;
use crate::protocol::messaging::attachments::{
    extract_link_info, is_zalo_photo_cdn_url, resolve_attachment_file_name,
};
use crate::protocol::messaging::model::WireMessageRef;
use crate::protocol::messaging::reactions::icon_to_emoji;
use crate::protocol::messaging::send::format_incoming_zalo_rtf;
use crate::protocol::messaging::threads::ChatMessage;
use crate::protocol::wire::{
    field_str, is_group_flag, opt_url_str, pick_str, val_i64, wire_identity, WireData,
};

/// Message `content` object as emitted by Zalo for text, RTF, and file cards.
///
/// # Wire Response Example
/// ```json
/// {
///   "action": "rtf",
///   "title": "Chào bạn",
///   "params": "{\"styles\":[],\"ver\":0}",
///   "href": "https://example.com",
///   "fileName": "report.pdf"
/// }
/// ```
///
/// # Core Transformation
/// Yields the display text of a message: RTF titles are re-rendered with Markdown markers,
/// caption-ish fields are preferred, and file cards become `[Tập tin] <name>`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ContentTextWireData {
    #[serde(rename = "action")]
    pub action: Option<Value>,
    #[serde(rename = "title")]
    pub title: Option<Value>,
    #[serde(rename = "params")]
    pub params: Option<Value>,
    #[serde(rename = "href")]
    pub href: Option<Value>,
    #[serde(rename = "description")]
    pub description: Option<Value>,
    #[serde(rename = "caption")]
    pub caption: Option<Value>,
    #[serde(rename = "text")]
    pub text: Option<Value>,
    #[serde(rename = "msg")]
    pub msg: Option<Value>,
    #[serde(rename = "fileName")]
    pub file_name: Option<Value>,
    #[serde(rename = "name")]
    pub name: Option<Value>,
    #[serde(rename = "thumb")]
    pub thumb_url: Option<Value>,
}

impl ContentTextWireData {
    /// `href` when it is a non-empty HTTP(S) URL.
    pub fn href(&self) -> Option<&str> {
        opt_url_str(self.href.as_ref())
    }

    /// Whether this is an RTF card (`action == "rtf"`).
    pub fn is_rtf(&self) -> bool {
        self.action.as_ref().and_then(Value::as_str) == Some("rtf")
    }

    /// RTF title, as sent.
    pub fn title(&self) -> Option<&str> {
        self.title.as_ref().and_then(Value::as_str)
    }

    /// RTF `params` JSON string (style spans).
    pub fn params(&self) -> Option<&str> {
        self.params.as_ref().and_then(Value::as_str)
    }

    /// First caption-ish string (`description` > `caption` > `text` > `msg` > `title`).
    pub fn caption_text(&self) -> Option<&str> {
        [
            &self.description,
            &self.caption,
            &self.text,
            &self.msg,
            &self.title,
        ]
        .into_iter()
        .find_map(|f| {
            let s = f.as_ref().and_then(Value::as_str)?;
            let t = s.trim();
            (!t.is_empty() && t != "sendBubbleMessage").then_some(s)
        })
    }

    /// Attachment name (`fileName` > `name`) when it is non-empty after trimming.
    pub fn attachment_name(&self) -> Option<&str> {
        self.file_name
            .as_ref()
            .or(self.name.as_ref())
            .and_then(Value::as_str)
            .filter(|s| !s.trim().is_empty())
    }
}

/// Message type field, which Zalo spells three ways and encodes as text or number.
///
/// # Wire Response Example
/// ```json
/// { "msgType": 2 }
/// ```
///
/// # Core Transformation
/// Normalizes to a `chat.*` token through the shipped numeric table; unknown numbers keep their
/// decimal form so nothing is silently dropped.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct MessageTypeWireData {
    #[serde(rename = "msgType")]
    pub msg_type: Option<Value>,
    #[serde(rename = "messageType")]
    pub message_type: Option<Value>,
    #[serde(rename = "type")]
    pub entry_type: Option<Value>,
}

impl MessageTypeWireData {
    /// Normalized message-type token.
    pub fn normalized(&self) -> String {
        let raw = self
            .msg_type
            .as_ref()
            .or(self.message_type.as_ref())
            .or(self.entry_type.as_ref());
        match raw {
            Some(Value::String(s)) => s.to_ascii_lowercase(),
            Some(Value::Number(n)) => match n.as_i64() {
                Some(1) => "chat.text".into(),
                Some(2) => "chat.photo".into(),
                Some(3) => "chat.voice".into(),
                Some(4) => "chat.video".into(),
                Some(5) => "chat.sticker".into(),
                Some(19) => "chat.file".into(),
                Some(24) => "chat.location".into(),
                Some(31) => "chat.undo".into(),
                _ => n.to_string(),
            },
            _ => String::new(),
        }
    }
}

/// Sticker card embedded in a message `content`.
///
/// # Wire Response Example
/// ```json
/// { "catId": 1234, "stickerId": 12345, "type": 7 }
/// ```
///
/// # Core Transformation
/// Returns `(sticker id, category id, sticker type)`; a bare `id` only counts as a sticker when
/// one of the category keys is present, matching the shipped client.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct StickerContentWireData {
    #[serde(rename = "catId")]
    pub cat_id: Option<Value>,
    #[serde(rename = "cateId")]
    pub cate_id: Option<Value>,
    #[serde(rename = "stickerId")]
    pub sticker_id: Option<Value>,
    #[serde(rename = "id")]
    pub id: Option<Value>,
    #[serde(rename = "type")]
    pub sticker_type: Option<Value>,
}

impl StickerContentWireData {
    fn parse_u64(val: &Value) -> Option<u64> {
        match val {
            Value::Number(n) => n.as_u64(),
            Value::String(s) => s.parse::<u64>().ok(),
            _ => None,
        }
    }

    /// `(sticker id, category id, type)` when the object really is a sticker card.
    pub fn sticker_info(&self) -> Option<(u64, u64, u32)> {
        let cat_id = self
            .cat_id
            .as_ref()
            .or(self.cate_id.as_ref())
            .and_then(Self::parse_u64);
        let stk_id = self
            .sticker_id
            .as_ref()
            .or(self.id.as_ref())
            .and_then(Self::parse_u64);
        let stk_type = self
            .sticker_type
            .as_ref()
            .and_then(Self::parse_u64)
            .map(|t| t as u32)
            .unwrap_or(7);
        let sid = stk_id?;
        if cat_id.is_some()
            || self.sticker_id.is_some()
            || self.cat_id.is_some()
            || self.cate_id.is_some()
        {
            return Some((sid, cat_id.unwrap_or(0), stk_type));
        }
        None
    }
}

/// Raw content selection of a message (`content`, else `message.content`, else `message`, else `msg`).
///
/// # Wire Response Example
/// ```json
/// { "content": "Chào bạn" }
/// ```
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct RawContentWireData {
    #[serde(rename = "content")]
    pub content: Option<Value>,
    #[serde(rename = "message")]
    pub message: Option<Value>,
    #[serde(rename = "msg")]
    pub msg: Option<Value>,
}

impl RawContentWireData {
    /// The raw content value, following the shipped fallback chain.
    pub fn raw(&self) -> Option<&Value> {
        self.content
            .as_ref()
            .or_else(|| {
                self.message
                    .as_ref()
                    .and_then(|m| m.get("content").or(Some(m)))
            })
            .or(self.msg.as_ref())
    }
}

/// Reply/quote block attached to an incoming message.
///
/// # Wire Response Example
/// ```json
/// { "cliMsgId": "8888", "fromD": "Ngọc Trần", "msg": "Tin trước đó" }
/// ```
///
/// # Core Transformation
/// Fills the `reply_to_*` trio; the quoted text falls back to `extract_content_text` when the
/// quote carries a structured `msg`/`content` object.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct QuoteWireData {
    #[serde(rename = "cliMsgId")]
    pub cli_msg_id: Option<Value>,
    #[serde(rename = "globalMsgId")]
    pub global_msg_id: Option<Value>,
    #[serde(rename = "msgId")]
    pub msg_id: Option<Value>,
    #[serde(rename = "fromD")]
    pub from_d: Option<Value>,
    #[serde(rename = "dName")]
    pub d_name: Option<Value>,
    #[serde(rename = "ownerId")]
    pub owner_id: Option<Value>,
    #[serde(rename = "fromUid")]
    pub from_uid: Option<Value>,
    #[serde(rename = "uidFrom")]
    pub uid_from: Option<Value>,
    #[serde(rename = "msg")]
    pub msg: Option<Value>,
    #[serde(rename = "message")]
    pub message: Option<Value>,
    #[serde(rename = "content")]
    pub content: Option<Value>,
}

impl QuoteWireData {
    /// Quoted message id (`cliMsgId` > `globalMsgId` > `msgId`).
    pub fn message_id(&self) -> Option<String> {
        [&self.cli_msg_id, &self.global_msg_id, &self.msg_id]
            .into_iter()
            .find_map(|f| field_str(f.as_ref()))
    }

    /// Quoted sender (`fromD` > `dName` > `ownerId` > `fromUid` > `uidFrom`).
    pub fn sender(&self) -> Option<String> {
        [
            &self.from_d,
            &self.d_name,
            &self.owner_id,
            &self.from_uid,
            &self.uid_from,
        ]
        .into_iter()
        .find_map(|f| field_str(f.as_ref()))
    }

    /// Quoted text (`msg` > `message` > `content`), else the structured content text.
    pub fn text(&self) -> Option<String> {
        field_str(self.msg.as_ref())
            .or_else(|| field_str(self.message.as_ref()))
            .or_else(|| field_str(self.content.as_ref()))
            .or_else(|| {
                self.msg
                    .as_ref()
                    .or(self.content.as_ref())
                    .and_then(extract_content_text)
            })
    }
}

/// Undo target embedded in the content of a recalled message.
///
/// # Wire Response Example
/// ```json
/// { "globalMsgId": "1731215399000_9", "msgId": "1731215399000_9" }
/// ```
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct UndoContentWireData {
    #[serde(rename = "globalMsgId")]
    pub global_msg_id: Option<Value>,
    #[serde(rename = "msgId")]
    pub msg_id: Option<Value>,
    #[serde(rename = "cliMsgId")]
    pub cli_msg_id: Option<Value>,
}

impl UndoContentWireData {
    /// Recalled message id (`globalMsgId` > `msgId` > `cliMsgId`).
    pub fn target_id(&self) -> Option<String> {
        [&self.global_msg_id, &self.msg_id, &self.cli_msg_id]
            .into_iter()
            .find_map(|f| field_str(f.as_ref()))
    }

    /// Decode the undo target from a content object or a JSON-encoded content string.
    pub fn from_undo_content(raw: &Value) -> Option<String> {
        match raw {
            Value::Object(_) => Self::from_value(raw).target_id(),
            Value::String(s) if s.trim().starts_with('{') => {
                serde_json::from_str::<Value>(s.trim())
                    .ok()
                    .and_then(|parsed| Self::from_value(&parsed).target_id())
            }
            _ => None,
        }
    }
}

/// Envelope of one incoming message in the long-poll / history stream.
///
/// # Wire Response Example
/// ```json
/// {
///   "msgId": "1731215400000_1",
///   "cliMsgId": "9999",
///   "uidFrom": "1234567890",
///   "dName": "Ngọc Trần",
///   "ts": 1731215400000,
///   "msgType": 1,
///   "content": "Chào bạn"
/// }
/// ```
///
/// # Core Transformation
/// Supplies the identity, sender, direction, timestamp, and quote fields of a
/// [`ChatMessage`]; every list keeps the alias order of the shipped client.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ChatMessageWireData {
    #[serde(rename = "msgId")]
    pub msg_id: Option<Value>,
    #[serde(rename = "cliMsgId")]
    pub cli_msg_id: Option<Value>,
    #[serde(rename = "globalMsgId")]
    pub global_msg_id: Option<Value>,
    #[serde(rename = "lastMsgId")]
    pub last_msg_id: Option<Value>,
    #[serde(rename = "id")]
    pub id: Option<Value>,
    #[serde(rename = "clientId")]
    pub client_id: Option<Value>,
    #[serde(rename = "client_message_id")]
    pub client_message_id: Option<Value>,
    #[serde(rename = "uidFrom")]
    pub uid_from: Option<Value>,
    #[serde(rename = "fromUid")]
    pub from_uid: Option<Value>,
    #[serde(rename = "fromId")]
    pub from_id: Option<Value>,
    #[serde(rename = "senderId")]
    pub sender_id: Option<Value>,
    #[serde(rename = "sender")]
    pub sender: Option<Value>,
    #[serde(rename = "dName")]
    pub d_name: Option<Value>,
    #[serde(rename = "displayName")]
    pub display_name: Option<Value>,
    #[serde(rename = "fromName")]
    pub from_name: Option<Value>,
    #[serde(rename = "senderName")]
    pub sender_name: Option<Value>,
    #[serde(rename = "isOutgoing")]
    pub is_outgoing: Option<Value>,
    #[serde(rename = "is_outgoing")]
    pub is_outgoing_snake: Option<Value>,
    #[serde(rename = "ts")]
    pub ts: Option<Value>,
    #[serde(rename = "sendTime")]
    pub send_time: Option<Value>,
    #[serde(rename = "serverTime")]
    pub server_time: Option<Value>,
    #[serde(rename = "time")]
    pub time: Option<Value>,
    #[serde(rename = "quote")]
    pub quote: Option<Value>,
}

impl ChatMessageWireData {
    /// Sender id (`uidFrom` > `fromUid` > `fromId` > `senderId` > `sender`).
    pub fn uid_from(&self) -> String {
        [
            &self.uid_from,
            &self.from_uid,
            &self.from_id,
            &self.sender_id,
            &self.sender,
        ]
        .into_iter()
        .find_map(|f| field_str(f.as_ref()))
        .unwrap_or_default()
    }

    /// Display sender name (`dName` > `displayName` > `fromName` > `senderName`).
    pub fn display_sender(&self) -> Option<String> {
        [
            &self.d_name,
            &self.display_name,
            &self.from_name,
            &self.sender_name,
        ]
        .into_iter()
        .find_map(|f| f.as_ref().and_then(Value::as_str))
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
    }

    /// Outgoing flags (`isOutgoing` boolean and the `is_outgoing` `0`/`1` form).
    pub fn is_outgoing_flag(&self) -> bool {
        self.is_outgoing
            .as_ref()
            .and_then(Value::as_bool)
            .unwrap_or(false)
            || self
                .is_outgoing_snake
                .as_ref()
                .and_then(Value::as_i64)
                .map(|n| n == 1)
                .unwrap_or(false)
    }

    /// Timestamp (`ts` > `cliMsgId` > `sendTime` > `serverTime` > `time`).
    pub fn timestamp(&self) -> i64 {
        [
            &self.ts,
            &self.cli_msg_id,
            &self.send_time,
            &self.server_time,
            &self.time,
        ]
        .into_iter()
        .find_map(|f| val_i64(f.as_ref()))
        .unwrap_or(0)
    }

    /// Server message id (`msgId` > `cliMsgId` > `globalMsgId` > `lastMsgId` > `id`).
    pub fn message_id(&self) -> String {
        [
            &self.msg_id,
            &self.cli_msg_id,
            &self.global_msg_id,
            &self.last_msg_id,
            &self.id,
        ]
        .into_iter()
        .find_map(|f| wire_identity(f.as_ref()))
        .unwrap_or_default()
    }

    /// Client message id (`cliMsgId` > `clientId` > `client_message_id`).
    pub fn client_message_id(&self) -> String {
        [&self.cli_msg_id, &self.client_id, &self.client_message_id]
            .into_iter()
            .find_map(|f| wire_identity(f.as_ref()))
            .unwrap_or_default()
    }

    /// Quote block, when the wire sends one as an object.
    pub fn quote(&self) -> Option<QuoteWireData> {
        let raw = self.quote.as_ref()?;
        raw.is_object().then(|| QuoteWireData::from_value(raw))
    }
}

fn extract_content_text(v: &Value) -> Option<String> {
    match v {
        Value::String(s) => {
            let trimmed = s.trim();
            if trimmed == "sendBubbleMessage" {
                return None;
            }
            if trimmed.starts_with('{') && trimmed.ends_with('}') {
                if let Ok(parsed) = serde_json::from_str::<Value>(trimmed) {
                    if let Some(inner) = extract_content_text(&parsed) {
                        if !inner.is_empty() {
                            return Some(inner);
                        }
                    }
                    return None;
                }
            }
            Some(s.clone())
        }
        Value::Object(_) => {
            let wire = ContentTextWireData::from_value(v);
            let href = wire.href();
            if wire.is_rtf() {
                if let Some(title) = wire.title() {
                    if let Some(params_s) = wire.params() {
                        return Some(format_incoming_zalo_rtf(title, params_s));
                    }
                    return Some(title.to_string());
                }
            }
            if let Some(s) = wire.caption_text() {
                return Some(s.to_string());
            }
            if let Some(name) = wire.attachment_name() {
                return Some(format!("[Tập tin] {name}"));
            }
            href.map(str::to_string)
        }
        _ => None,
    }
}

fn normalize_msg_type(it: &Value) -> String {
    MessageTypeWireData::from_value(it).normalized()
}

fn extract_sticker_info(v: &Value) -> Option<(u64, u64, u32)> {
    match v {
        Value::String(s) => {
            let trimmed = s.trim();
            if trimmed.starts_with('{') && trimmed.ends_with('}') {
                if let Ok(parsed) = serde_json::from_str::<Value>(trimmed) {
                    return extract_sticker_info(&parsed);
                }
            }
            None
        }
        Value::Object(_) => StickerContentWireData::from_value(v).sticker_info(),
        _ => None,
    }
}

fn extract_reactions(it: &Value) -> Vec<String> {
    let summaries = crate::protocol::messaging::reactions::parse_reactions(it);
    if !summaries.is_empty() {
        return crate::protocol::messaging::reactions::legacy_emojis(&summaries);
    }
    let mut res = Vec::new();
    let root = it.get("reacts").unwrap_or(it);
    let payload = root.get("reactions").unwrap_or(root);
    if let Some(arr) = payload.as_array() {
        for item in arr {
            if let Some(s) = item.as_str() {
                res.push(s.to_string());
            } else if let Some(icon) = item
                .get("rIcon")
                .or_else(|| item.get("icon"))
                .and_then(Value::as_str)
            {
                let em = icon_to_emoji(icon);
                if !res.contains(&em.to_string()) {
                    res.push(em.to_string());
                }
            }
        }
    }
    res
}

pub(super) fn parse_chat_message(it: &Value) -> Option<ChatMessage> {
    let msg_type = normalize_msg_type(it);
    if matches!(msg_type.as_str(), "chat.delete" | "delete" | "30") {
        return None;
    }
    let wire = ChatMessageWireData::from_value(it);
    let raw_content = RawContentWireData::from_value(it);
    let raw_content_val = raw_content.raw();

    let mut sticker_meta = raw_content_val.and_then(extract_sticker_info);
    if sticker_meta.is_none()
        && (msg_type == "chat.sticker" || msg_type == "sticker" || msg_type == "5")
    {
        sticker_meta = extract_sticker_info(it);
    }
    let (link_url, mut link_title, link_desc, mut link_thumb, mut link_src) = raw_content_val
        .map(extract_link_info)
        .unwrap_or((None, None, None, None, None));

    let mut content = raw_content_val
        .and_then(extract_content_text)
        .unwrap_or_default();

    let is_photo_msg = matches!(msg_type.as_str(), "chat.photo" | "photo" | "2")
        || link_src.as_deref() == Some("Hình ảnh Zalo")
        || link_url.as_deref().is_some_and(is_zalo_photo_cdn_url)
        || link_thumb.as_deref().is_some_and(is_zalo_photo_cdn_url);
    let is_file_msg = matches!(
        msg_type.as_str(),
        "chat.file" | "file" | "19" | "share.file"
    ) || link_src.as_deref() == Some("Tập tin Zalo")
        || link_url
            .as_deref()
            .is_some_and(|u| u.to_ascii_lowercase().contains("/renewmedia"));

    if is_photo_msg && !is_file_msg && sticker_meta.is_none() {
        link_src = Some("Hình ảnh Zalo".to_string());
        if link_thumb.is_none() {
            link_thumb = link_url.clone();
        }
        let trimmed_c = content.trim();
        let is_raw_url_content =
            trimmed_c.starts_with("http://") || trimmed_c.starts_with("https://");
        if trimmed_c.is_empty() || is_raw_url_content {
            if let Some(cap) = link_desc.as_deref().map(str::trim).filter(|d| {
                !d.is_empty() && !d.starts_with("http://") && !d.starts_with("https://")
            }) {
                content = format!("[Hình ảnh] {cap}");
            } else {
                content = "[Hình ảnh]".to_string();
            }
        }
    } else if is_file_msg && sticker_meta.is_none() {
        link_src = Some("Tập tin Zalo".to_string());
        if link_url.is_some() || link_title.is_some() {
            let resolved_name = resolve_attachment_file_name(
                link_title.as_deref(),
                &content,
                link_url.as_deref().unwrap_or_default(),
            );
            if link_title.is_none()
                || link_title
                    .as_deref()
                    .is_some_and(|t| t.eq_ignore_ascii_case("renewmedia"))
            {
                link_title = Some(resolved_name.clone());
            }
            let final_title = link_title.as_deref().unwrap_or(&resolved_name);
            let trimmed_c = content.trim();
            if !trimmed_c.starts_with("[Tập tin]") {
                content = format!("[Tập tin] {final_title}");
            }
        }
    }
    let is_undo_type = matches!(msg_type.as_str(), "chat.undo" | "undo" | "31");
    if is_undo_type {
        content = "[Tin nhắn đã thu hồi]".to_string();
        sticker_meta = None;
    } else if sticker_meta.is_some() && (content.is_empty() || content.starts_with('{')) {
        content = "[Nhãn dán]".to_string();
    }
    if sticker_meta.is_none() {
        let parse_ami = |s: &str| -> Option<(u64, u64, u32)> {
            let pos = s.find("ami_fat_cat_")?;
            let num_str = s.get(pos + 12..pos + 14)?;
            let idx = num_str.parse::<u64>().ok()?;
            (1..=15).contains(&idx).then_some((999000 + idx, 99901, 7))
        };
        if let Some(m) = parse_ami(&content)
            .or_else(|| link_desc.as_deref().and_then(parse_ami))
            .or_else(|| link_title.as_deref().and_then(parse_ami))
            .or_else(|| link_url.as_deref().and_then(parse_ami))
        {
            sticker_meta = Some(m);
            content = "[Nhãn dán]".to_string();
        }
    }

    if content.is_empty() {
        content = match msg_type.as_str() {
            "chat.sticker" | "sticker" | "5" => "[Nhãn dán]".to_string(),
            "chat.photo" | "photo" | "2" => "[Hình ảnh]".to_string(),
            "chat.voice" | "voice" | "3" => "[Tin nhắn thoại]".to_string(),
            "chat.video" | "video" | "4" => "[Video]".to_string(),
            "chat.file" | "file" | "19" => "[Tập tin]".to_string(),
            "chat.location" | "location" | "24" => "[Vị trí]".to_string(),
            "chat.contact" | "contact" => "[Danh thiếp]".to_string(),
            "chat.undo" | "undo" | "31" => "[Tin nhắn đã thu hồi]".to_string(),
            "chat.recommended" | "share.file" => "[Chia sẻ]".to_string(),
            _ => match it.get("content").or_else(|| it.get("message")) {
                Some(o) => {
                    let sticker = StickerContentWireData::from_value(o);
                    let card = ContentTextWireData::from_value(o);
                    if sticker.sticker_id.is_some()
                        || sticker.cat_id.is_some()
                        || sticker.id.is_some()
                    {
                        "[Nhãn dán]".to_string()
                    } else if card.href.is_some() || card.thumb_url.is_some() {
                        "[Hình ảnh]".to_string()
                    } else if o.is_object() {
                        "[Tin nhắn]".to_string()
                    } else {
                        String::new()
                    }
                }
                None => String::new(),
            },
        };
    }

    let undo_target_id = if is_undo_type {
        raw_content_val.and_then(UndoContentWireData::from_undo_content)
    } else {
        None
    };

    let msg_id = undo_target_id.unwrap_or_else(|| wire.message_id());

    if content.is_empty() && msg_id.is_empty() {
        return None;
    }

    let uid_from = wire.uid_from();
    let display_sender = wire.display_sender().unwrap_or_else(|| uid_from.clone());

    let is_outgoing = uid_from == "0" || uid_from.is_empty() || wire.is_outgoing_flag();

    let ts = wire.timestamp();

    let time = if ts > 0 {
        message_time(ts)
    } else {
        String::new()
    };

    let (reply_to_id, reply_to_sender, reply_to_text) = match wire.quote() {
        Some(quote) => (quote.message_id(), quote.sender(), quote.text()),
        None => (None, None, None),
    };

    if content.trim().is_empty()
        && link_url.is_none()
        && sticker_meta.is_none()
        && link_src.is_none()
    {
        return None;
    }

    Some(ChatMessage {
        id: msg_id,
        client_message_id: wire.client_message_id(),
        wire_ref: WireMessageRef::from_raw(it),
        sender: display_sender,
        text: content,
        time,
        is_outgoing,
        ts,
        sticker_id: sticker_meta.map(|(sid, _, _)| sid),
        sticker_cate_id: sticker_meta.map(|(_, cid, _)| cid),
        sticker_type: sticker_meta.map(|(_, _, st)| st),
        reply_to_id,
        reply_to_sender,
        reply_to_text,
        reactions: extract_reactions(it),
        link_url,
        link_title,
        link_desc,
        link_thumb,
        link_src,
        ..Default::default()
    })
}

pub(super) fn extract_message_thread_id_ext(
    entry: &Value,
    m_val: &Value,
    in_group_bucket: bool,
    cloud_thread_id: Option<&str>,
) -> Option<(String, bool)> {
    let is_group = in_group_bucket || is_group_flag(m_val) || is_group_flag(entry);

    for obj in [m_val, entry] {
        if let Some(grid) = pick_str(obj, &["grid", "groupId"]) {
            if !grid.is_empty() && grid != "0" {
                let clean = grid.trim_start_matches("group_").to_string();
                return Some((clean, true));
            }
        }
    }
    if is_group {
        if let Some(gid) = pick_str(m_val, &["idTo", "toid", "toId", "threadId", "conversId"])
            .or_else(|| pick_str(entry, &["idTo", "toid", "toId", "threadId", "conversId"]))
        {
            if !gid.is_empty() && gid != "0" {
                let clean = gid.trim_start_matches("group_").to_string();
                return Some((clean, true));
            }
        }
    }

    let uid_from = pick_str(m_val, &["uidFrom", "fromUid", "fromId", "senderId"])
        .or_else(|| pick_str(entry, &["uidFrom", "fromUid", "fromId", "senderId"]))
        .unwrap_or_default();
    let id_to = pick_str(m_val, &["idTo", "toid", "toId"])
        .or_else(|| pick_str(entry, &["idTo", "toid", "toId"]))
        .unwrap_or_default();

    if (id_to.is_empty() || id_to == "0") && (uid_from.is_empty() || uid_from == "0") {
        if let Some(cid) = cloud_thread_id {
            if !cid.is_empty() {
                return Some((cid.to_string(), false));
            }
        }
    }

    if !uid_from.is_empty()
        && uid_from != "0"
        && (id_to.is_empty() || id_to == "0" || Some(id_to.as_str()) == cloud_thread_id)
    {
        Some((uid_from, false))
    } else if !id_to.is_empty() && id_to != "0" {
        Some((id_to, false))
    } else if !uid_from.is_empty() && uid_from != "0" {
        Some((uid_from, false))
    } else {
        pick_str(
            m_val,
            &["threadId", "conversId", "userId", "friendId", "id"],
        )
        .or_else(|| {
            pick_str(
                entry,
                &["threadId", "conversId", "userId", "friendId", "id"],
            )
        })
        .filter(|s| s != "0" && !s.is_empty())
        .map(|s| (s, is_group))
    }
}

pub(super) fn unwrap_polling_data(resp: &Value) -> (Option<Value>, &Value) {
    let parsed_str_data: Option<Value> = resp
        .get("data")
        .and_then(Value::as_str)
        .and_then(|s| serde_json::from_str::<Value>(s.trim()).ok());
    (parsed_str_data, resp.get("data").unwrap_or(resp))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn local_time_when_parsing_millisecond_and_second_timestamps() {
        let expected = message_time(1_793_467_800_000);
        for ts in [
            json!(1_793_467_800),
            json!("1793467800"),
            json!(1_793_467_800_000_i64),
        ] {
            let message = parse_chat_message(&json!({
                "msgId": "msg-0001", "content": "hello", "ts": ts
            }))
            .unwrap();
            assert_eq!(message.time, expected);
        }
        let message = parse_chat_message(&json!({
            "msgId": "msg-0001", "content": "hello", "ts": i64::MAX
        }))
        .unwrap();
        assert_eq!(message.time, "");
    }

    #[test]
    fn parse_photo_and_share_file_messages_preserve_filenames_and_hide_cdn_text() {
        let raw_photo = json!({
            "msgId": "photo_72",
            "msgType": "chat.photo",
            "uidFrom": "1000001",
            "ts": 1700000000000_i64,
            "content": {
                "href": "https://photo-stal-zmp.zdn.vn/jpg/55022364c4202d7e7431/999888777.jpg",
                "thumb": "https://photo-stal-zmp.zdn.vn/jpg/55022364c4202d7e7431/999888777_thumb.jpg"
            }
        });
        let photo = parse_chat_message(&raw_photo).unwrap();
        assert_eq!(photo.link_src.as_deref(), Some("Hình ảnh Zalo"));
        assert_eq!(photo.text, "[Hình ảnh]");

        let raw_file = json!({
            "msgId": "file_74",
            "msgType": "share.file",
            "uidFrom": "1000002",
            "ts": 1700000001000_i64,
            "content": {
                "title": "De_cuong_mon_hoc",
                "href": "https://f16-zpc.zdn.vn/renewMedia?p=def456",
                "params": "{\"fileSize\":\"12345\",\"fileExt\":\"doc\"}"
            }
        });
        let file = parse_chat_message(&raw_file).unwrap();
        assert_eq!(file.link_src.as_deref(), Some("Tập tin Zalo"));
        assert_eq!(file.link_title.as_deref(), Some("De_cuong_mon_hoc.doc"));
    }

    #[test]
    fn parse_chat_message_sticker_and_integer_msg_types() {
        let stk = parse_chat_message(&json!({
            "msgId": "stk_1", "msgType": "chat.sticker",
            "content": "{\"catId\":1002,\"id\":2005,\"type\":1}", "uidFrom": "a", "ts": 1000_i64
        }))
        .unwrap();
        assert_eq!(stk.text, "[Nhãn dán]");
        assert_eq!(stk.sticker_id, Some(2005));

        for (code, expected) in [
            (5, "[Nhãn dán]"),
            (2, "[Hình ảnh]"),
            (3, "[Tin nhắn thoại]"),
            (19, "[Tập tin]"),
            (31, "[Tin nhắn đã thu hồi]"),
        ] {
            let m = parse_chat_message(&json!({
                "msgId": "m", "msgType": code, "content": "", "uidFrom": "b", "ts": 1000_i64
            }))
            .unwrap();
            assert_eq!(m.text, expected);
        }
    }
}
