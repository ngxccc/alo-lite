//! Working endpoint table, taken from a live Zalo Web session (2026-10-21).
//!
//! Replaces guesswork with observed values. Every entry below answered `error_code 0`
//! during a real QR-authenticated session.

/// Service map as returned by `zpw_service_map_v3` on `chat.zalo.me`.
///
/// Note the domains are `*.chat.zalo.me`, **not** `*.zaloapp.com` — the PC bundle's
/// default table is stale for the web/QR session flavour.
pub const SERVICE_MAP_V3: &[(&str, &str)] = &[
    ("profile", "https://tt-profile-wpa.chat.zalo.me"),
    ("friend", "https://tt-friend-wpa.chat.zalo.me"),
    ("conversation", "https://tt-convers-wpa.chat.zalo.me"),
    ("chat", "https://tt-chat3-wpa.chat.zalo.me"),
    ("group", "https://tt-group-wpa.chat.zalo.me"),
    ("sticker", "https://tt-sticker-wpa.chat.zalo.me"),
    ("alias", "https://tt-alias-wpa.chat.zalo.me"),
    ("label", "https://label-wpa.chat.zalo.me"),
    ("reaction", "https://reaction.chat.zalo.me"),
    ("emoji_status", "https://emoji-status.chat.zalo.me"),
    ("zcloud", "https://zcld.chat.zalo.me"),
    ("file", "https://tt-files-wpa.chat.zalo.me"),
    ("voice_call", "https://voicecall-wpa.chat.zalo.me"),
];

/// `zpw_ctrs_new[0]` — conversation control plane.
pub const CONVERS_CTL: &str = "https://p5-ctl-chat.zaloapp.com";

/// `zpw_chts_new[0]` — message plane.
pub const MSG_CHAT: &str = "https://p5-msg-chat.zaloapp.com";

/// Auth host for `reqqr` / password login. The official Linux client posts
/// `getServerInfo` / `reqqr` / `getLoginInfo` to `wpa.chat.zalo.me` (captured 2026-11-07).
pub const AUTH: &str = "https://wpa.chat.zalo.me";

/// Base host that serves `getServerInfo` for an authenticated web session.
pub const WPA_CHAT: &str = "https://wpa.chat.zalo.me";

/// Plain-to-noised ID translation host (`/api/znoise`, command `12390`) used by Sync V1 backup import.
pub const ZWID: &str = "https://zwid.api.zalo.me";

/// Reaction API host used by `/api/message/reaction` and `/api/group/reaction`.
pub const REACTION_API: &str = "https://reaction.api.zaloapp.com";
/// Canonical Origin header sent with Zalo Web/PC requests.
pub const CHAT_ORIGIN: &str = "https://chat.zalo.me";
/// Canonical Referer header sent with Zalo Web/PC requests.
pub const CHAT_REFERER: &str = "https://chat.zalo.me/";
/// Default long-polling message host (`zpw_chts_new` fallback).
pub const LP_CHAT_DEFAULT: &str = "https://p4-msg-chat.zaloapp.com";
/// Default long-polling control host (`zpw_ctrs_new` fallback).
pub const LP_CONTROL_DEFAULT: &str = "https://p4-ctl-chat.zaloapp.com";
/// Default WebSocket fallback endpoint (`zpw_ws[0]`).
pub const WS_DEFAULT: &str = "wss://ws1-msg.chat.zalo.me";
/// Fallback message-send host when `domains.chat` holds a long-polling or legacy slot.
pub const CHAT_SEND_FALLBACK: &str = "https://tt-chat3-wpa.api.zaloapp.com";
/// Fallback file upload host when `domains.file` is empty.
pub const FILE_FALLBACK_HOST: &str = "https://tt-files-wpa.zaloapp.com";
/// Secondary sticker API host (`*.zaloapp.com` mirror).
pub const STICKER_LEGACY_HOST: &str = "https://tt-sticker-wpa.zaloapp.com";
/// Default CDN catalog URL for `fetch_sticker_store_packs`.
pub const STICKER_CDN_ALL_CATE: &str = "https://zalo-api.zadn.vn/api/emoticon/pcweb/getAllCate";
/// Static thumbnail base URL for built-in default stickers (`cate_id == 0`, `1..=30`).
pub const STICKER_DEFAULT_THUMB_BASE: &str =
    "https://stc-chat.zdn.vn/images/stickers/default/thumb";
/// WebP/PNG sticker rendering endpoint base URL.
pub const STICKER_WEBPC_BASE: &str = "https://api.zaloapp.com/api/emoticon/sticker/webpc";

/// Seed hosts where `zpw_sek` session cookies are pre-attached on login.
pub const COOKIE_SEED_HOSTS: &[&str] = &[
    "https://chat.zalo.me",
    "https://wpa.zaloapp.com",
    "https://wpa.chat.zalo.me",
    "https://tt-sticker-wpa.chat.zalo.me",
    "https://tt-sticker-wpa.zaloapp.com",
    "https://sticker-wpa.chat.zalo.me",
];

/// Group/Cloud history candidate hosts (`zpw_service_map_v3.group_cloud_message`).
pub const GROUP_CLOUD_CANDIDATES: &[&str] = &[
    "https://tt-group-cm.chat.zalo.me",
    "https://tt-group-cm.api.zaloapp.com",
];

/// Fallback hosts for extended `zpw_service_map_v3` keys not listed in [`SERVICE_MAP_V3`]
/// when a session calls [`crate::protocol::registry`] commands before or without a dynamic map entry.
pub const EXTENDED_FALLBACK_MAP: &[(&str, &str)] = &[
    ("auth", AUTH),
    ("wpa_chat", WPA_CHAT),
    ("zwid", ZWID),
    ("group_cloud_message", "https://tt-group-cm.chat.zalo.me"),
    ("chat_e2e", "https://tt-e2ee-wpa.chat.zalo.me"),
    ("group_e2e", "https://tt-group-e2ee-wpa.chat.zalo.me"),
    ("group_board", "https://groupboard-wpa.chat.zalo.me"),
    ("group_poll", "https://tt-group-poll-wpa.chat.zalo.me"),
    ("boards", "https://boards-wpa.chat.zalo.me"),
    ("up_file", "https://tt-files-wpa.chat.zalo.me"),
    ("zcloud_up_file", "https://zcld.chat.zalo.me"),
    ("media_store", "https://tt-mediastore-wpa.chat.zalo.me"),
    (
        "media_store_send2me",
        "https://tt-mediastore-wpa.chat.zalo.me",
    ),
    ("quick_message", "https://quickmsg-wpa.chat.zalo.me"),
    ("auto_reply", "https://autoreply-wpa.chat.zalo.me"),
    ("recent_search", "https://tt-convers-wpa.chat.zalo.me"),
    ("catalog", "https://catalog-wpa.chat.zalo.me"),
    ("friend_board", "https://friendboard-wpa.chat.zalo.me"),
    ("friendLan", "https://tt-friend-wpa.chat.zalo.me"),
    ("emoji_stt", "https://emoji-status.chat.zalo.me"),
    ("ai_features", "https://aifeatures-wpa.chat.zalo.me"),
    ("backuppcmobile", "https://bkpc2mb.zaloapp.com"),
    ("zimsg", "https://zimsg-wpa.chat.zalo.me"),
    ("banner", "https://banner-wpa.chat.zalo.me"),
    ("sync_action", "https://tt-convers-wpa.chat.zalo.me"),
    ("fallback_LP", "https://wpa.chat.zalo.me"),
    ("qr_code", "https://qr.id.zalo.me"),
];
/// Look up a service domain by key.
pub fn service(key: &str) -> Option<&'static str> {
    SERVICE_MAP_V3
        .iter()
        .find(|(k, _)| *k == key)
        .map(|(_, v)| *v)
}

/// Look up either a verified [`SERVICE_MAP_V3`] host or an [`EXTENDED_FALLBACK_MAP`] host by service key.
pub fn fallback_host(key: &str) -> Option<&'static str> {
    service(key).or_else(|| {
        EXTENDED_FALLBACK_MAP
            .iter()
            .find(|(k, _)| *k == key)
            .map(|(_, v)| *v)
    })
}

/// Default `wss://ws{1..8}-msg.chat.zalo.me` WebSocket shard list (`re-notes/02-chat-data.md`).
pub fn default_ws_endpoints() -> Vec<String> {
    (1..=8)
        .map(|n| format!("wss://ws{n}-msg.chat.zalo.me"))
        .collect()
}

/// Return `true` when `url` targets a `zalo.me` or `zaloapp.com` domain eligible for `zpw_sek` cookies.
pub fn is_zalo_cookie_host(url: &str) -> bool {
    reqwest::Url::parse(url)
        .map(|u| {
            u.host_str().is_some_and(|h| {
                h == "zalo.me"
                    || h.ends_with(".zalo.me")
                    || h == "zaloapp.com"
                    || h.ends_with(".zaloapp.com")
            })
        })
        .unwrap_or_else(|_| url.contains("zalo.me") || url.contains("zaloapp.com"))
}
