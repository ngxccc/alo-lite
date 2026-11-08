//! Message identity indexing, sent message merging, thread merging, and thread deduplication.

use std::collections::{HashMap, HashSet};

use serde_json::Value;

use super::contact::{
    avatar_url_time, is_generic_thread_name, should_replace_avatar, threads_represent_same_contact,
};
use super::entity::Thread;
use super::message::ChatMessage;
use crate::protocol::wire::wire_identity;

fn wire_sender(value: Option<&Value>) -> Option<String> {
    match value? {
        Value::String(value) if !value.is_empty() => Some(value.clone()),
        Value::Number(value) => value.as_u64().map(|id| id.to_string()),
        _ => None,
    }
}

fn server_message_id(message: &ChatMessage) -> Option<String> {
    let wire_server = wire_identity(message.wire_ref.server_id.as_ref());
    let wire_client = wire_identity(message.wire_ref.client_id.as_ref());
    if message.id.starts_with("out_")
        || message.id.is_empty()
        || wire_client.as_deref() == Some(message.id.as_str())
    {
        wire_server
    } else {
        wire_identity(Some(&Value::String(message.id.clone())))
    }
}

fn client_message_id(message: &ChatMessage) -> Option<String> {
    let wire = wire_identity(message.wire_ref.client_id.as_ref());
    if message.client_message_id.trim().is_empty() || message.client_message_id == "0" {
        wire
    } else if wire
        .as_ref()
        .is_some_and(|id| id != &message.client_message_id)
    {
        None
    } else {
        Some(message.client_message_id.clone())
    }
}

#[derive(Default)]
pub struct MessageIdentityIndex {
    rows: HashMap<(u8, String), HashSet<usize>>,
}

impl MessageIdentityIndex {
    fn keys(message: &ChatMessage) -> Vec<(u8, String)> {
        let mut keys = Vec::with_capacity(3);
        if let Some(id) = server_message_id(message) {
            keys.push((0, id));
        } else if message.id.starts_with("out_") {
            keys.push((1, message.id.clone()));
        }
        if !message.id.starts_with("out_") {
            if let Some(id) = wire_identity(Some(&Value::String(message.id.clone()))) {
                keys.push((3, id));
            }
        }
        if let Some(id) = client_message_id(message) {
            keys.push((2, id));
        }
        keys
    }

    pub fn insert(&mut self, message: &ChatMessage, index: usize) {
        for key in Self::keys(message) {
            self.rows.entry(key).or_default().insert(index);
        }
    }

    pub fn remove(&mut self, message: &ChatMessage, index: usize) {
        for key in Self::keys(message) {
            if let Some(rows) = self.rows.get_mut(&key) {
                rows.remove(&index);
            }
        }
    }

    pub fn find(&self, messages: &[ChatMessage], incoming: &ChatMessage) -> Option<usize> {
        for key in Self::keys(incoming) {
            let Some(rows) = self.rows.get(&key).filter(|rows| !rows.is_empty()) else {
                continue;
            };
            if rows.len() != 1 {
                return None;
            }
            let index = *rows.iter().next()?;
            let existing = &messages[index];
            if existing.is_outgoing != incoming.is_outgoing {
                return None;
            }
            if let (Some(a), Some(b)) = (
                wire_sender(existing.wire_ref.sender.as_ref()),
                wire_sender(incoming.wire_ref.sender.as_ref()),
            ) {
                if a != b {
                    return None;
                }
            }
            if key.0 != 2 {
                if key.0 == 3 {
                    if let (Some(a), Some(b)) = (
                        wire_identity(existing.wire_ref.server_id.as_ref()),
                        wire_identity(incoming.wire_ref.server_id.as_ref()),
                    ) {
                        if a != b {
                            return None;
                        }
                    }
                }
                return Some(index);
            }
            let existing_server = server_message_id(existing);
            let incoming_server = server_message_id(incoming);
            let existing_local = existing.id.starts_with("out_") && existing_server.is_none();
            let incoming_local = incoming.id.starts_with("out_") && incoming_server.is_none();
            let client_only_replay = existing_server.is_none()
                && incoming_server.is_none()
                && (existing.id.is_empty() || incoming.id.is_empty());
            return (client_only_replay
                || (existing.is_outgoing
                    && ((existing_local && incoming_server.is_some())
                        || (incoming_local && existing_server.is_some()))))
            .then_some(index);
        }
        None
    }
}

/// Merge a local send acknowledgement without replacing an existing server clock.
pub fn merge_sent_message(thread: &mut Thread, mut message: ChatMessage) {
    let mut identities = MessageIdentityIndex::default();
    for (row, existing) in thread.messages.iter().enumerate() {
        identities.insert(existing, row);
    }
    if identities
        .find(&thread.messages, &message)
        .is_some_and(|row| server_message_id(&thread.messages[row]).is_some())
    {
        message.ts = 0;
        message.time.clear();
    }
    merge_thread_into(
        thread,
        Thread {
            messages: vec![message],
            ..Default::default()
        },
        false,
    );
}

/// Merge `incoming` thread into `target` (`INV-4`).
pub fn merge_thread_into(
    target: &mut Thread,
    incoming: Thread,
    incoming_id_is_authoritative: bool,
) {
    if incoming_id_is_authoritative && !incoming.id.is_empty() && incoming.id != "0" {
        target.id = incoming.id;
    }
    if !incoming.name.is_empty()
        && (!is_generic_thread_name(&incoming.name) || is_generic_thread_name(&target.name))
    {
        target.name = incoming.name;
    }
    if should_replace_avatar(target.avatar.as_deref(), incoming.avatar.as_deref()) {
        target.avatar = incoming.avatar;
    }
    target.is_group = target.is_group || incoming.is_group;

    let mut identities = MessageIdentityIndex::default();
    for (index, message) in target.messages.iter().enumerate() {
        identities.insert(message, index);
    }

    for msg in incoming.messages {
        if msg.text == "sendBubbleMessage" {
            continue;
        }
        if let Some(idx) = identities.find(&target.messages, &msg) {
            let existing_msg = &mut target.messages[idx];
            identities.remove(existing_msg, idx);
            existing_msg.reconcile_incoming(msg, false);
            identities.insert(existing_msg, idx);
        } else {
            let idx = target.messages.len();
            identities.insert(&msg, idx);
            target.messages.push(msg);
        }
    }

    target.messages.retain(|m| m.text != "sendBubbleMessage");
    target.messages.sort_by_key(|m| m.ts);

    if let Some(newest) = target.messages.last() {
        if !newest.text.is_empty() {
            target.preview = newest.text.replace('\n', " ");
            target.time = newest.time.clone();
        }
    } else if !incoming.preview.is_empty() && target.preview.is_empty() {
        target.preview = incoming.preview;
        if !incoming.time.is_empty() {
            target.time = incoming.time;
        }
    } else if target.time.is_empty() && !incoming.time.is_empty() {
        target.time = incoming.time;
    }
}

/// Deduplicate `threads` in-place, merging entries representing the same contact/group.
pub fn deduplicate_threads(threads: &mut Vec<Thread>, authoritative_ids: Option<&HashSet<String>>) {
    let mut merged: Vec<Thread> = Vec::with_capacity(threads.len());
    for mut t in threads.drain(..) {
        if t.id.is_empty() || t.id == "0" {
            continue;
        }
        t.messages.retain(|m| m.text != "sendBubbleMessage");
        if let Some(newest) = t.messages.last() {
            if !newest.text.is_empty() && (t.preview.is_empty() || t.preview == "sendBubbleMessage")
            {
                t.preview = newest.text.replace('\n', " ");
                t.time = newest.time.clone();
            }
        }
        if let Some(existing) = merged.iter_mut().find(|e| {
            if e.id == t.id {
                return true;
            }
            if let Some(auth) = authoritative_ids {
                if auth.contains(&e.id) && auth.contains(&t.id) {
                    return false;
                }
            }
            threads_represent_same_contact(e, &t)
        }) {
            let incoming_auth = if let Some(auth) = authoritative_ids {
                auth.contains(&t.id) && !auth.contains(&existing.id)
            } else {
                let t_time = avatar_url_time(t.avatar.as_deref());
                let e_time = avatar_url_time(existing.avatar.as_deref());
                if t_time != e_time {
                    t_time > e_time
                } else {
                    true
                }
            };
            merge_thread_into(existing, t, incoming_auth);
        } else {
            merged.push(t);
        }
    }
    let cloud_msg_ids: HashSet<String> = merged
        .iter()
        .filter(|t| t.name.contains("Cloud của tôi"))
        .flat_map(|t| t.messages.iter())
        .filter_map(|m| {
            if !m.id.is_empty() && !m.id.starts_with("out_") && !m.id.starts_with("local_") {
                Some(m.id.clone())
            } else {
                None
            }
        })
        .collect();
    if !cloud_msg_ids.is_empty() {
        for t in merged.iter_mut() {
            if !t.name.contains("Cloud của tôi") {
                let before = t.messages.len();
                t.messages.retain(|m| !cloud_msg_ids.contains(&m.id));
                if t.messages.len() != before {
                    if let Some(newest) = t.messages.last() {
                        t.preview = newest.text.replace('\n', " ");
                        t.time = newest.time.clone();
                    } else {
                        t.preview.clear();
                        t.time.clear();
                    }
                }
            }
        }
    }
    *threads = merged;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merge_thread_into_matches_exact_client_reference() {
        let m = |id: &str, text: &str, ts: i64, out: bool| ChatMessage {
            id: id.into(),
            client_message_id: if id == "out_9" || id == "77" {
                "client-9".into()
            } else {
                String::new()
            },
            text: text.into(),
            ts,
            time: format!("{}", ts / 60_000),
            is_outgoing: out,
            ..Default::default()
        };
        let mut target = Thread {
            id: "t".into(),
            name: "A".into(),
            messages: vec![
                m("1", "chào", 1_000_000, false),
                m("out_9", "ok", 2_000_000, true),
            ],
            ..Default::default()
        };
        let incoming = Thread {
            messages: vec![
                m("1", "chào", 1_000_000, false),
                m("77", "ok", 2_002_000, true),
                m("78", "ok", 9_000_000_000, true),
                m("79", "mới", 3_000_000, false),
            ],
            ..target.clone()
        };
        merge_thread_into(&mut target, incoming, false);
        let ids: Vec<&str> = target.messages.iter().map(|x| x.id.as_str()).collect();
        assert_eq!(ids, vec!["1", "77", "79", "78"]);
    }

    #[test]
    fn duplicate_display_names_with_different_ids_stay_distinct() {
        let contact_a = Thread {
            id: "1000000000000000111".into(),
            name: "Nguyễn Văn A".into(),
            avatar: Some(
                "https://s120-ava-talk.zadn.vn/1/aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa.jpg".into(),
            ),
            messages: vec![ChatMessage {
                id: "m_a".into(),
                text: "Xin chào từ người 1".into(),
                ts: 1_700_000_001_000,
                ..Default::default()
            }],
            ..Default::default()
        };
        let contact_b = Thread {
            id: "1000000000000000222".into(),
            name: "Nguyễn Văn A".into(),
            messages: vec![ChatMessage {
                id: "m_b".into(),
                text: "Tin nhắn mới từ người 2".into(),
                ts: 1_700_000_002_000,
                ..Default::default()
            }],
            ..Default::default()
        };
        assert!(!threads_represent_same_contact(&contact_a, &contact_b));
        let mut threads = vec![contact_a, contact_b];
        deduplicate_threads(&mut threads, None);
        assert_eq!(threads.len(), 2);
    }

    #[test]
    fn deduplicate_threads_migrates_session_scoped_ids_with_matching_avatar_hash() {
        let old_dtc = Thread {
            id: "1000000000000000005".into(),
            name: "Đtc".into(),
            avatar: Some("https://s120-26-ava-talk.zadn.vn/5/00000000000000000000000000000001.jpg?key=abc&time=1797763127".into()),
            messages: vec![ChatMessage {
                id: "m1".into(),
                text: "Hi".into(),
                ts: 1_792_578_088_952,
                ..Default::default()
            }],
            ..Default::default()
        };
        let new_dtc = Thread {
            id: "1000000000000000006".into(),
            name: "Đtc".into(),
            avatar: Some("https://s160-26-ava-talk.zadn.vn/5/00000000000000000000000000000001.jpg?key=xyz&time=1797876325".into()),
            ..Default::default()
        };
        let mut threads = vec![old_dtc, new_dtc];
        deduplicate_threads(&mut threads, None);
        assert_eq!(threads.len(), 1);
        assert_eq!(threads[0].id, "1000000000000000006");
        assert_eq!(threads[0].preview, "Hi");
    }
}
