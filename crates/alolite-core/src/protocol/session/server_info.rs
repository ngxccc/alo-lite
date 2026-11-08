//! Server configuration application, credential expiration tracking, and profile synchronization.

use serde_json::Value;

use super::domains;
use super::status::SessionStatus;
use super::Session;

impl Session {
    /// The stored session is already known to be refused by the server (#21).
    pub fn is_expired_marked(&self) -> bool {
        self.expired_at.is_some()
    }

    /// Record that the server refused these credentials.
    pub fn mark_expired(&mut self) {
        self.expired_at = Some(crate::clock::unix_now());
    }

    /// A fresh login succeeded: the marker must not leak into the new session.
    pub fn clear_expired(&mut self) {
        self.expired_at = None;
    }

    /// Ask the server whether the stored session still works (#21).
    pub async fn session_status(&self) -> SessionStatus {
        if self.login.is_none() || self.zpw_sek.is_none() {
            return SessionStatus::Expired;
        }
        let body = serde_json::json!({
            "avatar_size": 160,
            "imei": self.imei,
        });
        let url = match self.encoded_params(&body) {
            Ok(enc) => format!(
                "{}/api/social/profile/me-v2?{}&params={enc}&os=8",
                self.domains.profile,
                self.common_params()
            ),
            Err(_) => format!(
                "{}/api/social/profile/me-v2?{}",
                self.domains.profile,
                self.common_params()
            ),
        };
        let resp = match self.get(&url).await {
            Ok(r) => r,
            Err(e) => return SessionStatus::from_error(&e),
        };
        let dec = self.maybe_decrypt(resp, self.session_key_bytes()).await;
        match dec.get("error_code").and_then(Value::as_i64) {
            Some(code) => SessionStatus::from_error_code(code),
            None => SessionStatus::Unreachable,
        }
    }

    /// Fetch the authenticated user's own numeric `userId` and `displayName` from `/api/social/profile/me-v2`.
    pub async fn sync_profile_me(&mut self) -> Option<(String, String)> {
        if self.login.is_none() || self.zpw_sek.is_none() {
            return None;
        }
        let body = serde_json::json!({
            "avatar_size": 160,
            "imei": self.imei,
        });
        let enc = self.encoded_params(&body).ok()?;
        let url = format!(
            "{}/api/social/profile/me-v2?{}&params={enc}&os=8",
            self.domains.profile,
            self.common_params()
        );
        let resp = self.get(&url).await.ok()?;
        let dec = self.maybe_decrypt(resp, self.session_key_bytes()).await;
        let profile = dec
            .get("data")
            .and_then(|d| d.get("profile"))
            .or_else(|| dec.get("profile"))?;
        let uid = profile
            .get("userId")
            .or_else(|| profile.get("uid"))
            .and_then(|v| match v {
                Value::String(s) if !s.is_empty() => Some(s.clone()),
                Value::Number(n) => Some(n.to_string()),
                _ => None,
            })?;
        let name = profile
            .get("displayName")
            .or_else(|| profile.get("zaloName"))
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        let avatar = profile
            .get("avatar")
            .or_else(|| profile.get("avt"))
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        if let Some(ref mut li) = self.login {
            li.uin = uid.clone();
            if !name.is_empty() {
                li.display_name = name.clone();
            }
            if !avatar.is_empty() {
                li.avatar = avatar;
            }
        }
        Some((uid, name))
    }

    /// Re-apply the endpoint table from a `getServerInfo` / `getLoginInfo` payload.
    pub fn apply_server_info(&mut self, data: &Value) {
        if let Some(v) = data.get("zpw_chts_new").and_then(domains::first_url) {
            self.domains.lp_chat = v;
        }
        if let Some(v) = data.get("zpw_ctrs_new").and_then(domains::first_url) {
            self.domains.lp_control = v;
        }
        if let Some(arr) = data.get("zpw_ws").and_then(Value::as_array) {
            let list: Vec<String> = arr
                .iter()
                .filter_map(Value::as_str)
                .map(|s| s.trim_end_matches('/').to_string())
                .filter(|s| s.starts_with("wss://") || s.starts_with("ws://"))
                .collect();
            if !list.is_empty() {
                self.domains.ws = list;
            }
        }
        if let Some(map) = data
            .get("zpw_service_map_v3")
            .or_else(|| data.get("zpw_service_map_new"))
        {
            self.domains.apply_service_map(map);
        }
        if let Some(v) = data
            .get("setttings")
            .and_then(|v| v.get("authen"))
            .and_then(|v| v.get("trusted_device"))
            .and_then(|v| v.get("qr_format_trust_device"))
            .and_then(Value::as_i64)
        {
            self.qr_format_trust_device = v == 1;
        }
    }
}
