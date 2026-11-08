//! UI-agnostic events emitted by [`super::CoreEngine`] to any attached frontend.

use std::collections::HashMap;

use crate::protocol::auth::QrLogin;
use crate::protocol::messaging::unread::MuteControlEvent;
use crate::protocol::messaging::{ChatMessage, LinkMetadata, StickerItem, StickerPackInfo, Thread};
use crate::protocol::social::{FriendshipStatus, PhoneLookupResult};
use crate::services::native_notifications::{AuthorizationStatus, NotificationError};
use crate::services::updater::UpdateCheckResult;

/// Decoded RGBA8 image pixels independent of any UI toolkit (`egui::ColorImage`, WebView, etc.).
#[derive(Debug, Clone)]
pub struct CoreImageData {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

/// Sink notified whenever [`super::CoreEngine`] produces a domain event for an active UI frontend.
pub trait CoreEventSink: Send + Sync + 'static {
    fn emit(&self, event: CoreEvent);
}

/// Default no-op sink used in headless `--mcp` / `--api-only` mode.
pub struct NoopEventSink;

impl CoreEventSink for NoopEventSink {
    fn emit(&self, _event: CoreEvent) {}
}

/// Domain events emitted by [`super::CoreEngine`].
#[derive(Debug, Clone)]
pub enum CoreEvent {
    NotificationAuthorization {
        request_id: u64,
        result: Result<AuthorizationStatus, NotificationError>,
    },
    QrReady {
        qr_login: QrLogin,
        qr_image: CoreImageData,
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
        image: CoreImageData,
    },
    StickerStoreLoaded(Vec<StickerPackInfo>),
    StickerPackItemsLoaded {
        cate_id: u64,
        name: String,
        items: Vec<StickerItem>,
    },
    StickerSearchResults {
        keyword: String,
        items: Vec<StickerItem>,
    },
    StickerSuggestKeywordsLoaded(Vec<String>),
    ComposerStickerSuggestions {
        keyword: String,
        items: Vec<StickerItem>,
    },
    LinkPreviewReady(LinkMetadata),
    PhoneLookupSuccess(PhoneLookupResult),
    PhoneLookupFailed(String),
    FriendStatusChecked {
        user_id: String,
        status: FriendshipStatus,
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
    SessionUnreachable(String),
    LocalDataWiped,
    StatusUpdate(String),
    RefreshFinished {
        threads: usize,
        messages: usize,
        errors: Vec<String>,
        manual: bool,
    },
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
    MuteSnapshotSynced(HashMap<String, i64>),
    MuteEventsReceived(Vec<MuteControlEvent>),
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
