//! Text/RTF messaging, quoted replies, forwarding, delete-for-me, recall, and typing indicators.

use crate::protocol::wire::{
    chat_send_host, error_text, next_client_message_id, obj, require_login, strip_group_prefix,
    ActionAck, SendMessageReceipt, WireData,
};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use super::stickers::send_sticker_with_client_id;
use super::threads::ChatMessage;
use crate::clock::now_ms;
use crate::protocol::session::{Session, SessionError};

/// Convert inline Markdown (`**bold**`, `__underline__`, `~~strikethrough~~`, `*italic*`) and
/// line-prefix styles (`## `, `- `, `1. `) into Zalo PC's native `(clean_text, Option<textProperties>)`.
pub fn extract_zalo_rtf_text_properties(raw: &str) -> (String, Option<String>) {
    let mut out_text = String::with_capacity(raw.len());
    let mut styles: Vec<Value> = Vec::new();

    for (line_idx, raw_line) in raw.split('\n').enumerate() {
        if line_idx > 0 {
            out_text.push('\n');
        }
        let mut line = raw_line;
        let mut line_tags: Vec<&'static str> = Vec::new();
        if let Some(rest) = line.strip_prefix("### ") {
            line = rest;
            line_tags.push("b");
            line_tags.push("f_16");
        } else if let Some(rest) = line.strip_prefix("## ") {
            line = rest;
            line_tags.push("b");
            line_tags.push("f_18");
        } else if let Some(rest) = line.strip_prefix("# ") {
            line = rest;
            line_tags.push("b");
            line_tags.push("f_18");
        } else if let Some(rest) = line.strip_prefix("- ").or_else(|| line.strip_prefix("* ")) {
            line = rest;
            line_tags.push("lst_1");
        } else if let Some(rest) = line.strip_prefix("1. ") {
            line = rest;
            line_tags.push("lst_2");
        }

        let line_start_u16 = out_text.encode_utf16().count();
        let mut rest = line;
        while !rest.is_empty() {
            let mut matched_span = false;
            for (delim, st_tag) in [("`", ""), ("**", "b"), ("__", "u"), ("~~", "s"), ("*", "i")] {
                if rest.starts_with(delim) {
                    let dlen = delim.len();
                    let after = &rest[dlen..];
                    if let Some(end_pos) = after.find(delim) {
                        let inner = &after[..end_pos];
                        if !inner.is_empty() && !inner.contains('\n') {
                            let span_start = out_text.encode_utf16().count();
                            let span_len = inner.encode_utf16().count();
                            out_text.push_str(inner);
                            if !st_tag.is_empty() && span_len > 0 {
                                styles.push(json!({
                                    "start": span_start,
                                    "len": span_len,
                                    "st": st_tag,
                                }));
                            }
                            rest = &after[end_pos + dlen..];
                            matched_span = true;
                            break;
                        }
                    }
                }
            }
            if matched_span {
                continue;
            }
            let mut chars = rest.chars();
            if let Some(ch) = chars.next() {
                out_text.push(ch);
                rest = chars.as_str();
            } else {
                break;
            }
        }
        let line_len_u16 = out_text
            .encode_utf16()
            .count()
            .saturating_sub(line_start_u16);
        if !line_tags.is_empty() && line_len_u16 > 0 {
            styles.push(json!({
                "start": line_start_u16,
                "len": line_len_u16,
                "st": line_tags.join(","),
            }));
        }
    }

    if styles.is_empty() {
        (raw.to_string(), None)
    } else {
        let prop_str = json!({
            "styles": styles,
            "ver": 0,
        })
        .to_string();
        (out_text, Some(prop_str))
    }
}

/// Incoming `rtf` rich-text payload, sent as a JSON-encoded `params` string.
///
/// # Wire Response Example
/// ```json
/// { "styles": [ { "start": 0, "len": 4, "st": "b" } ], "ver": 0 }
/// ```
///
/// # Core Transformation
/// Style spans are measured in UTF-16 code units, matching Zalo's `title` indexing.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct RtfParamsWireData {
    #[serde(rename = "styles")]
    pub styles: Option<Vec<Value>>,
}

impl RtfParamsWireData {
    /// Decode the JSON-encoded `params` string; `None` when it is not an object.
    pub fn parse(params_str: &str) -> Option<Self> {
        serde_json::from_str(params_str.trim()).ok()
    }

    /// Raw style spans in wire order.
    pub fn styles(&self) -> &[Value] {
        self.styles.as_deref().unwrap_or_default()
    }
}

/// One `styles` span of an incoming `rtf` payload.
///
/// # Wire Response Example
/// ```json
/// { "start": 0, "len": 4, "st": "b" }
/// ```
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct RtfStyleWireData {
    #[serde(rename = "start")]
    pub start: Option<Value>,
    #[serde(rename = "len")]
    pub len: Option<Value>,
    #[serde(rename = "st")]
    pub style: Option<Value>,
}

impl RtfStyleWireData {
    /// `(start, end, style)` when all three fields are present and well-typed.
    pub fn span(&self) -> Option<(usize, usize, String)> {
        let start = self.start.as_ref()?.as_u64()? as usize;
        let len = self.len.as_ref()?.as_u64()? as usize;
        let style = self.style.as_ref()?.as_str()?.to_string();
        Some((start, len, style))
    }
}

/// Reconstruct Markdown markers from Zalo's incoming `rtf` rich-text payload.
pub(super) fn format_incoming_zalo_rtf(title: &str, params_str: &str) -> String {
    let Some(params) = RtfParamsWireData::parse(params_str) else {
        return title.to_string();
    };
    let styles = params.styles();
    if styles.is_empty() {
        return title.to_string();
    }
    let u16_units: Vec<u16> = title.encode_utf16().collect();
    let mut spans: Vec<(usize, usize, String)> = styles
        .iter()
        .filter_map(|s| {
            let (start, len, st) = RtfStyleWireData::from_value(s).span()?;
            (len > 0 && start + len <= u16_units.len()).then_some((start, start + len, st))
        })
        .collect();
    if spans.is_empty() {
        return title.to_string();
    }
    spans.sort_by_key(|x| x.0);
    let mut out = String::with_capacity(title.len() + spans.len() * 4);
    let mut cursor = 0usize;
    for (s, e, st) in spans {
        if s < cursor {
            continue;
        }
        if s > cursor {
            out.push_str(&String::from_utf16_lossy(&u16_units[cursor..s]));
        }
        let seg = String::from_utf16_lossy(&u16_units[s..e]);
        let tags: Vec<&str> = st.split(',').collect();
        let is_bold = tags.contains(&"b") || tags.iter().any(|t| t.starts_with("f_"));
        let is_italic = tags.contains(&"i");
        let is_under = tags.contains(&"u");
        let is_strike = tags.contains(&"s");
        let mut wrapped = seg;
        if is_italic && !is_bold {
            wrapped = format!("*{wrapped}*");
        }
        if is_bold {
            wrapped = format!("**{wrapped}**");
        }
        if is_under {
            wrapped = format!("__{wrapped}__");
        }
        if is_strike {
            wrapped = format!("~~{wrapped}~~");
        }
        out.push_str(&wrapped);
        cursor = e;
    }
    if cursor < u16_units.len() {
        out.push_str(&String::from_utf16_lossy(&u16_units[cursor..]));
    }
    out
}

fn check_session_error(resp: &Value) -> Result<()> {
    let env = crate::protocol::wire::ErrorEnvelopeWireData::parse(resp);
    if let Some(code) = env.code_opt() {
        if code != 0 {
            return Err(SessionError::new(code, error_text(env.message())).into());
        }
    }
    Ok(())
}

/// Send a 1-1 text message (`POST {domains.chat}/api/message/sms`).
///
/// # Errors
/// Returns an error if unauthenticated, encryption fails, or the server rejects the message.
pub async fn send_message(s: &Session, to_id: &str, text: &str) -> Result<SendMessageReceipt> {
    send_message_with_client_id(s, to_id, text, next_client_message_id()).await
}

/// Send a 1-1 text message preserving caller-supplied `clientId` (`INV-4`).
///
/// # Errors
/// Returns an error if unauthenticated, encryption fails, or the server rejects the message.
pub async fn send_message_with_client_id(
    s: &Session,
    to_id: &str,
    text: &str,
    now: u64,
) -> Result<SendMessageReceipt> {
    require_login(s)?;
    let (clean_text, text_props_opt) = extract_zalo_rtf_text_properties(text);
    let mut body = obj(&[
        ("toid", json!(to_id)),
        ("message", json!(clean_text)),
        ("clientId", json!(now)),
        ("imei", json!(s.imei)),
    ]);
    if let (Some(tp), Some(map)) = (text_props_opt, body.as_object_mut()) {
        map.insert("textProperties".into(), json!(tp));
    }
    let params = s.encoded_params(&body)?;
    let url = format!(
        "{}/api/message/sms?{}",
        chat_send_host(s),
        s.common_params()
    );
    let resp = s.post_form(&url, format!("params={params}")).await?;
    let resp = s.maybe_decrypt(resp, s.session_key_bytes()).await;
    check_session_error(&resp)?;
    Ok(SendMessageReceipt::from_wire(&resp))
}

/// Send a group text message (`POST {domains.group}/api/group/sendmsg`).
///
/// # Errors
/// Returns an error if unauthenticated, encryption fails, or the server rejects the message.
pub async fn send_group_message(s: &Session, grid: &str, text: &str) -> Result<SendMessageReceipt> {
    send_group_message_with_client_id(s, grid, text, next_client_message_id()).await
}

/// Send a group text message preserving caller-supplied `clientId` (`INV-4`).
///
/// # Errors
/// Returns an error if unauthenticated, encryption fails, or the server rejects the message.
pub async fn send_group_message_with_client_id(
    s: &Session,
    grid: &str,
    text: &str,
    now: u64,
) -> Result<SendMessageReceipt> {
    require_login(s)?;
    let clean_grid = strip_group_prefix(grid);
    let (clean_text, text_props_opt) = extract_zalo_rtf_text_properties(text);
    let mut body = obj(&[
        ("grid", json!(clean_grid)),
        ("message", json!(clean_text)),
        ("clientId", json!(now)),
        ("imei", json!(s.imei)),
        ("visibility", json!(0)),
    ]);
    if let (Some(tp), Some(map)) = (text_props_opt, body.as_object_mut()) {
        map.insert("textProperties".into(), json!(tp));
    }
    let params = s.encoded_params(&body)?;
    let url = format!(
        "{}/api/group/sendmsg?{}",
        s.domains.group,
        s.common_params()
    );
    let resp = s.post_form(&url, format!("params={params}")).await?;
    let resp = s.maybe_decrypt(resp, s.session_key_bytes()).await;
    check_session_error(&resp)?;
    Ok(SendMessageReceipt::from_wire(&resp))
}

/// Send a quote/reply message (`cmd 11771` 1-1 / `cmd 11197` Group), falling back to plain text.
///
/// # Errors
/// Returns an error if unauthenticated or both quote send and text fallback fail.
pub async fn send_quote_message(
    s: &Session,
    to_id: &str,
    text: &str,
    quoted: &ChatMessage,
    is_group: bool,
) -> Result<SendMessageReceipt> {
    send_quote_message_with_client_id(s, to_id, text, quoted, is_group, next_client_message_id())
        .await
}

/// Send a quote/reply message preserving caller-supplied `clientId`.
///
/// # Errors
/// Returns an error if unauthenticated or both quote send and text fallback fail.
pub async fn send_quote_message_with_client_id(
    s: &Session,
    to_id: &str,
    text: &str,
    quoted: &ChatMessage,
    is_group: bool,
    now: u64,
) -> Result<SendMessageReceipt> {
    let login = require_login(s)?;
    let my_uid = login.uin.as_str();
    let qmsg_owner = if quoted.is_outgoing {
        my_uid
    } else if !quoted.sender.is_empty() && quoted.sender.chars().all(|c| c.is_ascii_digit()) {
        quoted.sender.as_str()
    } else if !is_group {
        to_id
    } else {
        "0"
    };

    let qmsg_id_num = quoted.id.parse::<u64>().unwrap_or(now.saturating_sub(1000));
    let qmsg_ts = if quoted.ts > 0 {
        quoted.ts as u64
    } else {
        now.saturating_sub(1000)
    };
    let qmsg_cli_id = qmsg_ts;

    let quote_res: Result<Value> = async {
        if quoted.id.starts_with("out_") || quoted.id.starts_with("local_") || quoted.id.is_empty()
        {
            anyhow::bail!("local quote id");
        }
        let (clean_text, text_props_opt) = extract_zalo_rtf_text_properties(text);
        if is_group {
            let clean_grid = strip_group_prefix(to_id);
            let mut body = obj(&[
                ("grid", json!(clean_grid)),
                ("message", json!(clean_text)),
                ("clientId", json!(now)),
                ("language", json!("vi")),
                ("qmsgOwner", json!(qmsg_owner)),
                ("qmsgId", json!(qmsg_id_num)),
                ("qmsgCliId", json!(qmsg_cli_id)),
                ("qmsgType", json!(1)),
                ("qmsg", json!(quoted.text)),
                ("qmsgTs", json!(qmsg_ts)),
                ("visibility", json!(0)),
                ("ttl", json!(0)),
                ("imei", json!(s.imei)),
                ("zsource", json!(101)),
            ]);
            if let (Some(tp), Some(map)) = (text_props_opt, body.as_object_mut()) {
                map.insert("textProperties".into(), json!(tp));
            }
            let params = s.encoded_params(&body)?;
            let url = format!("{}/api/group/quote?{}", s.domains.group, s.common_params());
            let resp = s.post_form(&url, format!("params={params}")).await?;
            let resp = s.maybe_decrypt(resp, s.session_key_bytes()).await;
            if crate::protocol::wire::ErrorEnvelopeWireData::parse(&resp).code() != 0 {
                anyhow::bail!("quote error");
            }
            Ok(resp)
        } else {
            let mut body = obj(&[
                ("toid", json!(to_id)),
                ("message", json!(clean_text)),
                ("clientId", json!(now)),
                ("qmsgOwner", json!(qmsg_owner)),
                ("qmsgId", json!(qmsg_id_num)),
                ("qmsgCliId", json!(qmsg_cli_id)),
                ("qmsgType", json!(1)),
                ("qmsgTs", json!(qmsg_ts)),
                ("qmsg", json!(quoted.text)),
                ("imei", json!(s.imei)),
                ("ttl", json!(0)),
                ("zsource", json!(101)),
            ]);
            if let (Some(tp), Some(map)) = (text_props_opt, body.as_object_mut()) {
                map.insert("textProperties".into(), json!(tp));
            }
            let params = s.encoded_params(&body)?;
            let url = format!(
                "{}/api/message/quote?{}",
                chat_send_host(s),
                s.common_params()
            );
            let resp = s.post_form(&url, format!("params={params}")).await?;
            let resp = s.maybe_decrypt(resp, s.session_key_bytes()).await;
            if crate::protocol::wire::ErrorEnvelopeWireData::parse(&resp).code() != 0 {
                anyhow::bail!("quote error");
            }
            Ok(resp)
        }
    }
    .await;

    match quote_res {
        Ok(r) => Ok(SendMessageReceipt::from_wire(&r)),
        Err(_) => {
            if is_group {
                send_group_message_with_client_id(s, to_id, text, now).await
            } else {
                send_message_with_client_id(s, to_id, text, now).await
            }
        }
    }
}

/// Forward a text message or sticker to another conversation (`forwardMultiMessage`).
///
/// # Errors
/// Returns an error if unauthenticated or the send fails.
pub async fn forward_message(
    s: &Session,
    target_id: &str,
    target_is_group: bool,
    msg: &ChatMessage,
) -> Result<SendMessageReceipt> {
    forward_message_with_client_id(s, target_id, target_is_group, msg, next_client_message_id())
        .await
}

/// Forward a text message or sticker preserving caller-supplied `clientId`.
///
/// # Errors
/// Returns an error if unauthenticated or the send fails.
pub async fn forward_message_with_client_id(
    s: &Session,
    target_id: &str,
    target_is_group: bool,
    msg: &ChatMessage,
    now: u64,
) -> Result<SendMessageReceipt> {
    if let Some(sid) = msg.sticker_id.filter(|&id| id > 0) {
        let cid = msg.sticker_cate_id.unwrap_or(0);
        let stype = msg.sticker_type.unwrap_or(7);
        return send_sticker_with_client_id(s, target_id, sid, cid, stype, target_is_group, now)
            .await;
    }
    if target_is_group {
        send_group_message_with_client_id(s, target_id, &msg.text, now).await
    } else {
        send_message_with_client_id(s, target_id, &msg.text, now).await
    }
}

/// Delete a message for me (`deleteOneOneMessageV2` / `deleteGroupMessageV2`, cmd `11779` / `11834`).
///
/// # Errors
/// Returns an error if unauthenticated, encryption fails, or the request fails.
pub async fn delete_message_for_me(
    s: &Session,
    to_id: &str,
    is_group: bool,
    msg: &ChatMessage,
) -> Result<ActionAck> {
    let login = require_login(s)?;
    let now = now_ms();
    let owner_id = if msg.is_outgoing {
        login.uin.as_str()
    } else if !is_group {
        to_id
    } else {
        &msg.sender
    };
    let cli_id = if msg.ts > 0 {
        msg.ts.to_string()
    } else {
        now.to_string()
    };

    let msgs_arr = json!([{
        "cliMsgId": cli_id,
        "globalMsgId": msg.id,
        "ownerId": owner_id,
        "destId": to_id
    }]);

    if is_group {
        let clean_grid = strip_group_prefix(to_id);
        let body = obj(&[
            ("grid", json!(clean_grid)),
            ("cliMsgId", json!(now)),
            ("msgs", msgs_arr),
            ("onlyMe", json!(1)),
        ]);
        let params = s.encoded_params(&body)?;
        let url = format!(
            "{}/api/group/deletemsg?{}",
            s.domains.group,
            s.common_params()
        );
        let resp = s.post_form(&url, format!("params={params}")).await?;
        let dec = s.maybe_decrypt(resp, s.session_key_bytes()).await;
        Ok(ActionAck::from_wire(&dec))
    } else {
        let body = obj(&[
            ("toid", json!(to_id)),
            ("cliMsgId", json!(now)),
            ("msgs", msgs_arr),
            ("onlyMe", json!(1)),
            ("imei", json!(s.imei)),
        ]);
        let params = s.encoded_params(&body)?;
        let url = format!(
            "{}/api/message/delete?{}",
            chat_send_host(s),
            s.common_params()
        );
        let resp = s.post_form(&url, format!("params={params}")).await?;
        let dec = s.maybe_decrypt(resp, s.session_key_bytes()).await;
        Ok(ActionAck::from_wire(&dec))
    }
}

/// Recall / undo an outgoing message for everyone (`undoMessage`, cmd `11281` / `11192`).
///
/// # Errors
/// Returns an error if unauthenticated, encryption fails, or the request fails.
pub async fn recall_message(
    s: &Session,
    to_id: &str,
    is_group: bool,
    msg: &ChatMessage,
) -> Result<ActionAck> {
    require_login(s)?;
    let now = now_ms();
    let cli_id = if msg.ts > 0 {
        msg.ts.to_string()
    } else {
        now.to_string()
    };

    if is_group {
        let clean_grid = strip_group_prefix(to_id);
        let body = obj(&[
            ("msgId", json!(msg.id)),
            ("grid", json!(clean_grid)),
            ("clientId", json!(now)),
            ("visibility", json!(0)),
            ("cliMsgIdUndo", json!(cli_id)),
            ("imei", json!(s.imei)),
        ]);
        let params = s.encoded_params(&body)?;
        let url = format!(
            "{}/api/group/undomsg?{}",
            s.domains.group,
            s.common_params()
        );
        let resp = s.post_form(&url, format!("params={params}")).await?;
        let dec = s.maybe_decrypt(resp, s.session_key_bytes()).await;
        Ok(ActionAck::from_wire(&dec))
    } else {
        let body = obj(&[
            ("msgId", json!(msg.id)),
            ("toid", json!(to_id)),
            ("clientId", json!(now)),
            ("cliMsgIdUndo", json!(cli_id)),
        ]);
        let params = s.encoded_params(&body)?;
        let url = format!(
            "{}/api/message/undo?{}",
            chat_send_host(s),
            s.common_params()
        );
        let resp = s.post_form(&url, format!("params={params}")).await?;
        let dec = s.maybe_decrypt(resp, s.session_key_bytes()).await;
        Ok(ActionAck::from_wire(&dec))
    }
}

/// Send a real-time "is typing" indicator unless blocked by Privacy Guard (`cmd 11144` / `11196`).
///
/// # Errors
/// Returns an error if unauthenticated or encryption fails.
pub async fn send_typing_indicator(
    s: &Session,
    thread_id: &str,
    is_group: bool,
) -> Result<ActionAck> {
    require_login(s)?;
    if crate::session::privacy_blocker_state().0 {
        return Ok(ActionAck::ok());
    }
    let clean = strip_group_prefix(thread_id).trim_start_matches('u').trim();
    if clean.is_empty() {
        return Ok(ActionAck::ok());
    }
    let (endpoint, path_and_q, body) = if is_group {
        (
            "group",
            format!("/api/group/typing?{}", s.common_params()),
            json!({
                "grid": clean,
                "imei": s.imei,
            }),
        )
    } else {
        (
            "chat",
            format!("/api/message/typing?{}", s.common_params()),
            json!({
                "toid": clean,
                "destType": 0,
                "imei": s.imei,
            }),
        )
    };
    let params = s.encoded_params(&body)?;
    let resp = s
        .post_endpoint_form(endpoint, &path_and_q, format!("params={params}"))
        .await?;
    let dec = s.maybe_decrypt(resp, s.session_key_bytes()).await;
    Ok(ActionAck::from_wire(&dec))
}

#[cfg(test)]
mod tests {
    use super::super::links::{send_link_message_with_client_id, LinkMetadata};
    use super::*;

    #[test]
    fn request_ids_are_unique_when_sends_share_a_clock_tick() {
        let ids: Vec<_> = (0..10_000).map(|_| next_client_message_id()).collect();
        assert!(ids.windows(2).all(|ids| ids[0] < ids[1]));
    }

    #[tokio::test]
    async fn request_client_id_survives_send_variants_and_fallbacks() {
        use std::io::{Read, Write};
        use std::net::TcpListener;
        use std::time::Duration;

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let host = format!("http://{}", listener.local_addr().unwrap());
        let fixture = std::thread::spawn(move || {
            let mut requests = Vec::new();
            for _ in 0..9 {
                let (mut stream, _) = listener.accept().unwrap();
                stream
                    .set_read_timeout(Some(Duration::from_secs(3)))
                    .unwrap();
                let mut headers = Vec::new();
                while !headers.ends_with(b"\r\n\r\n") {
                    let mut byte = [0];
                    stream.read_exact(&mut byte).unwrap();
                    headers.push(byte[0]);
                    assert!(headers.len() < 16_384);
                }
                let headers = String::from_utf8(headers).unwrap();
                let size: usize = headers
                    .lines()
                    .find_map(|line| {
                        let (name, value) = line.split_once(':')?;
                        name.eq_ignore_ascii_case("content-length")
                            .then(|| value.trim().parse().unwrap())
                    })
                    .unwrap();
                let mut body = vec![0; size];
                stream.read_exact(&mut body).unwrap();
                let query = reqwest::Url::parse(&format!(
                    "http://fixture.invalid/?{}",
                    String::from_utf8(body).unwrap()
                ))
                .unwrap();
                let encrypted = query
                    .query_pairs()
                    .find(|(key, _)| key == "params")
                    .unwrap()
                    .1
                    .into_owned();
                let body: Value =
                    serde_json::from_str(&crate::crypto::decode_aes(&[7; 32], &encrypted).unwrap())
                        .unwrap();
                let path = headers
                    .lines()
                    .next()
                    .unwrap()
                    .split_whitespace()
                    .nth(1)
                    .unwrap()
                    .to_string();
                let fail = path.contains("/link?") || path.contains("/quote?");
                requests.push((path, body));
                let response = if fail {
                    r#"{"error_code":114}"#
                } else {
                    r#"{"error_code":0,"data":{"msgId":"server-a"}}"#
                };
                write!(
                    stream,
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\nContent-Length: {}\r\n\r\n{}",
                    response.len(),
                    response
                )
                .unwrap();
            }
            requests
        });
        let mut session = Session::new(false, false, false).unwrap();
        session.login = Some(crate::session::LoginInfo::default());
        session.secret_key = Some(vec![7; 32]);
        session.domains.chat = host.clone();
        session.domains.group = host;
        let client_id = 1_793_425_680_123;
        send_message_with_client_id(&session, "contact-1", "Text", client_id)
            .await
            .unwrap();
        send_group_message_with_client_id(&session, "group-1", "Text", client_id)
            .await
            .unwrap();
        send_link_message_with_client_id(
            &session,
            "contact-1",
            "Text",
            &LinkMetadata {
                url: "https://example.invalid/link".into(),
                title: "Title".into(),
                desc: String::new(),
                thumb: String::new(),
                src: String::new(),
            },
            false,
            client_id,
        )
        .await
        .unwrap();
        send_sticker_with_client_id(&session, "contact-1", 1, 2, 7, false, client_id)
            .await
            .unwrap();
        send_quote_message_with_client_id(
            &session,
            "contact-1",
            "Text",
            &ChatMessage {
                id: "server-quote".into(),
                ..Default::default()
            },
            false,
            client_id,
        )
        .await
        .unwrap();
        forward_message_with_client_id(
            &session,
            "contact-1",
            false,
            &ChatMessage {
                text: "Forward".into(),
                ..Default::default()
            },
            client_id,
        )
        .await
        .unwrap();
        forward_message_with_client_id(
            &session,
            "contact-1",
            false,
            &ChatMessage {
                sticker_id: Some(1),
                ..Default::default()
            },
            client_id,
        )
        .await
        .unwrap();
        let requests = fixture.join().unwrap();
        assert_eq!(requests.len(), 9);
        for (_, body) in requests {
            assert_eq!(body["clientId"], json!(client_id));
        }
    }
}
