//! Persistent session credentials (`session.json`) and stable device identity (`device.json`).
//!
//! Uses `write_private_atomic` (`0600`) so secrets never touch world-readable files and
//! preserves `imei` + `known_uids` across logouts to avoid "unknown device" warnings.

use std::sync::Arc;

use anyhow::Result;
use reqwest::cookie::Jar;
use serde::{Deserialize, Serialize};

use super::domains::{default_lp_chat, default_lp_control, default_ws, Domains};
use super::{build_http_client, Session, ZALO_UA};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LoginInfo {
    pub uin: String,
    /// `zpw_enk` — base64 AES-256 key for parameter encryption.
    pub zpw_enk: String,
    /// `zpw_sek` — session authentication cookie token.
    pub zpw_sek: String,
    pub display_name: String,
    pub send2me_id: String,
    #[serde(default)]
    pub avatar: String,
}

/// Persistent non-secret device identity (`z_uuid` + previously logged-in UIDs) retained across
/// logouts so Zalo recognizes subsequent logins on the same computer (`re-notes/10` §1.2.3).
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct DeviceIdentity {
    pub imei: String,
    pub computer_name: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub known_uids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavedSession {
    pub imei: String,
    pub computer_name: String,
    pub secret_key_b64: Option<String>,
    pub session_key: Option<String>,
    pub zpw_sek: Option<String>,
    pub login: Option<LoginInfo>,
    pub domains: Domains,
    /// Epoch seconds at which the server last refused these credentials (#21).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expired_at: Option<i64>,
    /// Optional HTTP/HTTPS/SOCKS5 proxy URL configured for this account/profile (#64).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub proxy_url: Option<String>,
    /// Companion identity public key (base64) when linked via Trusted Device flow.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub companion_ik: Option<String>,
    /// Companion identity private key (base64) when linked via Trusted Device flow.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub companion_priv: Option<String>,
    /// Master UID of the primary account that linked this companion.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub companion_master_id: Option<String>,
    /// Link ID assigned by the primary account.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub companion_link_id: Option<u64>,
    /// Previously authenticated UIDs on this device, sent as `logged_uids` during re-login.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub known_uids: Vec<String>,
}

/// Device name reported as `computer_name` in every request.
pub fn detect_device_name() -> String {
    if let Ok(name) = std::env::var("ZALO_DEVICE_NAME") {
        if !name.trim().is_empty() {
            return name.trim().to_string();
        }
    }
    let host = std::env::var("HOSTNAME")
        .or_else(|_| std::env::var("HOST"))
        .or_else(|_| std::env::var("COMPUTERNAME"))
        .ok()
        .filter(|h| !h.trim().is_empty())
        .or_else(|| {
            std::fs::read_to_string("/etc/hostname")
                .ok()
                .map(|s| s.trim().to_string())
                .filter(|h| !h.is_empty())
        });
    host.unwrap_or_else(|| "alolite".to_string())
}

/// Standard official UUID-like IMEI format (`<uuid4>-<md5(ua)>`, 69 chars) used by Zalo PC.
pub fn official_imei() -> String {
    let hex = crate::param_cipher::random_hex(32);
    let variant = ['8', '9', 'a', 'b'][(hex.as_bytes()[16] as usize) % 4];
    let uuid = format!(
        "{}-{}-4{}-{}{}-{}",
        &hex[0..8],
        &hex[8..12],
        &hex[13..16],
        variant,
        &hex[17..20],
        &hex[20..32]
    );
    format!("{uuid}-{}", crate::crypto::md5_hex(ZALO_UA))
}

impl Session {
    /// Record an authenticated account UID into `known_uids` (deduplicated, preserving order).
    pub fn record_known_uid(&mut self, uin: &str) {
        let trimmed = uin.trim();
        if !trimmed.is_empty() && !self.known_uids.iter().any(|u| u == trimmed) {
            self.known_uids.push(trimmed.to_string());
        }
    }

    /// Comma-separated list of all UIDs that have logged in on this device (`logged_uids` in `reqqr`).
    pub fn logged_uids_csv(&self) -> String {
        let mut list = self.known_uids.clone();
        if let Some(cur) = self
            .login
            .as_ref()
            .map(|l| l.uin.trim())
            .filter(|u| !u.is_empty())
        {
            if !list.iter().any(|u| u == cur) {
                list.push(cur.to_string());
            }
        }
        list.join(",")
    }

    /// Persist non-secret device identity (`imei`, `computer_name`, `known_uids`) atomically (`0600`).
    ///
    /// # Errors
    /// Returns an error if serialization or atomic file writing fails.
    pub fn save_device_identity(&self, path: &str) -> Result<()> {
        let mut known = self.known_uids.clone();
        if let Some(cur) = self
            .login
            .as_ref()
            .map(|l| l.uin.trim())
            .filter(|u| !u.is_empty())
        {
            if !known.iter().any(|u| u == cur) {
                known.push(cur.to_string());
            }
        }
        let ident = DeviceIdentity {
            imei: self.imei.clone(),
            computer_name: self.computer_name.clone(),
            known_uids: known,
        };
        let text = serde_json::to_string_pretty(&ident)?;
        crate::data_dir::write_private_atomic(std::path::Path::new(path), text.as_bytes())?;
        Ok(())
    }

    /// Load stable `imei` and `known_uids` from `path` if present, or initialize `path` with the
    /// current device identity so restarts and logouts do not mint a new "unknown device" IMEI.
    pub fn load_or_init_device_identity(&mut self, path: &str) {
        if let Ok(text) = std::fs::read_to_string(path) {
            if let Ok(ident) = serde_json::from_str::<DeviceIdentity>(&text) {
                if ident.imei.len() == 69 {
                    self.imei = ident.imei;
                }
                for uid in ident.known_uids {
                    self.record_known_uid(&uid);
                }
                return;
            }
        }
        let _ = self.save_device_identity(path);
    }

    /// Clear all session credentials, tokens, keys, and cookies while retaining stable device
    /// identity (`imei`, `computer_name`, `known_uids`).
    pub fn logout(&mut self) {
        if let Some(uin) = self.login.as_ref().map(|l| l.uin.clone()) {
            self.record_known_uid(&uin);
        }
        self.login = None;
        self.zpw_sek = None;
        self.secret_key = None;
        self.session_key = None;
        self.expired_at = None;
        self.jar = Arc::new(Jar::default());
        if let Ok(c) = build_http_client(&self.jar, self.proxy_url.as_deref()) {
            self.http = c;
        }
    }

    /// Save the authenticated session to disk (`0600` atomic write).
    ///
    /// # Errors
    /// Returns an error if serialization or atomic disk persistence fails.
    pub fn save_to_file(&self, path: &str) -> Result<()> {
        use base64::Engine;
        let saved = SavedSession {
            imei: self.imei.clone(),
            computer_name: self.computer_name.clone(),
            secret_key_b64: self
                .secret_key
                .as_ref()
                .map(|k| base64::engine::general_purpose::STANDARD.encode(k)),
            session_key: self.session_key.clone(),
            zpw_sek: self.zpw_sek.clone(),
            login: self.login.clone(),
            domains: self.domains.clone(),
            expired_at: self.expired_at,
            proxy_url: self.proxy_url.clone(),
            companion_ik: self
                .companion_ik
                .as_ref()
                .map(|k| base64::engine::general_purpose::STANDARD.encode(k)),
            companion_priv: self
                .companion_priv
                .as_ref()
                .map(|k| base64::engine::general_purpose::STANDARD.encode(k)),
            companion_master_id: self.companion_master_id.clone(),
            companion_link_id: self.companion_link_id,
            known_uids: {
                let mut ku = self.known_uids.clone();
                if let Some(cur) = self
                    .login
                    .as_ref()
                    .map(|l| l.uin.trim())
                    .filter(|u| !u.is_empty())
                {
                    if !ku.iter().any(|u| u == cur) {
                        ku.push(cur.to_string());
                    }
                }
                ku
            },
        };
        let text = serde_json::to_string_pretty(&saved)?;
        crate::data_dir::write_private_atomic(std::path::Path::new(path), text.as_bytes())?;
        Ok(())
    }

    /// Load a previously saved session from disk and restore cookies, keys, and endpoints.
    ///
    /// # Errors
    /// Returns an error if the file is unreadable, malformed JSON, or lacks `zpw_sek`.
    pub fn load_from_file(&mut self, path: &str) -> Result<()> {
        use base64::Engine;
        let text = std::fs::read_to_string(path)?;
        let saved: SavedSession = serde_json::from_str(&text)?;

        if self.proxy_url.is_none() {
            if let Some(ref p) = saved.proxy_url {
                let _ = self.set_proxy(Some(p));
            }
        }
        self.imei = if saved.imei.len() == 69 {
            saved.imei
        } else {
            official_imei()
        };
        self.computer_name = saved.computer_name;
        self.domains = saved.domains;
        if self.domains.chat.contains("-msg-chat.") && self.domains.lp_chat.is_empty() {
            self.domains.lp_chat = self.domains.chat.clone();
        }
        if self.domains.conversation.contains("-ctl-chat.") && self.domains.lp_control.is_empty() {
            self.domains.lp_control = self.domains.conversation.clone();
        }
        self.domains.heal_from_verified();
        if self.domains.lp_chat.is_empty() {
            self.domains.lp_chat = default_lp_chat();
        }
        if self.domains.lp_control.is_empty() {
            self.domains.lp_control = default_lp_control();
        }
        if self.domains.ws.is_empty() {
            self.domains.ws = default_ws();
        }
        self.session_key = saved.session_key;
        self.expired_at = saved.expired_at;
        let sek = saved
            .zpw_sek
            .filter(|s| !s.is_empty())
            .or_else(|| saved.login.as_ref().map(|l| l.zpw_sek.clone()))
            .filter(|s| !s.is_empty());

        if let Some(b64) = saved.secret_key_b64 {
            if let Ok(raw) = base64::engine::general_purpose::STANDARD.decode(b64) {
                self.secret_key = Some(raw);
            }
        }

        for uid in saved.known_uids {
            self.record_known_uid(&uid);
        }
        if let Some(li) = &saved.login {
            self.record_known_uid(&li.uin);
        }
        self.login = saved.login;

        if let Some(s) = sek {
            self.set_zpw_sek(s);
        } else {
            self.login = None;
            anyhow::bail!("session.json thiếu mã xác thực zpw_sek");
        }
        self.companion_ik = saved
            .companion_ik
            .and_then(|b64| base64::engine::general_purpose::STANDARD.decode(b64).ok());
        self.companion_priv = saved
            .companion_priv
            .and_then(|b64| base64::engine::general_purpose::STANDARD.decode(b64).ok());
        self.companion_master_id = saved.companion_master_id;
        self.companion_link_id = saved.companion_link_id;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_device_name_resolves_non_empty_hostname() {
        let dev = detect_device_name();
        assert!(!dev.is_empty());
        assert_ne!(dev, "DESKTOP");
    }

    #[test]
    fn session_atomic_save_preserves_saved_format_and_restores_fixture() {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "alolite-session-test-{}-{nonce}",
            std::process::id()
        ));
        std::fs::create_dir(&root).unwrap();
        let path = root.join("session.json");
        let mut source = Session::new(false, false, false).unwrap();
        source.computer_name = "fixture-device".into();
        source.secret_key = Some(vec![7; 32]);
        source.session_key = Some("fixture-session-key".into());
        source.zpw_sek = Some("fixture-auth-token".into());
        source.save_to_file(path.to_str().unwrap()).unwrap();
        let bytes = std::fs::read(&path).unwrap();
        let saved: SavedSession = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(saved.computer_name, "fixture-device");
        assert_eq!(saved.session_key.as_deref(), Some("fixture-session-key"));
        let mut restored = Session::new(false, false, false).unwrap();
        restored.load_from_file(path.to_str().unwrap()).unwrap();
        assert_eq!(restored.computer_name, source.computer_name);
        assert_eq!(restored.secret_key, source.secret_key);
        assert_eq!(restored.session_key, source.session_key);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn loading_a_legacy_session_heals_every_verified_domain() {
        let fixture = serde_json::json!({
            "imei": "",
            "computer_name": "DESKTOP-TEST",
            "secret_key_b64": null,
            "session_key": null,
            "zpw_sek": "fixture-sek",
            "login": null,
            "domains": {
                "auth": crate::endpoints::AUTH,
                "wpa_chat": crate::endpoints::WPA_CHAT,
                "chat": "https://p4-msg-chat.zaloapp.com",
                "conversation": "https://p4-ctl-chat.zaloapp.com",
                "lp_chat": "https://p9-msg-chat.zaloapp.com",
                "lp_control": "https://p9-ctl-chat.zaloapp.com",
                "friend": "https://tt-friend-wpa.zaloapp.com",
                "convers_api": "https://tt-convers-wpa.zaloapp.com",
                "profile": "https://tt-profile-wpa.zaloapp.com",
                "group": "https://tt-group-wpa.zaloapp.com",
                "file": "https://tt-files-wpa.zaloapp.com",
                "cloud_msg": "https://tt-group-cm.api.zaloapp.com",
                "voice_call": "https://voicecall-wpa.zaloapp.com",
                "sticker": "https://tt-sticker-wpa.zaloapp.com",
                "ws": ["wss://ws9-msg.chat.zalo.me"],
            },
        });
        let path = std::env::temp_dir().join(format!("alolite-legacy-{}.json", std::process::id()));
        std::fs::write(&path, fixture.to_string()).unwrap();

        let mut s = Session::new(false, false, false).unwrap();
        let loaded = s.load_from_file(path.to_str().unwrap());
        let _ = std::fs::remove_file(&path);
        loaded.unwrap();

        for (key, actual) in [
            ("friend", &s.domains.friend),
            ("profile", &s.domains.profile),
            ("group", &s.domains.group),
            ("file", &s.domains.file),
            ("voice_call", &s.domains.voice_call),
            ("sticker", &s.domains.sticker),
            ("chat", &s.domains.chat),
            ("conversation", &s.domains.conversation),
            ("conversation", &s.domains.convers_api),
        ] {
            assert_eq!(
                actual.as_str(),
                crate::endpoints::service(key).unwrap(),
                "endpoint {key} was not healed on load"
            );
        }
        assert_eq!(s.domains.lp_chat, "https://p9-msg-chat.zaloapp.com");
        assert_eq!(s.domains.lp_control, "https://p9-ctl-chat.zaloapp.com");
        assert_eq!(s.domains.cloud_msg, "https://tt-group-cm.api.zaloapp.com");
        assert_eq!(s.domains.ws, vec!["wss://ws9-msg.chat.zalo.me"]);
        assert_eq!(s.zpw_sek.as_deref(), Some("fixture-sek"));
    }

    #[test]
    fn device_identity_and_logged_uids_survive_logout_and_restart() {
        let root = std::env::temp_dir().join(format!(
            "alolite_dev_id_test_{}",
            crate::param_cipher::random_hex(8)
        ));
        std::fs::create_dir_all(&root).unwrap();
        let dev_file = root.join("device.json");
        let dev_str = dev_file.to_str().unwrap();

        let mut s1 = Session::new(false, false, false).unwrap();
        s1.load_or_init_device_identity(dev_str);
        let stable_imei = s1.imei.clone();
        s1.login = Some(LoginInfo {
            uin: "46784157848677799".into(),
            ..Default::default()
        });
        s1.logout();
        assert_eq!(s1.imei, stable_imei);
        assert_eq!(s1.logged_uids_csv(), "46784157848677799");
        s1.save_device_identity(dev_str).unwrap();

        let mut s2 = Session::new(false, false, false).unwrap();
        assert_ne!(s2.imei, stable_imei);
        s2.load_or_init_device_identity(dev_str);
        assert_eq!(s2.imei, stable_imei);
        assert_eq!(s2.logged_uids_csv(), "46784157848677799");

        let _ = std::fs::remove_dir_all(&root);
    }
}
