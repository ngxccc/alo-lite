//! HTTP session transport: cookie jar, `ParamCipher` encoding, multi-host failover, and AES response decryption.
//!
//! Submodules isolate domain routing ([`domains`]), credential/device persistence ([`persistence`]),
//! privacy blocking & log redaction ([`privacy`]), error status classification ([`status`]),
//! low-level HTTP transport & upload ([`transport`]), multi-candidate host failover ([`failover`]),
//! session cookie management ([`cookies`]), parameter encryption & signing ([`signing`]),
//! and server info / session health tracking ([`server_info`]).

use std::sync::Arc;

use anyhow::Result;
use reqwest::cookie::Jar;
use reqwest::Client;

pub mod cookies;
pub mod domains;
pub mod failover;
pub mod persistence;
pub mod privacy;
pub mod server_info;
pub mod signing;
pub mod status;
pub mod transport;

#[cfg(test)]
mod tests;

pub use crate::protocol::wire::obj;
pub use domains::Domains;
pub use persistence::{detect_device_name, official_imei, DeviceIdentity, LoginInfo, SavedSession};
pub use privacy::{
    is_url_blocked_by_privacy, normalize_proxy_url, privacy_blocker_state, redact_proxy_url,
    redact_secret, redact_url, set_privacy_blocker, truncate, urlencode,
};
pub use status::{SessionError, SessionStatus, AUTH_ERROR_CODES};
pub(crate) use transport::build_http_client;

pub const ZPW_VER: &str = "695";
/// `zpw_type` / `type` of the official Linux desktop client (`re-notes/10` §1.2.3).
pub const ZPW_TYPE: &str = "23";
/// User-Agent the official Linux client sends with API calls (`ZaloPC-<os>-<type>v<ver>`).
pub const ZALO_UA: &str = "ZaloPC-linux-23v695";
pub const CONFIG_DOMAIN: &str = "zaloapp.com";

#[derive(Clone)]
pub struct Session {
    pub http: Client,
    pub jar: Arc<Jar>,
    pub domains: Domains,
    pub imei: String,
    pub computer_name: String,
    pub secret_key: Option<Vec<u8>>,
    /// Same key as a string, for `maybe_decrypt`.
    pub session_key: Option<String>,
    pub zpw_sek: Option<String>,
    pub login: Option<LoginInfo>,
    /// Draw commands in front of the user so they can be reproduced by hand.
    pub verbose: bool,
    /// Append every exchange (pretty-printed) to `dump/<n>-<slug>.json`.
    pub dump: bool,
    /// Skip URL redaction in dumps (needed to reproduce signature failures).
    pub dump_raw: bool,
    /// `Some(epoch)` once the server has refused these credentials (#21).
    pub expired_at: Option<i64>,
    /// Optional outbound proxy URL (`http://`, `https://`, `socks5://`, `socks5h://`) (#64).
    pub proxy_url: Option<String>,
    /// Companion identity public key (33 bytes) when linked as a Trusted Companion Device.
    pub companion_ik: Option<Vec<u8>>,
    /// Companion identity private key (32 bytes) when linked as a Trusted Companion Device.
    pub companion_priv: Option<Vec<u8>>,
    /// Master UID of the primary account that linked this companion.
    pub companion_master_id: Option<String>,
    /// Link ID assigned by the primary account.
    pub companion_link_id: Option<u64>,
    /// `getServerInfo` → `setttings.authen.trusted_device.qr_format_trust_device`.
    pub qr_format_trust_device: bool,
    /// Previously authenticated UIDs on this device (`Le()` in `login-startup.js`).
    pub known_uids: Vec<String>,
}

impl Session {
    /// Construct a fresh unauthenticated [`Session`] with verified domain defaults.
    ///
    /// # Errors
    /// Returns an error if the underlying `reqwest::Client` or proxy configuration fails to build.
    pub fn new(verbose: bool, dump: bool, dump_raw: bool) -> Result<Self> {
        let jar = Arc::new(Jar::default());
        let env_proxy = std::env::var("ZALO_PROXY")
            .or_else(|_| std::env::var("ALOZLITE_PROXY"))
            .ok()
            .and_then(|v| normalize_proxy_url(&v).ok().flatten());
        let http = build_http_client(&jar, env_proxy.as_deref())?;
        Ok(Self {
            http,
            jar,
            domains: Domains::default(),
            imei: official_imei(),
            computer_name: detect_device_name(),
            secret_key: None,
            session_key: None,
            zpw_sek: None,
            login: None,
            verbose,
            dump,
            dump_raw,
            expired_at: None,
            proxy_url: env_proxy,
            companion_ik: None,
            companion_priv: None,
            companion_master_id: None,
            companion_link_id: None,
            qr_format_trust_device: false,
            known_uids: Vec::new(),
        })
    }
}
