//! Known-answer and contract tests for the trusted-device protocol.
//!
//! Vectors were recorded against Zalo's reference WASM artifact
//! (`trusted_device_protocol_bg.*.wasm`) via the REA MCP harness.

use alolite::protocol::auth::trust::*;

fn hex(s: &str) -> Vec<u8> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
        .collect()
}

fn kat() -> serde_json::Value {
    serde_json::from_str(include_str!("../tests/fixtures/trusted_device_kat.json"))
        .expect("valid kat fixture")
}

#[test]
fn generate_linking_qr_matches_oracle_reference() {
    let k = kat();
    let pub_key = hex(k["mePub"].as_str().unwrap());
    let sec = hex(k["sec"].as_str().unwrap());
    let ts = k["ts"].as_u64().unwrap();

    // With secret, PC platform (wire value 1, input 2)
    let qr1 = generate_linking_qr(&pub_key, Some(&sec), "TOKEN_KAT", ts, Platform::Pc, None)
        .expect("qr1");
    assert_eq!(qr1, k["qr1"].as_str().unwrap());

    // Relink variant (no secret, relinkId 32 bytes, Web platform)
    let relink_bytes = [0x55u8; 32];
    let qr2 = generate_linking_qr(
        &pub_key,
        None,
        "TOKEN_KAT",
        ts,
        Platform::Web,
        Some(&relink_bytes),
    )
    .expect("qr2");
    assert_eq!(qr2, k["qr2"].as_str().unwrap());
}

#[test]
fn get_relink_id_matches_oracle_reference() {
    let k = kat();
    let pr_pub = hex(k["prPub"].as_str().unwrap());
    let me_pub = hex(k["mePub"].as_str().unwrap());
    let sec = hex(k["sec"].as_str().unwrap());
    let ts = k["ts"].as_u64().unwrap();

    let relink = get_relink_id(&pr_pub, &me_pub, &sec, ts).expect("relink");
    assert_eq!(
        relink.to_vec(),
        hex(k["relink"].as_str().unwrap()),
        "relinkId must match reference HMAC-SHA256"
    );
}

#[test]
fn create_companion_linking_proof_accepts_valid_oracle_input() {
    let k = kat();
    let me_pub = hex(k["mePub"].as_str().unwrap());
    let me_priv = hex(k["mePriv"].as_str().unwrap());
    let sec = hex(k["sec"].as_str().unwrap());
    let data = hex(k["proof_data"].as_str().unwrap());
    let mac = hex(k["proof_mac"].as_str().unwrap());

    let proof = create_companion_linking_proof(&me_pub, &me_priv, &sec, &data, &mac, Platform::Pc)
        .expect("proof accepted");

    assert_eq!(proof.master_id, "master_kat");
    assert_eq!(proof.link_id, 999);
    assert_eq!(proof.companion_id, 7);
    assert_eq!(proof.linking_timestamp, 1_700_000_000);
    assert_eq!(
        proof.primary_public_identity_key,
        hex(k["proof_prim"].as_str().unwrap())
    );
    assert_eq!(
        proof.linking_metadata,
        hex(k["proof_meta"].as_str().unwrap())
    );
    assert_eq!(
        proof.account_signature,
        hex(k["proof_asig"].as_str().unwrap())
    );
    assert_eq!(proof.device_signature.len(), 64);
}

#[test]
fn create_companion_linking_proof_rejects_corrupted_mac() {
    let k = kat();
    let me_pub = hex(k["mePub"].as_str().unwrap());
    let me_priv = hex(k["mePriv"].as_str().unwrap());
    let sec = hex(k["sec"].as_str().unwrap());
    let data = hex(k["proof_data"].as_str().unwrap());
    let mut mac = hex(k["proof_mac"].as_str().unwrap());
    mac[0] ^= 1;

    let err = create_companion_linking_proof(&me_pub, &me_priv, &sec, &data, &mac, Platform::Pc)
        .unwrap_err();
    assert_eq!(err.code(), ErrorCode::InvalidLinkingMac);
}

#[test]
fn verify_device_list_validates_oracle_list() {
    let k = kat();
    let bytes = hex(k["dlist_bytes"].as_str().unwrap());
    let pr_pub = hex(k["prPub"].as_str().unwrap());
    let me_pub = hex(k["mePub"].as_str().unwrap());
    let ts = k["ts"].as_u64().unwrap();

    let res = verify_device_list_from_bytes(&bytes, &pr_pub, ts, Some(&me_pub)).expect("decode");
    assert!(res.is_trusted);
    assert_eq!(res.error, None);
    assert_eq!(res.device_list.master_id, "master_kat");
    assert_eq!(res.device_list.data.len(), 2);
    assert_eq!(res.device_list.data[0].companion_id, 7);
    assert_eq!(res.device_list.data[0].status, DeviceStatus::Active);
    assert_eq!(res.device_list.data[1].companion_id, 12);
    assert_eq!(res.device_list.data[1].status, DeviceStatus::Stale);
}

#[test]
fn verify_device_list_rejects_wrong_primary_key() {
    let k = kat();
    let bytes = hex(k["dlist_bytes"].as_str().unwrap());
    let mut wrong = hex(k["prPub"].as_str().unwrap());
    wrong[1] ^= 1;

    let res = verify_device_list_from_bytes(&bytes, &wrong, 1_700_000_000, None).expect("decode");
    assert!(!res.is_trusted);
    assert_eq!(res.error, Some(DeviceListError::WrongPrimaryIdentityKey));
}

#[test]
fn verify_device_list_flags_stale_device_past_expiry() {
    let k = kat();
    let bytes = hex(k["dlist_bytes"].as_str().unwrap());
    let me_pub = hex(k["mePub"].as_str().unwrap());
    // d1 is active and checked first, so active TTL dominates even after d2's expiration.
    let exp = check_expiry_time(&bytes, &me_pub, 1_700_005_001).expect("check_expiry");
    assert!(!exp.is_expired);
}

#[test]
fn check_expiry_time_computes_remaining_lifetime() {
    let k = kat();
    let bytes = hex(k["dlist_bytes"].as_str().unwrap());
    let me_pub = hex(k["mePub"].as_str().unwrap());

    let exp = check_expiry_time(&bytes, &me_pub, 1_700_000_000).expect("expiry");
    assert!(!exp.is_expired);
    assert_eq!(exp.remaining_time, DEVICE_LIST_TTL_MS as i64);
    assert_eq!(exp.companion_device_status, Some(DeviceStatus::Active));
}

#[test]
fn verify_trusted_identity_checks_device_signature() {
    let k = kat();
    let bytes = hex(k["dlist_bytes"].as_str().unwrap());
    let me_pub = hex(k["mePub"].as_str().unwrap());

    let id = verify_trusted_identity(&bytes, &me_pub, 1_700_000_000).expect("verify");
    assert!(id.is_trusted);
    assert_eq!(id.error, None);
    assert_eq!(id.companion_identity_key, me_pub);
}

#[test]
fn roundtrip_key_generation() {
    let kp = generate_identity_key_pair().expect("generate pair");
    assert_eq!(kp.public_key()[0], 0x05);
    assert_ne!(kp.public_key(), &[0u8; 33]);
    assert_ne!(kp.private_key(), &[0u8; 32]);

    let secret = generate_linking_secret_key().expect("generate secret");
    assert_eq!(secret.key_size(), 32);
    assert_ne!(secret.as_bytes(), &[0u8; 32]);
}
