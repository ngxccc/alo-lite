//! HTTP session transport: cookie jar, `ParamCipher` encoding, multi-host failover, and AES response decryption.
//!
//! Submodules isolate domain routing ([`domains`]), credential/device persistence ([`persistence`]),
//! privacy blocking & log redaction ([`privacy`]), and error status classification ([`status`]).

use std::future::Future;
use std::sync::Arc;

use anyhow::{bail, Result};
use reqwest::cookie::Jar;
use reqwest::header::{HeaderMap, HeaderValue, ORIGIN, REFERER};
use reqwest::{Client, Method};
use serde_json::Value;

use crate::crypto;

pub mod domains;
pub mod persistence;
pub mod privacy;
pub mod status;

pub use crate::protocol::wire::obj;
pub use domains::Domains;
pub use persistence::{detect_device_name, official_imei, DeviceIdentity, LoginInfo, SavedSession};
pub use privacy::{
    is_url_blocked_by_privacy, normalize_proxy_url, privacy_blocker_state, redact_proxy_url,
    redact_secret, redact_url, set_privacy_blocker, truncate, urlencode,
};
pub use status::{SessionError, SessionStatus, AUTH_ERROR_CODES};

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

pub(super) fn build_http_client(jar: &Arc<Jar>, proxy_url: Option<&str>) -> Result<Client> {
    let mut default_headers = HeaderMap::new();
    default_headers.insert(
        REFERER,
        HeaderValue::from_static(crate::endpoints::CHAT_REFERER),
    );
    default_headers.insert(
        ORIGIN,
        HeaderValue::from_static(crate::endpoints::CHAT_ORIGIN),
    );
    let mut builder = Client::builder()
        .cookie_provider(Arc::clone(jar))
        .default_headers(default_headers)
        .connect_timeout(std::time::Duration::from_secs(10))
        .timeout(std::time::Duration::from_secs(35))
        .user_agent(ZALO_UA);
    if let Some(raw_proxy) = proxy_url.map(str::trim).filter(|s| !s.is_empty()) {
        if let Some(norm) = normalize_proxy_url(raw_proxy)? {
            let mut proxy = reqwest::Proxy::all(&norm)?;
            if let Ok(parsed) = reqwest::Url::parse(&norm) {
                let user = parsed.username();
                if !user.is_empty() {
                    let pass = parsed.password().unwrap_or("");
                    proxy = proxy.basic_auth(user, pass);
                }
            }
            builder = builder.proxy(proxy);
        }
    }
    Ok(builder.build()?)
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

    /// Configure or clear the HTTP/HTTPS/SOCKS5 proxy for this session, rebuilding the underlying
    /// `reqwest::Client` while preserving all session cookies (`zpw_sek`, #64).
    ///
    /// # Errors
    /// Returns an error if `proxy_url` is malformed or uses an unsupported scheme.
    pub fn set_proxy(&mut self, proxy_url: Option<&str>) -> Result<Option<String>> {
        let norm = match proxy_url {
            Some(raw) => normalize_proxy_url(raw)?,
            None => None,
        };
        self.http = build_http_client(&self.jar, norm.as_deref())?;
        self.proxy_url = norm.clone();
        if let Some(sek) = self.zpw_sek.clone() {
            self.set_zpw_sek(sek);
        }
        Ok(norm)
    }

    /// Store `zpw_sek` session token, attach it to `self.login`, and insert the session cookie.
    pub fn set_zpw_sek(&mut self, v: String) {
        if v.is_empty() {
            return;
        }
        for &host in crate::endpoints::COOKIE_SEED_HOSTS {
            if let Ok(url) = reqwest::Url::parse(host) {
                let cookie = format!("zpw_sek={v}; Path=/");
                self.jar.add_cookie_str(&cookie, &url);
            }
        }
        for host in [
            &self.domains.auth,
            &self.domains.chat,
            &self.domains.conversation,
            &self.domains.lp_chat,
            &self.domains.lp_control,
            &self.domains.convers_api,
            &self.domains.profile,
            &self.domains.friend,
            &self.domains.group,
            &self.domains.cloud_msg,
            &self.domains.voice_call,
            &self.domains.sticker,
        ] {
            if let Ok(url) = reqwest::Url::parse(host) {
                let cookie = format!("zpw_sek={v}; Path=/");
                self.jar.add_cookie_str(&cookie, &url);
            }
        }
        self.zpw_sek = Some(v.clone());
        if let Some(ref mut li) = self.login {
            li.zpw_sek = v;
        }
    }

    /// The stored session is already known to be refused by the server (#21).
    pub fn is_expired_marked(&self) -> bool {
        self.expired_at.is_some()
    }

    /// Record that the server refused these credentials.
    pub fn mark_expired(&mut self) {
        self.expired_at = Some(crate::clock::unix_now());
    }

    /// A fresh login succeeded: the marker must not leak into the new session.
    pub fn clear_expired(&mut self) {
        self.expired_at = None;
    }

    /// Ask the server whether the stored session still works (#21).
    pub async fn session_status(&self) -> SessionStatus {
        if self.login.is_none() || self.zpw_sek.is_none() {
            return SessionStatus::Expired;
        }
        let body = serde_json::json!({
            "avatar_size": 160,
            "imei": self.imei,
        });
        let url = match self.encoded_params(&body) {
            Ok(enc) => format!(
                "{}/api/social/profile/me-v2?{}&params={enc}&os=8",
                self.domains.profile,
                self.common_params()
            ),
            Err(_) => format!(
                "{}/api/social/profile/me-v2?{}",
                self.domains.profile,
                self.common_params()
            ),
        };
        let resp = match self.get(&url).await {
            Ok(r) => r,
            Err(e) => return SessionStatus::from_error(&e),
        };
        let dec = self.maybe_decrypt(resp, self.session_key_bytes()).await;
        match dec.get("error_code").and_then(Value::as_i64) {
            Some(code) => SessionStatus::from_error_code(code),
            None => SessionStatus::Unreachable,
        }
    }

    /// Fetch the authenticated user's own numeric `userId` and `displayName` from `/api/social/profile/me-v2`.
    pub async fn sync_profile_me(&mut self) -> Option<(String, String)> {
        if self.login.is_none() || self.zpw_sek.is_none() {
            return None;
        }
        let body = serde_json::json!({
            "avatar_size": 160,
            "imei": self.imei,
        });
        let enc = self.encoded_params(&body).ok()?;
        let url = format!(
            "{}/api/social/profile/me-v2?{}&params={enc}&os=8",
            self.domains.profile,
            self.common_params()
        );
        let resp = self.get(&url).await.ok()?;
        let dec = self.maybe_decrypt(resp, self.session_key_bytes()).await;
        let profile = dec
            .get("data")
            .and_then(|d| d.get("profile"))
            .or_else(|| dec.get("profile"))?;
        let uid = profile
            .get("userId")
            .or_else(|| profile.get("uid"))
            .and_then(|v| match v {
                Value::String(s) if !s.is_empty() => Some(s.clone()),
                Value::Number(n) => Some(n.to_string()),
                _ => None,
            })?;
        let name = profile
            .get("displayName")
            .or_else(|| profile.get("zaloName"))
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        let avatar = profile
            .get("avatar")
            .or_else(|| profile.get("avt"))
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        if let Some(ref mut li) = self.login {
            li.uin = uid.clone();
            if !name.is_empty() {
                li.display_name = name.clone();
            }
            if !avatar.is_empty() {
                li.avatar = avatar;
            }
        }
        Some((uid, name))
    }

    fn dump_exchange(&self, method: &str, url: &str, status: u16, body: &str) {
        let dir = std::path::Path::new("dump");
        if let Err(e) = std::fs::create_dir_all(dir) {
            eprintln!("! dump dir: {e}");
            return;
        }
        let n = std::fs::read_dir(dir).map(|d| d.count()).unwrap_or(0) + 1;
        let path_part = url.split('?').next().unwrap_or(url);
        let slug: String = path_part
            .split("/api/")
            .nth(1)
            .unwrap_or("root")
            .split('/')
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join("-");
        let path = dir.join(format!("{n:02}-{slug}.json"));
        let body_json =
            serde_json::from_str::<Value>(body).unwrap_or_else(|_| Value::String(body.to_string()));
        let doc = serde_json::json!({
            "method": method,
            "url": if self.dump_raw { url.to_string() } else { redact_url(url) },
            "status": status,
            "body": body_json,
        });
        let text = serde_json::to_string_pretty(&doc).unwrap_or_default();
        if let Err(e) = std::fs::write(&path, text) {
            eprintln!("! dump write: {e}");
        } else {
            eprintln!("  dumped → {}", path.display());
        }
    }

    /// `_constructUrlParams(se)` → `zpw_ver=695&zpw_type=23`.
    pub fn common_params(&self) -> String {
        format!("zpw_ver={ZPW_VER}&zpw_type={ZPW_TYPE}")
    }

    /// The raw session AES key as bytes, for response decryption.
    pub fn session_key_bytes(&self) -> &[u8] {
        self.secret_key.as_deref().unwrap_or(&[])
    }

    /// `getEncodedParams(obj)` → url-encoded `encodeAES(json)` (`INV-1`).
    ///
    /// # Errors
    /// Returns an error if the session lacks an AES secret key or AES-CBC encryption fails.
    pub fn encoded_params(&self, v: &Value) -> Result<String> {
        let key = self
            .secret_key
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("no session key yet (pre-login)"))?;
        let b64 = crypto::encode_aes(key, &v.to_string())?;
        Ok(urlencode(&b64))
    }

    /// Plain `params=<url-encoded json>` for pre-login endpoints.
    pub fn plain_params(v: &Value) -> String {
        format!("params={}", urlencode(&v.to_string()))
    }

    /// Re-apply the endpoint table from a `getServerInfo` / `getLoginInfo` payload.
    pub fn apply_server_info(&mut self, data: &Value) {
        if let Some(v) = data.get("zpw_chts_new").and_then(domains::first_url) {
            self.domains.lp_chat = v;
        }
        if let Some(v) = data.get("zpw_ctrs_new").and_then(domains::first_url) {
            self.domains.lp_control = v;
        }
        if let Some(arr) = data.get("zpw_ws").and_then(Value::as_array) {
            let list: Vec<String> = arr
                .iter()
                .filter_map(Value::as_str)
                .map(|s| s.trim_end_matches('/').to_string())
                .filter(|s| s.starts_with("wss://") || s.starts_with("ws://"))
                .collect();
            if !list.is_empty() {
                self.domains.ws = list;
            }
        }
        if let Some(map) = data
            .get("zpw_service_map_v3")
            .or_else(|| data.get("zpw_service_map_new"))
        {
            self.domains.apply_service_map(map);
        }
        if let Some(v) = data
            .get("setttings")
            .and_then(|v| v.get("authen"))
            .and_then(|v| v.get("trusted_device"))
            .and_then(|v| v.get("qr_format_trust_device"))
            .and_then(Value::as_i64)
        {
            self.qr_format_trust_device = v == 1;
        }
    }

    pub async fn get(&self, url: &str) -> Result<Value> {
        self.send(Method::GET, url, None).await
    }

    pub async fn post_form(&self, url: &str, body: String) -> Result<Value> {
        self.send(Method::POST, url, Some(body)).await
    }

    /// True only when an error is a transport failure or HTTP 404 (#27).
    pub fn should_failover_candidate(err: &anyhow::Error) -> bool {
        if err.downcast_ref::<SessionError>().is_some() {
            return false;
        }
        if err.downcast_ref::<reqwest::Error>().is_some() {
            return true;
        }
        let msg = err.to_string();
        msg.starts_with("HTTP 404")
            || msg.contains("HTTP 404 ")
            || msg.contains("connection refused")
            || msg.contains("timed out")
            || msg.contains("dns error")
    }

    /// Try each candidate host for `endpoint` in priority order until one succeeds (#27).
    pub async fn request_with_candidates<F, Fut>(
        &self,
        endpoint: &str,
        path_and_query: &str,
        mut send_fn: F,
    ) -> Result<Value>
    where
        F: FnMut(String) -> Fut,
        Fut: Future<Output = Result<Value>>,
    {
        let candidates = self.domains.candidates(endpoint);
        let suffix = if path_and_query.starts_with('/') {
            path_and_query.to_string()
        } else {
            format!("/{path_and_query}")
        };
        let mut last_err: Option<anyhow::Error> = None;
        for host in candidates {
            let url = format!("{host}{suffix}");
            match send_fn(url).await {
                Ok(val) => {
                    self.domains.remember_winner(endpoint, &host);
                    return Ok(val);
                }
                Err(e) => {
                    if !Self::should_failover_candidate(&e) {
                        return Err(e);
                    }
                    crate::logger::log_error(
                        &format!("session::failover::{endpoint}"),
                        &format!("host {host} failed ({e}), trying next candidate"),
                    );
                    last_err = Some(e);
                }
            }
        }
        Err(last_err
            .unwrap_or_else(|| anyhow::anyhow!("no host candidates for endpoint {endpoint}")))
    }

    /// Perform a `GET` request against `endpoint` with multi-candidate host failover (#27).
    pub async fn get_endpoint(&self, endpoint: &str, path_and_query: &str) -> Result<Value> {
        self.request_with_candidates(endpoint, path_and_query, |url| async move {
            self.get(&url).await
        })
        .await
    }

    /// Perform a `POST` form request against `endpoint` with multi-candidate host failover (#27).
    pub async fn post_endpoint_form(
        &self,
        endpoint: &str,
        path_and_query: &str,
        body: String,
    ) -> Result<Value> {
        self.request_with_candidates(endpoint, path_and_query, |url| {
            let b = body.clone();
            async move { self.post_form(&url, b).await }
        })
        .await
    }

    /// Execute an authenticated `GET` against a centralized [`crate::protocol::registry::CommandSpec`] (`INV-1`).
    ///
    /// # Errors
    /// Returns an error if parameter encryption fails or all candidate hosts fail.
    pub async fn api_get(
        &self,
        spec: crate::protocol::registry::CommandSpec,
        params: Option<&Value>,
    ) -> Result<Value> {
        let pq = spec.query_path(self, params)?;
        let resp = self.get_endpoint(spec.domain.service_key(), &pq).await?;
        Ok(self.maybe_decrypt(resp, self.session_key_bytes()).await)
    }

    /// Execute an authenticated `POST` against a centralized [`crate::protocol::registry::CommandSpec`] (`INV-1`).
    ///
    /// # Errors
    /// Returns an error if parameter encryption fails or all candidate hosts fail.
    pub async fn api_post(
        &self,
        spec: crate::protocol::registry::CommandSpec,
        params: Option<&Value>,
    ) -> Result<Value> {
        let (pq, form) = spec.post_parts(self, params)?;
        let resp = self
            .post_endpoint_form(spec.domain.service_key(), &pq, form)
            .await?;
        Ok(self.maybe_decrypt(resp, self.session_key_bytes()).await)
    }

    /// Look up `command_id` in [`crate::protocol::registry::COMMAND_REGISTRY`] and execute [`Self::api_get`].
    ///
    /// # Errors
    /// Returns an error if `command_id` is unknown or the request fails.
    pub async fn api_get_id(&self, command_id: u32, params: Option<&Value>) -> Result<Value> {
        let spec = crate::protocol::registry::lookup_command(command_id)
            .ok_or_else(|| anyhow::anyhow!("unknown Zalo HTTP commandId: {command_id}"))?;
        self.api_get(spec, params).await
    }

    /// Look up `command_id` in [`crate::protocol::registry::COMMAND_REGISTRY`] and execute [`Self::api_post`].
    ///
    /// # Errors
    /// Returns an error if `command_id` is unknown or the request fails.
    pub async fn api_post_id(&self, command_id: u32, params: Option<&Value>) -> Result<Value> {
        let spec = crate::protocol::registry::lookup_command(command_id)
            .ok_or_else(|| anyhow::anyhow!("unknown Zalo HTTP commandId: {command_id}"))?;
        self.api_post(spec, params).await
    }

    /// Send an RFC 7578 `multipart/form-data` binary upload (`chunkContent` or `fileContent`).
    ///
    /// # Errors
    /// Returns an error if the HTTP upload fails or answers a non-2xx status.
    pub async fn post_multipart(
        &self,
        url: &str,
        field_name: &str,
        file_name: &str,
        mime_type: &str,
        file_bytes: &[u8],
    ) -> Result<Value> {
        let boundary = format!(
            "----WebKitFormBoundary{}",
            crate::param_cipher::random_hex(16)
        );
        let safe_name = file_name.replace(['"', '\r', '\n'], "_");
        let head = format!(
            "--{boundary}\r\nContent-Disposition: form-data; name=\"{field_name}\"; filename=\"{safe_name}\"\r\nContent-Type: {mime_type}\r\n\r\n"
        );
        let tail = format!("\r\n--{boundary}--\r\n");
        let mut body = Vec::with_capacity(head.len() + file_bytes.len() + tail.len());
        body.extend_from_slice(head.as_bytes());
        body.extend_from_slice(file_bytes);
        body.extend_from_slice(tail.as_bytes());

        let start = std::time::Instant::now();
        let mut rb = self
            .http
            .request(Method::POST, url)
            .timeout(std::time::Duration::from_secs(90))
            .header(
                "Content-Type",
                format!("multipart/form-data; boundary={boundary}"),
            );
        if let Some(sek) = &self.zpw_sek {
            if !sek.is_empty() {
                rb = rb
                    .header("Cookie", format!("zpw_sek={sek}; _zlang=vi"))
                    .header("zpw_sek", sek)
                    .header("Origin", crate::endpoints::CHAT_ORIGIN)
                    .header("Referer", crate::endpoints::CHAT_REFERER);
            }
        }
        let resp = rb.body(body).send().await?;
        let latency_ms = start.elapsed().as_millis();
        let status = resp.status();
        let text = resp.text().await?;
        crate::logger::log_request(
            "POST(multipart)",
            url,
            Some(status.as_u16()),
            latency_ms,
            &format!("[{field_name}: {safe_name} ({} bytes)]", file_bytes.len()),
            Some(&text),
            if status.is_success() {
                None
            } else {
                Some("HTTP multipart error")
            },
        );
        if !status.is_success() {
            bail!("HTTP {status} from {url}: {}", truncate(&text, 300));
        }
        if text.trim().is_empty() {
            return Ok(serde_json::json!({ "error_code": 0, "data": null }));
        }
        let v: Value = serde_json::from_str(&text)
            .unwrap_or_else(|_| serde_json::json!({ "data": text.trim() }));
        Ok(v)
    }

    async fn send(&self, m: Method, url: &str, body: Option<String>) -> Result<Value> {
        let start = std::time::Instant::now();
        if is_url_blocked_by_privacy(url) {
            let method = m.as_str().to_string();
            crate::logger::log_request(
                &method,
                url,
                Some(200),
                0,
                body.as_deref().unwrap_or(""),
                Some(r#"{"error_code":0,"error_message":"Blocked by Privacy Guard","data":{}}"#),
                None,
            );
            return Ok(serde_json::json!({
                "error_code": 0,
                "error_message": "Blocked by Privacy Guard",
                "data": {}
            }));
        }
        if self.verbose {
            eprintln!(
                "→ {} {}",
                m,
                if self.dump_raw {
                    url.to_string()
                } else {
                    redact_url(url)
                }
            );
            if let Some(b) = &body {
                eprintln!("  body: {}", truncate(b, 400));
            }
        }
        let method = m.as_str().to_string();
        let mut rb = self.http.request(m, url);
        if let Some(ref sek) = self.zpw_sek {
            if !sek.is_empty() && crate::endpoints::is_zalo_cookie_host(url) {
                rb = rb
                    .header("Cookie", format!("zpw_sek={sek}; _zlang=vi"))
                    .header("zpw_sek", sek)
                    .header("Origin", crate::endpoints::CHAT_ORIGIN)
                    .header("Referer", crate::endpoints::CHAT_REFERER);
            }
        }
        if let Some(b) = &body {
            rb = rb
                .header("Content-Type", "application/x-www-form-urlencoded")
                .body(b.clone());
        }
        let resp_result = rb.send().await;
        let latency_ms = start.elapsed().as_millis();

        let resp = match resp_result {
            Ok(r) => r,
            Err(e) => {
                crate::logger::log_request(
                    &method,
                    url,
                    None,
                    latency_ms,
                    body.as_deref().unwrap_or(""),
                    None,
                    Some(&e.to_string()),
                );
                return Err(e.into());
            }
        };

        let status = resp.status();
        let text = resp.text().await?;
        if self.verbose {
            eprintln!("← {status} {}", truncate(&text, 400));
        }
        if self.dump {
            self.dump_exchange(&method, url, status.as_u16(), &text);
        }

        let is_err = !status.is_success();
        let err_str = if is_err {
            Some(format!("HTTP {status}"))
        } else {
            None
        };

        crate::logger::log_request(
            &method,
            url,
            Some(status.as_u16()),
            latency_ms,
            body.as_deref().unwrap_or(""),
            Some(&text),
            err_str.as_deref(),
        );

        if is_err {
            bail!("HTTP {status} from {url}: {}", truncate(&text, 300));
        }
        if text.trim().is_empty() {
            return Ok(serde_json::json!({ "error_code": 0, "data": null }));
        }
        let v: Value = serde_json::from_str(&text).unwrap_or_else(|_| Value::String(text.clone()));
        let code = v.get("error_code").and_then(Value::as_i64).unwrap_or(0);
        if code != 0 {
            let msg = v.get("error_message").and_then(Value::as_str).unwrap_or("");
            crate::logger::log_request(
                &method,
                url,
                Some(status.as_u16()),
                latency_ms,
                body.as_deref().unwrap_or(""),
                Some(&text),
                Some(&format!("server error {code}: {msg}")),
            );
            return Err(SessionError::new(code, msg).at(url).into());
        }
        Ok(v)
    }

    /// `decryptResp` — when the request was param-encrypted, `data` comes back as base64 AES.
    pub async fn maybe_decrypt(&self, v: Value, key: &[u8]) -> Value {
        let Some(data) = v.get("data") else {
            return v;
        };
        let Some(b64) = data.as_str() else {
            return v;
        };
        if b64.is_empty() || b64.starts_with('{') || key.is_empty() {
            return v;
        }
        match crate::crypto::decode_aes(key, b64) {
            Ok(pt) => match serde_json::from_str::<Value>(&pt) {
                Ok(inner) => {
                    if self.verbose {
                        eprintln!("  data decrypted ({} bytes)", pt.len());
                    }
                    inner
                }
                Err(e) => {
                    if self.verbose {
                        eprintln!(
                            "  data decrypted but not JSON: {e} / {}",
                            truncate(&pt, 200)
                        );
                    }
                    v
                }
            },
            Err(e) => {
                if self.verbose {
                    eprintln!("  data decrypt failed: {e}");
                }
                v
            }
        }
    }
}

#[cfg(test)]
mod tests {
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

    #[test]
    fn session_set_proxy_rebuilds_client_and_preserves_zpw_sek() {
        let mut s = Session::new(false, false, false).unwrap();
        s.set_zpw_sek("sek_token_123".into());
        let configured = s.set_proxy(Some("http://127.0.0.1:8080")).unwrap();
        assert_eq!(configured.as_deref(), Some("http://127.0.0.1:8080"));
        assert_eq!(s.proxy_url.as_deref(), Some("http://127.0.0.1:8080"));
        assert_eq!(s.zpw_sek.as_deref(), Some("sek_token_123"));

        let cleared = s.set_proxy(None).unwrap();
        assert!(cleared.is_none());
        assert!(s.proxy_url.is_none());
    }
}
