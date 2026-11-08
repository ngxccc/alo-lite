//! Domain data structures for QR authentication tickets, profiles, and results.

use anyhow::{bail, Result};
use serde_json::Value;

use super::login;
use crate::session::Session;

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
        let s = |k: &str| d.get(k).and_then(Value::as_str).unwrap_or("").to_string();
        let png = d
            .get("base64_qr")
            .and_then(Value::as_str)
            .filter(|v| !v.is_empty())
            .map(str::to_string);

        let out = Self {
            token_id: s("token_id"),
            qr_payload: s("qr_cnt"),
            base64_png: png,
            chk_wait_scan: s("chk_wait_scan"),
            chk_wait_cfirm: s("chk_wait_cfirm"),
            qr_timeout: d.get("qr_timeout").and_then(Value::as_u64).unwrap_or(180),
            qr_retry: d.get("qr_retry").and_then(Value::as_u64).unwrap_or(3) as u32,
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
            login::apply_login_config(s, data);
        }
    }
}

pub fn extract_zpw_sek(v: &Value) -> Option<String> {
    for key in ["zpw_sek", "zpw_kt", "zpw_wpsk", "token", "session_key"] {
        if let Some(s) = v.get(key).and_then(Value::as_str) {
            if !s.is_empty() && s != "EXPIRED" {
                return Some(s.to_string());
            }
        }
        if let Some(s) = v
            .get("data")
            .and_then(|d| d.get(key))
            .and_then(Value::as_str)
        {
            if !s.is_empty() && s != "EXPIRED" {
                return Some(s.to_string());
            }
        }
        if let Some(s) = v
            .get("data")
            .and_then(|d| d.get("account"))
            .and_then(|a| a.get(key))
            .and_then(Value::as_str)
        {
            if !s.is_empty() && s != "EXPIRED" {
                return Some(s.to_string());
            }
        }
    }
    None
}
