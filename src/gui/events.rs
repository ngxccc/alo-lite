use std::collections::HashMap;
use std::sync::mpsc::Sender;

use crate::protocol::auth::QrLogin;
use crate::protocol::messaging::{ChatMessage, Thread};
use eframe::egui;

#[derive(Debug, Clone)]
pub struct UpdateCheckResult {
    pub current_version: String,
    pub latest_version: String,
    pub has_update: bool,
    pub release_title: String,
    pub release_notes: String,
    pub html_url: String,
    pub download_url: Option<String>,
    pub asset_name: Option<String>,
    pub asset_size: u64,
    pub expected_sha256: Option<String>,
    pub published_at: String,
}

pub fn is_newer_semver(latest: &str, current: &str) -> bool {
    let parse_semver = |v: &str| -> Option<(u64, u64, u64)> {
        let clean = v.trim_start_matches(['v', 'V']);
        let core = clean.split('-').next().unwrap_or(clean);
        let mut parts = core.split('.');
        let major = parts.next()?.parse().ok()?;
        let minor = parts.next()?.parse().ok()?;
        let patch = parts.next()?.parse().ok()?;
        Some((major, minor, patch))
    };

    match (parse_semver(latest), parse_semver(current)) {
        (Some(l), Some(c)) => l > c,
        _ => false,
    }
}

pub fn phone_sync_completion_message(
    added: usize,
    active_threads: usize,
    cancelled: bool,
    error: Option<&str>,
) -> String {
    if let Some(err) = error {
        return format!("Đồng bộ thất bại: {err}");
    }
    let prefix = if cancelled {
        "Đã dừng"
    } else {
        "Đã hoàn tất"
    };
    format!(
        "{prefix} yêu cầu: {added} tin mới từ Cloud/Nhóm ({active_threads} hội thoại). Lịch sử điện thoại chưa được tải vào ứng dụng."
    )
}

pub enum GuiEvent {
    /// Terminal response for a Settings authorization action.
    NotificationAuthorization {
        request_id: u64,
        result: Result<
            crate::native_notifications::AuthorizationStatus,
            crate::native_notifications::NotificationError,
        >,
    },
    QrReady {
        qr_login: QrLogin,
        qr_image: egui::ColorImage,
    },
    QrScanned {
        name: String,
        avatar: Option<String>,
    },
    QrExpired(String),
    LoginSuccess {
        name: String,
        uin: String,
        avatar: Option<String>,
    },
    LoginFailed(String),
    ThreadsUpdated(Vec<Thread>),
    MessageSent {
        client_message_id: String,
        thread_id: String,
        text: String,
        msg_id: String,
    },
    StickerSent {
        client_message_id: String,
        thread_id: String,
        msg_id: String,
        sticker_id: u64,
        cate_id: u64,
        sticker_type: u32,
    },
    AttachmentSent {
        thread_id: String,
        msg: ChatMessage,
    },
    StickerImageReady {
        key: String,
        image: egui::ColorImage,
    },
    StickerStoreLoaded(Vec<crate::protocol::messaging::StickerPackInfo>),
    StickerPackItemsLoaded {
        cate_id: u64,
        name: String,
        items: Vec<crate::protocol::messaging::StickerItem>,
    },
    StickerSearchResults {
        keyword: String,
        items: Vec<crate::protocol::messaging::StickerItem>,
    },
    StickerSuggestKeywordsLoaded(Vec<String>),
    ComposerStickerSuggestions {
        keyword: String,
        items: Vec<crate::protocol::messaging::StickerItem>,
    },
    LinkPreviewReady(crate::protocol::messaging::LinkMetadata),
    PhoneLookupSuccess(crate::protocol::social::PhoneLookupResult),
    PhoneLookupFailed(String),
    FriendStatusChecked {
        user_id: String,
        status: crate::protocol::social::FriendshipStatus,
        online: Option<(bool, String)>,
    },
    QuoteSent {
        client_message_id: String,
        thread_id: String,
        text: String,
        msg_id: String,
        quoted: ChatMessage,
    },
    Reacted {
        thread_id: String,
        msg_id: String,
        ts: i64,
        reaction: String,
    },
    MessageDeleted {
        thread_id: String,
        msg_id: String,
        ts: i64,
    },
    MessageRecalled {
        thread_id: String,
        msg_id: String,
        ts: i64,
    },
    SendFailed {
        thread_id: String,
        text: String,
        error: String,
    },
    SessionExpired(String),
    /// The server could not be reached: the session is intact, show it and retry (#21).
    SessionUnreachable(String),
    /// `zalo_history.db` + `threads_cache.json` were removed on request (#21).
    LocalDataWiped,
    CallStateChanged(String),
    IncomingCall {
        call_id: i64,
        caller_id: String,
        caller_name: String,
        is_video: bool,
        is_group: bool,
    },
    CallRejected {
        code: i64,
        reason: String,
    },
    StatusUpdate(String),
    RefreshFinished {
        threads: usize,
        messages: usize,
        errors: Vec<String>,
        manual: bool,
    },
    ScheduledListUpdated,
    SyncFinished {
        added: usize,
        errors: Vec<String>,
        cancelled: bool,
    },
    UpdateCheckFinished {
        result: Result<UpdateCheckResult, String>,
        silent: bool,
    },
    UpdateDownloadStatus {
        downloading: bool,
        message: String,
    },
    UpdateDownloadProgress {
        downloaded: u64,
        total: u64,
    },
    UpdateAppliedAndRestarting(String),
    SocialProfileLoaded(Result<crate::feed::SocialUserProfile, String>),
    SocialAvatarsLoaded(Result<Vec<crate::feed::FeedMedia>, String>),
    SocialTimelineLoaded(
        Result<
            (
                Vec<crate::feed::SocialUserProfile>,
                Vec<crate::feed::FeedItem>,
            ),
            String,
        >,
    ),
    MuteSnapshotSynced(HashMap<String, i64>),
    MuteEventsReceived(Vec<crate::protocol::messaging::unread::MuteControlEvent>),
    PhoneSyncProgress {
        progress: f32,
        status: String,
    },
    PhoneSyncFinished {
        threads: Vec<Thread>,
        added: usize,
        active_threads: usize,
        cancelled: bool,
        error: Option<String>,
    },
    ProxyUpdated(Result<Option<String>, String>),
    FileDownloaded(Result<(String, String), String>),
    LoggedOut,
    HistoryReachedStart(String),
    AccountSwitched {
        uin: String,
        name: String,
        avatar: Option<String>,
        threads: Vec<Thread>,
    },
}

/// A cloneable wrapper around the GUI channel that also requests a repaint.
///
/// Workers used to pair every `gui_tx.send(…)` with an explicit `egui_ctx.request_repaint()`.
/// The GUI repaints only on input, window events, or an explicit wake-up from the message poll's
/// 1.2 s timer, so an event sent without a repaint would sit unseen until the next mouse move.
#[derive(Clone)]
pub struct GuiTx {
    pub(crate) tx: Sender<GuiEvent>,
    pub(crate) ctx: egui::Context,
}

impl GuiTx {
    pub fn new(tx: Sender<GuiEvent>, ctx: egui::Context) -> Self {
        Self { tx, ctx }
    }

    /// Same contract as `Sender::send` (callers already `let _ =` it), plus the wake-up.
    #[allow(clippy::result_large_err)]
    pub fn send(&self, ev: GuiEvent) -> Result<(), std::sync::mpsc::SendError<GuiEvent>> {
        let res = self.tx.send(ev);
        self.ctx.request_repaint();
        res
    }
}
