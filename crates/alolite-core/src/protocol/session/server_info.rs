//! Server configuration application, credential expiration tracking, and profile synchronization.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::domains;
use super::status::SessionStatus;
use super::Session;
use crate::protocol::wire::WireData;

/// Response envelope of `/api/social/profile/me-v2` (cmd `11139`) when probing session validity.
///
/// # Wire Response Example
/// ```json
/// { "error_code": 0, "error_message": "Successful", "data": { "profile": { "userId": "1234567890" } } }
/// ```
///
/// # Core Transformation
/// `error_code` is mapped to [`SessionStatus`] via `SessionStatus::from_error_code`; a missing
/// `error_code` means the endpoint was unreachable.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct SessionProbeWireData {
    #[serde(rename = "error_code")]
    pub error_code: Option<Value>,
}

impl SessionProbeWireData {
    /// Numeric error code, ignoring non-numeric forms (mirrors `Value::as_i64`).
    pub fn error_code(&self) -> Option<i64> {
        self.error_code.as_ref().and_then(Value::as_i64)
    }
}

/// The `profile` object of `/api/social/profile/me-v2` (cmd `11139`).
///
/// # Wire Response Example
/// ```json
/// { "userId": "1234567890", "displayName": "Ngọc Trần", "avatar": "https://s120-ava-talk.zadn.vn/a.jpg" }
/// ```
///
/// # Core Transformation
/// Supplies the authenticated user's numeric id and display name used to refresh [`super::LoginInfo`].
/// Key priority mirrors the shipped client: `userId` > `uid`, `displayName` > `zaloName`, `avatar` > `avt`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ProfileMeWireData {
    #[serde(rename = "userId")]
    pub user_id: Option<Value>,
    #[serde(rename = "uid")]
    pub uid: Option<Value>,
    #[serde(rename = "displayName")]
    pub display_name: Option<Value>,
    #[serde(rename = "zaloName")]
    pub zalo_name: Option<Value>,
    #[serde(rename = "avatar")]
    pub avatar_url: Option<Value>,
    #[serde(rename = "avt")]
    pub avt: Option<Value>,
}

impl ProfileMeWireData {
    /// Numeric `userId` (`userId` then `uid`), accepting string or numeric wire forms.
    pub fn uid(&self) -> Option<String> {
        [&self.user_id, &self.uid]
            .into_iter()
            .find_map(|v| match v.as_ref()? {
                Value::String(s) if !s.is_empty() => Some(s.clone()),
                Value::Number(n) => Some(n.to_string()),
                _ => None,
            })
    }

    /// Display name (`displayName` then `zaloName`); empty string when absent.
    pub fn display_name(&self) -> String {
        [&self.display_name, &self.zalo_name]
            .into_iter()
            .find_map(|v| v.as_ref().and_then(Value::as_str))
            .unwrap_or("")
            .to_string()
    }

    /// Avatar URL (`avatar` then `avt`); empty string when absent.
    pub fn avatar(&self) -> String {
        [&self.avatar_url, &self.avt]
            .into_iter()
            .find_map(|v| v.as_ref().and_then(Value::as_str))
            .unwrap_or("")
            .to_string()
    }
}

/// `setttings` object carrying the auth and trusted-device configuration (note upstream typo `setttings`).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ServerAuthenSettingsWireData {
    #[serde(rename = "authen")]
    pub authen: Option<ServerAuthenWireData>,
}

/// `authen` block inside `setttings`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ServerAuthenWireData {
    #[serde(rename = "trusted_device")]
    pub trusted_device: Option<ServerTrustedDeviceWireData>,
}

/// `trusted_device` block carrying QR format settings.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ServerTrustedDeviceWireData {
    #[serde(rename = "qr_format_trust_device")]
    pub qr_format_trust_device: Option<Value>,
}

/// Payload of `getServerInfo` / `getLoginInfo` carrying the endpoint table and auth settings.
///
/// # Wire Response Example
/// ```json
/// {
///   "zpw_chts_new": ["https://p4-msg-chat.zaloapp.com"],
///   "zpw_ctrs_new": ["https://p4-ctl-chat.zaloapp.com"],
///   "zpw_ws": ["wss://ws1-msg.chat.zalo.me"],
///   "zpw_service_map_v3": { "chat": ["https://tt-chat3-wpa.api.zaloapp.com"] },
///   "setttings": { "authen": { "trusted_device": { "qr_format_trust_device": 1 } } }
/// }
/// ```
///
/// # Core Transformation
/// Re-applies the long-polling, WebSocket, and service-map endpoints onto [`super::Domains`], and
/// toggles [`Session::qr_format_trust_device`]. Fields stay `Value` so a single malformed entry
/// cannot drop the rest of the table.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ServerInfoWireData {
    #[serde(rename = "zpw_chts_new")]
    pub zpw_chts_new: Option<Value>,
    #[serde(rename = "zpw_ctrs_new")]
    pub zpw_ctrs_new: Option<Value>,
    #[serde(rename = "zpw_ws")]
    pub zpw_ws: Option<Value>,
    #[serde(rename = "zpw_service_map_v3")]
    pub zpw_service_map_v3: Option<Value>,
    #[serde(rename = "zpw_service_map_new")]
    pub zpw_service_map_new: Option<Value>,
    #[serde(rename = "setttings")]
    pub settings: Option<ServerAuthenSettingsWireData>,
}

impl ServerInfoWireData {
    /// WebSocket shard list from `zpw_ws`, stripped of trailing slashes and filtered to ws/wss URLs.
    pub fn ws_endpoints(&self) -> Vec<String> {
        self.zpw_ws
            .as_ref()
            .and_then(Value::as_array)
            .map(|arr| {
                arr.iter()
                    .filter_map(Value::as_str)
                    .map(|s| s.trim_end_matches('/').to_string())
                    .filter(|s| s.starts_with("wss://") || s.starts_with("ws://"))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// The service map, preferring `zpw_service_map_v3` over `zpw_service_map_new`.
    pub fn service_map(&self) -> Option<&Value> {
        self.zpw_service_map_v3
            .as_ref()
            .or(self.zpw_service_map_new.as_ref())
    }

    /// `setttings.authen.trusted_device.qr_format_trust_device` (note the upstream typo `setttings`).
    pub fn trusted_device_qr_format(&self) -> Option<i64> {
        let st = self.settings.as_ref()?;
        let auth = st.authen.as_ref()?;
        let td = auth.trusted_device.as_ref()?;
        td.qr_format_trust_device.as_ref().and_then(Value::as_i64)
    }
}

/// Response envelope of `GET /api/login/getServerInfo`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ServerInfoResponseWireData {
    #[serde(rename = "error_code")]
    pub error_code: Option<Value>,
    #[serde(rename = "error_message")]
    pub error_message: Option<Value>,
    #[serde(rename = "data")]
    pub data: Option<ServerInfoWireData>,
}

impl ServerInfoResponseWireData {
    /// Numeric error code (`0` for success).
    pub fn code(&self) -> i64 {
        self.error_code
            .as_ref()
            .and_then(Value::as_i64)
            .unwrap_or(0)
    }

    /// Server error message.
    pub fn message(&self) -> &str {
        crate::protocol::wire::opt_text(self.error_message.as_ref())
    }
}

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
        let probe: SessionProbeWireData = serde_json::from_value(dec.clone()).unwrap_or_default();
        match probe.error_code() {
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
        let profile_value = dec
            .get("data")
            .and_then(|d| d.get("profile"))
            .or_else(|| dec.get("profile"))?;
        let profile: ProfileMeWireData =
            serde_json::from_value(profile_value.clone()).unwrap_or_default();
        let uid = profile.uid()?;
        let name = profile.display_name();
        let avatar = profile.avatar();
        if let Some(li) = &mut self.login {
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

    /// Re-apply the endpoint table from a decoded [`ServerInfoWireData`].
    pub fn apply_server_info_wire(&mut self, info: &ServerInfoWireData) {
        if let Some(host) = info.zpw_chts_new.as_ref().and_then(domains::first_url) {
            self.domains.lp_chat = host;
        }
        if let Some(host) = info.zpw_ctrs_new.as_ref().and_then(domains::first_url) {
            self.domains.lp_control = host;
        }
        let ws_list = info.ws_endpoints();
        if !ws_list.is_empty() {
            self.domains.ws = ws_list;
        }
        if let Some(map) = info.service_map() {
            self.domains.apply_service_map(map);
        }
        if let Some(flag) = info.trusted_device_qr_format() {
            self.qr_format_trust_device = flag == 1;
        }
    }

    /// Re-apply the endpoint table from a `getServerInfo` / `getLoginInfo` payload.
    pub fn apply_server_info(&mut self, data: &Value) {
        let info = ServerInfoWireData::from_value(data);
        self.apply_server_info_wire(&info);
    }
}
