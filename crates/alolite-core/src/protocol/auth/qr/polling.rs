//! Long-polling status checks for QR scan events and mobile confirmation.

use std::time::Duration;

use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::types::{extract_zpw_sek, CompanionState, QrConfirmResult, QrLogin, QrScannedProfile};
use crate::protocol::wire::{ErrorEnvelopeWireData, WireData};
use crate::session::{urlencode, Session};

/// `link_data` object carried by the QR scan payload (trusted-device linking proofs).
///
/// # Wire Response Example
/// ```json
/// { "enc_link_data": "<base64>", "enc_link_hmac": "<base64>" }
/// ```
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct QrLinkDataWireData {
    #[serde(rename = "enc_link_data")]
    pub enc_link_data_snake: Option<Value>,
    #[serde(rename = "encLinkData")]
    pub enc_link_data_camel: Option<Value>,
    #[serde(rename = "enc_link_hmac")]
    pub enc_link_hmac_snake: Option<Value>,
    #[serde(rename = "encLinkHmac")]
    pub enc_link_hmac_camel: Option<Value>,
}

impl QrLinkDataWireData {
    /// Base64 linking payload (`enc_link_data` > `encLinkData`).
    pub fn enc_link_data(&self) -> Option<&str> {
        self.enc_link_data_snake
            .as_ref()
            .or(self.enc_link_data_camel.as_ref())
            .and_then(Value::as_str)
    }

    /// Base64 HMAC over the linking payload (`enc_link_hmac` > `encLinkHmac`).
    pub fn enc_link_hmac(&self) -> Option<&str> {
        self.enc_link_hmac_snake
            .as_ref()
            .or(self.enc_link_hmac_camel.as_ref())
            .and_then(Value::as_str)
    }
}

/// Scan-result payload (`data`) of the QR `waiting-scan` poll.
///
/// # Wire Response Example
/// ```json
/// {
///   "display_name": "Ngọc Trần",
///   "avatar": "https://avatar.example/240",
///   "link_data": { "enc_link_data": "<base64>", "enc_link_hmac": "<base64>" }
/// }
/// ```
///
/// # Core Transformation
/// Yields the [`QrScannedProfile`] shown while the user confirms on the phone, plus the
/// companion-linking proof material consumed by `process_companion_link_data`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct QrScannedUserWireData {
    #[serde(rename = "display_name")]
    pub display_name_snake: Option<Value>,
    #[serde(rename = "displayName")]
    pub display_name_camel: Option<Value>,
    #[serde(rename = "avatar")]
    pub avatar_url: Option<Value>,
    #[serde(rename = "avatar_url")]
    pub avatar_url_snake: Option<Value>,
    #[serde(rename = "thumb")]
    pub thumb: Option<Value>,
    #[serde(rename = "avt")]
    pub avt: Option<Value>,
    #[serde(rename = "link_data")]
    pub link_data: Option<Value>,
}

impl QrScannedUserWireData {
    /// Display name, defaulting to `Zalo User` when the account hides it.
    pub fn display_name(&self) -> String {
        self.display_name_snake
            .as_ref()
            .or(self.display_name_camel.as_ref())
            .and_then(Value::as_str)
            .unwrap_or("Zalo User")
            .to_string()
    }

    /// Avatar URL (`avatar` > `avatar_url` > `thumb` > `avt`); empty values are dropped.
    pub fn avatar(&self) -> Option<String> {
        [
            &self.avatar_url,
            &self.avatar_url_snake,
            &self.thumb,
            &self.avt,
        ]
        .into_iter()
        .find_map(|v| v.as_ref().and_then(Value::as_str))
        .filter(|s| !s.is_empty())
        .map(str::to_string)
    }

    /// Companion linking proof material, if the payload carries any.
    pub fn link_data(&self) -> QrLinkDataWireData {
        self.link_data
            .as_ref()
            .map(QrLinkDataWireData::from_value)
            .unwrap_or_default()
    }
}

/// Long-poll the scan endpoint until the phone reports the scan.
///
/// \[O\] The client appends `&lang=<code>` and loops with `qr_timeout` / `qr_retry`.
/// While nobody has scanned, the endpoint answers **HTTP 408** — that is the
/// "still waiting" signal, not a failure.
pub(crate) fn process_companion_link_data(cs: &mut CompanionState, data: &Value) -> Result<()> {
    let link = QrScannedUserWireData::from_value(data).link_data();
    let enc_data_str = link.enc_link_data();
    let enc_hmac_str = link.enc_link_hmac();

    if let (Some(enc_data), Some(enc_hmac)) = (enc_data_str, enc_hmac_str) {
        if !enc_data.is_empty() && !enc_hmac.is_empty() {
            use base64::{engine::general_purpose::STANDARD as B64, Engine};
            let data_bytes = B64
                .decode(enc_data.trim())
                .map_err(|e| anyhow::anyhow!("Giải mã enc_link_data base64 thất bại: {e}"))?;
            let hmac_bytes = B64
                .decode(enc_hmac.trim())
                .map_err(|e| anyhow::anyhow!("Giải mã enc_link_hmac base64 thất bại: {e}"))?;

            let proof = super::trust::create_companion_linking_proof(
                cs.identity_key.public_key(),
                cs.identity_key.private_key(),
                cs.linking_secret.as_bytes(),
                &data_bytes,
                &hmac_bytes,
                super::trust::Platform::Pc,
            )
            .map_err(|e| {
                anyhow::anyhow!("Xác minh chữ ký liên kết thiết bị tin cậy thất bại: {e}")
            })?;

            let enc_meta = B64.encode(&proof.linking_metadata);
            let enc_asig = B64.encode(&proof.account_signature);
            let enc_dsig = B64.encode(&proof.device_signature);
            let enc_pk = B64.encode(cs.identity_key.public_key());

            let proof_query = format!(
                "&encLinkingMetaData={}&encAccountSign={}&encDeviceSign={}&encPK={}",
                urlencode(&enc_meta),
                urlencode(&enc_asig),
                urlencode(&enc_dsig),
                urlencode(&enc_pk),
            );
            cs.proof_query_params = Some(proof_query);
            cs.proof = Some(proof);
        }
    }
    Ok(())
}

pub async fn wait_for_scan(s: &Session, q: &mut QrLogin) -> Result<QrScannedProfile> {
    let url = format!("{}&lang=vi", q.chk_wait_scan);
    let deadline = std::time::Instant::now() + Duration::from_secs(q.qr_timeout.max(30));
    let mut attempts: u32 = 0;
    loop {
        if std::time::Instant::now() >= deadline {
            bail!("hết thời gian chờ quét QR ({}s)", q.qr_timeout);
        }

        let (status, text, _) = get_raw(s, &url).await?;
        match status {
            // Nothing scanned yet — the long poll expired.
            408 => {
                tokio::time::sleep(Duration::from_millis(800)).await;
                continue;
            }
            200 => {
                let v: Value = serde_json::from_str(&text).unwrap_or(Value::Null);
                let env = ErrorEnvelopeWireData::parse(&v);
                if env.code() == 0 {
                    if let Some(data) = v.get("data").filter(|d| !d.is_null()) {
                        if let Some(cs) = &mut q.companion_state {
                            if let Err(e) = process_companion_link_data(cs, data) {
                                if s.verbose {
                                    eprintln!("[QR] Lỗi xử lý chứng thực liên kết companion: {e}");
                                }
                            }
                        }
                        let scanned = QrScannedUserWireData::from_value(data);
                        return Ok(QrScannedProfile {
                            display_name: scanned.display_name(),
                            avatar: scanned.avatar(),
                        });
                    }
                } else {
                    attempts += 1;
                    if attempts > q.qr_retry.max(1) {
                        bail!(
                            "waiting scan: server error {}: {}",
                            env.code(),
                            env.message()
                        );
                    }
                }
                tokio::time::sleep(Duration::from_millis(800)).await;
            }
            other => bail!(
                "waiting scan: HTTP {other}: {}",
                crate::session::truncate(&text, 200)
            ),
        }
    }
}

/// Confirm the scan and turn it into a session.
/// Polls until user confirms on mobile or timeout.
pub async fn confirm(s: &Session, q: &QrLogin) -> Result<QrConfirmResult> {
    if q.chk_wait_cfirm.is_empty() {
        bail!("reqqr: không có chk_wait_cfirm");
    }
    let mut cfirm_url = q.chk_wait_cfirm.clone();
    if let Some(cs) = &q.companion_state {
        if let Some(ref params) = cs.proof_query_params {
            cfirm_url.push_str(params);
        }
    }
    let deadline = std::time::Instant::now() + Duration::from_secs(q.qr_timeout.max(60));
    if q.companion_state.is_some() {
        println!(
            "Đã quét mã QR! Vui lòng xác thực sinh trắc học (FaceID/Vân tay) trên điện thoại…"
        );
    } else {
        println!("Đã quét mã QR! Vui lòng bấm 'Đăng nhập' trên điện thoại…");
    }
    loop {
        if std::time::Instant::now() >= deadline {
            bail!("hết thời gian chờ xác nhận trên điện thoại");
        }

        let (status, text, cookie_sek) = get_raw(s, &cfirm_url).await?;
        if status == 408 {
            tokio::time::sleep(Duration::from_millis(800)).await;
            continue;
        }
        if status != 200 {
            bail!(
                "pc-status: HTTP {status}: {}",
                crate::session::truncate(&text, 200)
            );
        }

        let mut v: Value = serde_json::from_str(&text).map_err(|e| {
            anyhow::anyhow!(
                "pc-status: không phải JSON ({e}): {}",
                crate::session::truncate(&text, 200)
            )
        })?;
        let env = ErrorEnvelopeWireData::parse(&v);
        let code = env.code_opt();
        if code == Some(0) {
            if let Some(ref sek) = cookie_sek {
                if let Some(d) = v.get_mut("data").and_then(Value::as_object_mut) {
                    d.insert("zpw_sek".into(), Value::String(sek.clone()));
                }
            }
            // A successful confirmation MUST provide a non-empty zpw_sek token.
            // If zpw_sek is not present, the user has only scanned the QR code and has NOT
            // yet tapped 'Đăng nhập' (Confirm) on their phone.
            if let Some(sek) = extract_zpw_sek(&v) {
                if !sek.is_empty() && sek != "EXPIRED" {
                    return Ok(QrConfirmResult::new(sek, v));
                }
            }
            // Still waiting for user confirmation
            tokio::time::sleep(Duration::from_millis(800)).await;
            continue;
        } else if code == Some(18012) || code == Some(18011) || code == Some(18013) {
            // 18012 / 18011 / 18013: phone scanned, waiting for user to tap Confirm
            tokio::time::sleep(Duration::from_millis(800)).await;
            continue;
        } else if let Some(err_code) = code {
            bail!("pc-status: server error {err_code}: {}", env.message());
        } else {
            tokio::time::sleep(Duration::from_millis(800)).await;
            continue;
        }
    }
}

/// GET without the error-code check in `Session::send` — the polling endpoint needs
/// the raw status (408 means "waiting"). Also extracts any `Set-Cookie` header.
pub(crate) async fn get_raw(s: &Session, url: &str) -> Result<(u16, String, Option<String>)> {
    let resp = s.http.get(url).send().await?;
    let status = resp.status().as_u16();

    let mut sek = None;
    for val in resp.headers().get_all("set-cookie") {
        if let Ok(sc) = val.to_str() {
            if let Some(pos) = sc.find("zpw_sek=") {
                let token = sc[pos + 8..].split(';').next().unwrap_or("").trim();
                if !token.is_empty() && token != "EXPIRED" {
                    sek = Some(token.to_string());
                }
            }
        }
    }

    let text = resp.text().await?;
    if s.verbose {
        eprintln!("→ GET {}", crate::session::redact_url(url));
        eprintln!("← {status} {}", crate::session::truncate(&text, 200));
        if let Some(ref k) = sek {
            eprintln!(
                "  Captured zpw_sek cookie: {}",
                crate::session::redact_secret(k)
            );
        }
    }
    Ok((status, text, sek))
}
