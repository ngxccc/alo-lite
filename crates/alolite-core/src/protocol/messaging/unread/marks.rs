//! Read points, seen receipts, and manual unread marks.

use serde_json::Value;

use super::state::numeric_id;
use crate::protocol::messaging::threads::Thread;

pub(super) fn string(value: &Value, key: &str) -> Option<String> {
    value.get(key).and_then(|v| {
        v.as_str()
            .map(str::to_owned)
            .or_else(|| v.as_u64().map(|v| v.to_string()))
    })
}

/// Original desktop consumer ignores subchat cursors (`type=2`, `sct=1/2/3`).
pub fn merge_read_points(threads: &mut [Thread], data: &Value) {
    let entries = data
        .get("clearUnreads")
        .or_else(|| data.get("clearUnreadInfo"))
        .or_else(|| data.get("statusMap").and_then(|v| v.get("clearUnreads")))
        .and_then(Value::as_array);
    for entry in entries.into_iter().flatten() {
        if entry.get("type").and_then(Value::as_i64) == Some(2) && entry.get("sct").is_some() {
            continue;
        }
        let (Some(id), Some(point)) = (string(entry, "idTo"), string(entry, "lastMsgId")) else {
            continue;
        };
        if numeric_id(&point).is_none_or(|id| id == "0") || id.is_empty() || id == "0" {
            continue;
        }
        let group = entry
            .get("isGroup")
            .is_some_and(|v| v.as_bool() == Some(true) || v.as_i64() == Some(1));
        let tid = if group {
            format!("g{}", id.trim_start_matches('g'))
        } else {
            id
        };
        if let Some(thread) = threads.iter_mut().find(|t| {
            t.id == tid
                || (group
                    && t.is_group
                    && t.id.trim_start_matches('g') == tid.trim_start_matches('g'))
        }) {
            thread.unread.advance_seen(&point);
            thread.unread.fresh = true;
        }
    }
}

/// Complete read-only manual-mark snapshot. No conversations are created by this adapter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManualUnreadMark {
    pub thread_id: String,
    pub client_message_id: String,
    pub from_uid: String,
}

pub fn parse_manual_unread_marks(response: &Value) -> anyhow::Result<Vec<ManualUnreadMark>> {
    fn checked(value: &Value) -> anyhow::Result<()> {
        if let Some(code) = value.get("error_code") {
            if code.as_i64().or_else(|| code.as_str()?.parse().ok()) != Some(0) {
                anyhow::bail!("Manual unread request failed");
            }
        }
        Ok(())
    }
    checked(response)?;
    let raw = response.get("data").unwrap_or(response);
    let decoded;
    let data = if let Some(text) = raw.as_str() {
        decoded = serde_json::from_str::<Value>(text)
            .map_err(|_| anyhow::anyhow!("Invalid manual unread payload"))?;
        &decoded
    } else {
        raw
    };
    checked(data)?;
    let (Some(groups), Some(users)) = (
        data.get("convsGroup").and_then(Value::as_array),
        data.get("convsUser").and_then(Value::as_array),
    ) else {
        anyhow::bail!("Incomplete manual unread snapshot");
    };
    let mut marks = Vec::new();
    for (rows, group) in [(groups, true), (users, false)] {
        for row in rows {
            let id = string(row, "id")
                .filter(|id| !id.is_empty() && id != "0")
                .ok_or_else(|| anyhow::anyhow!("Invalid manual unread identity"))?;
            marks.push(ManualUnreadMark {
                thread_id: if group {
                    format!("g{}", id.trim_start_matches('g'))
                } else {
                    id
                },
                client_message_id: string(row, "cliMsgId").unwrap_or_default(),
                from_uid: string(row, "fromUid").unwrap_or_default(),
            });
        }
    }
    Ok(marks)
}

pub fn merge_manual_unread_marks(threads: &mut [Thread], marks: &[ManualUnreadMark]) {
    for thread in threads {
        thread.unread.marked_unread = marks.iter().any(|mark| mark.thread_id == thread.id);
        thread.unread.settings_fresh = true;
        thread.unread.fresh = true;
    }
}

/// Shipped static getMarkAsUnread, command 12011. Reading does not modify account state.
pub async fn fetch_manual_unread_marks(
    session: &crate::session::Session,
) -> anyhow::Result<Vec<ManualUnreadMark>> {
    if session.login.is_none() {
        anyhow::bail!("No active session");
    }
    let params = session.encoded_params(&serde_json::json!({}))?;
    let url = format!(
        "{}/api/conv/getUnreadMark?{}&params={params}",
        session.domains.conversation.trim_end_matches('/'),
        session.common_params()
    );
    let raw = session.get(&url).await?;
    let response = session
        .maybe_decrypt(raw, session.session_key_bytes())
        .await;
    parse_manual_unread_marks(&response)
}

/// Send a "seen" receipt (`cmd 12118` / `12124`) unless blocked by `block_seen`.
///
/// # Errors
/// Returns an error if unauthenticated or encryption/transport fails.
pub async fn send_seen_v2(
    s: &crate::protocol::session::Session,
    thread_id: &str,
    is_group: bool,
    last_msg_id: &str,
) -> anyhow::Result<crate::protocol::wire::ActionAck> {
    crate::protocol::wire::require_login(s)?;
    if crate::session::privacy_blocker_state().2 {
        return Ok(crate::protocol::wire::ActionAck::ok());
    }
    let clean = crate::protocol::wire::strip_group_prefix(thread_id)
        .trim_start_matches('u')
        .trim();
    if clean.is_empty() || last_msg_id.trim().is_empty() {
        return Ok(crate::protocol::wire::ActionAck::ok());
    }
    let (endpoint, path_and_q, body) = if is_group {
        (
            "group",
            format!("/api/group/seenv2?{}", s.common_params()),
            serde_json::json!({
                "msgInfos": serde_json::json!({
                    "data": [{ "gmi": last_msg_id, "cmi": last_msg_id }],
                    "grid": clean,
                }).to_string(),
                "imei": s.imei,
            }),
        )
    } else {
        (
            "chat",
            format!("/api/message/seenv2?{}", s.common_params()),
            serde_json::json!({
                "msgInfos": serde_json::json!({
                    "data": [{ "gmi": last_msg_id, "cmi": last_msg_id }],
                    "senderId": clean,
                }).to_string(),
                "imei": s.imei,
            }),
        )
    };
    let params = s.encoded_params(&body)?;
    let resp = s
        .post_endpoint_form(endpoint, &path_and_q, format!("params={params}"))
        .await?;
    let dec = s.maybe_decrypt(resp, s.session_key_bytes()).await;
    Ok(crate::protocol::wire::ActionAck::from_wire(&dec))
}

/// Mark a conversation as unread or read (`addMarkAsUnread` / `removeMarkAsUnread`, cmd `12013` / `12014`).
///
/// # Errors
/// Returns an error if unauthenticated, encryption fails, or the request fails.
pub async fn set_unread_mark(
    s: &crate::protocol::session::Session,
    thread_id: &str,
    is_group: bool,
    unread: bool,
) -> anyhow::Result<crate::protocol::wire::ActionAck> {
    crate::protocol::wire::require_login(s)?;
    let clean = crate::protocol::wire::strip_group_prefix(thread_id).trim();
    let (convs_group, convs_user) = if is_group {
        (vec![clean], Vec::<&str>::new())
    } else {
        (Vec::<&str>::new(), vec![clean])
    };
    let (path, inner) = if unread {
        (
            "/api/conv/addUnreadMark",
            serde_json::json!({
                "convsGroup": convs_group,
                "convsUser": convs_user,
                "imei": s.imei
            }),
        )
    } else {
        (
            "/api/conv/removeUnreadMark",
            serde_json::json!({
                "convsGroup": convs_group,
                "convsUser": convs_user,
                "convsGroupData": [],
                "convsUserData": []
            }),
        )
    };
    let outer = serde_json::json!({ "param": inner.to_string() });
    let params = s.encoded_params(&outer)?;
    let url = format!("{}{path}?{}", s.domains.conversation, s.common_params());
    let resp = s.post_form(&url, format!("params={params}")).await?;
    let dec = s.maybe_decrypt(resp, s.session_key_bytes()).await;
    Ok(crate::protocol::wire::ActionAck::from_wire(&dec))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unread_zero_and_subchat_cursors_do_not_fabricate_read_points() {
        let mut threads = Vec::new();
        merge_read_points(
            &mut threads,
            &serde_json::json!({"clearUnreads":[{"idTo":"42","lastMsgId":"0","isGroup":0},{"idTo":"43","lastMsgId":"99","isGroup":1,"type":2,"sct":1}]}),
        );
        assert!(threads.is_empty());
    }
}
