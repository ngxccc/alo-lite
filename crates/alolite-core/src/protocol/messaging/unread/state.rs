//! In-memory unread state entity following server read cursors.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use super::mentions::{classify_message_mention, MentionDetection};
use crate::protocol::messaging::model::MessageKind;
use crate::protocol::messaging::threads::ChatMessage;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct UnreadState {
    /// None means that no authoritative read point has been received.
    pub last_seen_id: Option<String>,
    #[serde(default)]
    pub unread_ids: BTreeSet<String>,
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    pub mention_ids: BTreeSet<String>,
    #[serde(default)]
    pub muted: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub muted_until: Option<i64>,
    #[serde(default)]
    pub marked_unread: bool,
    /// Network metadata copies may update state; ordinary history copies may not.
    #[serde(skip)]
    pub fresh: bool,
    #[serde(skip)]
    pub settings_fresh: bool,
}

pub(super) fn numeric_id(id: &str) -> Option<&str> {
    if id.is_empty() || !id.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let id = id.trim_start_matches('0');
    Some(if id.is_empty() { "0" } else { id })
}

pub fn compare_ids(a: &str, b: &str) -> Option<std::cmp::Ordering> {
    let (a, b) = (numeric_id(a)?, numeric_id(b)?);
    Some(a.len().cmp(&b.len()).then_with(|| a.cmp(b)))
}

impl UnreadState {
    pub fn has_unread_mentions(&self) -> bool {
        !self.mention_ids.is_empty()
    }
    pub fn count(&self) -> usize {
        self.unread_ids.len()
    }
    pub fn is_muted(&self) -> bool {
        self.muted
            && self
                .muted_until
                .is_none_or(|until| until > crate::clock::unix_now())
    }
    pub fn has_badge(&self) -> bool {
        self.count() > 0 || self.marked_unread
    }
    pub fn badge(&self, is_group: bool) -> Option<String> {
        if !self.has_badge() {
            return None;
        }
        if self.is_muted() || self.count() == 0 {
            return Some(String::new());
        }
        Some(if is_group && self.count() > 5 {
            "5+".into()
        } else if self.count() > 99 {
            "99+".into()
        } else {
            self.count().to_string()
        })
    }
    pub fn merge_metadata(&mut self, incoming: &Self) -> bool {
        let before = self.clone();
        if incoming.fresh {
            if let Some(point) = &incoming.last_seen_id {
                self.advance_seen(point);
            }
            if let Some(point) = self.last_seen_id.as_deref() {
                for id in &incoming.unread_ids {
                    if compare_ids(id, point).is_some_and(|order| order.is_gt()) {
                        self.unread_ids.insert(id.clone());
                    }
                }
            }
            if incoming.settings_fresh {
                self.muted = incoming.muted;
                self.muted_until = incoming.muted_until;
                self.marked_unread = incoming.marked_unread;
            }
            self.fresh = true;
        } else if self.last_seen_id.is_none() && incoming.last_seen_id.is_some() {
            *self = incoming.clone();
        }
        if let Some(point) = self.last_seen_id.as_deref() {
            for id in &incoming.mention_ids {
                if self.unread_ids.contains(id)
                    && compare_ids(id, point).is_some_and(|order| order.is_gt())
                {
                    self.mention_ids.insert(id.clone());
                }
            }
        }
        *self != before
    }
    pub fn advance_seen(&mut self, point: &str) {
        if numeric_id(point).is_none() {
            return;
        }
        if self
            .last_seen_id
            .as_ref()
            .is_none_or(|old| compare_ids(point, old).is_some_and(|v| !v.is_lt()))
        {
            self.last_seen_id = Some(point.into());
            self.unread_ids
                .retain(|id| compare_ids(id, point).is_some_and(|v| v.is_gt()));
            self.mention_ids
                .retain(|id| compare_ids(id, point).is_some_and(|order| order.is_gt()));
        }
    }
    /// Called only for new live messages, or a read-only server-cursor snapshot.
    pub fn observe(&mut self, messages: &[ChatMessage]) {
        let Some(point) = &self.last_seen_id else {
            return;
        };
        for message in messages {
            if !message.is_outgoing
                && message.kind != MessageKind::System
                && message.unread_policy.counts_unread()
                && compare_ids(&message.id, point).is_some_and(|v| v.is_gt())
            {
                self.unread_ids.insert(message.id.clone());
            }
        }
    }
    pub fn observe_for_account(&mut self, messages: &[ChatMessage], account: &str) {
        self.observe(messages);
        self.observe_mentions(messages, account);
    }
    /// Missing mention metadata never erases a previously verified off-screen mention.
    pub fn observe_mentions(&mut self, messages: &[ChatMessage], account: &str) {
        for message in messages {
            if message.is_outgoing
                || message.kind == MessageKind::System
                || !message.unread_policy.counts_unread()
            {
                self.mention_ids.remove(&message.id);
                continue;
            }
            if !self.unread_ids.contains(&message.id) {
                continue;
            }
            match classify_message_mention(message, account) {
                MentionDetection::Mention => {
                    self.mention_ids.insert(message.id.clone());
                }
                MentionDetection::NoMention => {
                    self.mention_ids.remove(&message.id);
                }
                MentionDetection::Unknown => {}
            }
        }
    }
    /// Local foreground reading does not send a receipt or mutate the account.
    pub fn read_visible(&mut self, messages: &[ChatMessage]) {
        if let Some(last) = messages
            .iter()
            .filter(|m| numeric_id(&m.id).is_some())
            .max_by(|a, b| compare_ids(&a.id, &b.id).unwrap())
        {
            self.advance_seen(&last.id);
            self.marked_unread = false;
        }
        // Only the visible cursor is advanced; off-screen unread IDs remain.
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn message(id: &str, outgoing: bool) -> ChatMessage {
        ChatMessage {
            id: id.into(),
            is_outgoing: outgoing,
            ..Default::default()
        }
    }

    #[test]
    fn unread_unknown_is_not_fabricated_from_history() {
        let mut state = UnreadState::default();
        state.observe(&[message("22", false)]);
        assert_eq!(state.count(), 0);
        assert!(state.last_seen_id.is_none());
    }

    #[test]
    fn unread_cursor_deduplicates_and_never_regresses() {
        let mut state = UnreadState::default();
        state.advance_seen("20");
        for _ in 0..3 {
            state.observe(&[
                message("21", false),
                message("22", true),
                message("19", false),
            ]);
        }
        assert_eq!(state.count(), 1);
        state.advance_seen("21");
        state.advance_seen("19");
        assert_eq!(state.count(), 0);
        assert_eq!(state.last_seen_id.as_deref(), Some("21"));
    }

    #[test]
    fn unread_large_ids_remain_exact_and_muted_is_dot() {
        let mut state = UnreadState::default();
        state.advance_seen("9007199254740993000");
        for n in 1..=6 {
            state.observe(&[message(&format!("900719925474099300{n}"), false)]);
        }
        assert_eq!(state.badge(true).as_deref(), Some("5+"));
        state.muted = true;
        assert_eq!(state.badge(true).as_deref(), Some(""));
        let saved = serde_json::to_string(&state).unwrap();
        assert_eq!(serde_json::from_str::<UnreadState>(&saved).unwrap(), state);
    }

    #[test]
    fn unread_history_defaults_preserve_state_and_foreground_read_clears() {
        let mut state = UnreadState::default();
        state.advance_seen("10");
        state.observe(&[message("11", false)]);
        state.merge_metadata(&UnreadState::default());
        assert_eq!(state.count(), 1);
        state.read_visible(&[message("11", false)]);
        assert!(!state.has_badge());
    }
}
