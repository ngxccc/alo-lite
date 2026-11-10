//! Service-domain routing table, verified-host healing, and multi-host failover candidates.
//!
//! Resolves ordered host candidates for every [`crate::protocol::registry::ApiDomain`] while
//! preventing stale `*.zaloapp.com` login maps from overriding live-verified `*.chat.zalo.me` hosts.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Domain routing table for a [`super::Session`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Domains {
    /// `wpa.chat.zalo.me` — `reqqr`, password login.
    pub auth: String,
    /// `wpa.chat.zalo.me` — `getServerInfo` / `getLoginInfo` once a session exists.
    pub wpa_chat: String,
    /// `zpw_service_map_v3.chat[0]` — message send (`tt-chat*-wpa.*`).
    pub chat: String,
    /// `zpw_service_map_v3.conversation[0]` — conversation control (pins, preload).
    pub conversation: String,
    /// `zpw_chts_new[0]` — HTTP long-polling message receiver (`getChatFromPolling`).
    #[serde(default = "default_lp_chat")]
    pub lp_chat: String,
    /// `zpw_ctrs_new[0]` — HTTP long-polling control receiver (`getControllFromPolling`).
    #[serde(default = "default_lp_control")]
    pub lp_control: String,
    /// `zpw_service_map_v3.friend[0]` — `social/friend/getfriends`.
    pub friend: String,
    /// `zpw_service_map_v3.conversation[0]` — `conv/*`, `hiddenconvers/*`.
    pub convers_api: String,
    /// `zpw_service_map_v3.profile[0]` — `getprofiles`, `me-v2`.
    pub profile: String,
    pub group: String,
    pub file: String,
    /// `zpw_service_map_v3.group_cloud_message[0]` — `/api/cm/getoldv2`, `/api/cm/getrecentv2`.
    #[serde(default = "default_cloud_msg")]
    pub cloud_msg: String,
    /// `zpw_service_map_v3.voice_call[0]` — `/api/voicecall/*` signaling endpoints.
    #[serde(default = "default_voice_call")]
    pub voice_call: String,
    /// `zpw_service_map_v3.sticker[0]` — `/api/message/sticker/*` catalog & search endpoints.
    #[serde(default = "default_sticker")]
    pub sticker: String,
    /// `zpw_ws[...]` — control websocket endpoints (`wss://wsN-msg.chat.zalo.me`).
    #[serde(default = "default_ws")]
    pub ws: Vec<String>,
    /// Candidate hosts returned by `zpw_service_map_v3` for each endpoint (#27).
    #[serde(skip)]
    pub service_map_candidates: BTreeMap<String, Vec<String>>,
    /// Candidate hosts read from `session.json` before `heal_from_verified` replaced primary slots (#27).
    #[serde(skip)]
    pub saved_candidates: BTreeMap<String, String>,
    /// Winning host per endpoint remembered in-memory for the current session (#27).
    #[serde(skip)]
    pub preferred_host: Arc<Mutex<BTreeMap<String, String>>>,
}

pub(super) fn default_lp_chat() -> String {
    crate::endpoints::LP_CHAT_DEFAULT.into()
}

pub(super) fn default_lp_control() -> String {
    crate::endpoints::LP_CONTROL_DEFAULT.into()
}

pub(super) fn default_cloud_msg() -> String {
    crate::endpoints::GROUP_CLOUD_CANDIDATES[1].into()
}

pub(super) fn default_voice_call() -> String {
    crate::endpoints::service("voice_call")
        .unwrap_or_default()
        .into()
}

pub(super) fn default_sticker() -> String {
    crate::endpoints::service("sticker")
        .unwrap_or_default()
        .into()
}

/// \[O\] `re-notes/02-chat-data.md`: `zpw_ws` = `wss://ws{1..8}-msg.chat.zalo.me`.
pub(super) fn default_ws() -> Vec<String> {
    crate::endpoints::default_ws_endpoints()
}

impl Default for Domains {
    fn default() -> Self {
        let mut domains = Self {
            auth: crate::endpoints::AUTH.into(),
            wpa_chat: crate::endpoints::WPA_CHAT.into(),
            chat: crate::endpoints::MSG_CHAT.into(),
            conversation: crate::endpoints::CONVERS_CTL.into(),
            lp_chat: default_lp_chat(),
            lp_control: default_lp_control(),
            friend: crate::endpoints::service("friend")
                .unwrap_or_default()
                .into(),
            convers_api: crate::endpoints::service("conversation")
                .unwrap_or_default()
                .into(),
            profile: crate::endpoints::service("profile")
                .unwrap_or_default()
                .into(),
            group: crate::endpoints::service("group")
                .unwrap_or_default()
                .into(),
            file: crate::endpoints::service("file").unwrap_or_default().into(),
            cloud_msg: default_cloud_msg(),
            voice_call: default_voice_call(),
            sticker: default_sticker(),
            ws: default_ws(),
            service_map_candidates: BTreeMap::new(),
            saved_candidates: BTreeMap::new(),
            preferred_host: Arc::new(Mutex::new(BTreeMap::new())),
        };
        domains.heal_from_verified();
        domains
    }
}

/// First entry of a `zpw_service_map_v3` value, which can be either an array or a string.
pub(super) fn first_url(v: &Value) -> Option<String> {
    all_urls(v).into_iter().next()
}

/// All normalized URLs in a `zpw_service_map_v3` entry (array or string).
fn all_urls(v: &Value) -> Vec<String> {
    if let Some(arr) = v.as_array() {
        arr.iter()
            .filter_map(Value::as_str)
            .map(|s| s.trim_end_matches('/').to_string())
            .filter(|s| !s.is_empty())
            .collect()
    } else if let Some(s) = v.as_str() {
        let trimmed = s.trim_end_matches('/').to_string();
        if trimmed.is_empty() {
            Vec::new()
        } else {
            vec![trimmed]
        }
    } else {
        Vec::new()
    }
}

fn push_unique_host(out: &mut Vec<String>, candidate: &str) {
    let trimmed = candidate.trim().trim_end_matches('/');
    if !trimmed.is_empty() && !out.iter().any(|existing| existing == trimmed) {
        out.push(trimmed.to_string());
    }
}

impl Domains {
    /// Restore every endpoint that has a **verified** entry in [`crate::endpoints::SERVICE_MAP_V3`],
    /// overriding whatever a previous run persisted while keeping the previous host in `saved_candidates` (#27).
    pub fn heal_from_verified(&mut self) {
        use crate::endpoints::service;

        if let Some(v) = service("chat") {
            let prev = self.chat.trim_end_matches('/').to_string();
            if !prev.is_empty() && !prev.contains("-msg-chat.") {
                self.saved_candidates.insert("chat".into(), prev);
            }
            self.chat = v.into();
        }
        if let Some(v) = service("conversation") {
            let prev = self.convers_api.trim_end_matches('/').to_string();
            if !prev.is_empty() && !prev.contains("-ctl-chat.") {
                self.saved_candidates.insert("conversation".into(), prev);
            }
            self.conversation = v.into();
            self.convers_api = v.into();
        }
        for (key, slot) in [
            ("friend", &mut self.friend),
            ("profile", &mut self.profile),
            ("group", &mut self.group),
            ("file", &mut self.file),
            ("voice_call", &mut self.voice_call),
            ("sticker", &mut self.sticker),
        ] {
            if let Some(v) = service(key) {
                let prev = slot.trim_end_matches('/').to_string();
                if !prev.is_empty() {
                    self.saved_candidates.insert(key.into(), prev);
                }
                *slot = v.into();
            }
        }
    }

    /// Apply `zpw_service_map_v3` **without** overriding verified primary slots,
    /// while recording all server-returned hosts in `service_map_candidates` (#27).
    pub fn apply_service_map(&mut self, map: &Value) {
        if let Some(obj) = map.as_object() {
            for (k, val) in obj {
                let urls = all_urls(val);
                if !urls.is_empty() {
                    self.service_map_candidates.insert(k.clone(), urls);
                }
            }
        }
        for (key, slot) in [
            ("chat", &mut self.chat),
            ("friend", &mut self.friend),
            ("conversation", &mut self.convers_api),
            ("profile", &mut self.profile),
            ("group", &mut self.group),
            ("file", &mut self.file),
            ("group_cloud_message", &mut self.cloud_msg),
            ("voice_call", &mut self.voice_call),
            ("sticker", &mut self.sticker),
        ] {
            let urls = map.get(key).map(all_urls).unwrap_or_default();
            if !urls.is_empty() {
                self.service_map_candidates.insert(key.into(), urls.clone());
            }
            let Some(v) = urls.into_iter().next() else {
                continue;
            };
            let verified = crate::endpoints::service(key).is_some();
            if !verified || slot.is_empty() {
                *slot = v;
            }
        }
        if crate::endpoints::service("conversation").is_none() {
            if let Some(v) = map.get("conversation").and_then(first_url) {
                self.conversation = v;
            }
        }
    }

    /// Primary slot value for a named endpoint key.
    fn slot_for(&self, endpoint: &str) -> &str {
        match endpoint {
            "chat" => &self.chat,
            "friend" => &self.friend,
            "conversation" | "convers_api" => &self.convers_api,
            "profile" => &self.profile,
            "group" => &self.group,
            "file" => &self.file,
            "group_cloud_message" | "cloud_msg" => &self.cloud_msg,
            "voice_call" => &self.voice_call,
            "sticker" => &self.sticker,
            "auth" => &self.auth,
            "wpa_chat" => &self.wpa_chat,
            _ => "",
        }
    }

    /// Ordered candidate hosts for `endpoint` (#27).
    pub fn candidates(&self, endpoint: &str) -> Vec<String> {
        let canonical = if endpoint == "convers_api" {
            "conversation"
        } else if endpoint == "cloud_msg" {
            "group_cloud_message"
        } else {
            endpoint
        };
        let mut list = Vec::new();
        if let Ok(pref) = self.preferred_host.lock() {
            if let Some(winner) = pref.get(canonical) {
                push_unique_host(&mut list, winner);
            }
        }
        push_unique_host(&mut list, self.slot_for(canonical));
        if let Some(v) = crate::endpoints::service(canonical) {
            push_unique_host(&mut list, v);
        }
        if let Some(mapped) = self.service_map_candidates.get(canonical) {
            for u in mapped {
                push_unique_host(&mut list, u);
            }
        }
        if let Some(saved) = self.saved_candidates.get(canonical) {
            push_unique_host(&mut list, saved);
        }
        if canonical == "group_cloud_message" {
            for &host in crate::endpoints::GROUP_CLOUD_CANDIDATES {
                push_unique_host(&mut list, host);
            }
        }
        if canonical == "friend" {
            push_unique_host(&mut list, &self.profile);
        }
        if let Some(fb) = crate::endpoints::fallback_host(canonical) {
            push_unique_host(&mut list, fb);
        }
        list
    }

    /// Remember the winning host for `endpoint` for the remainder of the session (#27).
    pub fn remember_winner(&self, endpoint: &str, host: &str) {
        let canonical = if endpoint == "convers_api" {
            "conversation"
        } else if endpoint == "cloud_msg" {
            "group_cloud_message"
        } else {
            endpoint
        };
        let trimmed = host.trim().trim_end_matches('/');
        if !trimmed.is_empty() {
            if let Ok(mut pref) = self.preferred_host.lock() {
                pref.insert(canonical.to_string(), trimmed.to_string());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn server_map_cannot_override_verified_endpoints() {
        let mut domains = Domains::default();
        domains.apply_service_map(&serde_json::json!({
            "friend": ["https://tt-friend-wpa.zaloapp.com"],
            "profile": "https://tt-profile-wpa.zaloapp.com",
            "chat": "https://tt-chat9-wpa.api.zaloapp.com",
        }));

        assert_eq!(domains.friend, "https://tt-friend-wpa.chat.zalo.me");
        assert_eq!(domains.profile, "https://tt-profile-wpa.chat.zalo.me");
        assert_eq!(domains.chat, crate::endpoints::service("chat").unwrap());
    }

    #[test]
    fn server_map_fills_endpoints_without_a_verified_constant() {
        let mut domains = Domains::default();
        domains.apply_service_map(&serde_json::json!({
            "group_cloud_message": "https://tt-group-cm.example.test/",
        }));

        assert_eq!(domains.cloud_msg, "https://tt-group-cm.example.test");
    }

    #[test]
    fn server_map_fills_an_empty_verified_slot() {
        let mut domains = Domains::default();
        domains.sticker.clear();
        domains.apply_service_map(&serde_json::json!({"sticker": "https://from-server.test"}));

        assert_eq!(domains.sticker, "https://from-server.test");
    }

    #[test]
    fn heal_restores_every_verified_domain() {
        let mut domains = Domains::default();
        domains.chat = "https://p4-msg-chat.zaloapp.com".into();
        domains.conversation = "https://p4-ctl-chat.zaloapp.com".into();
        domains.convers_api = "https://tt-convers-wpa.zaloapp.com/".into();
        domains.friend = "https://tt-friend-wpa.zaloapp.com".into();
        domains.profile = "https://tt-profile-wpa.zaloapp.com".into();
        domains.group = String::new();
        domains.file = "https://wrong.example/".into();
        domains.voice_call = String::new();
        domains.sticker = String::new();

        domains.heal_from_verified();

        for (key, actual) in [
            ("friend", &domains.friend),
            ("profile", &domains.profile),
            ("group", &domains.group),
            ("file", &domains.file),
            ("voice_call", &domains.voice_call),
            ("sticker", &domains.sticker),
            ("chat", &domains.chat),
            ("conversation", &domains.conversation),
            ("conversation", &domains.convers_api),
        ] {
            assert_eq!(
                actual.as_str(),
                crate::endpoints::service(key).unwrap(),
                "endpoint {key} was not healed"
            );
        }
    }

    #[test]
    fn heal_leaves_server_owned_endpoints_untouched() {
        let mut domains = Domains::default();
        domains.lp_chat = "https://p9-msg-chat.zaloapp.com".into();
        domains.lp_control = "https://p9-ctl-chat.zaloapp.com".into();
        domains.cloud_msg = "https://tt-group-cm.example.test".into();
        domains.ws = vec!["wss://ws99-msg.chat.zalo.me".into()];

        domains.heal_from_verified();

        assert_eq!(domains.lp_chat, "https://p9-msg-chat.zaloapp.com");
        assert_eq!(domains.lp_control, "https://p9-ctl-chat.zaloapp.com");
        assert_eq!(domains.cloud_msg, "https://tt-group-cm.example.test");
        assert_eq!(domains.ws, vec!["wss://ws99-msg.chat.zalo.me"]);
    }

    #[test]
    fn candidates_preserve_priority_order_verified_then_service_map_then_saved() {
        let mut domains = Domains::default();
        domains.chat = "https://legacy-saved-chat.zaloapp.com/".into();
        domains.heal_from_verified();
        domains.apply_service_map(&serde_json::json!({
            "chat": [
                "https://tt-chat4-wpa.api.zaloapp.com/",
                "https://tt-chat5-wpa.api.zaloapp.com"
            ]
        }));

        let list = domains.candidates("chat");
        assert_eq!(
            list,
            vec![
                "https://tt-chat3-wpa.chat.zalo.me",
                "https://tt-chat4-wpa.api.zaloapp.com",
                "https://tt-chat5-wpa.api.zaloapp.com",
                "https://legacy-saved-chat.zaloapp.com",
            ]
        );
    }
}
