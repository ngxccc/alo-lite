//! Serializable DTO snapshots and WebView/IPC event projections for [`super::CoreEngine`].
//!
//! Translates raw RGBA pixel buffers (`CoreImageData`) into compact PNG data URLs and maps every
//! [`CoreEvent`] variant to a flat JSON-friendly enum so non-egui hosts (Tauri, headless web UI)
//! never duplicate domain logic or process multi-megabyte integer arrays over IPC.

use std::collections::HashMap;

use base64::Engine as _;
use serde::Serialize;

use super::events::CoreEvent;
use super::settings::AppSettings;
use crate::common::logger::{LogEntry, ScanFinding};
use crate::protocol::messaging::unread::MuteControlEvent;
use crate::protocol::messaging::{ChatMessage, LinkMetadata, StickerItem, StickerPackInfo, Thread};
use crate::protocol::social::{FriendshipStatus, PhoneLookupResult};
use crate::services::account_manager::AccountProfile;
use crate::services::updater::UpdateCheckResult;

#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct PinnedPackMeta {
    pub cate_id: u64,
    pub name: String,
    pub items: Vec<StickerItem>,
}

#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct ImportSummaryDto {
    pub threads_imported: usize,
    pub messages_imported: usize,
    pub source_path: String,
}

#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct DevCryptoInfo {
    pub zpw_ver: &'static str,
    pub zpw_type: &'static str,
    pub cipher_algo: &'static str,
    pub sign_key_formula: &'static str,
    pub param_cipher_key_len: usize,
}

#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct DevDiagnosticsSnapshot {
    pub dev_mode: bool,
    pub logs: Vec<LogEntry>,
    pub findings: Vec<ScanFinding>,
    pub crypto: DevCryptoInfo,
}

/// Complete initial state loaded once when an external UI (Tauri WebView) mounts.
#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct CoreBootstrapSnapshot {
    pub app_version: &'static str,
    pub dev_mode: bool,
    pub logged_in: bool,
    pub uin: String,
    pub display_name: String,
    pub avatar: Option<String>,
    pub threads: Vec<Thread>,
    pub settings: AppSettings,
    pub saved_accounts: Vec<AccountProfile>,
    pub popular_stickers: Vec<StickerItem>,
    pub onion_stickers: Vec<StickerItem>,
    pub pinned_packs: Vec<PinnedPackMeta>,
    pub app_logo_data_url: String,
    pub active_db_path: String,
    pub sync_running: bool,
    pub phone_sync_running: bool,
    pub inline_video_supported: bool,
    pub inline_video_reason: Option<String>,
}

fn encode_png_asset(bytes: &[u8]) -> String {
    let b64 = base64::engine::general_purpose::STANDARD.encode(bytes);
    format!("data:image/png;base64,{b64}")
}

pub fn embedded_app_logo_data_url() -> String {
    encode_png_asset(include_bytes!("../../../../assets/app_icon.png"))
}

/// Probe whether the host WebView has the runtime codecs/sinks needed for inline `<video>`
/// without blocking or crashing `WebKitWebProcess` on missing GStreamer elements.
pub fn detect_inline_video_support() -> (bool, Option<String>) {
    #[cfg(target_os = "linux")]
    {
        let has_element = |name: &str| -> Option<bool> {
            std::process::Command::new("gst-inspect-1.0")
                .args(["--exists", name])
                .status()
                .ok()
                .map(|st| st.success())
        };
        let has_so = |file: &str| -> bool {
            [
                "/usr/lib/gstreamer-1.0",
                "/usr/lib/x86_64-linux-gnu/gstreamer-1.0",
                "/usr/lib/aarch64-linux-gnu/gstreamer-1.0",
                "/usr/lib64/gstreamer-1.0",
            ]
            .iter()
            .any(|dir| std::path::Path::new(dir).join(file).exists())
        };
        let has_auto =
            has_element("autoaudiosink").unwrap_or_else(|| has_so("libgstautodetect.so"));
        let has_mp4 = has_element("qtdemux").unwrap_or_else(|| has_so("libgstisomp4.so"));
        if !has_auto || !has_mp4 {
            return (
                false,
                Some(
                    "Thiếu GStreamer (gst-plugins-base, gst-plugins-good, gst-libav) — hãy cài để phát video trực tiếp hoặc mở bằng trình phát ngoài."
                        .to_string(),
                ),
            );
        }
    }
    (true, None)
}
/// JSON-serializable projection of [`CoreEvent`] for Tauri WebView IPC.
#[derive(Debug, Clone, Serialize, specta::Type)]
#[serde(tag = "type", content = "payload")]
pub enum CoreEventDto {
    NotificationAuthorization {
        request_id: u64,
        ok: bool,
        status_label: Option<String>,
        error: Option<String>,
    },
    QrReady {
        qr_token: String,
        qr_url: String,
        qr_data_url: String,
    },
    QrScanned {
        name: String,
        avatar: Option<String>,
    },
    QrExpired {
        message: String,
    },
    LoginSuccess {
        name: String,
        uin: String,
        avatar: Option<String>,
    },
    LoginFailed {
        message: String,
    },
    ThreadsUpdated {
        threads: Vec<Thread>,
    },
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
        data_url: String,
    },
    StickerStoreLoaded {
        packs: Vec<StickerPackInfo>,
    },
    StickerPackItemsLoaded {
        cate_id: u64,
        name: String,
        items: Vec<StickerItem>,
    },
    StickerSearchResults {
        keyword: String,
        items: Vec<StickerItem>,
    },
    StickerSuggestKeywordsLoaded {
        keywords: Vec<String>,
    },
    ComposerStickerSuggestions {
        keyword: String,
        items: Vec<StickerItem>,
    },
    LinkPreviewReady {
        meta: LinkMetadata,
    },
    PhoneLookupSuccess {
        profile: PhoneLookupResult,
    },
    PhoneLookupFailed {
        error: String,
    },
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
    SessionExpired {
        message: String,
    },
    SessionUnreachable {
        message: String,
    },
    LocalDataWiped,
    StatusUpdate {
        message: String,
    },
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
        silent: bool,
        ok: bool,
        info: Option<UpdateCheckResult>,
        error: Option<String>,
    },
    UpdateDownloadStatus {
        downloading: bool,
        message: String,
    },
    UpdateDownloadProgress {
        downloaded: u64,
        total: u64,
    },
    UpdateAppliedAndRestarting {
        version: String,
    },
    MuteSnapshotSynced {
        muted_threads: HashMap<String, i64>,
    },
    MuteEventsReceived {
        events: Vec<MuteControlEvent>,
    },
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
    FileDownloaded {
        ok: bool,
        file_name: Option<String>,
        path: Option<String>,
        error: Option<String>,
    },
    LoggedOut,
    HistoryReachedStart {
        thread_id: String,
    },
    AccountSwitched {
        uin: String,
        name: String,
        avatar: Option<String>,
        threads: Vec<Thread>,
    },
}

impl From<CoreEvent> for CoreEventDto {
    fn from(ev: CoreEvent) -> Self {
        match ev {
            CoreEvent::NotificationAuthorization { request_id, result } => match result {
                Ok(status) => Self::NotificationAuthorization {
                    request_id,
                    ok: true,
                    status_label: Some(status.label().to_string()),
                    error: None,
                },
                Err(e) => Self::NotificationAuthorization {
                    request_id,
                    ok: false,
                    status_label: None,
                    error: Some(e.to_string()),
                },
            },
            CoreEvent::QrReady { qr_login, qr_image } => Self::QrReady {
                qr_token: qr_login.token_id.clone(),
                qr_url: qr_login.payload().unwrap_or_default().to_string(),
                qr_data_url: qr_image.to_png_data_url().unwrap_or_default(),
            },
            CoreEvent::QrScanned { name, avatar } => Self::QrScanned { name, avatar },
            CoreEvent::QrExpired(message) => Self::QrExpired { message },
            CoreEvent::LoginSuccess { name, uin, avatar } => {
                Self::LoginSuccess { name, uin, avatar }
            }
            CoreEvent::LoginFailed(message) => Self::LoginFailed { message },
            CoreEvent::ThreadsUpdated(threads) => Self::ThreadsUpdated { threads },
            CoreEvent::MessageSent {
                client_message_id,
                thread_id,
                text,
                msg_id,
            } => Self::MessageSent {
                client_message_id,
                thread_id,
                text,
                msg_id,
            },
            CoreEvent::StickerSent {
                client_message_id,
                thread_id,
                msg_id,
                sticker_id,
                cate_id,
                sticker_type,
            } => Self::StickerSent {
                client_message_id,
                thread_id,
                msg_id,
                sticker_id,
                cate_id,
                sticker_type,
            },
            CoreEvent::AttachmentSent { thread_id, msg } => Self::AttachmentSent { thread_id, msg },
            CoreEvent::StickerImageReady { key, image } => Self::StickerImageReady {
                key,
                data_url: image.to_png_data_url().unwrap_or_default(),
            },
            CoreEvent::StickerStoreLoaded(packs) => Self::StickerStoreLoaded { packs },
            CoreEvent::StickerPackItemsLoaded {
                cate_id,
                name,
                items,
            } => Self::StickerPackItemsLoaded {
                cate_id,
                name,
                items,
            },
            CoreEvent::StickerSearchResults { keyword, items } => {
                Self::StickerSearchResults { keyword, items }
            }
            CoreEvent::StickerSuggestKeywordsLoaded(keywords) => {
                Self::StickerSuggestKeywordsLoaded { keywords }
            }
            CoreEvent::ComposerStickerSuggestions { keyword, items } => {
                Self::ComposerStickerSuggestions { keyword, items }
            }
            CoreEvent::LinkPreviewReady(meta) => Self::LinkPreviewReady { meta },
            CoreEvent::PhoneLookupSuccess(profile) => Self::PhoneLookupSuccess { profile },
            CoreEvent::PhoneLookupFailed(error) => Self::PhoneLookupFailed { error },
            CoreEvent::FriendStatusChecked {
                user_id,
                status,
                online,
            } => Self::FriendStatusChecked {
                user_id,
                status,
                online,
            },
            CoreEvent::QuoteSent {
                client_message_id,
                thread_id,
                text,
                msg_id,
                quoted,
            } => Self::QuoteSent {
                client_message_id,
                thread_id,
                text,
                msg_id,
                quoted,
            },
            CoreEvent::Reacted {
                thread_id,
                msg_id,
                ts,
                reaction,
            } => Self::Reacted {
                thread_id,
                msg_id,
                ts,
                reaction,
            },
            CoreEvent::MessageDeleted {
                thread_id,
                msg_id,
                ts,
            } => Self::MessageDeleted {
                thread_id,
                msg_id,
                ts,
            },
            CoreEvent::MessageRecalled {
                thread_id,
                msg_id,
                ts,
            } => Self::MessageRecalled {
                thread_id,
                msg_id,
                ts,
            },
            CoreEvent::SendFailed {
                thread_id,
                text,
                error,
            } => Self::SendFailed {
                thread_id,
                text,
                error,
            },
            CoreEvent::SessionExpired(message) => Self::SessionExpired { message },
            CoreEvent::SessionUnreachable(message) => Self::SessionUnreachable { message },
            CoreEvent::LocalDataWiped => Self::LocalDataWiped,
            CoreEvent::StatusUpdate(message) => Self::StatusUpdate { message },
            CoreEvent::RefreshFinished {
                threads,
                messages,
                errors,
                manual,
            } => Self::RefreshFinished {
                threads,
                messages,
                errors,
                manual,
            },
            CoreEvent::SyncFinished {
                added,
                errors,
                cancelled,
            } => Self::SyncFinished {
                added,
                errors,
                cancelled,
            },
            CoreEvent::UpdateCheckFinished { result, silent } => match result {
                Ok(info) => Self::UpdateCheckFinished {
                    silent,
                    ok: true,
                    info: Some(info),
                    error: None,
                },
                Err(error) => Self::UpdateCheckFinished {
                    silent,
                    ok: false,
                    info: None,
                    error: Some(error),
                },
            },
            CoreEvent::UpdateDownloadStatus {
                downloading,
                message,
            } => Self::UpdateDownloadStatus {
                downloading,
                message,
            },
            CoreEvent::UpdateDownloadProgress { downloaded, total } => {
                Self::UpdateDownloadProgress { downloaded, total }
            }
            CoreEvent::UpdateAppliedAndRestarting(version) => {
                Self::UpdateAppliedAndRestarting { version }
            }
            CoreEvent::MuteSnapshotSynced(muted_threads) => {
                Self::MuteSnapshotSynced { muted_threads }
            }
            CoreEvent::MuteEventsReceived { 0: events } => Self::MuteEventsReceived { events },
            CoreEvent::PhoneSyncProgress { progress, status } => {
                Self::PhoneSyncProgress { progress, status }
            }
            CoreEvent::PhoneSyncFinished {
                threads,
                added,
                active_threads,
                cancelled,
                error,
            } => Self::PhoneSyncFinished {
                threads,
                added,
                active_threads,
                cancelled,
                error,
            },
            CoreEvent::FileDownloaded(res) => match res {
                Ok((file_name, path)) => Self::FileDownloaded {
                    ok: true,
                    file_name: Some(file_name),
                    path: Some(path),
                    error: None,
                },
                Err(error) => Self::FileDownloaded {
                    ok: false,
                    file_name: None,
                    path: None,
                    error: Some(error),
                },
            },
            CoreEvent::LoggedOut => Self::LoggedOut,
            CoreEvent::HistoryReachedStart(thread_id) => Self::HistoryReachedStart { thread_id },
            CoreEvent::AccountSwitched {
                uin,
                name,
                avatar,
                threads,
            } => Self::AccountSwitched {
                uin,
                name,
                avatar,
                threads,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::events::CoreImageData;

    #[test]
    fn core_image_data_encodes_valid_png_data_url() {
        let img = CoreImageData {
            width: 2,
            height: 2,
            rgba: vec![
                255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 255, 255,
            ],
        };
        let uri = img.to_png_data_url().expect("valid PNG data url");
        assert!(uri.starts_with("data:image/png;base64,"));
        let b64 = uri.trim_start_matches("data:image/png;base64,");
        let decoded = base64::engine::general_purpose::STANDARD
            .decode(b64)
            .unwrap();
        assert_eq!(&decoded[..8], b"\x89PNG\r\n\x1a\n");
    }

    #[test]
    fn core_event_dto_serializes_tagged_payload() {
        let dto = CoreEventDto::from(CoreEvent::StickerImageReady {
            key: "stk_1_2".into(),
            image: CoreImageData {
                width: 1,
                height: 1,
                rgba: vec![1, 2, 3, 4],
            },
        });
        let val = serde_json::to_value(dto).unwrap();
        assert_eq!(val["type"], "StickerImageReady");
        assert_eq!(val["payload"]["key"], "stk_1_2");
        assert!(val["payload"]["data_url"]
            .as_str()
            .unwrap()
            .starts_with("data:image/png;base64,"));
    }

    #[tokio::test]
    async fn bootstrap_snapshot_hydrates_assets_stickers_and_redacted_diagnostics() {
        let session = crate::protocol::Session::new(false, false, false).unwrap();
        let engine = crate::core::CoreEngine::headless(session, ":memory:");
        let snap = engine.bootstrap_snapshot(Vec::new()).await;
        assert!(!snap.logged_in);
        assert!(!snap.popular_stickers.is_empty());
        assert!(!snap.onion_stickers.is_empty());
        assert!(!snap.pinned_packs.is_empty());
        assert!(snap.app_logo_data_url.starts_with("data:image/png;base64,"));

        let diag = engine.dev_diagnostics();
        assert_eq!(diag.crypto.zpw_ver, crate::session::ZPW_VER);
        assert_eq!(diag.crypto.param_cipher_key_len, 32);
    }
}
