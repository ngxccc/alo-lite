//! Long-polling status checks for QR scan events and mobile confirmation.

use std::time::Duration;

use anyhow::{bail, Result};
use serde_json::Value;

use super::types::{extract_zpw_sek, CompanionState, QrConfirmResult, QrLogin, QrScannedProfile};
use crate::session::{urlencode, Session};

/// Long-poll the scan endpoint until the phone reports the scan.
///
/// [O] The client appends `&lang=<code>` and loops with `qr_timeout` / `qr_retry`.
/// While nobody has scanned, the endpoint answers **HTTP 408** — that is the
/// "still waiting" signal, not a failure.
pub(crate) fn process_companion_link_data(cs: &mut CompanionState, data: &Value) -> Result<()> {
    let link_data = data.get("link_data");
    let enc_data_str = link_data
        .and_then(|ld| ld.get("enc_link_data").or_else(|| ld.get("encLinkData")))
        .and_then(Value::as_str);
    let enc_hmac_str = link_data
        .and_then(|ld| ld.get("enc_link_hmac").or_else(|| ld.get("encLinkHmac")))
        .and_then(Value::as_str);

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
                let code = v.get("error_code").and_then(Value::as_i64).unwrap_or(0);
                if code == 0 {
                    if let Some(data) = v.get("data") {
                        if !data.is_null() {
                            if let Some(cs) = &mut q.companion_state {
                                if let Err(e) = process_companion_link_data(cs, data) {
                                    if s.verbose {
                                        eprintln!(
                                            "[QR] Lỗi xử lý chứng thực liên kết companion: {e}"
                                        );
                                    }
                                }
                            }
                            let display_name = data
                                .get("display_name")
                                .or_else(|| data.get("displayName"))
                                .and_then(Value::as_str)
                                .unwrap_or("Zalo User")
                                .to_string();
                            let avatar = data
                                .get("avatar")
                                .or_else(|| data.get("avatar_url"))
                                .or_else(|| data.get("thumb"))
                                .or_else(|| data.get("avt"))
                                .and_then(Value::as_str)
                                .filter(|s| !s.is_empty())
                                .map(str::to_string);
                            return Ok(QrScannedProfile {
                                display_name,
                                avatar,
                            });
                        }
                    }
                } else {
                    attempts += 1;
                    if attempts > q.qr_retry.max(1) {
                        let msg = v.get("error_message").and_then(Value::as_str).unwrap_or("");
                        bail!("waiting scan: server error {code}: {msg}");
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
        let code = v.get("error_code").and_then(Value::as_i64);
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
            let msg = v.get("error_message").and_then(Value::as_str).unwrap_or("");
            bail!("pc-status: server error {err_code}: {msg}");
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
