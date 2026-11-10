//! Shared JSON wire helpers, authentication guards, and monotonic message ID generation.
//!
//! Eliminates repetitive envelope unwrapping, mixed string/integer JSON coercion,
//! `group_` prefix stripping, and `s.login` checks across protocol capability modules.

use std::sync::atomic::{AtomicU64, Ordering};

use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::clock::now_ms;
use crate::protocol::session::{LoginInfo, Session};

/// Ensure `s` carries an authenticated [`LoginInfo`], returning a reference to it.
///
/// # Errors
/// Returns an error if the session has not completed QR or password login.
pub fn require_login(s: &Session) -> Result<&LoginInfo> {
    s.login
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("chưa đăng nhập"))
}

/// Build a `serde_json::Value::Object` from `(key, value)` pairs.
pub fn obj(pairs: &[(&str, Value)]) -> Value {
    let mut m = Map::new();
    for (k, v) in pairs {
        m.insert((*k).to_string(), v.clone());
    }
    Value::Object(m)
}

/// Unwrap Zalo's single (`{"data": ...}`) or nested (`{"data": {"data": ...}}`) JSON envelope.
pub fn unwrap_data(v: &Value) -> &Value {
    v.get("data")
        .and_then(|x| x.get("data").or(Some(x)))
        .unwrap_or(v)
}

/// Return the first non-empty string (or rendered number) among `keys` in object `v`.
pub fn pick_str(v: &Value, keys: &[&str]) -> Option<String> {
    for k in keys {
        if let Some(s) = v.get(*k).and_then(Value::as_str) {
            if !s.is_empty() {
                return Some(s.to_string());
            }
        }
        if let Some(n) = v.get(*k).and_then(Value::as_i64) {
            return Some(n.to_string());
        }
        if let Some(n) = v.get(*k).and_then(Value::as_u64) {
            return Some(n.to_string());
        }
    }
    None
}

/// Extract a lossless string or integer value from an optional JSON field.
pub fn val_str(value: Option<&Value>) -> String {
    match value {
        Some(Value::String(s)) => s.clone(),
        Some(Value::Number(n)) => n.to_string(),
        _ => String::new(),
    }
}

/// Parse an `i64` whether Zalo encoded the JSON field as a number or a stringified integer.
pub fn val_i64(value: Option<&Value>) -> Option<i64> {
    value.and_then(|v| v.as_i64().or_else(|| v.as_str()?.trim().parse().ok()))
}

/// Parse a `u64` whether Zalo encoded the JSON field as a number or a stringified integer.
pub fn val_u64(value: Option<&Value>) -> Option<u64> {
    value.and_then(|v| v.as_u64().or_else(|| v.as_str()?.trim().parse().ok()))
}

/// Single-field form of [`pick_str`]: a non-empty string, else the numeric form.
///
/// Use this for typed wire structs whose fields are already selected, so that alias ordering
/// lives in the accessor instead of in a key list.
pub fn field_str(v: Option<&Value>) -> Option<String> {
    v.and_then(|v| {
        if let Some(s) = v.as_str() {
            if !s.is_empty() {
                return Some(s.to_string());
            }
        }
        v.as_i64()
            .map(|n| n.to_string())
            .or_else(|| v.as_u64().map(|n| n.to_string()))
    })
}

/// Field as a borrowed string, when the wire sent a JSON string.
///
/// Unlike [`field_str`] there is no numeric fallback and no empty-string filter, so callers that
/// must distinguish `""` from "absent" keep that distinction.
pub fn opt_str(v: Option<&Value>) -> Option<&str> {
    v.and_then(Value::as_str)
}

/// Field as a borrowed string, empty when absent or not a JSON string.
pub fn opt_text(v: Option<&Value>) -> &str {
    opt_str(v).unwrap_or("")
}

/// Field as an owned string, when the wire sent a JSON string.
pub fn opt_string(v: Option<&Value>) -> Option<String> {
    opt_str(v).map(str::to_string)
}

/// Field as a `u64`, only when the wire sent a non-negative JSON number.
///
/// Use [`val_u64`] instead when the wire may stringify the integer.
pub fn opt_u64(v: Option<&Value>) -> Option<u64> {
    v.and_then(Value::as_u64)
}

/// Field as a trimmed, non-empty `http(s)://` URL borrowed slice.
pub fn opt_url_str(v: Option<&Value>) -> Option<&str> {
    opt_str(v)
        .map(str::trim)
        .filter(|s| !s.is_empty() && (s.starts_with("http://") || s.starts_with("https://")))
}

/// Field as a trimmed, non-empty `http(s)://` URL.
pub fn opt_url(v: Option<&Value>) -> Option<String> {
    opt_url_str(v).map(str::to_string)
}

/// Failure text for a send/upload error, falling back to `unknown` when the envelope omits it.
pub fn error_text(msg: &str) -> &str {
    if msg.is_empty() {
        "unknown"
    } else {
        msg
    }
}

/// Parse a wire payload into its typed struct without ever failing.
///
/// Implemented for every `DeserializeOwned + Default` type, so each `*WireData` struct inherits a
/// single `from_value` instead of repeating the same three lines. A payload whose shape does not
/// match degrades to `Default`; each struct's docs state what that means for that payload.
pub trait WireData: Sized + Default + serde::de::DeserializeOwned {
    /// Deserialize `v`, falling back to `Default` when the shape does not match.
    fn from_value(v: &Value) -> Self {
        serde_json::from_value(v.clone()).unwrap_or_default()
    }
}

impl<T> WireData for T where T: Sized + Default + serde::de::DeserializeOwned {}

/// Extract a trimmed non-empty wire message/client identifier (`String` != `"0"` or positive `u64`).
pub fn wire_identity(value: Option<&Value>) -> Option<String> {
    match value? {
        Value::String(s) if !s.trim().is_empty() && s.trim() != "0" => Some(s.trim().to_string()),
        Value::Number(n) => n.as_u64().filter(|id| *id > 0).map(|id| id.to_string()),
        _ => None,
    }
}

/// Normalize Zalo's heterogeneous boolean/integer/string `isGroup` wire flag on a message or thread object.
pub fn is_group_flag(v: &Value) -> bool {
    v.get("isGroup")
        .and_then(|g| {
            g.as_bool()
                .or_else(|| g.as_i64().map(|n| n != 0))
                .or_else(|| {
                    g.as_str()
                        .map(|s| s == "1" || s.eq_ignore_ascii_case("true"))
                })
        })
        .unwrap_or(false)
}

/// Strip AloLite's local `"group_"` / `"g"` / `"G"` thread prefix to obtain the raw numeric `grid`.
pub fn strip_group_prefix(thread_id: &str) -> &str {
    thread_id
        .trim_start_matches("group_")
        .trim_start_matches('g')
        .trim_start_matches('G')
}

/// Reject outbound requests carrying a non-zero Zalo `error_code`.
///
/// # Errors
/// Returns an error containing the server's `error_code` and `error_message` when non-zero.
pub fn ensure_ok_code(dec: &Value, context: &str) -> Result<()> {
    let code = val_i64(dec.get("error_code")).unwrap_or(0);
    if code != 0 {
        let msg = dec
            .get("error_message")
            .and_then(Value::as_str)
            .unwrap_or("unknown");
        bail!("{context}: error_code={code} ({msg})");
    }
    Ok(())
}

/// Return trimmed non-empty `slot` (without trailing `/`), or `fallback` when empty.
pub fn domain_or<'a>(slot: &'a str, fallback: &'a str) -> &'a str {
    let trimmed = slot.trim().trim_end_matches('/');
    if trimmed.is_empty() {
        fallback.trim_end_matches('/')
    } else {
        trimmed
    }
}

/// Select the active 1-1 message-send host, falling back to [`crate::endpoints::CHAT_SEND_FALLBACK`]
/// when `s.domains.chat` points at a long-polling host or empty slot.
pub fn chat_send_host(s: &Session) -> &str {
    if s.domains.chat.contains("-msg-chat.")
        || s.domains.chat.contains("tt-chat2-wpa")
        || s.domains.chat.trim().is_empty()
    {
        crate::endpoints::CHAT_SEND_FALLBACK
    } else {
        s.domains.chat.trim_end_matches('/')
    }
}

/// Flatten a WebSocket `cmd 601`/`602`/`603` or `lp_control` payload into individual control
/// `content` items (`{"controls": [...]}` | `[{"controls": [...]}, ...]` | single control).
pub fn iter_control_items(data_node: &Value) -> Vec<&Value> {
    let mut out = Vec::new();
    if let Some(arr) = data_node.get("controls").and_then(Value::as_array) {
        for item in arr {
            out.push(item.get("content").unwrap_or(item));
        }
    } else if let Some(arr) = data_node.as_array() {
        for item in arr {
            if let Some(sub) = item.get("controls").and_then(Value::as_array) {
                for s in sub {
                    out.push(s.get("content").unwrap_or(s));
                }
            } else {
                out.push(item.get("content").unwrap_or(item));
            }
        }
    } else {
        out.push(data_node.get("content").unwrap_or(data_node));
    }
    out
}

/// Parse a control item's `"data"` field whether Zalo encoded it as JSON object or stringified JSON.
pub fn parse_control_data(raw: Option<&Value>) -> Value {
    match raw {
        Some(Value::String(s)) => serde_json::from_str(s).unwrap_or(Value::Null),
        Some(v) => v.clone(),
        None => Value::Null,
    }
}

/// Fold Vietnamese diacritics to lowercase ASCII and collapse whitespace (`simpleStripVietnamese`).
pub fn strip_vn_diacritics(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for raw in s.chars() {
        for c in raw.to_lowercase() {
            let mapped = match c {
                'à' | 'á' | 'ạ' | 'ả' | 'ã' | 'â' | 'ầ' | 'ấ' | 'ậ' | 'ẩ' | 'ẫ' | 'ă' | 'ằ'
                | 'ắ' | 'ặ' | 'ẳ' | 'ẵ' => 'a',
                'è' | 'é' | 'ẹ' | 'ẻ' | 'ẽ' | 'ê' | 'ề' | 'ế' | 'ệ' | 'ể' | 'ễ' => {
                    'e'
                }
                'ì' | 'í' | 'ị' | 'ỉ' | 'ĩ' => 'i',
                'ò' | 'ó' | 'ọ' | 'ỏ' | 'õ' | 'ô' | 'ồ' | 'ố' | 'ộ' | 'ổ' | 'ỗ' | 'ơ' | 'ờ'
                | 'ớ' | 'ợ' | 'ở' | 'ỡ' => 'o',
                'ù' | 'ú' | 'ụ' | 'ủ' | 'ũ' | 'ư' | 'ừ' | 'ứ' | 'ự' | 'ử' | 'ữ' => {
                    'u'
                }
                'ỳ' | 'ý' | 'ỵ' | 'ỷ' | 'ỹ' => 'y',
                'đ' => 'd',
                '\u{a0}' => ' ',
                other => other,
            };
            out.push(mapped);
        }
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

static LAST_CLIENT_MESSAGE_ID: AtomicU64 = AtomicU64::new(0);

/// Allocate a strictly monotonic millisecond `clientId` so two outbound messages created
/// within the same OS clock tick never collide on optimistic UI reconciliation (`INV-4`).
pub fn next_client_message_id() -> u64 {
    let now = now_ms();
    let mut current = LAST_CLIENT_MESSAGE_ID.load(Ordering::Relaxed);
    loop {
        let next = now.max(current.saturating_add(1));
        match LAST_CLIENT_MESSAGE_ID.compare_exchange_weak(
            current,
            next,
            Ordering::Relaxed,
            Ordering::Relaxed,
        ) {
            Ok(_) => return next,
            Err(actual) => current = actual,
        }
    }
}

/// Standard Zalo upstream response envelope.
///
/// Zalo HTTP API endpoints consistently return JSON responses wrapped in this structure:
///
/// ```json
/// {
///   "error_code": 0,
///   "error_message": "Successful",
///   "data": { ... }
/// }
/// ```
///
/// Canonical error code taxonomy:
/// - `0`: Success (contains typed payload in `data`).
/// - `18012`: QR code scanned, pending user confirmation on mobile device.
/// - `18022`: Invalid signature (`signkey` computation mismatch).
/// - `18060`: Invalid encryption protocol (`ParamCipher v2` mismatch).
/// - `600` / `-102`: Session expired (`zpw_sek` missing, invalid, or invalidated by another login).
/// - `114`: Invalid parameter type / malformed nested JSON.
/// - `429`: Rate limited (requires exponential backoff).
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct ZaloWireEnvelope<T> {
    pub error_code: i64,
    pub error_message: String,
    #[serde(default)]
    pub data: Option<T>,
}

/// Never-failing view of the `{ error_code, error_message }` envelope head.
///
/// # Wire Response Example
/// ```json
/// { "error_code": 18022, "error_message": "Invalid Signature" }
/// ```
///
/// # Core Transformation
/// Both fields stay [`Value`] so a single unexpected scalar type cannot fail the parse and
/// silently turn an error response into a success. Used by the transport error gate and the
/// pre-login flows before the payload is handed to a domain-specific wire struct.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ErrorEnvelopeWireData {
    pub error_code: Option<Value>,
    pub error_message: Option<Value>,
}

impl ErrorEnvelopeWireData {
    /// Parse the envelope head from a raw response `Value`; never fails.
    pub fn parse(resp: &Value) -> Self {
        serde_json::from_value(resp.clone()).unwrap_or_default()
    }

    /// Numeric error code, or `None` when the field is absent or non-numeric.
    pub fn code_opt(&self) -> Option<i64> {
        self.error_code.as_ref().and_then(Value::as_i64)
    }

    /// Numeric error code; missing or non-numeric values mean success (`0`).
    pub fn code(&self) -> i64 {
        self.error_code
            .as_ref()
            .and_then(Value::as_i64)
            .unwrap_or(0)
    }

    /// Error message rendered as text; non-string forms yield an empty string.
    pub fn message(&self) -> &str {
        self.error_message
            .as_ref()
            .and_then(Value::as_str)
            .unwrap_or("")
    }
}

/// Typed outcome for outbound message sends (`send_message`, `send_quote`, `send_link`, `send_sticker`, `forward`).
///
/// # Wire Response Examples
///
/// Success response:
/// ```json
/// {
///   "error_code": 0,
///   "error_message": "Successful",
///   "data": {
///     "msgId": "1731215400000_123456789"
///   }
/// }
/// ```
///
/// Failure response:
/// ```json
/// {
///   "error_code": 18022,
///   "error_message": "Invalid Signature",
///   "data": null
/// }
/// ```
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct SendMessageReceipt {
    pub msg_id: String,
    pub error_code: Option<i64>,
    pub error_message: Option<String>,
}

impl SendMessageReceipt {
    pub fn from_wire(resp: &Value) -> Self {
        let msg_id = resp
            .get("data")
            .and_then(|d| pick_str(d, &["msgId", "msg_id"]))
            .unwrap_or_default();
        let error_code = resp
            .get("error_code")
            .and_then(Value::as_i64)
            .filter(|c| *c != 0);
        let error_message = resp
            .get("error_message")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string);
        Self {
            msg_id,
            error_code,
            error_message,
        }
    }

    pub fn is_ok(&self) -> bool {
        self.error_code.is_none()
    }

    pub fn err_code(&self) -> Option<i64> {
        self.error_code
    }
}

/// Typed outcome for mutation/control endpoints without a message-ID payload (`react_message`, `delete_for_me`, `recall_message`, `pin_thread`, etc.).
///
/// # Wire Response Examples
///
/// Success:
/// ```json
/// {
///   "error_code": 0,
///   "error_message": "Successful"
/// }
/// ```
///
/// Failure:
/// ```json
/// {
///   "error_code": 600,
///   "error_message": "Session expired"
/// }
/// ```
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ActionAck {
    pub error_code: Option<i64>,
    pub error_message: Option<String>,
}
impl ActionAck {
    pub fn ok() -> Self {
        Self::default()
    }

    pub fn from_wire(resp: &Value) -> Self {
        let error_code = resp
            .get("error_code")
            .and_then(Value::as_i64)
            .filter(|c| *c != 0);
        let error_message = resp
            .get("error_message")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string);
        Self {
            error_code,
            error_message,
        }
    }

    pub fn is_ok(&self) -> bool {
        self.error_code.is_none()
    }

    pub fn err_code(&self) -> Option<i64> {
        self.error_code
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn unwrap_data_and_mixed_numeric_coercions_handle_zalo_wire_shapes() {
        let nested = json!({"error_code": 0, "data": {"data": {"count": "42", "ts": -5}}});
        let inner = unwrap_data(&nested);
        assert_eq!(val_u64(inner.get("count")), Some(42));
        assert_eq!(val_i64(inner.get("ts")), Some(-5));
        assert_eq!(strip_group_prefix("group_12345"), "12345");
        assert_eq!(strip_group_prefix("g999"), "999");
        assert!(is_group_flag(&json!({"isGroup": 1})));
        assert!(is_group_flag(&json!({"isGroup": true})));
        assert!(is_group_flag(&json!({"isGroup": "true"})));
        assert!(!is_group_flag(&json!({"isGroup": 0})));
    }

    #[test]
    fn request_ids_are_strictly_monotonic_within_same_clock_tick() {
        let a = next_client_message_id();
        let b = next_client_message_id();
        let c = next_client_message_id();
        assert!(a < b && b < c);
    }

    #[test]
    fn typed_receipts_normalize_nonzero_error_codes_and_numeric_msg_ids() {
        let ok_num = json!({"error_code": 0, "data": {"msgId": 987654321_u64}});
        let r = SendMessageReceipt::from_wire(&ok_num);
        assert!(r.is_ok());
        assert_eq!(r.msg_id, "987654321");
        assert_eq!(r.err_code(), None);

        let err_ack = json!({"error_code": 18022, "error_message": "Invalid Signature"});
        let ack = ActionAck::from_wire(&err_ack);
        assert!(!ack.is_ok());
        assert_eq!(ack.err_code(), Some(18022));
        assert_eq!(ack.error_message.as_deref(), Some("Invalid Signature"));
    }
}
