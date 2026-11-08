//! Centralized Zalo protocol command registry (`commandId` → `(ApiDomain, path)` and WebSocket `cmd` catalog).
//!
//! Ground-truth specification derived from `Zalo 26.11.10-universal/app.asar` (`re-notes/12` & `re-notes/13`).
//! All outbound HTTP calls should reference [`CommandSpec`] constants or [`lookup_command`] via
//! [`crate::session::Session::api_get`] / [`crate::session::Session::api_post`] rather than hardcoding paths.

use anyhow::Result;
use serde_json::Value;

use crate::session::Session;

/// Logical service domain in `zpw_service_map_v3` (`pUq9` in `app.asar`).
///
/// Host URLs are never hardcoded here; every variant maps to its wire `zpw_service_map_v3` key via
/// [`Self::service_key`] and resolves fallback hosts exclusively through [`crate::endpoints`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ApiDomain {
    AiFeatures,
    Alias,
    Auth,
    AutoReply,
    BackupPcMobile,
    Banner,
    Board,
    Catalog,
    Chat,
    ChatE2ee,
    ChatE2eeGroup,
    CloudApi,
    Conversation,
    EmojiStatus,
    FallbackLp,
    File,
    Friend,
    FriendBoard,
    FriendLan,
    Group,
    GroupBoard,
    GroupCloud,
    GroupPoll,
    Label,
    MediaStore,
    MediaStoreSend2Me,
    Photo,
    PhotoOriginal,
    Profile,
    QrCode,
    QuickMessage,
    Reaction,
    RecentSearch,
    Sticker,
    SyncAction,
    VoiceCall,
    ZInstantMsg,
    Zwid,
}

impl ApiDomain {
    /// Wire key used inside `zpw_service_map_v3` and [`crate::session::Domains::candidates`].
    pub const fn service_key(self) -> &'static str {
        match self {
            Self::AiFeatures => "ai_features",
            Self::Alias => "alias",
            Self::Auth => "auth",
            Self::AutoReply => "auto_reply",
            Self::BackupPcMobile => "backuppcmobile",
            Self::Banner => "banner",
            Self::Board => "boards",
            Self::Catalog => "catalog",
            Self::Chat => "chat",
            Self::ChatE2ee => "chat_e2e",
            Self::ChatE2eeGroup => "group_e2e",
            Self::CloudApi => "zcloud",
            Self::Conversation => "conversation",
            Self::EmojiStatus => "emoji_stt",
            Self::FallbackLp => "fallback_LP",
            Self::File => "file",
            Self::Friend => "friend",
            Self::FriendBoard => "friend_board",
            Self::FriendLan => "friendLan",
            Self::Group => "group",
            Self::GroupBoard => "group_board",
            Self::GroupCloud => "group_cloud_message",
            Self::GroupPoll => "group_poll",
            Self::Label => "label",
            Self::MediaStore => "media_store",
            Self::MediaStoreSend2Me => "media_store_send2me",
            Self::Photo => "up_file",
            Self::PhotoOriginal => "zcloud_up_file",
            Self::Profile => "profile",
            Self::QrCode => "qr_code",
            Self::QuickMessage => "quick_message",
            Self::Reaction => "reaction",
            Self::RecentSearch => "recent_search",
            Self::Sticker => "sticker",
            Self::SyncAction => "sync_action",
            Self::VoiceCall => "voice_call",
            Self::ZInstantMsg => "zimsg",
            Self::Zwid => "zwid",
        }
    }

    /// Parse a wire `zpw_service_map_v3` key into its typed [`ApiDomain`].
    pub fn from_service_key(key: &str) -> Option<Self> {
        Some(match key {
            "ai_features" => Self::AiFeatures,
            "alias" => Self::Alias,
            "auth" | "wpa_chat" => Self::Auth,
            "auto_reply" => Self::AutoReply,
            "backuppcmobile" => Self::BackupPcMobile,
            "banner" => Self::Banner,
            "boards" => Self::Board,
            "catalog" => Self::Catalog,
            "chat" => Self::Chat,
            "chat_e2e" => Self::ChatE2ee,
            "group_e2e" => Self::ChatE2eeGroup,
            "zcloud" => Self::CloudApi,
            "conversation" | "convers_api" => Self::Conversation,
            "emoji_stt" | "emoji_status" => Self::EmojiStatus,
            "fallback_LP" => Self::FallbackLp,
            "file" => Self::File,
            "friend" => Self::Friend,
            "friend_board" => Self::FriendBoard,
            "friendLan" => Self::FriendLan,
            "group" => Self::Group,
            "group_board" => Self::GroupBoard,
            "group_cloud_message" | "cloud_msg" => Self::GroupCloud,
            "group_poll" => Self::GroupPoll,
            "label" => Self::Label,
            "media_store" => Self::MediaStore,
            "media_store_send2me" => Self::MediaStoreSend2Me,
            "up_file" => Self::Photo,
            "zcloud_up_file" => Self::PhotoOriginal,
            "profile" => Self::Profile,
            "qr_code" => Self::QrCode,
            "quick_message" => Self::QuickMessage,
            "reaction" => Self::Reaction,
            "recent_search" => Self::RecentSearch,
            "sticker" => Self::Sticker,
            "sync_action" => Self::SyncAction,
            "voice_call" => Self::VoiceCall,
            "zimsg" => Self::ZInstantMsg,
            "zwid" => Self::Zwid,
            _ => return None,
        })
    }

    /// Resolve the verified fallback host from [`crate::endpoints`].
    pub fn fallback_host(self) -> Option<&'static str> {
        crate::endpoints::fallback_host(self.service_key())
    }
}

/// Immutable descriptor for one Zalo HTTP protocol command (`commandId`, service domain, and path).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CommandSpec {
    pub id: u32,
    pub domain: ApiDomain,
    pub path: &'static str,
}

impl CommandSpec {
    /// Construct a compile-time command specification.
    pub const fn new(id: u32, domain: ApiDomain, path: &'static str) -> Self {
        Self { id, domain, path }
    }

    /// Build `"{path}?{common_params}"` (plus `&params={encoded_params}` when `params` is `Some`),
    /// suitable for [`Session::get_endpoint`].
    ///
    /// # Errors
    /// Returns an error if `params` is `Some` and session AES encryption fails (`INV-1`).
    pub fn query_path(self, s: &Session, params: Option<&Value>) -> Result<String> {
        match params {
            Some(v) => {
                let enc = s.encoded_params(v)?;
                Ok(format!("{}?{}&params={enc}", self.path, s.common_params()))
            }
            None => Ok(format!("{}?{}", self.path, s.common_params())),
        }
    }

    /// Build `(query_path, form_body)` suitable for [`Session::post_endpoint_form`].
    ///
    /// # Errors
    /// Returns an error if `params` is `Some` and session AES encryption fails (`INV-1`).
    pub fn post_parts(self, s: &Session, params: Option<&Value>) -> Result<(String, String)> {
        let pq = format!("{}?{}", self.path, s.common_params());
        let form = match params {
            Some(v) => format!("params={}", s.encoded_params(v)?),
            None => String::new(),
        };
        Ok((pq, form))
    }
}

mod http_table;
pub use http_table::COMMAND_REGISTRY;

/// Look up an HTTP [`CommandSpec`] by numeric `commandId` in $O(\log N)$ time with zero allocation.
pub fn lookup_command(id: u32) -> Option<CommandSpec> {
    COMMAND_REGISTRY
        .binary_search_by_key(&id, |spec| spec.id)
        .ok()
        .map(|idx| COMMAND_REGISTRY[idx])
}

/// Look up the first [`CommandSpec`] matching an exact endpoint `path` (e.g. `"/api/znoise"`).
pub fn lookup_command_by_path(path: &str) -> Option<CommandSpec> {
    COMMAND_REGISTRY
        .iter()
        .copied()
        .find(|spec| spec.path == path)
}

/// Iterate over every registered [`CommandSpec`] belonging to `domain`.
pub fn commands_in_domain(domain: ApiDomain) -> impl Iterator<Item = CommandSpec> {
    COMMAND_REGISTRY
        .iter()
        .copied()
        .filter(move |spec| spec.domain == domain)
}

// ---------------------------------------------------------------------------
// Ergonomic Named HTTP Command Constants by Feature Domain
// ---------------------------------------------------------------------------

/// Authentication, QR login, password login, and account security commands.
pub mod auth {
    use super::{ApiDomain, CommandSpec};

    pub const REQ_QR: CommandSpec = CommandSpec::new(11133, ApiDomain::Auth, "/api/login/reqqr");
    pub const GET_LOGIN_INFO: CommandSpec =
        CommandSpec::new(11134, ApiDomain::Auth, "/api/login/getLoginInfo");
    pub const LOG_OUT: CommandSpec = CommandSpec::new(11135, ApiDomain::Auth, "/api/login/logOut");
    pub const CHK_UPDATE: CommandSpec =
        CommandSpec::new(11160, ApiDomain::Auth, "/api/login/chkUpdate");
    pub const LOGIN_BY_PASSW_V2: CommandSpec =
        CommandSpec::new(11176, ApiDomain::Auth, "/api/login/loginByPasswv2");
    pub const GET_SERVER_INFO: CommandSpec =
        CommandSpec::new(11323, ApiDomain::Auth, "/api/login/getServerInfo");
    pub const GET_CAPTCHA: CommandSpec =
        CommandSpec::new(11325, ApiDomain::Auth, "/api/login/getCaptcha");
    pub const ACT_LIST_V2: CommandSpec =
        CommandSpec::new(11346, ApiDomain::Auth, "/api/login/actlistv2");
    pub const SETTING_UPDATE: CommandSpec =
        CommandSpec::new(12033, ApiDomain::Auth, "/api/setting/update");
    pub const SETTING_ME: CommandSpec = CommandSpec::new(12113, ApiDomain::Auth, "/api/setting/me");
    pub const UPDATE_2FA: CommandSpec =
        CommandSpec::new(12066, ApiDomain::Auth, "/api/update/setting/2FA");
}

/// Mobile-to-PC Message Sync (V1 & V2) and PC-to-Mobile Backup commands (`re-notes/12` §3).
pub mod sync {
    use super::{ApiDomain, CommandSpec};

    pub const PULL_MOBILE_MSG: CommandSpec =
        CommandSpec::new(12000, ApiDomain::File, "/api/message/pull_mobile_msg");
    pub const DELETE_SNAPSHOT_MOBILE_MSG: CommandSpec = CommandSpec::new(
        12003,
        ApiDomain::File,
        "/api/message/delete_snapshot_mobile_msg",
    );
    pub const GET_BACKUP_MSG_INFO: CommandSpec =
        CommandSpec::new(12096, ApiDomain::File, "/api/message/get_backupmsginfo");
    pub const GET_CROSS_DB: CommandSpec =
        CommandSpec::new(12412, ApiDomain::File, "/api/message/get_crossdb");
    pub const CANCEL_PULL_MOBILE_MSG: CommandSpec = CommandSpec::new(
        12700,
        ApiDomain::File,
        "/api/message/cancel_pull_mobile_msg",
    );
    pub const REQUEST_SYNC_V2: CommandSpec =
        CommandSpec::new(12888, ApiDomain::File, "/api/transfer-sync-v2/request-sync");
    pub const BACKUP_PC2MB_UPLOAD: CommandSpec =
        CommandSpec::new(13538, ApiDomain::BackupPcMobile, "/api/backuppc2mb/upload");
    pub const ZNOISE: CommandSpec = CommandSpec::new(12390, ApiDomain::Zwid, "/api/znoise");
    pub const ZNOISE_PROFILE_FALLBACK: CommandSpec =
        CommandSpec::new(12390, ApiDomain::Profile, "/api/znoise");
}

/// Standard 1-1 messaging commands (`chat` & `file` domains).
pub mod message {
    use super::{ApiDomain, CommandSpec};

    pub const TYPING: CommandSpec = CommandSpec::new(11144, ApiDomain::Chat, "/api/message/typing");
    pub const SHARE_FILE: CommandSpec =
        CommandSpec::new(11145, ApiDomain::File, "/api/message/sharefile");
    pub const SHARE_FILE_FULL: CommandSpec =
        CommandSpec::new(11146, ApiDomain::File, "/api/message/sharefile_full");
    pub const VOICE_MP3: CommandSpec =
        CommandSpec::new(11148, ApiDomain::File, "/api/message/voicemp3");
    pub const SEEN: CommandSpec = CommandSpec::new(11150, ApiDomain::Chat, "/api/message/seen");
    pub const SMS: CommandSpec = CommandSpec::new(11151, ApiDomain::Chat, "/api/message/sms");
    pub const PHOTO: CommandSpec = CommandSpec::new(11152, ApiDomain::File, "/api/message/photo");
    pub const DELIVERED: CommandSpec =
        CommandSpec::new(11153, ApiDomain::Chat, "/api/message/delivered");
    pub const STICKER: CommandSpec =
        CommandSpec::new(11154, ApiDomain::Chat, "/api/message/sticker");
    pub const FORWARD: CommandSpec =
        CommandSpec::new(11199, ApiDomain::File, "/api/message/forward");
    pub const UNDO: CommandSpec = CommandSpec::new(11281, ApiDomain::Chat, "/api/message/undo");
    pub const PARSE_LINK: CommandSpec =
        CommandSpec::new(11285, ApiDomain::File, "/api/message/parselink");
    pub const LINK: CommandSpec = CommandSpec::new(11286, ApiDomain::Chat, "/api/message/link");
    pub const GIF: CommandSpec = CommandSpec::new(11292, ApiDomain::File, "/api/message/gif");
    pub const QUOTE: CommandSpec = CommandSpec::new(11771, ApiDomain::Chat, "/api/message/quote");
    pub const CONTACT: CommandSpec =
        CommandSpec::new(11774, ApiDomain::Chat, "/api/message/contact");
    pub const ASYNC_FILE_UPLOAD: CommandSpec =
        CommandSpec::new(11776, ApiDomain::File, "/api/message/asyncfile/upload");
    pub const ASYNC_FILE_MSG: CommandSpec =
        CommandSpec::new(11777, ApiDomain::File, "/api/message/asyncfile/msg");
    pub const DELETE_CONVER: CommandSpec =
        CommandSpec::new(11778, ApiDomain::Chat, "/api/message/deleteconver");
    pub const DELETE: CommandSpec = CommandSpec::new(11779, ApiDomain::Chat, "/api/message/delete");
    pub const MULTI_UNDO: CommandSpec =
        CommandSpec::new(12115, ApiDomain::Chat, "/api/message/multi_undo");
    pub const DELIVERED_V2: CommandSpec =
        CommandSpec::new(12117, ApiDomain::Chat, "/api/message/deliveredv2");
    pub const SEEN_V2: CommandSpec =
        CommandSpec::new(12118, ApiDomain::Chat, "/api/message/seenv2");
    pub const UPLOAD_PHOTO: CommandSpec =
        CommandSpec::new(12651, ApiDomain::Photo, "/api/message/upload/photo");
}

/// Group messaging, membership, and community commands (`group` & `file` domains).
pub mod group {
    use super::{ApiDomain, CommandSpec};

    pub const GET_LIST_V3: CommandSpec =
        CommandSpec::new(11161, ApiDomain::Group, "/api/group/getlg/v3");
    pub const GET_LIST_V4: CommandSpec =
        CommandSpec::new(12027, ApiDomain::Group, "/api/group/getlg/v4");
    pub const GET_INFO_V2: CommandSpec =
        CommandSpec::new(11162, ApiDomain::Group, "/api/group/getgi/v2");
    pub const SEND_MSG: CommandSpec =
        CommandSpec::new(11163, ApiDomain::Group, "/api/group/sendmsg");
    pub const SEND_PHOTO: CommandSpec =
        CommandSpec::new(11164, ApiDomain::File, "/api/group/sendpt");
    pub const STICKER: CommandSpec =
        CommandSpec::new(11165, ApiDomain::Group, "/api/group/sticker");
    pub const CREATE: CommandSpec = CommandSpec::new(11185, ApiDomain::Group, "/api/group/create");
    pub const CREATE_V2: CommandSpec =
        CommandSpec::new(12443, ApiDomain::Group, "/api/group/create/v2");
    pub const INVITE: CommandSpec = CommandSpec::new(11186, ApiDomain::Group, "/api/group/invite");
    pub const KICKOUT: CommandSpec =
        CommandSpec::new(11187, ApiDomain::Group, "/api/group/kickout");
    pub const UPDATE_INFO: CommandSpec =
        CommandSpec::new(11188, ApiDomain::Group, "/api/group/updateinfo");
    pub const LEAVE: CommandSpec = CommandSpec::new(11189, ApiDomain::Group, "/api/group/leave");
    pub const SHARE_FILE: CommandSpec =
        CommandSpec::new(11190, ApiDomain::File, "/api/group/sharefile");
    pub const UNDO_MSG: CommandSpec =
        CommandSpec::new(11192, ApiDomain::Group, "/api/group/undomsg");
    pub const SEND_LINK: CommandSpec =
        CommandSpec::new(11193, ApiDomain::Group, "/api/group/sendlink");
    pub const TYPING: CommandSpec = CommandSpec::new(11196, ApiDomain::Group, "/api/group/typing");
    pub const QUOTE: CommandSpec = CommandSpec::new(11197, ApiDomain::Group, "/api/group/quote");
    pub const MENTION: CommandSpec =
        CommandSpec::new(11198, ApiDomain::Group, "/api/group/mention");
    pub const HISTORY: CommandSpec =
        CommandSpec::new(11240, ApiDomain::Group, "/api/group/history");
    pub const DELIVERED_V2: CommandSpec =
        CommandSpec::new(12119, ApiDomain::Group, "/api/group/deliveredv2");
    pub const SEEN_V2: CommandSpec = CommandSpec::new(12124, ApiDomain::Group, "/api/group/seenv2");
    pub const UPLOAD_PHOTO: CommandSpec =
        CommandSpec::new(12652, ApiDomain::Photo, "/api/group/upload/photo");
}

/// Group & My Cloud message history commands (`group_cloud_message` domain).
pub mod cloud {
    use super::{ApiDomain, CommandSpec};

    pub const MGET: CommandSpec = CommandSpec::new(12503, ApiDomain::GroupCloud, "/api/cm/mget");
    pub const GET_PATH: CommandSpec =
        CommandSpec::new(12504, ApiDomain::GroupCloud, "/api/cm/getpath");
    pub const GET_RECENT_V2: CommandSpec =
        CommandSpec::new(12506, ApiDomain::GroupCloud, "/api/cm/getrecentv2");
    pub const GET_OLD_V2: CommandSpec =
        CommandSpec::new(12507, ApiDomain::GroupCloud, "/api/cm/getoldv2");
    pub const RGET_V2: CommandSpec =
        CommandSpec::new(12508, ApiDomain::GroupCloud, "/api/cm/rgetv2");
    pub const PC_DL: CommandSpec = CommandSpec::new(12509, ApiDomain::GroupCloud, "/api/cm/pc/dl");
}

/// Conversation metadata, preload, pin, hidden, unread mark, and label commands.
pub mod conv {
    use super::{ApiDomain, CommandSpec};

    pub const HIDDEN_GET_ALL: CommandSpec =
        CommandSpec::new(10048, ApiDomain::Conversation, "/api/hiddenconvers/get-all");
    pub const HIDDEN_ADD_REMOVE: CommandSpec = CommandSpec::new(
        10049,
        ApiDomain::Conversation,
        "/api/hiddenconvers/add-remove",
    );
    pub const PRELOAD_GET_LAST_MSGS: CommandSpec = CommandSpec::new(
        10053,
        ApiDomain::Conversation,
        "/api/preloadconvers/get-last-msgs",
    );
    pub const GET_UNREAD_MARK: CommandSpec =
        CommandSpec::new(12011, ApiDomain::Conversation, "/api/conv/getUnreadMark");
    pub const ADD_UNREAD_MARK: CommandSpec =
        CommandSpec::new(12013, ApiDomain::Conversation, "/api/conv/addUnreadMark");
    pub const REMOVE_UNREAD_MARK: CommandSpec =
        CommandSpec::new(12014, ApiDomain::Conversation, "/api/conv/removeUnreadMark");
    pub const PIN_LIST: CommandSpec =
        CommandSpec::new(12120, ApiDomain::Conversation, "/api/pinconvers/list");
    pub const PIN_UPDATE_V2: CommandSpec =
        CommandSpec::new(12122, ApiDomain::Conversation, "/api/pinconvers/updatev2");
    pub const LABEL_UPDATE: CommandSpec =
        CommandSpec::new(12160, ApiDomain::Label, "/api/convlabel/update");
    pub const LABEL_GET: CommandSpec =
        CommandSpec::new(12161, ApiDomain::Label, "/api/convlabel/get");
}

/// Outbound End-to-End Encryption (E2EE) HTTP commands (`16000..16204`, `re-notes/12` §4.4).
pub mod e2ee {
    use super::{ApiDomain, CommandSpec};

    pub const MSG_SMS: CommandSpec =
        CommandSpec::new(16000, ApiDomain::ChatE2ee, "/api/e2ee/pc/message/sms");
    pub const MSG_STICKER: CommandSpec =
        CommandSpec::new(16001, ApiDomain::ChatE2ee, "/api/e2ee/pc/message/sticker");
    pub const MSG_LINK: CommandSpec =
        CommandSpec::new(16002, ApiDomain::ChatE2ee, "/api/e2ee/pc/message/link");
    pub const MSG_QUOTE: CommandSpec =
        CommandSpec::new(16003, ApiDomain::ChatE2ee, "/api/e2ee/pc/message/quote");
    pub const MSG_PHOTO: CommandSpec =
        CommandSpec::new(16004, ApiDomain::ChatE2ee, "/api/e2ee/pc/message/photo");
    pub const MSG_FILE: CommandSpec =
        CommandSpec::new(16005, ApiDomain::ChatE2ee, "/api/e2ee/pc/message/file");
    pub const MSG_CONTACT: CommandSpec =
        CommandSpec::new(16006, ApiDomain::ChatE2ee, "/api/e2ee/pc/message/contact");
    pub const MSG_UNDO: CommandSpec =
        CommandSpec::new(16007, ApiDomain::ChatE2ee, "/api/e2ee/pc/message/undo");
    pub const MSG_MULTI_UNDO: CommandSpec = CommandSpec::new(
        16008,
        ApiDomain::ChatE2ee,
        "/api/e2ee/pc/message/multi_undo",
    );
    pub const MSG_DELETE: CommandSpec =
        CommandSpec::new(16009, ApiDomain::ChatE2ee, "/api/e2ee/pc/message/delete");
    pub const MSG_DELETE_CONVER: CommandSpec = CommandSpec::new(
        16010,
        ApiDomain::ChatE2ee,
        "/api/e2ee/pc/message/deleteconver",
    );
    pub const MSG_GIF: CommandSpec =
        CommandSpec::new(16011, ApiDomain::ChatE2ee, "/api/e2ee/pc/message/gif");
    pub const MSG_FORWARD: CommandSpec =
        CommandSpec::new(16012, ApiDomain::ChatE2ee, "/api/e2ee/pc/message/forward");
    pub const MSG_DELIVERED: CommandSpec = CommandSpec::new(
        16200,
        ApiDomain::ChatE2ee,
        "/api/e2ee/pc/t/message/delivered",
    );
    pub const MSG_SEEN: CommandSpec =
        CommandSpec::new(16201, ApiDomain::ChatE2ee, "/api/e2ee/pc/t/message/seen");

    pub const GROUP_SMS: CommandSpec =
        CommandSpec::new(16100, ApiDomain::ChatE2eeGroup, "/api/e2ee/pc/group/sms");
    pub const GROUP_STICKER: CommandSpec = CommandSpec::new(
        16101,
        ApiDomain::ChatE2eeGroup,
        "/api/e2ee/pc/group/sticker",
    );
    pub const GROUP_LINK: CommandSpec =
        CommandSpec::new(16102, ApiDomain::ChatE2eeGroup, "/api/e2ee/pc/group/link");
    pub const GROUP_QUOTE: CommandSpec =
        CommandSpec::new(16103, ApiDomain::ChatE2eeGroup, "/api/e2ee/pc/group/quote");
    pub const GROUP_PHOTO: CommandSpec =
        CommandSpec::new(16104, ApiDomain::ChatE2eeGroup, "/api/e2ee/pc/group/photo");
    pub const GROUP_FILE: CommandSpec =
        CommandSpec::new(16105, ApiDomain::ChatE2eeGroup, "/api/e2ee/pc/group/file");
    pub const GROUP_CONTACT: CommandSpec = CommandSpec::new(
        16106,
        ApiDomain::ChatE2eeGroup,
        "/api/e2ee/pc/group/contact",
    );
    pub const GROUP_UNDO_MSG: CommandSpec = CommandSpec::new(
        16107,
        ApiDomain::ChatE2eeGroup,
        "/api/e2ee/pc/group/undoMsg",
    );
    pub const GROUP_MULTI_UNDO_MSG: CommandSpec = CommandSpec::new(
        16108,
        ApiDomain::ChatE2eeGroup,
        "/api/e2ee/pc/group/multiUndoMsg",
    );
    pub const GROUP_DELETE_MSG: CommandSpec = CommandSpec::new(
        16109,
        ApiDomain::ChatE2eeGroup,
        "/api/e2ee/pc/group/deleteMsg",
    );
    pub const GROUP_DELETE_CONV: CommandSpec = CommandSpec::new(
        16110,
        ApiDomain::ChatE2eeGroup,
        "/api/e2ee/pc/group/deleteConv",
    );
    pub const GROUP_MENTION: CommandSpec = CommandSpec::new(
        16111,
        ApiDomain::ChatE2eeGroup,
        "/api/e2ee/pc/group/mention",
    );
    pub const GROUP_FORWARD: CommandSpec = CommandSpec::new(
        16112,
        ApiDomain::ChatE2eeGroup,
        "/api/e2ee/pc/group/forward",
    );
    pub const GROUP_GIF: CommandSpec =
        CommandSpec::new(16113, ApiDomain::ChatE2eeGroup, "/api/e2ee/pc/group/gif");
    pub const GROUP_DELIVERED: CommandSpec = CommandSpec::new(
        16203,
        ApiDomain::ChatE2eeGroup,
        "/api/e2ee/pc/t/group/delivered",
    );
    pub const GROUP_SEEN: CommandSpec =
        CommandSpec::new(16204, ApiDomain::ChatE2eeGroup, "/api/e2ee/pc/t/group/seen");
}

/// Social profile, contacts, mute, alias, reaction, and sticker commands.
pub mod social {
    use super::{ApiDomain, CommandSpec};

    pub const GET_FRIENDS: CommandSpec =
        CommandSpec::new(11137, ApiDomain::Profile, "/api/social/friend/getfriends");
    pub const PROFILE_ME_V2: CommandSpec =
        CommandSpec::new(11139, ApiDomain::Profile, "/api/social/profile/me-v2");
    pub const GET_PROFILES_V2: CommandSpec = CommandSpec::new(
        11140,
        ApiDomain::Profile,
        "/api/social/friend/getprofiles/v2",
    );
    pub const GET_MUTE: CommandSpec =
        CommandSpec::new(11280, ApiDomain::Profile, "/api/social/profile/getmute");
    pub const SET_MUTE: CommandSpec =
        CommandSpec::new(11288, ApiDomain::Profile, "/api/social/profile/setmute");
    pub const SEARCH_FRIEND_BY_USERNAME: CommandSpec =
        CommandSpec::new(10004, ApiDomain::Friend, "/api/friend/search/by-user-name");
    pub const FRIEND_SEND_REQ: CommandSpec =
        CommandSpec::new(11424, ApiDomain::Friend, "/api/friend/sendreq");
    pub const FRIEND_ACCEPT: CommandSpec =
        CommandSpec::new(11427, ApiDomain::Friend, "/api/friend/accept");
    pub const FRIEND_REJECT: CommandSpec =
        CommandSpec::new(11428, ApiDomain::Friend, "/api/friend/reject");
    pub const ALIAS_LIST_V2: CommandSpec =
        CommandSpec::new(12205, ApiDomain::Alias, "/api/alias/list/v2");
    pub const ALIAS_UPDATE: CommandSpec =
        CommandSpec::new(12201, ApiDomain::Alias, "/api/alias/update");
    pub const MSG_REACTION: CommandSpec =
        CommandSpec::new(12410, ApiDomain::Reaction, "/api/message/reaction");
    pub const GROUP_REACTION: CommandSpec =
        CommandSpec::new(12411, ApiDomain::Reaction, "/api/group/reaction");
}

// ---------------------------------------------------------------------------
// Complete WebSocket Frame `cmd` & Close-Code Catalog (`re-notes/12` & `13`)
// ---------------------------------------------------------------------------

/// Complete WebSocket frame `cmd` identifiers (`u16` little-endian header bytes `1..3`).
pub mod ws;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_registry_is_strictly_sorted_unique_and_lookup_consistent() {
        assert_eq!(COMMAND_REGISTRY.len(), 437);
        for pair in COMMAND_REGISTRY.windows(2) {
            assert!(
                pair[0].id < pair[1].id,
                "COMMAND_REGISTRY must be strictly sorted: {} >= {}",
                pair[0].id,
                pair[1].id
            );
        }
        for &spec in COMMAND_REGISTRY {
            assert!(
                spec.path.starts_with('/'),
                "command {} path must start with '/': {}",
                spec.id,
                spec.path
            );
            assert_eq!(lookup_command(spec.id), Some(spec));
            let key = spec.domain.service_key();
            assert_eq!(ApiDomain::from_service_key(key), Some(spec.domain));
            assert!(
                spec.domain.fallback_host().is_some(),
                "every ApiDomain must resolve a fallback host in endpoints.rs: {:?} ({key})",
                spec.domain
            );
        }
    }

    #[test]
    fn named_constants_match_master_registry_and_build_encrypted_paths() {
        assert_eq!(lookup_command(12000), Some(sync::PULL_MOBILE_MSG));
        assert_eq!(lookup_command(12390), Some(sync::ZNOISE_PROFILE_FALLBACK));
        assert_eq!(lookup_command(12506), Some(cloud::GET_RECENT_V2));
        assert_eq!(lookup_command(12507), Some(cloud::GET_OLD_V2));
        assert_eq!(lookup_command(16000), Some(e2ee::MSG_SMS));
        assert_eq!(lookup_command(16100), Some(e2ee::GROUP_SMS));
        assert_eq!(
            lookup_command_by_path("/api/preloadconvers/get-last-msgs"),
            Some(conv::PRELOAD_GET_LAST_MSGS)
        );
        assert_eq!(ws::cmd_name(540), Some("E2EE_SUBMIT_BUNDLE"));
        assert_eq!(ws::cmd_name(590), Some("SYNC_MESSAGE_REQUEST"));

        let mut s = Session::new(false, false, false).unwrap();
        s.secret_key = Some(vec![0x42; 32]);
        let payload = serde_json::json!({"pc_name": "arch-t14s"});
        let q = sync::PULL_MOBILE_MSG
            .query_path(&s, Some(&payload))
            .unwrap();
        assert!(q.starts_with("/api/message/pull_mobile_msg?zpw_ver="));
        assert!(q.contains("&params=")); // (INV-1)
    }

    #[test]
    fn session_candidates_resolve_for_every_registered_command_and_dynamic_service_map() {
        let mut s = Session::new(false, false, false).unwrap();
        for &spec in COMMAND_REGISTRY {
            let cands = s.domains.candidates(spec.domain.service_key());
            assert!(
                !cands.is_empty(),
                "command {} ({:?}) resolved zero host candidates",
                spec.id,
                spec.domain
            );
            assert!(cands[0].starts_with("https://"));
        }

        // Verify extended zpw_service_map_v3 keys (e.g. `chat_e2e`, `backuppcmobile`) populate first!
        s.apply_server_info(&serde_json::json!({
            "zpw_service_map_v3": {
                "chat_e2e": ["https://custom-e2ee.chat.zalo.me"],
                "backuppcmobile": ["https://custom-bk.zaloapp.com"]
            }
        }));
        let e2ee_cands = s.domains.candidates(e2ee::MSG_SMS.domain.service_key());
        assert_eq!(e2ee_cands[0], "https://custom-e2ee.chat.zalo.me");
        let bk_cands = s
            .domains
            .candidates(sync::BACKUP_PC2MB_UPLOAD.domain.service_key());
        assert_eq!(bk_cands[0], "https://custom-bk.zaloapp.com");
    }
}
