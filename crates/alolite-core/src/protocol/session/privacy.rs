//! Privacy Guard outbound interceptor and secret/URL redaction.
//!
//! Ensures credentials, `zpw_sek`, `zpw_enk`, and `params` never leak into
//! logs, status banners, or debug dump files (`docs/standards/security-and-cryptography.md`).

use std::sync::atomic::{AtomicBool, Ordering};

static PRIVACY_BLOCK_TYPING: AtomicBool = AtomicBool::new(false);
static PRIVACY_BLOCK_DELIVERED: AtomicBool = AtomicBool::new(false);
static PRIVACY_BLOCK_SEEN: AtomicBool = AtomicBool::new(false);

/// Update the active client-side privacy blocker switches (`block_typing`, `block_delivered`, `block_seen`).
pub fn set_privacy_blocker(block_typing: bool, block_delivered: bool, block_seen: bool) {
    PRIVACY_BLOCK_TYPING.store(block_typing, Ordering::Relaxed);
    PRIVACY_BLOCK_DELIVERED.store(block_delivered, Ordering::Relaxed);
    PRIVACY_BLOCK_SEEN.store(block_seen, Ordering::Relaxed);
}

/// Return `(block_typing, block_delivered, block_seen)`.
pub fn privacy_blocker_state() -> (bool, bool, bool) {
    (
        PRIVACY_BLOCK_TYPING.load(Ordering::Relaxed),
        PRIVACY_BLOCK_DELIVERED.load(Ordering::Relaxed),
        PRIVACY_BLOCK_SEEN.load(Ordering::Relaxed),
    )
}

/// Check whether `url` matches an outbound typing / delivered / seen telemetry endpoint
/// that the user has actively blocked in Privacy Guard.
pub fn is_url_blocked_by_privacy(url: &str) -> bool {
    if PRIVACY_BLOCK_TYPING.load(Ordering::Relaxed)
        && (url.contains("/api/message/typing") || url.contains("/api/group/typing"))
    {
        return true;
    }
    if PRIVACY_BLOCK_DELIVERED.load(Ordering::Relaxed)
        && (url.contains("/api/message/delivered")
            || url.contains("/api/group/delivered")
            || url.contains("/api/e2ee/pc/t/message/delivered"))
    {
        return true;
    }
    if PRIVACY_BLOCK_SEEN.load(Ordering::Relaxed)
        && (url.contains("/api/message/seen")
            || url.contains("/api/group/seen")
            || url.contains("/api/e2ee/pc/t/message/seen")
            || url.contains("/api/e2ee/pc/t/group/seen"))
    {
        return true;
    }
    false
}

/// Strip anything that could be a credential before a URL lands in a log or dump file.
pub fn redact_url(url: &str) -> String {
    const SENSITIVE: &[&str] = &[
        "password", "passw", "zpw_sek", "zpw_enk", "cookie", "token", "signkey", "params", "imei",
    ];
    let Some((base, query)) = url.split_once('?') else {
        return url.to_string();
    };
    let cleaned: Vec<String> = query
        .split('&')
        .map(|kv| {
            let key = kv.split('=').next().unwrap_or("").to_ascii_lowercase();
            if SENSITIVE.iter().any(|s| key.contains(s)) {
                format!("{}=<redacted>", kv.split('=').next().unwrap_or(""))
            } else {
                kv.to_string()
            }
        })
        .collect();
    format!("{base}?{}", cleaned.join("&"))
}

/// Redact a sensitive secret string, showing at most the first 8 characters followed by `…<redacted>`.
pub fn redact_secret(s: &str) -> String {
    let prefix: String = s.chars().take(8).collect();
    format!("{prefix}…<redacted>")
}

/// RFC 3986 unreserved percent-encoder (`A-Z`, `a-z`, `0-9`, `-`, `_`, `.`, `~` pass through).
pub fn urlencode(s: &str) -> String {
    let mut out = String::with_capacity(s.len() * 3);
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char);
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

/// Truncate `s` to at most `n` Unicode scalar values, appending `…` when clipped.
pub fn truncate(s: &str, n: usize) -> String {
    if s.chars().count() <= n {
        s.to_string()
    } else {
        let t: String = s.chars().take(n).collect();
        format!("{t}…")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redact_secret_truncates_to_eight_chars() {
        assert_eq!(redact_secret("1234567890abcdef"), "12345678…<redacted>");
        assert_eq!(redact_secret("short"), "short…<redacted>");
        assert_eq!(redact_secret(""), "…<redacted>");
    }

    #[test]
    fn privacy_blocker_intercepts_typing_delivered_and_seen_endpoints() {
        set_privacy_blocker(false, false, false);
        assert!(!is_url_blocked_by_privacy(
            "https://tt-chat1-wpa.chat.zalo.me/api/message/typing?zpw_ver=694"
        ));
        assert!(!is_url_blocked_by_privacy(
            "https://tt-chat1-wpa.chat.zalo.me/api/message/seenv2?zpw_ver=694"
        ));

        set_privacy_blocker(true, false, true);
        assert!(is_url_blocked_by_privacy(
            "https://tt-chat1-wpa.chat.zalo.me/api/message/typing?zpw_ver=694"
        ));
        assert!(is_url_blocked_by_privacy(
            "https://tt-group-wpa.chat.zalo.me/api/group/typing?zpw_ver=694"
        ));
        assert!(is_url_blocked_by_privacy(
            "https://tt-chat1-wpa.chat.zalo.me/api/message/seenv2?zpw_ver=694"
        ));
        assert!(is_url_blocked_by_privacy(
            "https://tt-group-wpa.chat.zalo.me/api/group/seenv2?zpw_ver=694"
        ));
        assert!(!is_url_blocked_by_privacy(
            "https://tt-chat1-wpa.chat.zalo.me/api/message/deliveredv2?zpw_ver=694"
        ));
        assert!(!is_url_blocked_by_privacy(
            "https://tt-chat1-wpa.chat.zalo.me/api/message/sms?zpw_ver=694"
        ));
        set_privacy_blocker(false, false, false);
    }
}
