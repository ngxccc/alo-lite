//! Server mutations for conversation threads: pin/unpin, hide/unhide, and delete conversation history.

use anyhow::Result;
use serde_json::{json, Value};

use crate::clock::now_ms;
use crate::protocol::session::Session;
use crate::protocol::wire::{pick_str, require_login, strip_group_prefix, unwrap_data, ActionAck};

/// `GET /api/pinconvers/list` (cmd `12120`) — fetch pinned thread IDs.
///
/// # Errors
/// Returns an error if unauthenticated or the HTTP call fails.
pub async fn get_pinned(s: &Session) -> Result<Vec<String>> {
    require_login(s)?;
    let url = format!(
        "{}/api/pinconvers/list?{}",
        s.domains.conversation,
        s.common_params()
    );
    let resp = s.get(&url).await?;
    let data = unwrap_data(&resp);
    let arr = data
        .get("conversations")
        .and_then(Value::as_array)
        .or_else(|| data.as_array())
        .cloned()
        .unwrap_or_default();
    Ok(arr
        .into_iter()
        .filter_map(|item| {
            item.as_str()
                .map(str::to_string)
                .or_else(|| pick_str(&item, &["id", "threadId", "thread_id"]))
        })
        .collect())
}

/// Pin or unpin a conversation (`updatePinnedConversationsV2`, cmd `12122`).
///
/// # Errors
/// Returns an error if unauthenticated, encryption fails, or the HTTP call fails.
pub async fn update_pinned_conversation(
    s: &Session,
    thread_id: &str,
    is_group: bool,
    pin: bool,
) -> Result<ActionAck> {
    require_login(s)?;
    let clean = strip_group_prefix(thread_id).trim_start_matches('u').trim();
    let prefixed = if is_group {
        format!("g{clean}")
    } else {
        format!("u{clean}")
    };
    let body = json!({
        "actionType": if pin { 1 } else { 2 },
        "conversations": [prefixed],
        "tab": 0
    });
    let params = s.encoded_params(&body)?;
    let url = format!(
        "{}/api/pinconvers/updatev2?{}",
        s.domains.conversation,
        s.common_params()
    );
    let resp = s.post_form(&url, format!("params={params}")).await?;
    let dec = s.maybe_decrypt(resp, s.session_key_bytes()).await;
    Ok(ActionAck::from_wire(&dec))
}

/// Hide or unhide a conversation (`updateItemHiddenChat`, cmd `10049`).
///
/// # Errors
/// Returns an error if unauthenticated, encryption fails, or the HTTP call fails.
pub async fn set_hidden_conversation(
    s: &Session,
    thread_id: &str,
    is_group: bool,
    hide: bool,
) -> Result<ActionAck> {
    require_login(s)?;
    let clean = strip_group_prefix(thread_id).trim();
    let entry = json!([{
        "thread_id": clean,
        "is_group": if is_group { 1 } else { 0 }
    }]);
    let (add_str, del_str) = if hide {
        (entry.to_string(), "[]".to_string())
    } else {
        ("[]".to_string(), entry.to_string())
    };
    let body = json!({
        "del_threads": del_str,
        "add_threads": add_str,
        "imei": s.imei
    });
    let params = s.encoded_params(&body)?;
    let url = format!(
        "{}/api/hiddenconvers/add-remove?{}",
        s.domains.conversation,
        s.common_params()
    );
    let resp = s.post_form(&url, format!("params={params}")).await?;
    let dec = s.maybe_decrypt(resp, s.session_key_bytes()).await;
    Ok(ActionAck::from_wire(&dec))
}

/// Delete conversation history for me (`deleteOneOneConversationV2` / `deleteGroupConversationV2`, cmd `11778` / `11835`).
///
/// # Errors
/// Returns an error if unauthenticated, encryption fails, or the HTTP call fails.
pub async fn delete_conversation(
    s: &Session,
    thread_id: &str,
    is_group: bool,
    last_cli_msg_id: &str,
) -> Result<ActionAck> {
    require_login(s)?;
    let clean = strip_group_prefix(thread_id).trim();
    let cli_id = if last_cli_msg_id.is_empty() {
        now_ms().to_string()
    } else {
        last_cli_msg_id.to_string()
    };
    if is_group {
        let body = json!({
            "grid": clean,
            "cliMsgId": cli_id,
            "conver": {},
            "onlyMe": 1,
            "imei": s.imei
        });
        let params = s.encoded_params(&body)?;
        let url = format!(
            "{}/api/group/deleteconver?{}",
            s.domains.group,
            s.common_params()
        );
        let resp = s.post_form(&url, format!("params={params}")).await?;
        let dec = s.maybe_decrypt(resp, s.session_key_bytes()).await;
        Ok(ActionAck::from_wire(&dec))
    } else {
        let body = json!({
            "toid": clean,
            "cliMsgId": cli_id,
            "conver": {},
            "onlyMe": 1,
            "imei": s.imei
        });
        let params = s.encoded_params(&body)?;
        let url = format!(
            "{}/api/message/deleteconver?{}",
            s.domains.chat,
            s.common_params()
        );
        let resp = s.post_form(&url, format!("params={params}")).await?;
        let dec = s.maybe_decrypt(resp, s.session_key_bytes()).await;
        Ok(ActionAck::from_wire(&dec))
    }
}
