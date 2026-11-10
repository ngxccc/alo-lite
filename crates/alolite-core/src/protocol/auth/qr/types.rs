//! Domain data structures for QR authentication tickets, profiles, and results.

use crate::protocol::wire::{opt_text, WireData};
use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::login;
use crate::session::Session;

/// `data` object of a `reqqr` reply: the QR ticket plus long-poll cursors.
///
/// # Wire Response Example
/// ```json
/// {
///   "token_id": "<redacted>",
///   "qr_cnt": "http://zaloapp.com/qr/l?tk=<redacted>",
///   "base64_qr": "iVBORw0KGgo...",
///   "chk_wait_scan": "https://wpa.chat.zalo.me/api/login/qr/waiting-scan?...",
///   "chk_wait_cfirm": "https://wpa.chat.zalo.me/api/login/qr/pc-status?...",
///   "qr_timeout": 180,
///   "qr_retry": 3
/// }
/// ```
///
/// # Core Transformation
/// Fills [`QrLogin`]; `chk_wait_scan` is mandatory and `qr_cnt`/`base64_qr` must supply at least
/// one renderable QR payload.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct QrRequestWireData {
    #[serde(rename = "token_id")]
    pub token_id: Option<Value>,
    #[serde(rename = "qr_cnt")]
    pub qr_cnt: Option<Value>,
    #[serde(rename = "base64_qr")]
    pub base64_qr: Option<Value>,
    #[serde(rename = "chk_wait_scan")]
    pub chk_wait_scan: Option<Value>,
    #[serde(rename = "chk_wait_cfirm")]
    pub chk_wait_cfirm: Option<Value>,
    #[serde(rename = "qr_timeout")]
    pub qr_timeout: Option<Value>,
    #[serde(rename = "qr_retry")]
    pub qr_retry: Option<Value>,
}

impl QrRequestWireData {
    /// Ticket id (`token_id`).
    pub fn token_id(&self) -> String {
        opt_text(self.token_id.as_ref()).to_string()
    }

    /// Deep-link QR payload (`qr_cnt`).
    pub fn qr_cnt(&self) -> String {
        opt_text(self.qr_cnt.as_ref()).to_string()
    }

    /// Server-rendered QR PNG (`base64_qr`), when the plain flow was requested.
    pub fn base64_png(&self) -> Option<String> {
        self.base64_qr
            .as_ref()
            .and_then(Value::as_str)
            .filter(|v| !v.is_empty())
            .map(str::to_string)
    }

    /// Long-poll URL awaiting the phone's scan (`chk_wait_scan`).
    pub fn chk_wait_scan(&self) -> String {
        opt_text(self.chk_wait_scan.as_ref()).to_string()
    }

    /// Long-poll URL awaiting the phone's confirm (`chk_wait_cfirm`).
    pub fn chk_wait_cfirm(&self) -> String {
        opt_text(self.chk_wait_cfirm.as_ref()).to_string()
    }

    /// Seconds the ticket stays valid (default `180`).
    pub fn qr_timeout(&self) -> u64 {
        self.qr_timeout
            .as_ref()
            .and_then(Value::as_u64)
            .unwrap_or(180)
    }

    /// Re-poll budget (default `3`).
    pub fn qr_retry(&self) -> u32 {
        self.qr_retry.as_ref().and_then(Value::as_u64).unwrap_or(3) as u32
    }
}

/// Session-token aliases probed at the envelope root, under `data`, and under `data.account`.
///
/// # Wire Response Example
/// ```json
/// { "zpw_sek": "<redacted>", "data": { "account": { "zpw_kt": "<redacted>" } } }
/// ```
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct SessionTokenWireData {
    #[serde(rename = "zpw_sek")]
    pub zpw_sek: Option<Value>,
    #[serde(rename = "zpw_kt")]
    pub zpw_kt: Option<Value>,
    #[serde(rename = "zpw_wpsk")]
    pub zpw_wpsk: Option<Value>,
    #[serde(rename = "token")]
    pub token: Option<Value>,
    #[serde(rename = "session_key")]
    pub session_key: Option<Value>,
}

impl SessionTokenWireData {
    /// Alias strings in probe order (`zpw_sek`, `zpw_kt`, `zpw_wpsk`, `token`, `session_key`).
    fn aliases(&self) -> [Option<&str>; 5] {
        [
            self.zpw_sek.as_ref().and_then(Value::as_str),
            self.zpw_kt.as_ref().and_then(Value::as_str),
            self.zpw_wpsk.as_ref().and_then(Value::as_str),
            self.token.as_ref().and_then(Value::as_str),
            self.session_key.as_ref().and_then(Value::as_str),
        ]
    }
}

/// Cryptographic companion identity and state when using Trusted Companion Device flow (QR2).
#[derive(Debug, Clone)]
pub struct CompanionState {
    /// Generated Curve25519 identity keypair of this companion.
    pub identity_key: super::trust::IdentityKeyPair,
    /// Random 32-byte secret sent inside the QR code to authenticate the phone's reply.
    pub linking_secret: super::trust::LinkingSecretKey,
    /// Confirmation URL query parameters computed after the phone signs the link:
    /// `&encLinkingMetaData=...&encAccountSign=...&encDeviceSign=...&encPK=...`
    pub proof_query_params: Option<String>,
    /// Confirmed companion linking proof once verified.
    pub proof: Option<super::trust::CompanionLinkingProof>,
}

#[derive(Debug, Clone, Default)]
pub struct QrLogin {
    /// `zaloqr:v2_…` — the token inside the QR.
    pub token_id: String,
    /// `http://zaloapp.com/qr/l?tk=…` — what the QR encodes.
    pub qr_payload: String,
    /// Server-rendered PNG, when the server produced one.
    pub base64_png: Option<String>,
    /// Long-poll endpoint that resolves once the phone scans the code.
    pub chk_wait_scan: String,
    /// Endpoint that turns a confirmed scan into a session.
    pub chk_wait_cfirm: String,
    /// Seconds the code stays valid.
    pub qr_timeout: u64,
    /// How many times the client may re-poll.
    pub qr_retry: u32,
    /// Cryptographic companion device linking state when QR2 flow is active.
    pub companion_state: Option<CompanionState>,
}

impl QrLogin {
    /// Parse the `data` object of a `reqqr` reply.
    pub fn from_data(d: &Value) -> Result<Self> {
        let wire = QrRequestWireData::from_value(d);

        let out = Self {
            token_id: wire.token_id(),
            qr_payload: wire.qr_cnt(),
            base64_png: wire.base64_png(),
            chk_wait_scan: wire.chk_wait_scan(),
            chk_wait_cfirm: wire.chk_wait_cfirm(),
            qr_timeout: wire.qr_timeout(),
            qr_retry: wire.qr_retry(),
            companion_state: None,
        };

        if out.chk_wait_scan.is_empty() {
            bail!("reqqr: không có chk_wait_scan");
        }
        if out.qr_payload.is_empty() && out.base64_png.is_none() {
            bail!("reqqr: không có qr_cnt lẫn base64_qr");
        }
        Ok(out)
    }

    /// The text the QR encodes.
    /// Prioritizes the HTTP deep link URL (`qr_payload`: `http://zaloapp.com/qr/l?tk=...`)
    /// so phone cameras, Google Lens, and system scanners recognize it as a Zalo link
    /// and open the Zalo mobile app directly instead of displaying raw text.
    ///
    /// `None` when the server rendered the code itself (`self_g = 0`): the raw `token_id` is
    /// *not* what the QR encodes, so it is never used as a substitute.
    pub fn payload(&self) -> Option<&str> {
        if !self.qr_payload.is_empty() {
            Some(&self.qr_payload)
        } else {
            None
        }
    }

    /// RGBA pixels of the code to display: the server-rendered PNG when present
    /// (`self_g = 0`), otherwise the locally rendered payload (`self_g = 1`).
    ///
    /// Returns `(width, height, rgba)`.
    pub fn rgba(&self) -> Result<(u32, u32, Vec<u8>)> {
        if let Some(b64) = self.base64_png.as_deref() {
            use base64::{engine::general_purpose::STANDARD as B64, Engine};
            let raw = B64.decode(b64.trim())?;
            let img = image::load_from_memory(&raw)?.into_rgba8();
            let (w, h) = img.dimensions();
            return Ok((w, h, img.into_raw()));
        }
        let payload = self
            .payload()
            .ok_or_else(|| anyhow::anyhow!("không có qr_cnt lẫn base64_qr để hiển thị"))?;
        let code = qrcode::QrCode::new(payload.as_bytes())?;
        let quiet_zone = 4;
        let w = code.width() + quiet_zone * 2;
        let mut pixels = Vec::with_capacity(w * w * 4);
        for y in 0..w {
            for x in 0..w {
                let in_qr = x >= quiet_zone
                    && x < quiet_zone + code.width()
                    && y >= quiet_zone
                    && y < quiet_zone + code.width();
                let dark = in_qr && (code[(x - quiet_zone, y - quiet_zone)] == qrcode::Color::Dark);
                let c = if dark {
                    [15u8, 23, 42, 255]
                } else {
                    [255u8, 255, 255, 255]
                };
                pixels.extend_from_slice(&c);
            }
        }
        Ok((w as u32, w as u32, pixels))
    }
}

/// Typed display summary returned once a user scans a QR code on mobile.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QrScannedProfile {
    pub display_name: String,
    pub avatar: Option<String>,
}

/// Typed confirmation outcome carrying the newly issued session token (`zpw_sek`).
#[derive(Debug, Clone)]
pub struct QrConfirmResult {
    pub zpw_sek: String,
    pub(crate) raw_resp: Value,
}

impl QrConfirmResult {
    pub(crate) fn new(zpw_sek: String, raw_resp: Value) -> Self {
        Self { zpw_sek, raw_resp }
    }

    /// Apply confirmed session token, companion identity keys, and config fields onto `s`.
    pub fn apply_to_session(&self, s: &mut Session, q: &QrLogin) {
        s.set_zpw_sek(self.zpw_sek.clone());
        if let Some(cs) = &q.companion_state {
            if let Some(proof) = &cs.proof {
                s.companion_ik = Some(cs.identity_key.public_key().to_vec());
                s.companion_priv = Some(cs.identity_key.private_key().to_vec());
                s.companion_master_id = Some(proof.master_id.clone());
                s.companion_link_id = Some(proof.link_id);
            }
        }
        if let Some(data) = self.raw_resp.get("data") {
            let cfg = login::LoginConfigWireData::from_value(data);
            login::apply_login_config(s, &cfg);
        }
    }
}

/// Probe the known session-token aliases at the envelope root, under `data`, and under
/// `data.account`, returning the first non-empty, non-`EXPIRED` value.
pub fn extract_zpw_sek(v: &Value) -> Option<String> {
    let data = v.get("data");
    let root = SessionTokenWireData::from_value(v);
    let nested = SessionTokenWireData::from_value(data.unwrap_or(&Value::Null));
    let account = SessionTokenWireData::from_value(
        data.and_then(|d| d.get("account")).unwrap_or(&Value::Null),
    );
    for index in 0..5 {
        for scope in [&root, &nested, &account] {
            if let Some(s) = scope.aliases()[index].filter(|s| !s.is_empty() && *s != "EXPIRED") {
                return Some(s.to_string());
            }
        }
    }
    None
}
