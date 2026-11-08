//! QR login — `GET /api/login/reqqr` → render → wait for scan → confirm.
//!
//! Evidence: `re-notes/01_login.md` §5.2, plus the live capture in the tests below.
//! The owner of this analysis is the maker of the client on this machine (Zalo PC
//! 26.10.11, extracted to `asar_extracted/`).

use anyhow::{bail, Result};
use serde_json::{json, Map, Value};
use std::time::Duration;

use super::login;
use crate::session::{urlencode, Session};

/// A QR login ticket handed out by `/api/login/reqqr`.
///
/// [O] Live capture, 2026-10-20, `self_g = 1`:
///
/// ```text
/// qr_timeout     180
/// qr_retry       3
/// token_id       "zaloqr:v2_DZ1DIqmkC2unDpaoDJ4rD3GtEJKpBZKpOpXcDMOqOcOpP3HaE3anDJXZOpCqD3GuD3XXD6Cu"
/// qr_cnt         "http://zaloapp.com/qr/l?tk=zaloqr:v2_DZ1DIqmkC2unDpaoDJ4rD3GtEJKpBZKpOpXcDMOqOcOpP3HaE3anDJXZOpCqD3GuD3XXD6Cu"
/// chk_wait_scan  "https://qr-id.zaloapp.com/polling/qr/waiting?src=pc&code=60MKL.0.1792515447953.53c8f5f4bf3d4d89158cc3444848a4c8"
/// chk_wait_cfirm "https://qr-id.zaloapp.com/polling/qr/pc-status?src=pc&code=60MKL.0.1792515447953.53c8f5f4bf3d4d89158cc3444848a4c8"
/// ```
///
/// With `self_g = 0` the server renders the image itself and answers `base64_qr`
/// with a 22 KB PNG instead of `qr_cnt`; the encoded payload is the same URL.
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
fn reqqr_url(
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

/// Draw the QR as terminal art.
///
/// With `self_g = 1` the payload is rendered by `Dense1x2`, which packs two module rows into
/// one character cell so the code keeps its square proportions. With `self_g = 0` the code
/// only exists as the server's PNG, so its pixels are downscaled to the terminal grid and
/// drawn with half-block characters.
pub fn render_terminal(q: &QrLogin) -> Result<String> {
    use qrcode::render::unicode::Dense1x2;

    if q.payload().is_none() {
        return render_terminal_png(q, 88);
    }

    let payload = q
        .payload()
        .ok_or_else(|| anyhow::anyhow!("không có payload để vẽ"))?;
    let code = qrcode::QrCode::new(payload.as_bytes())?;
    Ok(code
        .render::<Dense1x2>()
        .quiet_zone(true)
        .module_dimensions(1, 1)
        .min_dimensions(25, 25)
        .build())
}

/// Half-block rendering of the server PNG, scaled with nearest-neighbour so the modules stay
/// crisp. `cols` is the maximum character width the terminal is assumed to have.
fn render_terminal_png(q: &QrLogin, cols: u32) -> Result<String> {
    let (w, h, rgba) = q.rgba()?;
    let scale = (w.max(1)).div_ceil(cols.max(1));
    let out_w = (w / scale).max(1) as usize;
    // Each cell holds two vertical pixels.
    let out_h_px = (h / scale).max(2);

    let mut art = String::new();
    let at = |x: u32, y: u32| -> bool {
        let sx = (x * scale).min(w - 1);
        let sy = (y * scale).min(h - 1);
        let idx = ((sy * w + sx) * 4) as usize;
        rgba.get(idx).copied().unwrap_or(255) < 128
    };

    let mut y = 0;
    while y + 1 < out_h_px {
        for x in 0..out_w as u32 {
            let top = at(x, y);
            let bottom = at(x, y + 1);
            art.push(match (top, bottom) {
                (true, true) => '█',
                (true, false) => '▀',
                (false, true) => '▄',
                (false, false) => ' ',
            });
        }
        art.push('\n');
        y += 2;
    }
    Ok(art)
}

/// Write the server-rendered PNG to disk, when there is one.
pub fn save_png(q: &QrLogin, path: &str) -> Result<()> {
    use base64::{engine::general_purpose::STANDARD as B64, Engine};
    let Some(b64) = q.base64_png.as_deref() else {
        bail!("không có ảnh PNG để lưu");
    };
    let raw = B64.decode(b64.trim())?;
    std::fs::write(path, raw)?;
    Ok(())
}

/// Long-poll the scan endpoint until the phone reports the scan.
///
/// [O] The client appends `&lang=<code>` and loops with `qr_timeout` / `qr_retry`.
/// While nobody has scanned, the endpoint answers **HTTP 408** — that is the
/// "still waiting" signal, not a failure.
fn process_companion_link_data(cs: &mut CompanionState, data: &Value) -> Result<()> {
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

/// Long-poll the scan endpoint until the phone reports the scan.
///
/// [O] The client appends `&lang=<code>` and loops with `qr_timeout` / `qr_retry`.
/// While nobody has scanned, the endpoint answers **HTTP 408** — that is the
/// "still waiting" signal, not a failure.
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
    raw_resp: Value,
}

impl QrConfirmResult {
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
                    return Ok(QrConfirmResult {
                        zpw_sek: sek,
                        raw_resp: v,
                    });
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

/// The whole QR login: ticket → code on screen → wait → confirm → session.
pub async fn login_via_qr(
    s: &mut Session,
    language: &str,
) -> Result<crate::protocol::wire::ActionAck> {
    let mut q = request_qr(s, language).await?;

    match render_terminal(&q) {
        Ok(art) => println!("{art}"),
        Err(e) => {
            eprintln!("! không vẽ được QR trong terminal: {e}");
            let path = "qr.png";
            save_png(&q, path)?;
            println!("  ảnh QR đã lưu: {path}");
        }
    }
    if q.companion_state.is_some() {
        println!(
            "Quét mã QR bằng Zalo trên điện thoại (Yêu cầu xác thực sinh trắc học FaceID/Vân tay)…"
        );
    } else {
        println!("Quét mã bằng Zalo trên điện thoại…");
    }

    let scanned = wait_for_scan(s, &mut q).await?;
    if !scanned.display_name.is_empty() {
        println!("✓ Đã quét: {}", scanned.display_name);
    }

    let confirmed = confirm(s, &q).await?;
    confirmed.apply_to_session(s, &q);
    if let Some(cs) = &q.companion_state {
        if let Some(proof) = &cs.proof {
            println!(
                "✓ Đã xác thực sinh trắc học & hoàn tất liên kết thiết bị tin cậy! (Master UID: {}, Link ID: {})",
                proof.master_id, proof.link_id
            );
        }
    }
    println!("✓ Đăng nhập thành công trên điện thoại! Đang đồng bộ khoá phiên…");
    login::fetch_and_apply_login_info(s).await?;
    println!("✓ Đã nạp khoá phiên zpw_enk và cập nhật endpoints!");
    if let Err(e) = s.save_to_file("session.json") {
        eprintln!("! Lỗi lưu session.json: {e}");
    } else {
        println!("✓ Đã lưu phiên vào session.json (tự động khôi phục cho các lần chạy sau)");
    }

    Ok(crate::protocol::wire::ActionAck::ok())
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

/// GET without the error-code check in `Session::send` — the polling endpoint needs
/// the raw status (408 means "waiting"). Also extracts any `Set-Cookie` header.
async fn get_raw(s: &Session, url: &str) -> Result<(u16, String, Option<String>)> {
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Live capture, 2026-10-20, `self_g = 1`.
    const LIVE_RESP: &str = r#"{
      "qr_timeout": 180,
      "token_id": "zaloqr:v2_DZ1DIqmkC2unDpaoDJ4rD3GtEJKpBZKpOpXcDMOqOcOpP3HaE3anDJXZOpCqD3GuD3XXD6Cu",
      "qr_retry": 3,
      "chk_wait_cfirm": "https://qr-id.zaloapp.com/polling/qr/pc-status?src=pc&code=60MKL.0.1792515447953.53c8f5f4bf3d4d89158cc3444848a4c8",
      "qr_cnt": "http://zaloapp.com/qr/l?tk=zaloqr:v2_DZ1DIqmkC2unDpaoDJ4rD3GtEJKpBZKpOpXcDMOqOcOpP3HaE3anDJXZOpCqD3GuD3XXD6Cu",
      "chk_wait_scan": "https://qr-id.zaloapp.com/polling/qr/waiting?src=pc&code=60MKL.0.1792515447953.53c8f5f4bf3d4d89158cc3444848a4c8"
    }"#;

    #[test]
    fn parses_live_response() {
        let v: Value = serde_json::from_str(LIVE_RESP).unwrap();
        let q = QrLogin::from_data(&v).unwrap();
        assert_eq!(q.qr_timeout, 180);
        assert_eq!(q.qr_retry, 3);
        assert!(q.qr_payload.starts_with("http://zaloapp.com/qr/l?tk="));
        assert!(q.chk_wait_scan.contains("/polling/qr/waiting"));
        assert!(q.chk_wait_cfirm.contains("/polling/qr/pc-status"));
        assert!(q.base64_png.is_none());
    }

    #[test]
    fn payload_carries_the_token() {
        let v: Value = serde_json::from_str(LIVE_RESP).unwrap();
        let q = QrLogin::from_data(&v).unwrap();
        assert!(q.payload().unwrap().contains(&q.token_id));
    }

    #[test]
    fn renders_scannable_looking_art() {
        let v: Value = serde_json::from_str(LIVE_RESP).unwrap();
        let q = QrLogin::from_data(&v).unwrap();
        let art = render_terminal(&q).unwrap();
        let lines: Vec<&str> = art.lines().collect();
        // A version-6 code is 41 modules + a 4-module quiet zone; two rows per line.
        assert!(lines.len() >= 20, "rows = {}", lines.len());
        assert!(
            lines[0].chars().count() >= 40,
            "cols = {}",
            lines[0].chars().count()
        );
        assert!(
            art.contains('█') || art.contains('▀') || art.contains('▄'),
            "the code must use block characters"
        );
        // The quiet zone has to be blank on every side.
        assert!(lines[0].trim().is_empty());
        assert!(lines[lines.len() - 1].trim().is_empty());
    }

    /// The `reqqr` wire shape captured from the official Linux client (2026-11-07): the
    /// encrypted envelope plus `type`, `client_version` and the `reqqr` signature
    /// (`re-notes/10` §1.2.3). Without the signature the server answers `18022`.
    #[test]
    fn reqqr_url_carries_the_signed_encrypted_envelope() {
        let cipher =
            crate::param_cipher::ParamCipher::new("23", "imei-1", 1_700_000_000_000).unwrap();
        let payload = json!({"language": "vi", "self_g": 0, "qr_form": 0});
        let url = reqqr_url("https://wpa.chat.zalo.me", &cipher, &payload, "23", "695").unwrap();

        let query = url.split_once('?').expect("query").1;
        let keys: Vec<&str> = query
            .split('&')
            .map(|kv| kv.split('=').next().unwrap())
            .collect();
        assert_eq!(
            keys,
            [
                "zcid",
                "zcid_ext",
                "enc_ver",
                "params",
                "type",
                "client_version",
                "signkey"
            ]
        );
        assert!(query.contains(&format!("zcid={}", cipher.zcid)));
        assert!(query.contains("enc_ver=v2"));
        assert!(query.contains("type=23"));
        assert!(query.contains("client_version=695"));
    }

    /// `self_g = 0`: the server returns the rendered PNG and no payload, so the raw token is
    /// never used as a substitute — and `rgba()` still produces drawable pixels.
    #[test]
    fn png_only_ticket_renders_without_a_payload() {
        // 1x1 white PNG.
        let png = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8BQDwAEhQGAhKmMIQAAAABJRU5ErkJggg==";
        let v = json!({
            "token_id": "zaloqr:v2_abc",
            "base64_qr": png,
            "chk_wait_scan": "https://qr.id.zalo.me/polling/qr/waiting?code=1",
            "chk_wait_cfirm": "https://qr.id.zalo.me/polling/qr/pc-status?code=1",
        });
        let q = QrLogin::from_data(&v).unwrap();
        assert!(
            q.payload().is_none(),
            "token_id must not stand in for the QR"
        );
        let (w, h, rgba) = q.rgba().unwrap();
        assert_eq!((w, h), (1, 1));
        assert_eq!(rgba.len(), 4);
        assert!(!render_terminal(&q).unwrap().is_empty());
    }

    #[test]
    fn params_carry_the_documented_fields() {
        let s = Session::new(false, false, false).unwrap();
        let p = req_qr_params(&s, "vi", 1);
        for k in [
            "language",
            "client_time",
            "imei",
            "computer_name",
            "logged_uids",
            "self_g",
            "qr_form",
        ] {
            assert!(p.get(k).is_some(), "missing {k}");
        }
        // A device that never logged in sends an empty uid string, matching the official
        // client's `logged_uids` field.
        assert_eq!(p["logged_uids"], json!(""));
        assert_eq!(p["self_g"], json!(1));
        assert_eq!(p["qr_form"], json!(0));
    }

    #[test]
    fn missing_wait_url_is_rejected() {
        let v = json!({"qr_cnt": "http://x/"});
        assert!(QrLogin::from_data(&v).is_err());
    }

    #[test]
    fn trusted_companion_qr_linking_and_confirm_url_construction() {
        let v: Value = serde_json::from_str(LIVE_RESP).unwrap();
        let mut q = QrLogin::from_data(&v).unwrap();

        let kat: Value = serde_json::from_str(include_str!(
            "../../../tests/fixtures/trusted_device_kat.json"
        ))
        .unwrap();

        let kat_pub: [u8; 33] = {
            let b: Vec<u8> = (0..66)
                .step_by(2)
                .map(|i| u8::from_str_radix(&kat["mePub"].as_str().unwrap()[i..i + 2], 16).unwrap())
                .collect();
            b.try_into().unwrap()
        };
        let kat_priv: [u8; 32] = {
            let b: Vec<u8> = (0..64)
                .step_by(2)
                .map(|i| {
                    u8::from_str_radix(&kat["mePriv"].as_str().unwrap()[i..i + 2], 16).unwrap()
                })
                .collect();
            b.try_into().unwrap()
        };
        let kat_sec: [u8; 32] = {
            let b: Vec<u8> = (0..64)
                .step_by(2)
                .map(|i| u8::from_str_radix(&kat["sec"].as_str().unwrap()[i..i + 2], 16).unwrap())
                .collect();
            b.try_into().unwrap()
        };

        let keypair = super::super::trust::IdentityKeyPair::from_raw(kat_pub, kat_priv);
        let secret = super::super::trust::LinkingSecretKey::from_bytes(kat_sec);
        let qr2 = super::super::trust::generate_linking_qr(
            keypair.public_key(),
            Some(secret.as_bytes()),
            &q.token_id,
            1_700_000_000,
            super::super::trust::Platform::Pc,
            None,
        )
        .unwrap();
        assert!(qr2.starts_with("Z1@"));
        q.qr_payload = qr2.clone();
        q.companion_state = Some(CompanionState {
            identity_key: keypair,
            linking_secret: secret,
            proof_query_params: None,
            proof: None,
        });
        assert_eq!(q.payload(), Some(qr2.as_str()));

        use base64::{engine::general_purpose::STANDARD as B64, Engine};
        let raw_data: Vec<u8> = (0..kat["proof_data"].as_str().unwrap().len())
            .step_by(2)
            .map(|i| {
                u8::from_str_radix(&kat["proof_data"].as_str().unwrap()[i..i + 2], 16).unwrap()
            })
            .collect();
        let raw_mac: Vec<u8> = (0..kat["proof_mac"].as_str().unwrap().len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&kat["proof_mac"].as_str().unwrap()[i..i + 2], 16).unwrap())
            .collect();

        let scan_mock = json!({
            "display_name": "Test User",
            "link_data": {
                "enc_link_data": B64.encode(raw_data),
                "enc_link_hmac": B64.encode(raw_mac),
            }
        });

        let cs = q.companion_state.as_mut().unwrap();
        process_companion_link_data(cs, &scan_mock).expect("process link_data");
        assert!(cs.proof.is_some());
        let params = cs.proof_query_params.as_ref().unwrap();
        assert!(params.contains("&encLinkingMetaData="));
        assert!(params.contains("&encAccountSign="));
        assert!(params.contains("&encDeviceSign="));
        assert!(params.contains("&encPK="));
    }
}
