//! Cookie jar seeding.

use reqwest::header::HeaderMap;

use super::Session;

/// Pull the `zpw_sek` value out of a `Set-Cookie` header block.
///
/// The login edge answers with the cookie it actually minted; that value — not the one echoed
/// inside the encrypted JSON body — is what the next hop validates.
pub(crate) fn extract_set_cookie_sek(headers: &HeaderMap) -> Option<String> {
    headers
        .get_all(reqwest::header::SET_COOKIE)
        .iter()
        .filter_map(|v| v.to_str().ok())
        .filter_map(|raw| {
            let (pair, _) = raw.split_once(';')?;
            let (name, value) = pair.split_once('=')?;
            (name.trim().eq_ignore_ascii_case("zpw_sek") && !value.trim().is_empty())
                .then(|| value.trim().to_string())
        })
        .next_back()
}

impl Session {
    /// Take the last server-issued `zpw_sek` seen in a `Set-Cookie` header, clearing the slot.
    pub(crate) fn take_server_zpw_sek(&self) -> Option<String> {
        self.server_zpw_sek
            .lock()
            .ok()
            .and_then(|mut slot| slot.take())
    }

    /// Record a server-issued `zpw_sek` for the caller that owns the session state.
    pub(crate) fn stash_server_zpw_sek(&self, value: String) {
        if let Ok(mut slot) = self.server_zpw_sek.lock() {
            *slot = Some(value);
        }
    }

    /// Insert `name=value` as one wildcard cookie per registrable Zalo domain (`.zalo.me`,
    /// `.zaloapp.com`) so every API subdomain sees exactly one copy.
    pub(crate) fn seed_wildcard_cookie(&self, name: &str, value: &str) {
        for (bare, domain) in [("zalo.me", ".zalo.me"), ("zaloapp.com", ".zaloapp.com")] {
            if let Ok(url) = reqwest::Url::parse(&format!("https://{bare}")) {
                let cookie = format!("{name}={value}; Domain={domain}; Path=/");
                self.jar.add_cookie_str(&cookie, &url);
            }
        }
    }

    /// Store `zpw_sek` session token, attach it to `self.login`, and insert the session cookies.
    pub fn set_zpw_sek(&mut self, v: String) {
        if v.is_empty() {
            return;
        }
        // One wildcard cookie per registrable domain reaches every API subdomain
        // (`tt-profile-wpa`, `tt-group-cm`, `lp9-msg`, …) without emitting the same
        // `zpw_sek` twice in the outgoing `Cookie` header. `_zlang` travels with it
        // because the transport no longer hand-builds the `Cookie` header.
        self.seed_wildcard_cookie("zpw_sek", &v);
        self.seed_wildcard_cookie("_zlang", "vi");
        // Host-only fallback for any seeded or discovered host outside those domains.
        let mut hosts: Vec<&str> = crate::endpoints::COOKIE_SEED_HOSTS.to_vec();
        hosts.extend([
            self.domains.auth.as_str(),
            self.domains.chat.as_str(),
            self.domains.conversation.as_str(),
            self.domains.lp_chat.as_str(),
            self.domains.lp_control.as_str(),
            self.domains.convers_api.as_str(),
            self.domains.profile.as_str(),
            self.domains.friend.as_str(),
            self.domains.group.as_str(),
            self.domains.cloud_msg.as_str(),
            self.domains.voice_call.as_str(),
            self.domains.sticker.as_str(),
        ]);
        for host in hosts {
            let bare = host.trim_start_matches("https://");
            if bare.ends_with("zalo.me") || bare.ends_with("zaloapp.com") {
                continue;
            }
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
