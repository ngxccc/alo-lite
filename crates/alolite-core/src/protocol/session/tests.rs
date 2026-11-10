use super::*;

#[test]
fn session_new_initializes_zpw_sek_and_headers() {
    let s = Session::new(false, false, false).unwrap();
    assert!(s.zpw_sek.is_none());
    assert!(s.login.is_none());
}

#[test]
fn apply_server_info_reads_trusted_device_qr_flag() {
    let mut s = Session::new(false, false, false).unwrap();
    assert!(!s.qr_format_trust_device);

    s.apply_server_info(&serde_json::json!({
        "setttings": { "authen": { "trusted_device": {
            "logo_qr": 0, "qr_format_trust_device": 0, "self_g": 0 } } }
    }));
    assert!(!s.qr_format_trust_device);

    s.apply_server_info(&serde_json::json!({
        "setttings": { "authen": { "trusted_device": { "qr_format_trust_device": 1 } } }
    }));
    assert!(s.qr_format_trust_device);

    s.apply_server_info(&serde_json::json!({ "setttings": { "authen": {} } }));
    assert!(s.qr_format_trust_device);
}

#[test]
fn session_set_zpw_sek_updates_fields() {
    let mut s = Session::new(false, false, false).unwrap();
    s.login = Some(LoginInfo::default());
    s.set_zpw_sek("test_sek_value".into());
    assert_eq!(s.zpw_sek.as_deref(), Some("test_sek_value"));
    assert_eq!(s.login.as_ref().unwrap().zpw_sek, "test_sek_value");
}

#[test]
fn has_live_credentials_requires_login_cookie_and_secret_key() {
    let mut s = Session::new(false, false, false).unwrap();
    assert!(!s.has_live_credentials());

    s.login = Some(LoginInfo::default());
    assert!(!s.has_live_credentials(), "login alone is not enough");

    s.set_zpw_sek("test_sek_value".into());
    assert!(!s.has_live_credentials(), "AES key is still missing");

    s.secret_key = Some(vec![0u8; 32]);
    assert!(s.has_live_credentials());

    s.logout();
    assert!(!s.has_live_credentials(), "logout must clear credentials");
}

#[test]
fn apply_server_info_takes_long_polling_hosts_from_the_server() {
    let mut s = Session::new(false, false, false).unwrap();
    s.apply_server_info(&serde_json::json!({
        "zpw_chts_new": ["https://p7-msg-chat.zaloapp.com"],
        "zpw_ctrs_new": ["https://p7-ctl-chat.zaloapp.com"],
        "zpw_ws": ["wss://ws7-msg.chat.zalo.me"],
        "zpw_service_map_v3": {
            "friend": ["https://tt-friend-wpa.zaloapp.com"],
            "group_cloud_message": ["https://tt-group-cm.example.test"],
        },
    }));

    assert_eq!(s.domains.lp_chat, "https://p7-msg-chat.zaloapp.com");
    assert_eq!(s.domains.lp_control, "https://p7-ctl-chat.zaloapp.com");
    assert_eq!(s.domains.ws, vec!["wss://ws7-msg.chat.zalo.me"]);
    assert_eq!(s.domains.friend, "https://tt-friend-wpa.chat.zalo.me");
    assert_eq!(s.domains.cloud_msg, "https://tt-group-cm.example.test");
}

#[tokio::test]
async fn request_with_candidates_fails_over_on_404_and_remembers_winner() {
    let mut s = Session::new(false, false, false).unwrap();
    s.domains.apply_service_map(&serde_json::json!({
        "chat": [
            "https://tt-chat4-wpa.api.zaloapp.com",
            "https://tt-chat5-wpa.api.zaloapp.com"
        ]
    }));

    let tried = std::sync::Arc::new(std::sync::Mutex::new(Vec::<String>::new()));
    let tried_c = tried.clone();
    let res = s
        .request_with_candidates("chat", "/api/message/sms?zpw_ver=694", move |url| {
            let tried_inner = tried_c.clone();
            async move {
                tried_inner.lock().unwrap().push(url.clone());
                if url.starts_with("https://tt-chat3-wpa.chat.zalo.me") {
                    anyhow::bail!("HTTP 404 Not Found from {url}: nginx")
                } else {
                    Ok(serde_json::json!({ "error_code": 0, "winner": url }))
                }
            }
        })
        .await
        .unwrap();

    assert_eq!(
        res["winner"],
        "https://tt-chat4-wpa.api.zaloapp.com/api/message/sms?zpw_ver=694"
    );

    let tried2 = std::sync::Arc::new(std::sync::Mutex::new(Vec::<String>::new()));
    let tried2_c = tried2.clone();
    let _ = s
        .request_with_candidates("chat", "/api/message/sms?zpw_ver=694", move |url| {
            let tried_inner = tried2_c.clone();
            async move {
                tried_inner.lock().unwrap().push(url.clone());
                Ok(serde_json::json!({ "error_code": 0 }))
            }
        })
        .await
        .unwrap();
    assert_eq!(
        *tried2.lock().unwrap(),
        vec!["https://tt-chat4-wpa.api.zaloapp.com/api/message/sms?zpw_ver=694"]
    );
}

#[tokio::test]
async fn request_with_candidates_does_not_failover_on_auth_errors() {
    let mut s = Session::new(false, false, false).unwrap();
    s.domains.apply_service_map(&serde_json::json!({
        "friend": ["https://tt-friend2-wpa.chat.zalo.me"]
    }));

    for code in [600_i64, -102_i64] {
        let calls = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let calls_c = calls.clone();
        let err = s
            .request_with_candidates("friend", "/api/social/friend/getfriends", move |url| {
                let calls_inner = calls_c.clone();
                async move {
                    calls_inner.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    Err(SessionError::new(code, "session expired").at(url).into())
                }
            })
            .await
            .unwrap_err();
        assert_eq!(SessionStatus::from_error(&err), SessionStatus::Expired);
        assert_eq!(calls.load(std::sync::atomic::Ordering::SeqCst), 1);
    }
}
#[tokio::test]
async fn maybe_decrypt_unwraps_inner_decrypted_envelope() {
    let s = Session::new(false, false, false).unwrap();
    let key = b"1234567890123456";
    let inner_json = serde_json::json!({
        "error_code": 0,
        "error_message": "Successful.",
        "data": { "foo": "bar", "count": 42 }
    });
    let enc = crate::crypto::encode_aes(key, &inner_json.to_string()).unwrap();
    let raw_resp = serde_json::json!({
        "error_code": 0,
        "error_message": "Successful.",
        "data": enc
    });

    let dec = s.maybe_decrypt(raw_resp, key).await;
    assert_eq!(
        dec.get("error_code").and_then(serde_json::Value::as_i64),
        Some(0)
    );
    assert_eq!(
        dec.get("error_message").and_then(serde_json::Value::as_str),
        Some("Successful.")
    );
    assert_eq!(
        dec.get("data"),
        Some(&serde_json::json!({ "foo": "bar", "count": 42 }))
    );
}

#[test]
fn set_zpw_sek_reaches_every_chat_subdomain() {
    use reqwest::cookie::CookieStore;
    let mut s = Session::new(false, false, false).unwrap();
    s.set_zpw_sek("sek123".into());
    for host in [
        "https://tt-profile-wpa.chat.zalo.me",
        "https://tt-convers-wpa.chat.zalo.me",
        "https://tt-group-wpa.chat.zalo.me",
        "https://tt-group-cm.chat.zalo.me",
        "https://tt-files-wpa.chat.zalo.me",
        "https://lp9-msg.chat.zalo.me",
        "https://wpa.chat.zalo.me",
        "https://wpa.zaloapp.com",
    ] {
        let url = reqwest::Url::parse(host).unwrap();
        let header = s
            .jar
            .cookies(&url)
            .map(|v| v.to_str().unwrap_or_default().to_string())
            .unwrap_or_default();
        assert!(header.contains("zpw_sek=sek123"), "{host} -> {header:?}");
        assert!(header.contains("_zlang=vi"), "{host} -> {header:?}");
        assert_eq!(
            header.matches("zpw_sek=").count(),
            1,
            "{host} received duplicate cookies: {header:?}"
        );
    }
}

#[test]
fn rotated_cookie_replaces_the_seeded_wildcard() {
    use reqwest::cookie::CookieStore;
    let mut s = Session::new(false, false, false).unwrap();
    s.set_zpw_sek("old".into());
    s.seed_wildcard_cookie("zpw_sek", "new");
    let url = reqwest::Url::parse("https://tt-group-cm.chat.zalo.me").unwrap();
    let header = s
        .jar
        .cookies(&url)
        .map(|v| v.to_str().unwrap_or_default().to_string())
        .unwrap_or_default();
    assert!(header.contains("zpw_sek=new"), "{header:?}");
    assert!(!header.contains("zpw_sek=old"), "{header:?}");
    assert_eq!(header.matches("zpw_sek=").count(), 1, "{header:?}");
}

#[test]
fn extract_set_cookie_sek_ignores_other_cookies_and_attributes() {
    use reqwest::header::{HeaderMap, HeaderValue, SET_COOKIE};
    let mut headers = HeaderMap::new();
    headers.append(SET_COOKIE, HeaderValue::from_static("_zlang=vi; Path=/"));
    headers.append(
        SET_COOKIE,
        HeaderValue::from_static("zpw_sek=abc.def; Path=/; HttpOnly; Secure"),
    );
    assert_eq!(
        super::cookies::extract_set_cookie_sek(&headers).as_deref(),
        Some("abc.def")
    );

    let mut none = HeaderMap::new();
    none.append(SET_COOKIE, HeaderValue::from_static("zpw_sek=; Path=/"));
    assert!(super::cookies::extract_set_cookie_sek(&none).is_none());
}
