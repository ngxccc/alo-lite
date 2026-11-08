use super::*;
use crate::session::Session;
use serde_json::{json, Value};

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
    let cipher = crate::param_cipher::ParamCipher::new("23", "imei-1", 1_700_000_000_000).unwrap();
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
        "../../../../tests/fixtures/trusted_device_kat.json"
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
            .map(|i| u8::from_str_radix(&kat["mePriv"].as_str().unwrap()[i..i + 2], 16).unwrap())
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

    let keypair = super::trust::IdentityKeyPair::from_raw(kat_pub, kat_priv);
    let secret = super::trust::LinkingSecretKey::from_bytes(kat_sec);
    let qr2 = super::trust::generate_linking_qr(
        keypair.public_key(),
        Some(secret.as_bytes()),
        &q.token_id,
        1_700_000_000,
        super::trust::Platform::Pc,
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
        .map(|i| u8::from_str_radix(&kat["proof_data"].as_str().unwrap()[i..i + 2], 16).unwrap())
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
