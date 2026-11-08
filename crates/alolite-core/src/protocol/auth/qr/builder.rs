//! Parameter encoding, URL formatting, and server request initiation for QR login tickets.

use anyhow::{bail, Result};
use serde_json::{json, Map, Value};

use super::polling::get_raw;
use super::types::{CompanionState, QrLogin};
use crate::session::{urlencode, Session};

/// Build the request parameters for `/api/login/reqqr`.
///
/// [O] `getQRCode(imei, logged_uids, language, self_g, qr_form)`:
///
/// ```text
/// { language, client_time, imei, computer_name, logged_uids, self_g, qr_form }
/// ```
///
/// `self_g = 1` asks the server for the *payload* (`qr_cnt`) instead of a rendered
/// image, so the QR can be drawn in the terminal.
pub fn req_qr_params(s: &Session, language: &str, self_g: u8) -> Map<String, Value> {
    req_qr_params_form(s, language, self_g, 0)
}

pub fn req_qr_params_form(
    s: &Session,
    language: &str,
    self_g: u8,
    qr_form: u8,
) -> Map<String, Value> {
    let mut m = Map::new();
    m.insert("language".into(), json!(language));
    m.insert("client_time".into(), json!(crate::clock::unix_now()));
    m.insert("imei".into(), json!(s.imei));
    m.insert("computer_name".into(), json!(s.computer_name));
    // The official client (`Le()` in `login-startup.js`) sends all previously logged-in UIDs
    // on this device joined by commas (empty only on the very first login on a machine).
    m.insert("logged_uids".into(), json!(s.logged_uids_csv()));
    m.insert("self_g".into(), json!(self_g));
    m.insert("qr_form".into(), json!(qr_form));
    m
}

/// Milliseconds since the epoch — the unit the official client stores in `zcid`.
fn now_ms() -> i64 {
    crate::clock::now_ms() as i64
}

/// `{auth}/api/login/reqqr?zcid=…&zcid_ext=…&enc_ver=…&params=…&type=…&client_version=…&signkey=…`
///
/// Field order matches the official client byte for byte. `signkey` is
/// `md5("zsecure" + "reqqr" + values sorted by key)` over the *encrypted* envelope —
/// verified against the live capture's `16cc9dfd…`; without it the server answers
/// `18022 Invalid Signature` (`re-notes/10` §1.2.3).
pub(crate) fn reqqr_url(
    auth: &str,
    cipher: &crate::param_cipher::ParamCipher,
    payload: &Value,
    api_type: &str,
    api_version: &str,
) -> Result<String> {
    let body = cipher.build_body(api_type, api_version, payload, "reqqr")?;
    const ORDER: [&str; 7] = [
        "zcid",
        "zcid_ext",
        "enc_ver",
        "params",
        "type",
        "client_version",
        "signkey",
    ];
    let mut pairs: Vec<String> = Vec::with_capacity(ORDER.len());
    for k in ORDER {
        let v = body
            .get(k)
            .ok_or_else(|| anyhow::anyhow!("reqqr: thiếu tham số {k}"))?;
        let val = match v {
            Value::String(s) => s.clone(),
            other => other.to_string(),
        };
        pairs.push(format!("{k}={}", urlencode(&val)));
    }
    Ok(format!("{auth}/api/login/reqqr?{}", pairs.join("&")))
}

/// `GET {auth}/api/login/reqqr` with the official client's encrypted envelope.
///
/// The response `data` comes back AES-encrypted with the same per-request key and is
/// decrypted here (captured live 2026-11-07, `re-notes/10` §1.2.3).
async fn reqqr(s: &Session, params: Map<String, Value>, tag: &str) -> Result<QrLogin> {
    let cipher =
        crate::param_cipher::ParamCipher::new(crate::session::ZPW_TYPE, &s.imei, now_ms())?;
    let url = reqqr_url(
        &s.domains.auth,
        &cipher,
        &Value::Object(params),
        crate::session::ZPW_TYPE,
        crate::session::ZPW_VER,
    )?;

    let (status, text, _) = get_raw(s, &url).await?;
    if status != 200 {
        bail!(
            "{tag}: HTTP {status}: {}",
            crate::session::truncate(&text, 200)
        );
    }
    let v: Value = serde_json::from_str(&text).map_err(|e| {
        anyhow::anyhow!(
            "{tag}: không phải JSON ({e}): {}",
            crate::session::truncate(&text, 200)
        )
    })?;
    let v = s.maybe_decrypt(v, cipher.key().as_bytes()).await;
    let code = v.get("error_code").and_then(Value::as_i64).unwrap_or(0);
    if code != 0 {
        let msg = v.get("error_message").and_then(Value::as_str).unwrap_or("");
        bail!("{tag}: server error {code}: {msg}");
    }
    QrLogin::from_data(
        v.get("data")
            .ok_or_else(|| anyhow::anyhow!("{tag}: response has no data"))?,
    )
}

/// Ask the server for a plain QR ticket without companion cryptographic linking.
///
/// `self_g = 0` / `qr_form = 0`, exactly like the official desktop client: the server renders
/// the QR PNG itself (`base64_qr`) and Zalo Mobile only asks for "Đăng nhập" — the
/// biometric step belongs to the message-key transfer, not to the QR (see `re-notes/10`).
pub async fn request_qr_plain(s: &Session, language: &str) -> Result<QrLogin> {
    reqqr(s, req_qr_params_form(s, language, 0, 0), "reqqr").await
}

/// Ask the server for a Trusted Companion Device linking QR ticket (`qr_form = 1` / QR2).
///
/// Generates a Curve25519 identity keypair and 32-byte linking secret, requests
/// a ticket with `qr_form: 1`, and encodes them into a `"Z1@"` QR code.
/// Dormant in production: the server keeps `qr_format_trust_device` at `0`, so the reference
/// client never takes this path either (`re-notes/10` §1.2.1).
pub async fn request_qr_trusted(s: &Session, language: &str) -> Result<QrLogin> {
    let identity_key = super::trust::generate_identity_key_pair()
        .map_err(|e| anyhow::anyhow!("Không sinh được cặp khoá Curve25519: {e}"))?;
    let linking_secret = super::trust::generate_linking_secret_key()
        .map_err(|e| anyhow::anyhow!("Không sinh được linking secret: {e}"))?;

    let mut q = reqqr(s, req_qr_params_form(s, language, 1, 1), "reqqr (trusted)").await?;

    let linking_ts_ms = (crate::clock::unix_now() as u64) * 1000;
    let qr2_payload = super::trust::generate_linking_qr(
        identity_key.public_key(),
        Some(linking_secret.as_bytes()),
        &q.token_id,
        linking_ts_ms,
        super::trust::Platform::Pc,
        None,
    )
    .map_err(|e| anyhow::anyhow!("Không tạo được mã QR2 liên kết thiết bị tin cậy: {e}"))?;

    q.qr_payload = qr2_payload;
    q.companion_state = Some(CompanionState {
        identity_key,
        linking_secret,
        proof_query_params: None,
        proof: None,
    });

    Ok(q)
}

/// Ask the server for a QR ticket.
///
/// The QR format follows the login-time server flag
/// [`Session::qr_format_trust_device`] (`getServerInfo` →
/// `setttings.authen.trusted_device.qr_format_trust_device`), exactly like the reference
/// client, which feeds the same flag into both the `reqqr` `qr_form` argument and the QR
/// payload builder:
///
/// * flag `0` (all live Zalo auth hosts, 2026-11-07) → plain QR1: the payload is the
///   `qr_cnt` deep link, Zalo Mobile shows "Đăng nhập trên …" and logs in without any
///   biometric check.
/// * flag `1` → Trusted Companion QR2: a `Z1@` linking payload, the PC runs the companion
///   linking handshake and appends `encLinkingMetaData`/`encAccountSign`/`encDeviceSign`/`encPK`
///   to the confirm URL; Zalo Mobile then authorizes with FaceID/fingerprint.
///
/// `ZALO_QR_TRUSTED=1|0` overrides the flag for protocol experiments.
pub async fn request_qr(s: &Session, language: &str) -> Result<QrLogin> {
    let trusted = trusted_qr_override().unwrap_or(s.qr_format_trust_device);
    if !trusted {
        return request_qr_plain(s, language).await;
    }
    match request_qr_trusted(s, language).await {
        Ok(q) => Ok(q),
        Err(e) => {
            if s.verbose {
                eprintln!(
                    "[QR] Liên kết thiết bị tin cậy thất bại ({e}), chuyển sang QR trực tiếp"
                );
            }
            request_qr_plain(s, language).await
        }
    }
}

/// `ZALO_QR_TRUSTED=1|0` (`true`/`false` also accepted) for protocol experiments.
fn trusted_qr_override() -> Option<bool> {
    match std::env::var("ZALO_QR_TRUSTED")
        .ok()
        .as_deref()
        .map(str::trim)
    {
        Some(v) if v == "1" || v.eq_ignore_ascii_case("true") => Some(true),
        Some(v) if v == "0" || v.eq_ignore_ascii_case("false") => Some(false),
        _ => None,
    }
}
