//! Read points, seen receipts, and manual unread marks.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::state::numeric_id;
use crate::protocol::messaging::threads::Thread;

use crate::protocol::wire::{opt_str, opt_u64, WireData};

/// String-or-numeric coercion of one optional field (single-key form of the legacy `string` helper).
fn field_string(v: Option<&Value>) -> Option<String> {
    opt_str(v)
        .map(str::to_owned)
        .or_else(|| opt_u64(v).map(|n| n.to_string()))
}

/// Read-point payload carrying `clearUnreads` cursors.
///
/// # Wire Response Example
/// ```json
/// { "clearUnreads": [ { "idTo": "1234567890", "lastMsgId": "1731215400000_9", "isGroup": 1 } ] }
/// ```
///
/// # Core Transformation
/// Keys are kept raw so a non-array `clearUnreads` cannot silently fall through to
/// `clearUnreadInfo` (the shipped client picks the first present key, then requires an array).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ReadPointsWireData {
    #[serde(rename = "clearUnreads")]
    pub clear_unreads: Option<Value>,
    #[serde(rename = "clearUnreadInfo")]
    pub clear_unread_info: Option<Value>,
    #[serde(rename = "statusMap")]
    pub status_map: Option<Value>,
}

impl ReadPointsWireData {
    /// Cursor entries from the first present key (`clearUnreads`, `clearUnreadInfo`,
    /// `statusMap.clearUnreads`).
    pub fn entries(&self) -> &[Value] {
        self.clear_unreads
            .as_ref()
            .or(self.clear_unread_info.as_ref())
            .or_else(|| self.status_map.as_ref().and_then(|m| m.get("clearUnreads")))
            .and_then(Value::as_array)
            .map(Vec::as_slice)
            .unwrap_or_default()
    }
}

/// One `clearUnreads` cursor entry.
///
/// # Wire Response Example
/// ```json
/// { "idTo": "1234567890", "lastMsgId": "1731215400000_9", "isGroup": 1, "type": 1 }
/// ```
///
/// # Core Transformation
/// Advances `Thread::unread` seen cursors; subchat cursors (`type == 2` with `sct`) are ignored
/// because the shipped desktop consumer ignores them.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ReadPointWireData {
    #[serde(rename = "idTo")]
    pub id_to: Option<Value>,
    #[serde(rename = "lastMsgId")]
    pub last_msg_id: Option<Value>,
    #[serde(rename = "isGroup")]
    pub is_group: Option<Value>,
    #[serde(rename = "type")]
    pub entry_type: Option<Value>,
    #[serde(rename = "sct")]
    pub sct: Option<Value>,
}

impl ReadPointWireData {
    /// Subchat cursor the desktop consumer ignores (`type == 2` with `sct` present).
    pub fn is_ignored_subchat(&self) -> bool {
        self.entry_type.as_ref().and_then(Value::as_i64) == Some(2) && self.sct.is_some()
    }

    /// Conversation id (`idTo`).
    pub fn id_to(&self) -> Option<String> {
        field_string(self.id_to.as_ref())
    }

    /// Last message id (`lastMsgId`).
    pub fn last_msg_id(&self) -> Option<String> {
        field_string(self.last_msg_id.as_ref())
    }

    /// `isGroup` in boolean or `0`/`1` numeric form.
    pub fn is_group(&self) -> bool {
        self.is_group
            .as_ref()
            .is_some_and(|v| v.as_bool() == Some(true) || v.as_i64() == Some(1))
    }
}

/// Manual-unread snapshot: `convsGroup` / `convsUser` rows plus the envelope error code.
///
/// # Wire Response Example
/// ```json
/// { "error_code": 0, "convsUser": [ { "id": "1234567890", "cliMsgId": "8888" } ] }
/// ```
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ManualUnreadSnapshotWireData {
    #[serde(rename = "error_code")]
    pub error_code: Option<Value>,
    #[serde(rename = "convsGroup")]
    pub convs_group: Option<Vec<Value>>,
    #[serde(rename = "convsUser")]
    pub convs_user: Option<Vec<Value>>,
}

impl ManualUnreadSnapshotWireData {
    /// Fail when `error_code` is present and non-zero (number or numeric string).
    pub fn check(&self) -> anyhow::Result<()> {
        if let Some(code) = self.error_code.as_ref() {
            if code.as_i64().or_else(|| code.as_str()?.parse().ok()) != Some(0) {
                anyhow::bail!("Manual unread request failed");
            }
        }
        Ok(())
    }

    /// Group and user rows, or `None` when either list is missing or not an array.
    pub fn rows(&self) -> Option<(&[Value], &[Value])> {
        Some((self.convs_group.as_deref()?, self.convs_user.as_deref()?))
    }
}

/// One manual-unread row (`convsGroup` / `convsUser` item).
///
/// # Wire Response Example
/// ```json
/// { "id": "1234567890", "cliMsgId": "8888", "fromUid": "1234567890" }
/// ```
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ManualUnreadRowWireData {
    #[serde(rename = "id")]
    pub id: Option<Value>,
    #[serde(rename = "cliMsgId")]
    pub cli_msg_id: Option<Value>,
    #[serde(rename = "fromUid")]
    pub from_uid: Option<Value>,
}

impl ManualUnreadRowWireData {
    /// Conversation id (`id`).
    pub fn id(&self) -> Option<String> {
        field_string(self.id.as_ref())
    }

    /// Marked client message id (`cliMsgId`).
    pub fn cli_msg_id(&self) -> Option<String> {
        field_string(self.cli_msg_id.as_ref())
    }

    /// Sender id (`fromUid`).
    pub fn from_uid(&self) -> Option<String> {
        field_string(self.from_uid.as_ref())
    }
}

/// Original desktop consumer ignores subchat cursors (`type=2`, `sct=1/2/3`).
pub fn merge_read_points(threads: &mut [Thread], data: &Value) {
    let wire = ReadPointsWireData::from_value(data);
    for entry in wire.entries() {
        let entry = ReadPointWireData::from_value(entry);
        if entry.is_ignored_subchat() {
            continue;
        }
        let (Some(id), Some(point)) = (entry.id_to(), entry.last_msg_id()) else {
            continue;
        };
        if numeric_id(&point).is_none_or(|id| id == "0") || id.is_empty() || id == "0" {
            continue;
        }
        let group = entry.is_group();
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
    ManualUnreadSnapshotWireData::from_value(response).check()?;
    let raw = response.get("data").unwrap_or(response);
    let decoded;
    let data = if let Some(text) = raw.as_str() {
        decoded = serde_json::from_str::<Value>(text)
            .map_err(|_| anyhow::anyhow!("Invalid manual unread payload"))?;
        &decoded
    } else {
        raw
    };
    let snapshot = ManualUnreadSnapshotWireData::from_value(data);
    snapshot.check()?;
    let Some((groups, users)) = snapshot.rows() else {
        anyhow::bail!("Incomplete manual unread snapshot");
    };
    let mut marks = Vec::new();
    for (rows, group) in [(groups, true), (users, false)] {
        for row in rows {
            let row = ManualUnreadRowWireData::from_value(row);
            let id = row
                .id()
                .filter(|id| !id.is_empty() && id != "0")
                .ok_or_else(|| anyhow::anyhow!("Invalid manual unread identity"))?;
            marks.push(ManualUnreadMark {
                thread_id: if group {
                    format!("g{}", id.trim_start_matches('g'))
                } else {
                    id
                },
                client_message_id: row.cli_msg_id().unwrap_or_default(),
                from_uid: row.from_uid().unwrap_or_default(),
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
