//! Low-level HTTP client construction, exchange dumping, and network dispatch.

use std::sync::Arc;

use anyhow::{bail, Result};
use reqwest::cookie::Jar;
use reqwest::header::{HeaderMap, HeaderValue, ORIGIN, REFERER};
use reqwest::{Client, Method};
use serde_json::Value;

use super::privacy::{is_url_blocked_by_privacy, redact_url, truncate};
use super::status::SessionError;
use super::{Session, ZALO_UA};
pub(crate) fn build_http_client(jar: &Arc<Jar>) -> Result<Client> {
    let mut default_headers = HeaderMap::new();
    default_headers.insert(
        REFERER,
        HeaderValue::from_static(crate::endpoints::CHAT_REFERER),
    );
    default_headers.insert(
        ORIGIN,
        HeaderValue::from_static(crate::endpoints::CHAT_ORIGIN),
    );
    let builder = Client::builder()
        .cookie_provider(Arc::clone(jar))
        .default_headers(default_headers)
        .connect_timeout(std::time::Duration::from_secs(10))
        .timeout(std::time::Duration::from_secs(35))
        .user_agent(ZALO_UA);
    Ok(builder.build()?)
}

impl Session {
    pub(crate) fn dump_exchange(&self, method: &str, url: &str, status: u16, body: &str) {
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

    pub async fn get(&self, url: &str) -> Result<Value> {
        self.send(Method::GET, url, None).await
    }

    pub async fn post_form(&self, url: &str, body: String) -> Result<Value> {
        self.send(Method::POST, url, Some(body)).await
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

    pub(crate) async fn send(&self, m: Method, url: &str, body: Option<String>) -> Result<Value> {
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
        let env: crate::protocol::wire::ErrorEnvelopeWireData =
            serde_json::from_value(v.clone()).unwrap_or_default();
        if env.code() != 0 {
            let code = env.code();
            let msg = env.message();
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
}
