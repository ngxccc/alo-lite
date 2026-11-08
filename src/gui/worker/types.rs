use crate::protocol::messaging::{ChatMessage, Thread};

pub enum WorkerCmd {
    /// Read OS authorization without prompting.
    ReadNotificationAuthorization {
        request_id: u64,
    },
    /// Request OS authorization following an explicit Settings click.
    RequestNotificationAuthorization {
        request_id: u64,
    },
    RequestQr,
    LoginPassword {
        phone: String,
        pass: String,
    },
    RefreshThreads,
    RefreshThreadsManual,
    PollMessages {
        known_threads: Vec<(String, String, bool)>,
    },
    LoadGroupHistory {
        group_id: String,
        is_group: bool,
        /// Messages the GUI already holds. SQLite is read `have + 300` deep, so each
        /// "Tải thêm" reaches further back instead of re-reading the same newest 500.
        have: usize,
    },
    /// Page group / Cloud history back from the server into SQLite: (id, name, is_group).
    SyncHistory {
        threads: Vec<(String, String, bool)>,
    },
    SendMessage {
        to_id: String,
        text: String,
        is_group: bool,
    },
    SendQuote {
        to_id: String,
        text: String,
        quoted: ChatMessage,
        is_group: bool,
    },
    Forward {
        target_id: String,
        target_is_group: bool,
        msg: ChatMessage,
    },
    React {
        to_id: String,
        is_group: bool,
        msg: ChatMessage,
        reaction: String,
    },
    DeleteForMe {
        to_id: String,
        is_group: bool,
        msg: ChatMessage,
    },
    RecallMessage {
        to_id: String,
        is_group: bool,
        msg: ChatMessage,
    },
    SendSticker {
        to_id: String,
        sticker_id: u64,
        cate_id: u64,
        sticker_type: u32,
        is_group: bool,
    },
    PickAndSendAttachment {
        to_id: String,
        is_group: bool,
        images_only: bool,
        caption: String,
        preset_path: Option<std::path::PathBuf>,
    },
    FetchStickerImage {
        key: String,
        url: String,
    },
    FetchStickerStore,
    FetchStickerPack {
        cate_id: u64,
        name: String,
    },
    SearchStickers {
        keyword: String,
    },
    SuggestComposerStickers {
        keyword: String,
    },
    ParseLinkPreview {
        url: String,
    },
    LookupPhone {
        phone: String,
    },
    StartCall {
        partner_id: String,
        partner_name: String,
        partner_avatar: Option<String>,
        is_video: bool,
        is_group: bool,
    },
    AnswerCall {
        accept: bool,
        cam_start_off: bool,
    },
    SetCallMediaState {
        is_on_mic: bool,
        is_on_cam: bool,
    },
    EndCall,
    AddFriend {
        user_id: String,
        user_name: String,
    },
    CheckFriendStatus {
        user_id: String,
    },
    AcceptFriend {
        user_id: String,
        user_name: String,
    },
    RejectFriend {
        user_id: String,
        user_name: String,
    },
    UndoFriend {
        user_id: String,
        user_name: String,
    },
    PersistThreads(Vec<Thread>),
    CheckForUpdates {
        silent: bool,
    },
    DownloadAndApplyUpdate {
        download_url: String,
        asset_name: String,
        target_version: String,
        expected_sha256: Option<String>,
    },
    Logout,
    /// Delete local message data (`zalo_history.db` + `threads_cache.json`) without
    /// touching credentials. Separate from [`WorkerCmd::Logout`] by design (#21).
    WipeLocalData,
    PinThread {
        thread_id: String,
        is_group: bool,
        pin: bool,
    },
    MarkThreadUnread {
        thread_id: String,
        is_group: bool,
        unread: bool,
    },
    MuteThread {
        thread_id: String,
        is_group: bool,
        duration_sec: i64,
        mute: bool,
    },
    HideThread {
        thread_id: String,
        is_group: bool,
        hide: bool,
    },
    DeleteThread {
        thread_id: String,
        is_group: bool,
        last_cli_msg_id: String,
    },
    FetchSocialProfile {
        target_uid: String,
    },
    FetchMySocialAvatars {
        page: u32,
        count: u32,
    },
    FetchTimelineFeed {
        friend_uids: Vec<String>,
        shared_media_by_uid: std::collections::HashMap<String, Vec<crate::feed::FeedMedia>>,
    },
    SyncPhoneMessages {
        threads: Vec<Thread>,
        send_phone_prompt: bool,
    },
    SetProxy {
        proxy_url: String,
    },
    DownloadFile {
        url: String,
        file_name: String,
        download_dir: String,
    },
    SendTyping {
        thread_id: String,
        is_group: bool,
    },
    SendSeen {
        thread_id: String,
        is_group: bool,
        last_msg_id: String,
    },
    SwitchAccount {
        target_uin: String,
    },
}

pub(super) fn extract_msg_id(resp: &serde_json::Value) -> String {
    resp.get("data")
        .and_then(|d| d.get("msgId"))
        .and_then(|v| match v {
            serde_json::Value::String(s) => Some(s.clone()),
            serde_json::Value::Number(n) => Some(n.to_string()),
            _ => None,
        })
        .unwrap_or_default()
}

pub(super) fn resp_error_code(resp: &serde_json::Value) -> Option<i64> {
    resp.get("error_code")
        .and_then(serde_json::Value::as_i64)
        .filter(|c| *c != 0)
}
