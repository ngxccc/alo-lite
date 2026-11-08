//! Typed Zalo API error classification and session validity states (`#21`).
//!
//! Distinguishes definitive server credential rejection (`AUTH_ERROR_CODES`) from
//! transient network/HTTP errors so `session.json` is never destroyed on offline glitches.

/// `error_code` values Zalo answers when the stored credentials are no longer usable.
pub const AUTH_ERROR_CODES: &[i64] = &[600, -102];

/// A non-zero `error_code` from Zalo, carrying the number instead of burying it in a
/// formatted message.
#[derive(Debug)]
pub struct SessionError {
    pub code: i64,
    pub message: String,
    pub url: String,
}

impl SessionError {
    pub fn new(code: i64, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            url: String::new(),
        }
    }

    /// Attach the endpoint that answered, when the caller knows it.
    pub fn at(mut self, url: impl Into<String>) -> Self {
        self.url = url.into();
        self
    }

    /// The server rejected the credentials, not the request.
    pub fn is_auth(&self) -> bool {
        AUTH_ERROR_CODES.contains(&self.code)
    }
}

impl std::fmt::Display for SessionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.url.is_empty() {
            write!(f, "server error {}: {}", self.code, self.message)
        } else {
            write!(
                f,
                "server error {}: {} (from {})",
                self.code, self.message, self.url
            )
        }
    }
}

impl std::error::Error for SessionError {}

/// What the server — or the network — last said about the stored session (`#21`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionStatus {
    Valid,
    /// The server refused the credentials: re-login required.
    Expired,
    /// No verdict: transport, HTTP, unreadable body, or an unrelated `error_code`.
    /// Never destroy `session.json` for this.
    Unreachable,
}

impl SessionStatus {
    /// Only [`AUTH_ERROR_CODES`] mean the session is gone; `0` is success and anything
    /// else leaves the status undetermined.
    pub fn from_error_code(code: i64) -> Self {
        if code == 0 {
            Self::Valid
        } else if AUTH_ERROR_CODES.contains(&code) {
            Self::Expired
        } else {
            Self::Unreachable
        }
    }

    /// Classify a failed request. Only a [`SessionError`] knows its code; every other
    /// error (timeout, DNS, `HTTP 500`, parse) is [`SessionStatus::Unreachable`].
    pub fn from_error(err: &anyhow::Error) -> Self {
        match err.downcast_ref::<SessionError>() {
            Some(e) => Self::from_error_code(e.code),
            None => Self::Unreachable,
        }
    }

    pub fn is_valid(self) -> bool {
        matches!(self, Self::Valid)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_maps_error_codes_and_only_auth_codes_expire() {
        assert_eq!(SessionStatus::from_error_code(0), SessionStatus::Valid);
        assert_eq!(SessionStatus::from_error_code(600), SessionStatus::Expired);
        assert_eq!(SessionStatus::from_error_code(-102), SessionStatus::Expired);
        for code in [111, 114, 18022, -1, 500, 401] {
            assert_eq!(
                SessionStatus::from_error_code(code),
                SessionStatus::Unreachable,
                "error_code {code} must not be treated as an expired session"
            );
        }
    }

    #[test]
    fn status_classifies_typed_server_errors_and_bare_failures() {
        let auth: anyhow::Error = SessionError::new(600, "zpw_sek bị thiếu hoặc không đúng").into();
        assert_eq!(SessionStatus::from_error(&auth), SessionStatus::Expired);

        let unrelated: anyhow::Error = SessionError::new(111, "invalid param").into();
        assert_eq!(
            SessionStatus::from_error(&unrelated),
            SessionStatus::Unreachable
        );

        for msg in [
            "HTTP 404 Not Found from https://tt-friend-wpa.chat.zalo.me/api/x",
            "error sending request for url (https://wpa.chat.zalo.me/...): connection refused",
            "error decoding response body",
        ] {
            let e = anyhow::anyhow!("{msg}");
            assert_eq!(
                SessionStatus::from_error(&e),
                SessionStatus::Unreachable,
                "{msg} must not expire the session"
            );
        }
    }
}
