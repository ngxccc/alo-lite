//! HTTP long-polling (`getChatFromPolling`), `/api/preloadconvers/get-last-msgs`,
//! cloud history (`/api/cm/getrecentv2` / `getoldv2`), and wire message parser.

use std::collections::BTreeMap;
use std::time::Duration;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};

use super::attachments::{extract_link_info, is_zalo_photo_cdn_url, resolve_attachment_file_name};
use super::model::WireMessageRef;
use super::reactions::icon_to_emoji;
use super::send::format_incoming_zalo_rtf;
use super::threads::{deduplicate_threads, ChatMessage, MessageIdentityIndex, Thread};
use crate::clock::{message_time, unix_now};
use crate::protocol::session::{urlencode, Session, SessionError};
use crate::protocol::social::groups::resolve_thread_names;
use crate::protocol::wire::{
    domain_or, is_group_flag, obj, pick_str, require_login, strip_group_prefix, val_i64,
    wire_identity,
};

/// Per-queue cursor for `getChatFromPolling` (`MultiQueueId` in `sync-v2-sub-worker`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct QueueCursor {
    #[serde(rename = "lastId")]
    pub last_id: String,
    pub ids: Vec<String>,
}

/// State machine for Zalo's native HTTP long-polling message receiver (`getChatFromPolling`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PollState {
    pub last_action_id: String,
    pub last_action_id_other: String,
    pub queues: BTreeMap<String, QueueCursor>,
}

impl Default for PollState {
    fn default() -> Self {
        let mut queues = BTreeMap::new();
        for qid in ["1", "2", "3"] {
            queues.insert(
                qid.to_string(),
                QueueCursor {
                    last_id: "1".into(),
                    ids: Vec::new(),
                },
            );
        }
        Self {
            last_action_id: "1".into(),
            last_action_id_other: "1".into(),
            queues,
        }
    }
}

fn max_action_id(a: &str, b: &str) -> String {
    let na = a.parse::<u128>().unwrap_or(0);
    let nb = b.parse::<u128>().unwrap_or(0);
    if nb >= na && nb > 0 {
        b.to_string()
    } else if na > 0 {
        a.to_string()
    } else {
        "1".into()
    }
}

impl PollState {
    pub fn encode_params(&self) -> String {
        use base64::Engine;
        let q_json = serde_json::to_string(&self.queues).unwrap_or_else(|_| "{}".into());
        let b64 = base64::engine::general_purpose::STANDARD.encode(q_json.as_bytes());
        let tail = urlencode(&format!("{b64},1"));
        format!(
            "{},{},{}",
            self.last_action_id, self.last_action_id_other, tail
        )
    }

    pub fn update_from_response(&mut self, resp: &Value) {
        let mut data_node = resp.get("data").unwrap_or(resp);
        if let Some(inner) = data_node.get("data").filter(|v| v.is_object()) {
            data_node = inner;
        }
        if let Some(reset_id) = pick_str(data_node, &["resetLastActionId"]) {
            if reset_id != "0" && reset_id.parse::<u128>().unwrap_or(0) > 0 {
                self.last_action_id = reset_id.clone();
                self.last_action_id_other = "1".into();
                for q in self.queues.values_mut() {
                    q.last_id = reset_id.clone();
                    q.ids.clear();
                }
                return;
            }
        }
        if let Some(lai) = pick_str(data_node, &["lastActionId"]) {
            if lai != "0" {
                self.last_action_id = max_action_id(&self.last_action_id, &lai);
            }
        }
        if let Some(laio) = pick_str(data_node, &["lastActionIdOther"]) {
            if laio != "0" {
                self.last_action_id_other = max_action_id(&self.last_action_id_other, &laio);
            }
        }
        if let Some(qs) = data_node.get("queueStatus").and_then(Value::as_object) {
            for (qid, qval) in qs {
                let Some(entry) = self.queues.get_mut(qid) else {
                    continue;
                };
                if let Some(lid) = pick_str(qval, &["lastId"]) {
                    if lid != "0" {
                        entry.last_id = max_action_id(&entry.last_id, &lid);
                    }
                }
                if let Some(ids_arr) = qval.get("ids").and_then(Value::as_array) {
                    let max_lid = entry.last_id.parse::<u128>().unwrap_or(u128::MAX);
                    let mut new_ids: Vec<String> = ids_arr
                        .iter()
                        .filter_map(|v| match v {
                            Value::String(s) if !s.is_empty() => Some(s.clone()),
                            Value::Number(n) => Some(n.to_string()),
                            _ => None,
                        })
                        .filter(|s| s.parse::<u128>().map(|n| n <= max_lid).unwrap_or(true))
                        .collect();
                    if !new_ids.is_empty() {
                        new_ids.sort_by_key(|s| std::cmp::Reverse(s.parse::<u128>().unwrap_or(0)));
                        new_ids.dedup();
                        new_ids.truncate(10);
                        new_ids.reverse();
                        entry.ids = new_ids;
                    }
                }
            }
        }
    }
}

/// `GET /api/preloadconvers/get-last-msgs` (`cmd 10053`).
///
/// # Errors
/// Returns an error if unauthenticated or the HTTP call fails.
pub async fn get_last_msgs(s: &Session, thread_ids: &[String]) -> Result<Value> {
    require_login(s)?;
    let mut map = Map::new();
    if thread_ids.len() <= 25 {
        for id in thread_ids {
            let clean = id.trim_start_matches("group_").trim_start_matches('g');
            let key = if id.starts_with("group_") || id.starts_with('g') {
                format!("{clean}_1")
            } else {
                format!("{clean}_0")
            };
            map.insert(key, json!("1"));
        }
    }
    let map_str = serde_json::to_string(&map).unwrap_or_else(|_| "{}".into());
    let body = obj(&[
        ("threadIdLocalMsgId", json!(map_str)),
        ("imei", json!(s.imei)),
    ]);
    let params = match s.encoded_params(&body) {
        Ok(p) => format!("params={p}"),
        Err(e) => {
            crate::logger::log_error("polling::get_last_msgs::encoded_params", &e.to_string());
            Session::plain_params(&body)
        }
    };
    let url = format!(
        "{}/api/preloadconvers/get-last-msgs?{}&{params}",
        s.domains.convers_api,
        s.common_params()
    );
    let resp = s.get(&url).await?;
    Ok(s.maybe_decrypt(resp, s.session_key_bytes()).await)
}

/// Fetch cloud/group message history (`/api/cm/getrecentv2` / `/api/cm/getoldv2`, cmd `12506` / `12507`).
///
/// # Errors
/// Returns an error if unauthenticated, `is_old` lacks a valid `global_msg_id`, or the HTTP call fails.
pub async fn get_cloud_messages(
    s: &Session,
    conv_id: &str,
    global_msg_id: &str,
    is_old: bool,
    count: u32,
) -> Result<Value> {
    let login = require_login(s)?;
    if is_old && (global_msg_id.is_empty() || global_msg_id == "0") {
        anyhow::bail!("getoldv2 yêu cầu con trỏ globalMsgId hợp lệ (khác 0)");
    }
    let clean_id = strip_group_prefix(conv_id);
    let is_send2me = !login.send2me_id.is_empty() && login.send2me_id == clean_id;
    let mut map = Map::new();
    map.insert("groupId".into(), json!(clean_id));
    map.insert(
        "globalMsgId".into(),
        json!(if global_msg_id.is_empty() {
            "0"
        } else {
            global_msg_id
        }),
    );
    map.insert("count".into(), json!(count));
    map.insert("msgIds".into(), json!([]));
    map.insert("imei".into(), json!(s.imei));
    if is_send2me {
        map.insert("isOA".into(), json!(1));
    }
    map.insert("src".into(), json!(-1));
    let body = Value::Object(map);
    let params = s.encoded_params(&body)?;
    let ep = if is_old {
        "/api/cm/getoldv2"
    } else {
        "/api/cm/getrecentv2"
    };
    let host = domain_or(
        &s.domains.cloud_msg,
        crate::endpoints::GROUP_CLOUD_CANDIDATES[1],
    );
    let url = format!("{host}{ep}?{}&params={params}&nretry=0", s.common_params());
    let resp = s.get(&url).await?;
    let mut dec = s.maybe_decrypt(resp, s.session_key_bytes()).await;
    if let Some(d_str) = dec.get("data").and_then(Value::as_str) {
        let trimmed = d_str.trim();
        if (trimmed.starts_with('{') && trimmed.ends_with('}'))
            || (trimmed.starts_with('[') && trimmed.ends_with(']'))
        {
            if let Ok(parsed_data) = serde_json::from_str::<Value>(trimmed) {
                dec["data"] = parsed_data;
            }
        }
    }
    Ok(dec)
}

/// Call Zalo's native HTTP long-polling message endpoint (`getChatFromPolling`, 12s window).
///
/// # Errors
/// Returns an error if unauthenticated or the server returns a non-zero `error_code` / HTTP error.
pub async fn get_chat_from_polling(s: &Session, state: &mut PollState) -> Result<Value> {
    require_login(s)?;
    let host = domain_or(&s.domains.lp_chat, crate::endpoints::LP_CHAT_DEFAULT);
    let ts = unix_now();
    let url = format!(
        "{host}/?{}&params={}&ts={ts}",
        s.common_params(),
        state.encode_params()
    );
    let fut = s.get(&url);
    match tokio::time::timeout(Duration::from_millis(12000), fut).await {
        Ok(Ok(resp)) => {
            let dec = s.maybe_decrypt(resp, s.session_key_bytes()).await;
            if let Some(code) = dec.get("error_code").and_then(Value::as_i64) {
                if code != 0 {
                    let msg = dec
                        .get("error_message")
                        .and_then(Value::as_str)
                        .unwrap_or("unknown");
                    return Err(SessionError::new(code, msg).into());
                }
            }
            state.update_from_response(&dec);
            Ok(dec)
        }
        Ok(Err(e)) => Err(e),
        Err(_) => Ok(json!({"error_code": 0, "data": {}})),
    }
}

/// Poll `getChatFromPolling`, `get-last-msgs`, and `/api/cm/getrecentv2`, merge messages, and persist.
///
/// # Errors
/// Returns an error if unauthenticated.
pub async fn poll_new_messages(
    s: &Session,
    state: &mut PollState,
    threads: &mut Vec<Thread>,
) -> Result<()> {
    let login = require_login(s)?;
    for _ in 0..3 {
        let Ok(lp_resp) = get_chat_from_polling(s, state).await else {
            break;
        };
        merge_previews(threads, &lp_resp);
        let data_node = lp_resp.get("data").unwrap_or(&lp_resp);
        let more = data_node.get("more").and_then(Value::as_i64).unwrap_or(0);
        let has_reset = pick_str(data_node, &["resetLastActionId"])
            .map(|r| r != "0" && !r.is_empty())
            .unwrap_or(false);
        if more == 0 && !has_reset {
            break;
        }
    }
    if let Ok(preview_resp) = get_last_msgs(s, &[]).await {
        merge_previews(threads, &preview_resp);
    }
    let known_ids: Vec<String> = threads
        .iter()
        .filter(|t| !t.id.is_empty())
        .map(|t| {
            if t.is_group && !t.id.starts_with('g') && !t.id.starts_with("group_") {
                format!("g{}", t.id)
            } else {
                t.id.clone()
            }
        })
        .collect();
    for chunk in known_ids.chunks(25) {
        if let Ok(preview_resp) = get_last_msgs(s, chunk).await {
            merge_previews(threads, &preview_resp);
        }
    }
    let send2me_id = login.send2me_id.clone();
    if !send2me_id.is_empty() {
        if let Ok(cm_resp) = get_cloud_messages(s, &send2me_id, "0", false, 50).await {
            if let Some(t) = threads.iter_mut().find(|t| t.id == send2me_id) {
                merge_group_history(t, &cm_resp);
            } else {
                merge_previews(threads, &cm_resp);
            }
        }
    }
    let group_ids: Vec<String> = threads
        .iter()
        .filter(|t| t.is_group && !t.id.is_empty())
        .map(|t| t.id.clone())
        .collect();
    for gid in group_ids {
        if let Ok(cm_resp) = get_cloud_messages(s, &gid, "0", false, 50).await {
            if let Some(t) = threads.iter_mut().find(|t| t.id == gid) {
                merge_group_history(t, &cm_resp);
            }
        }
    }
    resolve_thread_names(s, threads).await;
    threads.retain(|t| !t.messages.is_empty() || !t.name.starts_with("Người dùng "));
    deduplicate_threads(threads, None);
    let db_path = crate::store::db_path_for_account(&login.uin);
    if let Ok(mut store) = crate::store::MessageStore::open(&db_path) {
        let _ = crate::store::persist_threads(&mut store, threads);
    }
    crate::store::save_threads_cache(threads);
    Ok(())
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
        Value::Object(o) => {
            let href = o
                .get("href")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|h| {
                    !h.is_empty() && (h.starts_with("http://") || h.starts_with("https://"))
                });
            if o.get("action").and_then(Value::as_str) == Some("rtf") {
                if let Some(title) = o.get("title").and_then(Value::as_str) {
                    if let Some(params_s) = o.get("params").and_then(Value::as_str) {
                        return Some(format_incoming_zalo_rtf(title, params_s));
                    }
                    return Some(title.to_string());
                }
            }
            for k in ["description", "caption", "text", "msg", "title"] {
                if let Some(Value::String(s)) = o.get(k) {
                    let t = s.trim();
                    if !t.is_empty() && t != "sendBubbleMessage" {
                        return Some(s.clone());
                    }
                }
            }
            if let Some(Value::String(s)) = o.get("fileName").or_else(|| o.get("name")) {
                if !s.trim().is_empty() {
                    return Some(format!("[Tập tin] {s}"));
                }
            }
            href.map(str::to_string)
        }
        _ => None,
    }
}

fn normalize_msg_type(it: &Value) -> String {
    let raw = it
        .get("msgType")
        .or_else(|| it.get("messageType"))
        .or_else(|| it.get("type"));
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
        Value::Object(o) => {
            let parse_u64 = |val: &Value| -> Option<u64> {
                match val {
                    Value::Number(n) => n.as_u64(),
                    Value::String(s) => s.parse::<u64>().ok(),
                    _ => None,
                }
            };
            let cat_id = o
                .get("catId")
                .or_else(|| o.get("cateId"))
                .and_then(parse_u64);
            let stk_id = o
                .get("stickerId")
                .or_else(|| o.get("id"))
                .and_then(parse_u64);
            let stk_type = o
                .get("type")
                .and_then(parse_u64)
                .map(|t| t as u32)
                .unwrap_or(7);
            if let Some(sid) = stk_id {
                if cat_id.is_some()
                    || o.contains_key("stickerId")
                    || o.contains_key("catId")
                    || o.contains_key("cateId")
                {
                    return Some((sid, cat_id.unwrap_or(0), stk_type));
                }
            }
            None
        }
        _ => None,
    }
}

fn extract_reactions(it: &Value) -> Vec<String> {
    let mut res = Vec::new();
    if let Some(r) = it.get("reacts").or_else(|| it.get("reactions")) {
        if let Some(arr) = r.get("reactions").and_then(Value::as_array) {
            for item in arr {
                if let Some(icon) = item.get("rIcon").and_then(Value::as_str) {
                    let em = icon_to_emoji(icon);
                    if !res.contains(&em.to_string()) {
                        res.push(em.to_string());
                    }
                }
            }
        }
    }
    res
}

fn parse_chat_message(it: &Value) -> Option<ChatMessage> {
    let msg_type = normalize_msg_type(it);
    if matches!(msg_type.as_str(), "chat.delete" | "delete" | "30") {
        return None;
    }
    let raw_content_val = it
        .get("content")
        .or_else(|| {
            it.get("message")
                .and_then(|m| m.get("content"))
                .or_else(|| it.get("message"))
        })
        .or_else(|| it.get("msg"));

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
            _ => {
                let obj = it.get("content").or_else(|| it.get("message"));
                if let Some(o) = obj {
                    if o.get("catId").is_some()
                        || o.get("stickerId").is_some()
                        || o.get("id").is_some()
                    {
                        "[Nhãn dán]".to_string()
                    } else if o.get("href").is_some() || o.get("thumb").is_some() {
                        "[Hình ảnh]".to_string()
                    } else if o.is_object() {
                        "[Tin nhắn]".to_string()
                    } else {
                        String::new()
                    }
                } else {
                    String::new()
                }
            }
        };
    }

    let undo_target_id = if is_undo_type {
        raw_content_val.and_then(|rv| match rv {
            Value::Object(_) => pick_str(rv, &["globalMsgId", "msgId", "cliMsgId"]),
            Value::String(s) if s.trim().starts_with('{') => {
                serde_json::from_str::<Value>(s.trim())
                    .ok()
                    .and_then(|parsed| pick_str(&parsed, &["globalMsgId", "msgId", "cliMsgId"]))
            }
            _ => None,
        })
    } else {
        None
    };

    let msg_id = undo_target_id
        .or_else(|| {
            ["msgId", "cliMsgId", "globalMsgId", "lastMsgId", "id"]
                .iter()
                .find_map(|key| wire_identity(it.get(*key)))
        })
        .unwrap_or_default();

    if content.is_empty() && msg_id.is_empty() {
        return None;
    }

    let uid_from =
        pick_str(it, &["uidFrom", "fromUid", "fromId", "senderId", "sender"]).unwrap_or_default();
    let display_sender = it
        .get("dName")
        .or_else(|| it.get("displayName"))
        .or_else(|| it.get("fromName"))
        .or_else(|| it.get("senderName"))
        .and_then(Value::as_str)
        .filter(|s| !s.trim().is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| uid_from.clone());

    let is_outgoing = uid_from == "0"
        || uid_from.is_empty()
        || it
            .get("isOutgoing")
            .and_then(Value::as_bool)
            .unwrap_or(false)
        || it
            .get("is_outgoing")
            .and_then(Value::as_i64)
            .map(|n| n == 1)
            .unwrap_or(false);

    let ts = val_i64(
        it.get("ts")
            .or_else(|| it.get("cliMsgId"))
            .or_else(|| it.get("sendTime"))
            .or_else(|| it.get("serverTime"))
            .or_else(|| it.get("time")),
    )
    .unwrap_or(0);

    let time = if ts > 0 {
        message_time(ts)
    } else {
        String::new()
    };

    let (reply_to_id, reply_to_sender, reply_to_text) =
        if let Some(q) = it.get("quote").filter(|v| v.is_object()) {
            let q_id = pick_str(q, &["cliMsgId", "globalMsgId", "msgId"]);
            let q_sender = pick_str(q, &["fromD", "dName", "ownerId", "fromUid", "uidFrom"]);
            let q_text = pick_str(q, &["msg", "message", "content"]).or_else(|| {
                q.get("msg")
                    .or_else(|| q.get("content"))
                    .and_then(extract_content_text)
            });
            (q_id, q_sender, q_text)
        } else {
            (None, None, None)
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
        client_message_id: ["cliMsgId", "clientId", "client_message_id"]
            .iter()
            .find_map(|key| wire_identity(it.get(*key)))
            .unwrap_or_default(),
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

fn extract_message_thread_id_ext(
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

fn unwrap_polling_data(resp: &Value) -> (Option<Value>, &Value) {
    let parsed_str_data: Option<Value> = resp
        .get("data")
        .and_then(Value::as_str)
        .and_then(|s| serde_json::from_str::<Value>(s.trim()).ok());
    (parsed_str_data, resp.get("data").unwrap_or(resp))
}

/// Fold a `get-last-msgs` or `getChatFromPolling` response into thread previews and messages.
pub fn merge_previews(threads: &mut Vec<Thread>, resp: &Value) {
    let mut entries: Vec<(&Value, bool)> = Vec::new();
    let (parsed_str_data, fallback_data) = unwrap_polling_data(resp);
    let mut data_node = parsed_str_data.as_ref().unwrap_or(fallback_data);
    if let Some(inner_data) = data_node
        .get("data")
        .filter(|v| v.is_object() || v.is_array())
    {
        data_node = inner_data;
    }
    let cloud_tid: Option<String> = threads
        .iter()
        .find(|t| t.name.contains("Cloud của tôi"))
        .map(|t| t.id.clone());

    if let Some(arr) = data_node.as_array() {
        for item in arr {
            entries.push((item, is_group_flag(item)));
        }
    } else if let Some(obj) = data_node.as_object() {
        let mut found_list = false;
        for (key, is_grp) in [
            ("msgs", false),
            ("groupMsgs", true),
            ("pageMsgs", false),
            ("conversations", false),
            ("threads", false),
            ("allmsgs", false),
        ] {
            if let Some(arr) = obj.get(key).and_then(Value::as_array) {
                for item in arr {
                    entries.push((item, is_grp || is_group_flag(item)));
                }
                found_list = true;
            }
        }
        if let Some(cu_arr) = obj.get("clearUnreads").and_then(Value::as_array) {
            if !cu_arr.is_empty() {
                found_list = true;
            }
        }
        if !found_list {
            for (_, val) in obj {
                if val.is_object() {
                    entries.push((val, is_group_flag(val)));
                }
            }
        }
    }

    let mut message_indices: Vec<MessageIdentityIndex> = threads
        .iter()
        .map(|thread| {
            let mut index = MessageIdentityIndex::default();
            for (row, message) in thread.messages.iter().enumerate() {
                index.insert(message, row);
            }
            index
        })
        .collect();

    for (entry, is_grp_bucket) in entries {
        let mut raw_msgs: Vec<(&Value, bool)> = Vec::new();
        if let Some(arr) = entry.get("messages").and_then(Value::as_array) {
            for m in arr {
                raw_msgs.push((m, is_grp_bucket || is_group_flag(m)));
            }
        }
        if let Some(arr) = entry.get("msgs").and_then(Value::as_array) {
            for m in arr {
                raw_msgs.push((m, is_grp_bucket || is_group_flag(m)));
            }
        }
        if let Some(arr) = entry.get("groupMsgs").and_then(Value::as_array) {
            for m in arr {
                raw_msgs.push((m, true));
            }
        }
        if let Some(obj) = entry.get("lastMsg").filter(|v| v.is_object()) {
            raw_msgs.push((obj, is_grp_bucket || is_group_flag(obj)));
        }
        if let Some(obj) = entry.get("message").filter(|v| v.is_object()) {
            raw_msgs.push((obj, is_grp_bucket || is_group_flag(obj)));
        }
        if raw_msgs.is_empty()
            && (entry.get("content").is_some()
                || entry.get("msg").is_some()
                || entry.get("msgId").is_some())
        {
            raw_msgs.push((entry, is_grp_bucket));
        }

        for (m_val, is_grp) in raw_msgs {
            let Some(parsed) = parse_chat_message(m_val) else {
                continue;
            };
            let Some((tid, thread_is_group)) =
                extract_message_thread_id_ext(entry, m_val, is_grp, cloud_tid.as_deref())
            else {
                continue;
            };

            if let Some(thread_index) = threads.iter().position(|thread| thread.id == tid) {
                let t = &mut threads[thread_index];
                let identities = &mut message_indices[thread_index];
                if thread_is_group {
                    t.is_group = true;
                }
                if let Some(row) = identities.find(&t.messages, &parsed) {
                    let existing = &mut t.messages[row];
                    identities.remove(existing, row);
                    existing.reconcile_incoming(parsed, true);
                    identities.insert(existing, row);
                } else {
                    identities.insert(&parsed, t.messages.len());
                    t.messages.push(parsed);
                }
            } else {
                let default_name = if thread_is_group {
                    format!("Nhóm {tid}")
                } else {
                    format!("Người dùng {tid}")
                };
                let mut identities = MessageIdentityIndex::default();
                identities.insert(&parsed, 0);
                message_indices.push(identities);
                threads.push(Thread {
                    id: tid,
                    name: default_name,
                    preview: parsed.text.clone(),
                    time: parsed.time.clone(),
                    is_group: thread_is_group,
                    messages: vec![parsed],
                    ..Default::default()
                });
            }
        }
    }

    for t in threads.iter_mut() {
        if !t.messages.is_empty() {
            t.messages.sort_by_key(|m| m.ts);
            if let Some(newest) = t.messages.last() {
                if !newest.text.is_empty() {
                    t.preview = newest.text.replace('\n', " ");
                    t.time = newest.time.clone();
                }
            }
        }
    }
}

/// Merge group or Cloud history response into a thread's messages, sorted by timestamp and deduplicated.
pub fn merge_group_history(thread: &mut Thread, resp: &Value) {
    let mut raw_msgs: Vec<&Value> = Vec::new();
    let (parsed_str_data, fallback_data) = unwrap_polling_data(resp);
    let mut data_node = parsed_str_data.as_ref().unwrap_or(fallback_data);
    if let Some(inner_data) = data_node
        .get("data")
        .filter(|v| v.is_object() || v.is_array())
    {
        data_node = inner_data;
    }

    if let Some(arr) = data_node.as_array() {
        raw_msgs.extend(arr);
    } else if let Some(obj) = data_node.as_object() {
        for key in ["msgs", "groupMsgs", "messages"] {
            if let Some(arr) = obj.get(key).and_then(Value::as_array) {
                raw_msgs.extend(arr);
            }
        }
    }

    let clean_thread_id = strip_group_prefix(&thread.id).to_string();
    let mut identities = MessageIdentityIndex::default();
    for (row, message) in thread.messages.iter().enumerate() {
        identities.insert(message, row);
    }

    for it in raw_msgs {
        if !clean_thread_id.is_empty() {
            if let Some(id_to) = pick_str(it, &["idTo", "toId", "grid", "groupId"]) {
                let clean_to = strip_group_prefix(&id_to);
                if !clean_to.is_empty() && clean_to != "0" && clean_to != clean_thread_id {
                    continue;
                }
            }
        }
        if let Some(m) = parse_chat_message(it) {
            if let Some(row) = identities.find(&thread.messages, &m) {
                let existing = &mut thread.messages[row];
                identities.remove(existing, row);
                existing.reconcile_incoming(m, false);
                identities.insert(existing, row);
            } else {
                identities.insert(&m, thread.messages.len());
                thread.messages.push(m);
            }
        }
    }

    thread.messages.sort_by_key(|m| m.ts);
    if let Some(newest) = thread.messages.last() {
        if !newest.text.is_empty() {
            thread.preview = newest.text.replace('\n', " ");
            thread.time = newest.time.clone();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn poll_state_encode_and_update_ignores_socket_only_queues() {
        let mut ps = PollState::default();
        let encoded = ps.encode_params();
        assert!(encoded.starts_with("1,1,"));
        assert!(encoded.ends_with("%2C1"));

        let resp = json!({
            "data": {
                "lastActionId": "8399086511016",
                "lastActionIdOther": "8399036593265",
                "queueStatus": {
                    "1": { "lastId": "8399086511016", "ids": ["8399086511016"] },
                    "15": { "lastId": "8399036593265", "ids": ["8399036593265"] }
                }
            }
        });
        ps.update_from_response(&resp);
        assert_eq!(ps.last_action_id, "8399086511016");
        assert!(!ps.queues.contains_key("15"));
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
