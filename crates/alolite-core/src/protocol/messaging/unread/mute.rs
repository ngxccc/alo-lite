//! Mute metadata, mute API endpoints, mute control events, and snapshots.

use serde_json::Value;

use super::marks::string;
use crate::protocol::messaging::threads::Thread;

fn unwrap_mute_payload<'a>(data: &'a Value, decoded: &'a mut Option<Value>) -> &'a Value {
    let raw = data.get("data").unwrap_or(data);
    if let Some(text) = raw.as_str() {
        if let Ok(parsed) = serde_json::from_str::<Value>(text) {
            *decoded = Some(parsed);
            if let Some(d) = decoded.as_ref() {
                return d.get("data").unwrap_or(d);
            }
        }
    }
    raw
}

/// Complete read-only mute snapshots use mode 1 and duration -1 for indefinite mute.
pub fn merge_mute_metadata(threads: &mut [Thread], data: &Value) {
    let mut decoded = None;
    let payload = unwrap_mute_payload(data, &mut decoded);
    let (Some(peers), Some(groups)) = (
        payload.get("chatEntries").and_then(Value::as_array),
        payload.get("groupChatEntries").and_then(Value::as_array),
    ) else {
        return;
    };
    for thread in threads {
        let entries = if thread.is_group { groups } else { peers };
        let clean_tid = thread
            .id
            .trim_start_matches("group_")
            .trim_start_matches('g')
            .trim_start_matches('G');
        let entry = entries
            .iter()
            .find(|e| string(e, "id").is_some_and(|id| id.trim_start_matches('g') == clean_tid));
        let mode = entry
            .and_then(|e| e.get("muteMode"))
            .and_then(Value::as_i64);
        if mode.is_some_and(|m| m != 0 && m != 1) {
            continue;
        }
        let duration = entry
            .and_then(|e| e.get("duration"))
            .and_then(Value::as_i64);
        let start = entry.and_then(|e| {
            e.get("startTime")
                .or_else(|| e.get("systemTime"))
                .and_then(Value::as_i64)
        });
        let until = start.zip(duration).and_then(|(start, duration)| {
            if duration >= 0 {
                start.checked_add(duration)
            } else {
                None
            }
        });
        let is_muted_mode = match mode {
            Some(m) => m == 1,
            None => entry.is_some() && duration.is_some_and(|d| d != 0),
        };
        thread.unread.muted = is_muted_mode;
        thread.unread.muted_until = until;
        thread.unread.settings_fresh = true;
        thread.unread.fresh = true;
        thread.presentation.muted = thread.unread.is_muted();
    }
}

/// Extract an authoritative `thread_id -> until_unix` map (`-1` for indefinite mute) from a
/// `/api/social/profile/getmute` payload so `AppSettings::muted_threads` stays synchronized
/// with Official Zalo PC (#67).
pub fn extract_mute_map(
    data: &Value,
    now_unix: i64,
) -> Option<std::collections::HashMap<String, i64>> {
    let mut decoded = None;
    let payload = unwrap_mute_payload(data, &mut decoded);
    let (Some(peers), Some(groups)) = (
        payload.get("chatEntries").and_then(Value::as_array),
        payload.get("groupChatEntries").and_then(Value::as_array),
    ) else {
        return None;
    };
    let mut map = std::collections::HashMap::new();
    for (entries, is_group) in [(peers, false), (groups, true)] {
        for entry in entries {
            let Some(raw_id) = string(entry, "id").filter(|s| !s.is_empty() && s != "0") else {
                continue;
            };
            let mode = entry.get("muteMode").and_then(Value::as_i64).unwrap_or(1);
            if mode != 1 {
                continue;
            }
            let duration = entry.get("duration").and_then(Value::as_i64).unwrap_or(-1);
            if duration == 0 {
                continue;
            }
            let start = entry
                .get("startTime")
                .or_else(|| entry.get("systemTime"))
                .and_then(Value::as_i64)
                .unwrap_or(now_unix);
            let until = if duration < 0 {
                -1
            } else {
                let exp = start.saturating_add(duration);
                if exp <= now_unix {
                    continue;
                }
                exp
            };
            let clean = raw_id
                .trim_start_matches("group_")
                .trim_start_matches('g')
                .trim_start_matches('G');
            if is_group {
                map.insert(format!("g{clean}"), until);
            } else {
                map.insert(clean.to_string(), until);
            }
        }
    }
    Some(map)
}

/// Mute or unmute notifications for a conversation (`setMuteConversation`, cmd `11288`).
///
/// # Errors
/// Returns an error if unauthenticated, encryption fails, or the request fails.
pub async fn set_mute_conversation(
    s: &crate::protocol::session::Session,
    thread_id: &str,
    is_group: bool,
    duration_sec: i64,
    mute: bool,
) -> anyhow::Result<crate::protocol::wire::ActionAck> {
    crate::protocol::wire::require_login(s)?;
    let clean = crate::protocol::wire::strip_group_prefix(thread_id).trim();
    let body = serde_json::json!({
        "toid": clean,
        "duration": duration_sec,
        "action": if mute { 1 } else { 2 },
        "startTime": crate::clock::unix_now(),
        "muteType": if is_group { 2 } else { 1 },
        "imei": s.imei
    });
    let params = s.encoded_params(&body)?;
    let path = format!("/api/social/profile/setmute?{}", s.common_params());
    let resp = s
        .post_endpoint_form("profile", &path, format!("params={params}"))
        .await?;
    let dec = s.maybe_decrypt(resp, s.session_key_bytes()).await;
    Ok(crate::protocol::wire::ActionAck::from_wire(&dec))
}

/// Fetch the full mute snapshot (`chatEntries` + `groupChatEntries`, cmd `11280`).
///
/// # Errors
/// Returns an error if unauthenticated or the HTTP request fails.
pub async fn get_mute_conversations(
    s: &crate::protocol::session::Session,
) -> anyhow::Result<Value> {
    crate::protocol::wire::require_login(s)?;
    let body = serde_json::json!({ "imei": s.imei });
    let params = s.encoded_params(&body)?;
    let path = format!(
        "/api/social/profile/getmute?{}&params={params}",
        s.common_params()
    );
    let resp = s.get_endpoint("profile", &path).await?;
    Ok(s.maybe_decrypt(resp, s.session_key_bytes()).await)
}

/// Decoded conversation mute snapshot returned by `get_mute_snapshot`.
#[derive(Debug, Clone, Default)]
pub struct MuteSnapshot {
    pub mutes: std::collections::HashMap<String, i64>,
    raw: Value,
}

impl MuteSnapshot {
    /// Apply mute metadata from this snapshot onto in-memory threads.
    pub fn merge_into(&self, threads: &mut [Thread]) {
        merge_mute_metadata(threads, &self.raw);
    }
}

/// Fetch and decode the full mute snapshot (`chatEntries` + `groupChatEntries`, cmd `11280`).
///
/// # Errors
/// Returns an error if unauthenticated or the HTTP request fails.
pub async fn get_mute_snapshot(
    s: &crate::protocol::session::Session,
    now_unix: i64,
) -> anyhow::Result<MuteSnapshot> {
    let raw = get_mute_conversations(s).await?;
    let mutes = extract_mute_map(&raw, now_unix).unwrap_or_default();
    Ok(MuteSnapshot { mutes, raw })
}

/// Real-time mute/unmute control notification (`act_type: "mute"`, `act: "set" | "unset"`, `#67`).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct MuteControlEvent {
    pub thread_id: String,
    pub is_group: bool,
    pub muted: bool,
    pub muted_until: Option<i64>,
    pub settings_until: Option<i64>,
}

/// Parse `act_type: "mute"` items inside WebSocket `cmd 601`/`602`/`603` or `lp_control` payloads.
pub fn extract_mute_controls(data_node: &Value, now_unix: i64) -> Vec<MuteControlEvent> {
    let mut out = Vec::new();
    for c in crate::protocol::wire::iter_control_items(data_node) {
        if c.get("act_type")
            .and_then(Value::as_str)
            .unwrap_or_default()
            != "mute"
        {
            continue;
        }
        let act = c.get("act").and_then(Value::as_str).unwrap_or_default();
        let d = match c.get("data") {
            Some(Value::String(s)) => match serde_json::from_str::<Value>(s.as_str()) {
                Ok(v) => v,
                Err(_) => continue,
            },
            Some(v) => v.clone(),
            None => c.clone(),
        };
        let raw_id = d
            .get("toId")
            .or_else(|| d.get("toid"))
            .or_else(|| d.get("id"))
            .and_then(|v| {
                v.as_str()
                    .map(str::to_owned)
                    .or_else(|| v.as_u64().map(|n| n.to_string()))
                    .or_else(|| v.as_i64().map(|n| n.to_string()))
            })
            .unwrap_or_default();
        let clean = crate::protocol::wire::strip_group_prefix(raw_id.trim());
        if clean.is_empty() || clean == "0" {
            continue;
        }

        let mute_type = crate::protocol::wire::val_i64(d.get("muteType")).unwrap_or(1);
        let is_group = mute_type == 2 || raw_id.starts_with('g') || raw_id.starts_with("group_");
        let thread_id = if is_group {
            format!("g{clean}")
        } else {
            clean.to_string()
        };
        let duration = crate::protocol::wire::val_i64(d.get("duration")).unwrap_or(-1);
        let mute_mode = crate::protocol::wire::val_i64(d.get("muteMode")).unwrap_or(1);
        let start_time =
            crate::protocol::wire::val_i64(d.get("startTime").or_else(|| d.get("systemTime")))
                .unwrap_or(now_unix);

        let is_set = match act {
            "set" => mute_mode == 1 && duration != 0,
            "unset" => false,
            _ => {
                d.get("action").and_then(Value::as_i64).unwrap_or(1) == 1
                    && mute_mode == 1
                    && duration != 0
            }
        };

        if !is_set {
            out.push(MuteControlEvent {
                thread_id,
                is_group,
                muted: false,
                muted_until: None,
                settings_until: None,
            });
            continue;
        }

        if duration < 0 {
            out.push(MuteControlEvent {
                thread_id,
                is_group,
                muted: true,
                muted_until: None,
                settings_until: Some(-1),
            });
        } else {
            let until = start_time.saturating_add(duration);
            if until > now_unix {
                out.push(MuteControlEvent {
                    thread_id,
                    is_group,
                    muted: true,
                    muted_until: Some(until),
                    settings_until: Some(until),
                });
            } else {
                out.push(MuteControlEvent {
                    thread_id,
                    is_group,
                    muted: false,
                    muted_until: None,
                    settings_until: None,
                });
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unread_mute_snapshot_handles_indefinite_expiry_and_omitted_entries() {
        let mut threads = vec![Thread {
            presentation: Default::default(),
            id: "42".into(),
            name: String::new(),
            preview: String::new(),
            time: String::new(),
            avatar: None,
            is_group: false,
            messages: vec![],
            unread: Default::default(),
        }];
        merge_mute_metadata(
            &mut threads,
            &serde_json::json!({"chatEntries":[{"id":"42","muteMode":1,"duration":-1,"startTime":10}],"groupChatEntries":[]}),
        );
        assert!(threads[0].unread.is_muted());
        merge_mute_metadata(
            &mut threads,
            &serde_json::json!({"chatEntries":[{"id":"42","muteMode":1,"duration":10,"startTime":10}],"groupChatEntries":[]}),
        );
        assert!(!threads[0].unread.is_muted());
        merge_mute_metadata(
            &mut threads,
            &serde_json::json!({"chatEntries":[],"groupChatEntries":[]}),
        );
        assert!(!threads[0].unread.muted);
        assert!(!threads[0].presentation.muted);
    }

    #[test]
    fn unread_mute_snapshot_unwraps_data_envelope_and_extracts_mute_map() {
        let mut threads = vec![
            Thread {
                presentation: Default::default(),
                id: "12345".into(),
                name: String::new(),
                preview: String::new(),
                time: String::new(),
                avatar: None,
                is_group: false,
                messages: vec![],
                unread: Default::default(),
            },
            Thread {
                presentation: Default::default(),
                id: "g999888".into(),
                name: String::new(),
                preview: String::new(),
                time: String::new(),
                avatar: None,
                is_group: true,
                messages: vec![],
                unread: Default::default(),
            },
        ];
        let payload = serde_json::json!({
            "error_code": 0,
            "data": {
                "chatEntries": [{"id": "12345", "muteMode": 1, "duration": -1, "startTime": 1700000000}],
                "groupChatEntries": [{"id": "999888", "muteMode": 1, "duration": 3600, "startTime": 4000000000_i64}]
            }
        });
        merge_mute_metadata(&mut threads, &payload);
        assert!(threads[0].unread.is_muted());
        assert!(threads[0].presentation.muted);
        assert!(threads[1].unread.is_muted());
        assert!(threads[1].presentation.muted);

        let map = extract_mute_map(&payload, 1700000100).expect("valid mute snapshot");
        assert_eq!(map.get("12345"), Some(&-1));
        assert_eq!(map.get("g999888"), Some(&(4000000000_i64 + 3600)));
    }

    #[test]
    fn extract_mute_controls_parses_set_and_unset_events_for_peers_and_groups() {
        let now = 1700000000_i64;
        let payload = serde_json::json!({
            "controls": [
                {
                    "content": {
                        "act_type": "mute",
                        "act": "set",
                        "data": {
                            "toId": "123456789",
                            "muteType": 1,
                            "duration": -1,
                            "startTime": now,
                            "muteMode": 1
                        }
                    }
                },
                {
                    "content": {
                        "act_type": "mute",
                        "act": "set",
                        "data": "{\"toId\":\"987654321\",\"muteType\":2,\"duration\":3600,\"startTime\":1700000000,\"muteMode\":1}"
                    }
                },
                {
                    "content": {
                        "act_type": "mute",
                        "act": "unset",
                        "data": {
                            "toId": "123456789",
                            "muteType": 1,
                            "duration": 0,
                            "muteMode": 0
                        }
                    }
                }
            ]
        });
        let events = extract_mute_controls(&payload, now);
        assert_eq!(events.len(), 3);
        assert_eq!(events[0].settings_until, Some(-1));
        assert_eq!(events[1].thread_id, "g987654321");
        assert_eq!(events[1].muted_until, Some(now + 3600));
        assert!(!events[2].muted);
    }
}
