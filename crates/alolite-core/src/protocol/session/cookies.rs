//! Cookie jar seeding and network proxy configuration.

use anyhow::Result;

use super::privacy::normalize_proxy_url;
use super::transport::build_http_client;
use super::Session;

impl Session {
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
}
