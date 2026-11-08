//! Thread domain entity (`Thread`) and durable sidebar presentation structures and formatting rules.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::message::ChatMessage;
use crate::clock;
use crate::protocol::messaging::unread::UnreadState;
use crate::protocol::wire::{pick_str, val_u64};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Thread {
    #[serde(default)]
    pub presentation: SidebarPresentation,
    pub id: String,
    pub name: String,
    pub preview: String,
    pub time: String,
    pub avatar: Option<String>,
    pub is_group: bool,
    pub messages: Vec<ChatMessage>,
    #[serde(default)]
    pub unread: UnreadState,
}

/// Durable sidebar metadata, including metadata-only startup snapshots.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct SidebarPresentation {
    pub members: Vec<SidebarMember>,
    /// Server `totalMember` only; never inferred from message senders or avatar samples.
    pub member_count: Option<u64>,
    pub last_message_ts: i64,
    pub last_message_id: String,
    pub last_sender: String,
    pub last_sender_id: String,
    pub last_outgoing: bool,
    pub pinned: bool,
    pub muted: bool,
    pub hidden: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct SidebarMember {
    pub id: String,
    pub name: String,
    pub avatar: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SidebarFontStyle {
    Regular,
    Semibold,
}

impl Thread {
    pub fn sidebar_font_style(&self) -> SidebarFontStyle {
        if self.unread.has_badge() {
            SidebarFontStyle::Semibold
        } else {
            SidebarFontStyle::Regular
        }
    }

    pub fn refresh_sidebar_presentation(&mut self) {
        if let Some(message) = self
            .messages
            .iter()
            .max_by_key(|m| clock::normalize_ts_ms(m.ts))
        {
            let ts = clock::normalize_ts_ms(message.ts);
            if ts >= clock::normalize_ts_ms(self.presentation.last_message_ts) {
                let sender_id = sidebar_sender_id(message)
                    .or_else(|| {
                        (message.id == self.presentation.last_message_id
                            && ts == clock::normalize_ts_ms(self.presentation.last_message_ts))
                        .then(|| self.presentation.last_sender_id.clone())
                    })
                    .unwrap_or_default();
                let keep_resolved = message.id == self.presentation.last_message_id
                    && ts == clock::normalize_ts_ms(self.presentation.last_message_ts)
                    && sender_id == self.presentation.last_sender_id
                    && !self.presentation.last_sender.is_empty()
                    && !self
                        .presentation
                        .last_sender
                        .bytes()
                        .all(|b| b.is_ascii_digit())
                    && message.sender.bytes().all(|b| b.is_ascii_digit());
                self.presentation.last_message_ts = message.ts;
                self.presentation.last_message_id = message.id.clone();
                if !keep_resolved {
                    self.presentation.last_sender = message.sender.clone();
                }
                self.presentation.last_sender_id = sender_id;
                self.presentation.last_outgoing = message.is_outgoing;
            }
        }
    }

    pub fn sidebar_preview(&self) -> String {
        let latest = self
            .messages
            .iter()
            .max_by_key(|m| clock::normalize_ts_ms(m.ts))
            .filter(|m| {
                clock::normalize_ts_ms(m.ts)
                    > clock::normalize_ts_ms(self.presentation.last_message_ts)
                    || self.presentation.last_sender.is_empty()
            });
        let (sender, outgoing) = latest
            .map(|m| (m.sender.as_str(), m.is_outgoing))
            .unwrap_or((
                &self.presentation.last_sender,
                self.presentation.last_outgoing,
            ));
        let sender = self
            .presentation
            .members
            .iter()
            .find(|member| member.id == sender && !member.name.trim().is_empty())
            .map(|member| member.name.as_str())
            .unwrap_or(sender);
        let text = self.preview.replace('\n', " ");
        if text.is_empty() {
            return text;
        }
        if outgoing {
            if text.starts_with("Bạn: ") {
                text
            } else {
                format!("Bạn: {text}")
            }
        } else if self.is_group
            && !sender.trim().is_empty()
            && !sender.trim().chars().all(|c| c.is_ascii_digit())
        {
            let prefix = format!("{}: ", sender.trim());
            if text.starts_with(&prefix) {
                text
            } else {
                format!("{prefix}{text}")
            }
        } else {
            text
        }
    }

    pub fn sidebar_activity_ts(&self) -> i64 {
        self.messages
            .iter()
            .map(|m| clock::normalize_ts_ms(m.ts))
            .chain(std::iter::once(clock::normalize_ts_ms(
                self.presentation.last_message_ts,
            )))
            .max()
            .unwrap_or(0)
    }

    pub fn sidebar_timestamp(&self, now_ms: i64) -> String {
        let ts = self.sidebar_activity_ts();
        if ts > 0 {
            clock::sidebar_time_at(ts, now_ms)
        } else {
            self.time.clone()
        }
    }

    /// Parse only group roster metadata observed in the original `getmg-v2` consumer.
    pub fn apply_group_sidebar_metadata(&mut self, info: &Value) {
        if let Some(count) = val_u64(info.get("totalMember")) {
            self.presentation.member_count = Some(count);
        }
        let mut ids = Vec::new();
        if let Some(list) = info.get("memVerList").and_then(Value::as_array) {
            for item in list.iter().filter_map(Value::as_str) {
                let id = item.split('_').next().unwrap_or_default();
                if !id.is_empty() && !ids.iter().any(|s| s == id) {
                    ids.push(id.to_owned());
                }
            }
        } else if let Some(map) = info.get("memberVerMap").and_then(Value::as_object) {
            ids.extend(map.keys().cloned());
        }
        let mut leaders = Vec::new();
        if let Some(creator) = pick_str(info, &["creatorId"]) {
            leaders.push(creator);
        }
        if let Some(admins) = info.get("adminIds").and_then(Value::as_array) {
            leaders.extend(admins.iter().filter_map(Value::as_str).map(str::to_owned));
        }
        let mut ordered = Vec::new();
        for id in leaders.into_iter().chain(ids.iter().cloned()) {
            if ids.contains(&id) && !ordered.contains(&id) {
                ordered.push(id);
            }
        }
        ids = ordered;
        let top = info.get("topMember").and_then(Value::as_array);
        if let Some(top) = top {
            for profile in top {
                if let Some(id) = pick_str(profile, &["id", "userId", "uid"]) {
                    if !ids.contains(&id) {
                        ids.push(id);
                    }
                }
            }
        }
        if !ids.is_empty() {
            let previous = std::mem::take(&mut self.presentation.members);
            self.presentation.members =
                ids.into_iter()
                    .map(|id| {
                        let profile = top.and_then(|list| {
                            list.iter().find(|p| {
                                pick_str(p, &["id", "userId", "uid"]).as_ref() == Some(&id)
                            })
                        });
                        let mut member = previous.iter().find(|m| m.id == id).cloned().unwrap_or(
                            SidebarMember {
                                id,
                                ..Default::default()
                            },
                        );
                        if let Some(profile) = profile {
                            if let Some(name) =
                                pick_str(profile, &["displayName", "zaloName", "name"])
                            {
                                member.name = name;
                            }
                            if let Some(avatar) = pick_str(profile, &["avatar", "avt"]) {
                                member.avatar = Some(avatar);
                            }
                        }
                        member
                    })
                    .collect();
        }
    }
}

pub fn sidebar_sender_id(message: &ChatMessage) -> Option<String> {
    let raw = message.wire_ref.sender.as_ref().or_else(|| {
        message
            .history_source
            .as_ref()
            .and_then(|v| v.get("senderId"))
    })?;
    let id = match raw {
        Value::String(v) => v.clone(),
        Value::Number(v) if v.is_u64() => v.to_string(),
        _ => return None,
    };
    (!id.is_empty() && id != "0" && id.bytes().all(|b| b.is_ascii_digit())).then_some(id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::messaging::model::{DeliveryState, MessageKind};

    #[test]
    fn legacy_message_and_thread_defaults_roundtrip() {
        let message: ChatMessage = serde_json::from_value(serde_json::json!({
            "id": "msg-0001", "sender": "user-1", "text": "Hello",
            "time": "", "is_outgoing": false, "ts": 1
        }))
        .unwrap();
        assert_eq!(message.kind, MessageKind::Unknown);
        assert_eq!(message.delivery, DeliveryState::default());

        let thread: Thread = serde_json::from_value(serde_json::json!({
            "id": "thread-1", "name": "Example", "preview": "", "time": "",
            "avatar": null, "is_group": false, "messages": []
        }))
        .unwrap();
        assert_eq!(thread.presentation, SidebarPresentation::default());
        assert_eq!(thread.unread, UnreadState::default());
    }
}
