//! Network requests for long-polling, preview batches, and cloud history.

use std::time::Duration;

use crate::protocol::wire::{domain_or, error_text, obj, require_login, strip_group_prefix};
use anyhow::Result;
use serde_json::{json, Map, Value};

use super::batch::{CloudMessagesPage, LastMsgsBatch, PollBatch};
use super::state::PollState;
use crate::clock::unix_now;
use crate::protocol::messaging::threads::{deduplicate_threads, Thread};
use crate::protocol::session::{Session, SessionError};
use crate::protocol::social::groups::resolve_thread_names;

/// `GET /api/preloadconvers/get-last-msgs` (`cmd 10053`).
///
/// # Errors
/// Returns an error if unauthenticated or the HTTP call fails.
pub async fn get_last_msgs(s: &Session, thread_ids: &[String]) -> Result<LastMsgsBatch> {
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
    let raw = s.maybe_decrypt(resp, s.session_key_bytes()).await;
    Ok(LastMsgsBatch::new(raw))
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
) -> Result<CloudMessagesPage> {
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
    Ok(CloudMessagesPage::from_wire(dec))
}

/// Call Zalo's native HTTP long-polling message endpoint (`getChatFromPolling`, 12s window).
///
/// # Errors
/// Returns an error if unauthenticated or the server returns a non-zero `error_code` / HTTP error.
pub async fn get_chat_from_polling(s: &Session, state: &mut PollState) -> Result<PollBatch> {
    require_login(s)?;
    // Without the session cookie/AES key the long-poll host answers a bare `HTTP 400`,
    // so skip the round-trip instead of hammering it every tick.
    if !s.has_live_credentials() {
        anyhow::bail!("phiên thiếu zpw_sek/secret_key, bỏ qua long-poll");
    }
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
            let env = crate::protocol::wire::ErrorEnvelopeWireData::parse(&dec);
            if let Some(code) = env.code_opt() {
                if code != 0 {
                    return Err(SessionError::new(code, error_text(env.message())).into());
                }
            }
            state.update_from_response(&dec);
            Ok(PollBatch::from_wire(dec, ts))
        }
        Ok(Err(e)) => Err(e),
        Err(_) => Ok(PollBatch::from_wire(
            json!({"error_code": 0, "data": {}}),
            ts,
        )),
    }
}

/// Poll `getChatFromPolling`, `get-last-msgs`, and `/api/cm/getrecentv2`, and merge messages into `threads`.
///
/// Pure protocol: nothing is written to disk. The caller persists the merged `threads`
/// (see `storage::persist_account_threads`) when this returns `Ok`.
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
        let Ok(batch) = get_chat_from_polling(s, state).await else {
            break;
        };
        batch.merge_into(threads);
        if !batch.more && !batch.has_reset {
            break;
        }
    }
    if let Ok(preview_batch) = get_last_msgs(s, &[]).await {
        preview_batch.merge_into(threads);
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
        if let Ok(preview_batch) = get_last_msgs(s, chunk).await {
            preview_batch.merge_into(threads);
        }
    }
    let send2me_id = login.send2me_id.clone();
    if !send2me_id.is_empty() {
        if let Ok(cm_page) = get_cloud_messages(s, &send2me_id, "0", false, 50).await {
            if let Some(t) = threads.iter_mut().find(|t| t.id == send2me_id) {
                cm_page.merge_into_thread(t);
            } else {
                cm_page.merge_into_threads(threads);
            }
        }
    }
    let group_ids: Vec<String> = threads
        .iter()
        .filter(|t| t.is_group && !t.id.is_empty())
        .map(|t| t.id.clone())
        .collect();
    for gid in group_ids {
        if let Ok(cm_page) = get_cloud_messages(s, &gid, "0", false, 50).await {
            if let Some(t) = threads.iter_mut().find(|t| t.id == gid) {
                cm_page.merge_into_thread(t);
            }
        }
    }
    resolve_thread_names(s, threads).await;
    threads.retain(|t| !t.messages.is_empty() || !t.name.starts_with("Người dùng "));
    deduplicate_threads(threads, None);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::session::LoginInfo;

    #[tokio::test]
    async fn long_poll_skips_when_session_credentials_are_missing() {
        let mut s = Session::new(false, false, false).unwrap();
        s.login = Some(LoginInfo::default());
        let mut state = PollState::default();

        let err = get_chat_from_polling(&s, &mut state).await.unwrap_err();
        assert!(err.to_string().contains("zpw_sek/secret_key"));
    }
}
