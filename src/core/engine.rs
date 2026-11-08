//! Core application engine owning the single live [`Session`], WebSocket/poll loops,
//! SQLite persistence, and image/sticker decoding independent of any GUI framework.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex as StdMutex};
use std::time::Duration;

use anyhow::Result;
use serde::Serialize;
use tokio::sync::Mutex;

use super::events::{CoreEvent, CoreEventSink, CoreImageData, NoopEventSink};
use crate::clock::unix_now;
use crate::mcp::{load_autoreply_rules, match_autoreply_rule};
use crate::protocol::auth::{login, qr};
use crate::protocol::messaging::attachments::{dispatch_file_done_controls, send_attachment};
use crate::protocol::messaging::links::{
    extract_first_url, parse_link_metadata, send_link_message_with_client_id,
};
use crate::protocol::messaging::polling::{
    get_chat_from_polling, get_cloud_messages, get_last_msgs, poll_new_messages, PollState,
};
use crate::protocol::messaging::reactions::send_reaction;
use crate::protocol::messaging::send::{
    delete_message_for_me, forward_message_with_client_id, recall_message,
    send_group_message_with_client_id, send_message_with_client_id,
    send_quote_message_with_client_id, send_typing_indicator,
};
use crate::protocol::messaging::stickers::{
    fetch_sticker_pack_items, fetch_sticker_store_packs, fetch_sticker_suggest_keywords,
    search_stickers_by_keyword, send_sticker_with_client_id,
};
use crate::protocol::messaging::threads::{
    delete_conversation, is_generic_thread_name, set_hidden_conversation, should_replace_avatar,
    update_pinned_conversation,
};
use crate::protocol::messaging::unread::{
    extract_mute_controls, get_mute_snapshot, send_seen_v2, set_mute_conversation, set_unread_mark,
};
use crate::protocol::messaging::{ChatMessage, LinkMetadata, StickerItem, Thread};
use crate::protocol::session::{Session, SessionStatus};
use crate::protocol::social::friends::{
    accept_friend_request, check_friendship_status, find_user_by_phone, get_friends,
    get_user_online_status, reject_friend_request, send_friend_request, undo_friend_request,
    FriendshipStatus,
};
use crate::protocol::social::groups::{get_groups, resolve_thread_names};
use crate::protocol::sync::{
    set_last_mobile_sync_status, set_latest_mobile_temp_key, MobileSyncControlEvent,
    PHONE_SYNC_CANCEL, PHONE_SYNC_RUNNING,
};
use crate::protocol::wire::{next_client_message_id, SendMessageReceipt};
use crate::services::account_manager::{save_current_account_snapshot, switch_to_account};
use crate::services::native_notifications::{request_if_user_action, status as notif_status};
use crate::services::updater::{apply_github_release_update, fetch_github_latest_release};
use crate::storage::{
    db_path_for_account, import_mobile_backup_container_with_uid, import_sync_v2_messages,
    import_sync_v2_meta, jxl_to_jpg_url, load_history_into_threads, load_threads_cache,
    persist_threads, save_threads_cache, sync_phone_messages_opts, sync_thread, MessageStore,
    StoredMessage,
};

/// Summary returned by [`CoreEngine::status`].
#[derive(Debug, Clone, Serialize)]
pub struct CoreStatus {
    pub app: &'static str,
    pub version: &'static str,
    pub logged_in: bool,
    pub uin: String,
    pub display_name: String,
    pub threads_in_db: usize,
}

/// UI-independent runtime owning the active Zalo session, polling/WebSocket sync, and SQLite access.
pub struct CoreEngine {
    session: Arc<Mutex<Session>>,
    poll_state: Arc<Mutex<PollState>>,
    known_threads: Arc<StdMutex<Vec<(String, String, bool)>>>,
    explicit_db_path: Option<String>,
    sink: Arc<dyn CoreEventSink>,
    is_polling: Arc<AtomicBool>,
    current_qr_task: Mutex<Option<tokio::task::JoinHandle<()>>>,
}

impl CoreEngine {
    /// Create a UI-agnostic engine resolving per-account SQLite paths dynamically (`accounts/<uin>_history.db`).
    pub fn new(session: Session, sink: Arc<dyn CoreEventSink>) -> Arc<Self> {
        Arc::new(Self {
            session: Arc::new(Mutex::new(session)),
            poll_state: Arc::new(Mutex::new(PollState::default())),
            known_threads: Arc::new(StdMutex::new(Vec::new())),
            explicit_db_path: None,
            sink,
            is_polling: Arc::new(AtomicBool::new(false)),
            current_qr_task: Mutex::new(None),
        })
    }

    /// Create an engine pinned to an explicit SQLite file path (used by `--mcp` and tests).
    pub fn with_db_path(
        session: Session,
        db_path: impl Into<String>,
        sink: Arc<dyn CoreEventSink>,
    ) -> Arc<Self> {
        Arc::new(Self {
            session: Arc::new(Mutex::new(session)),
            poll_state: Arc::new(Mutex::new(PollState::default())),
            known_threads: Arc::new(StdMutex::new(Vec::new())),
            explicit_db_path: Some(db_path.into()),
            sink,
            is_polling: Arc::new(AtomicBool::new(false)),
            current_qr_task: Mutex::new(None),
        })
    }

    /// Create an engine for headless CLI/MCP usage with a no-op UI sink.
    pub fn headless(session: Session, db_path: impl Into<String>) -> Arc<Self> {
        Self::with_db_path(session, db_path, Arc::new(NoopEventSink))
    }

    /// Access the shared [`Session`] mutex.
    pub fn session_arc(&self) -> &Arc<Mutex<Session>> {
        &self.session
    }

    /// Emit an event directly to the attached UI sink.
    pub fn emit(&self, event: CoreEvent) {
        self.sink.emit(event);
    }

    /// Resolve the active SQLite path (explicit override or `accounts/<uin>_history.db`).
    pub async fn active_db_path(&self) -> String {
        if let Some(ref p) = self.explicit_db_path {
            return p.clone();
        }
        let s = self.session.lock().await;
        let uin = s.login.as_ref().map(|l| l.uin.as_str()).unwrap_or_default();
        db_path_for_account(uin)
    }

    /// Synchronous DB path lookup for places using an already-captured `uin`.
    pub fn resolve_db_path_for_uin(&self, uin: &str) -> String {
        if let Some(ref p) = self.explicit_db_path {
            p.clone()
        } else {
            db_path_for_account(uin)
        }
    }

    /// Publish the current known `(id, name, is_group)` threads to background polling.
    pub fn set_known_threads(&self, threads: Vec<(String, String, bool)>) {
        if let Ok(mut g) = self.known_threads.lock() {
            *g = threads;
        }
    }

    fn known_threads_snapshot(&self) -> Vec<(String, String, bool)> {
        self.known_threads
            .lock()
            .map(|g| g.clone())
            .unwrap_or_default()
    }

    /// Start background message polling, `zpw_ws` WebSocket push listener, update checks,
    /// initial thread refresh, and optional local HTTP REST/MCP server inside the active Tokio runtime.
    pub fn start_background_loops(self: &Arc<Self>, api_port: Option<u16>) {
        // 1. Apply server info and refresh threads/sticker keywords if logged in.
        {
            let engine = Arc::clone(self);
            tokio::spawn(async move {
                let is_logged = {
                    let mut s = engine.session.lock().await;
                    let _ = login::fetch_and_apply_server_info(&mut s).await;
                    s.login.is_some()
                };
                if is_logged {
                    engine.refresh_threads(false).await;
                    let s = engine.session.lock().await.clone();
                    if let Ok(kws) = fetch_sticker_suggest_keywords(&s).await {
                        if !kws.is_empty() {
                            engine.emit(CoreEvent::StickerSuggestKeywordsLoaded(kws));
                        }
                    }
                }
            });
        }

        // 2. Message poll timer (1200ms). Skips when a poll is already in flight or logged out.
        {
            let engine = Arc::clone(self);
            tokio::spawn(async move {
                let mut tick = tokio::time::interval(Duration::from_millis(1200));
                tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
                loop {
                    tick.tick().await;
                    engine.poll_once().await;
                }
            });
        }

        // 3. Optional local REST + MCP HTTP server sharing this single CoreEngine.
        if let Some(port) = api_port.filter(|&p| p > 0) {
            let engine = Arc::clone(self);
            tokio::spawn(async move {
                let _ = crate::mcp::run_local_http_server(engine, port).await;
            });
        }

        // 4. Real-time `zpw_ws` WebSocket listener for push chat messages, file_done, mute, and phone sync.
        {
            let engine = Arc::clone(self);
            tokio::spawn(async move {
                let mut ws_rx: Option<
                    tokio::sync::mpsc::UnboundedReceiver<crate::socket::WsEvent>,
                > = None;
                let mut ws_sek = String::new();
                loop {
                    let s_snap = engine.session.lock().await.clone();
                    if s_snap.login.is_some() {
                        let cur_sek = s_snap.zpw_sek.clone().unwrap_or_default();
                        if !cur_sek.is_empty() && (ws_rx.is_none() || ws_sek != cur_sek) {
                            ws_sek = cur_sek;
                            ws_rx = Some(crate::socket::spawn(s_snap.clone()));
                        }
                        if let Some(rx) = ws_rx.as_mut() {
                            match tokio::time::timeout(Duration::from_millis(1500), rx.recv()).await
                            {
                                Ok(Some(ev)) => {
                                    engine.handle_ws_event(&s_snap, ev).await;
                                }
                                Ok(None) => {
                                    ws_rx = None;
                                    tokio::time::sleep(Duration::from_secs(2)).await;
                                }
                                Err(_) => {}
                            }
                        } else {
                            tokio::time::sleep(Duration::from_secs(2)).await;
                        }
                    } else {
                        ws_rx = None;
                        ws_sek.clear();
                        tokio::time::sleep(Duration::from_secs(3)).await;
                    }
                }
            });
        }

        // 5. Periodic GitHub Releases API update check (every 30 minutes).
        {
            let engine = Arc::clone(self);
            tokio::spawn(async move {
                let mut ticker = tokio::time::interval(Duration::from_secs(30 * 60));
                ticker.tick().await;
                loop {
                    ticker.tick().await;
                    let res = fetch_github_latest_release().await;
                    engine.emit(CoreEvent::UpdateCheckFinished {
                        result: res,
                        silent: true,
                    });
                }
            });
        }
    }

    async fn handle_ws_event(self: &Arc<Self>, s_snap: &Session, ev: crate::socket::WsEvent) {
        let data_node = ev.payload.get("data").unwrap_or(&ev.payload);
        dispatch_file_done_controls(data_node);
        let mute_evs = extract_mute_controls(data_node, unix_now());
        if !mute_evs.is_empty() {
            self.emit(CoreEvent::MuteEventsReceived(mute_evs));
        }

        for sync_ev in crate::protocol::sync::extract_mobile_sync_controls(data_node) {
            self.handle_sync_control_event(s_snap, sync_ev).await;
        }

        if ev.is_chat_ish() {
            let mut ws_threads = Vec::new();
            if let Some(li) = s_snap.login.as_ref() {
                if !li.send2me_id.is_empty() {
                    ws_threads.push(Thread {
                        id: li.send2me_id.clone(),
                        name: "Cloud của tôi (Lưu trữ)".into(),
                        ..Default::default()
                    });
                }
            }
            crate::protocol::messaging::polling::merge_previews(&mut ws_threads, &ev.payload);
            ws_threads.retain(|t| !t.messages.is_empty());
            if !ws_threads.is_empty() {
                resolve_thread_names(s_snap, &mut ws_threads).await;
                self.emit(CoreEvent::ThreadsUpdated(ws_threads));
            }
        }
    }

    async fn handle_sync_control_event(
        self: &Arc<Self>,
        s_snap: &Session,
        sync_ev: MobileSyncControlEvent,
    ) {
        match sync_ev {
            MobileSyncControlEvent::TransferAfterLogin { temp_key } => {
                set_latest_mobile_temp_key(&temp_key);
                let known = load_threads_cache();
                let engine = Arc::clone(self);
                tokio::spawn(async move {
                    engine.sync_phone_messages(known, false).await;
                });
            }
            MobileSyncControlEvent::WaitingConfirmOnPhone { .. } => {
                let note = "Đang chờ bạn bấm 'Đồng bộ' trên màn hình Zalo điện thoại…".to_string();
                set_last_mobile_sync_status(Some(note.clone()));
                self.emit(CoreEvent::StatusUpdate(note));
            }
            MobileSyncControlEvent::MobileBusyRestoring => {
                let note = "Điện thoại đang bận sao lưu/khôi phục phiên trước — hãy mở Zalo trên điện thoại xác nhận hoặc thử lại sau ít phút".to_string();
                set_last_mobile_sync_status(Some(note.clone()));
                self.emit(CoreEvent::StatusUpdate(note));
            }
            MobileSyncControlEvent::UserConfirmed => {
                let note =
                    "Điện thoại đã xác nhận đồng bộ — đang nén và đóng gói tin nhắn…".to_string();
                set_last_mobile_sync_status(Some(note.clone()));
                self.emit(CoreEvent::StatusUpdate(note));
            }
            MobileSyncControlEvent::UserRejected => {
                let note = "Bạn đã từ chối yêu cầu đồng bộ tin nhắn trên điện thoại".to_string();
                set_last_mobile_sync_status(Some(note.clone()));
                self.emit(CoreEvent::StatusUpdate(note));
            }
            MobileSyncControlEvent::BackupReady {
                url,
                file_name,
                encrypted_key,
                raw_uid,
                ..
            } => {
                let s_sync = s_snap.clone();
                let engine = Arc::clone(self);
                tokio::spawn(async move {
                    let status_dl =
                        format!("Đang tải và giải mã gói đồng bộ tin nhắn ({file_name})…");
                    set_last_mobile_sync_status(Some(status_dl.clone()));
                    engine.emit(CoreEvent::StatusUpdate(status_dl));

                    match import_mobile_backup_container_with_uid(
                        &s_sync,
                        &url,
                        &encrypted_key,
                        raw_uid,
                    )
                    .await
                    {
                        Ok(count) => {
                            let note = format!(
                                "Đã giải mã & đồng bộ thành công {count} tin nhắn từ điện thoại vào ứng dụng!"
                            );
                            set_last_mobile_sync_status(Some(note.clone()));
                            engine.emit(CoreEvent::StatusUpdate(note));
                            engine.emit(CoreEvent::ThreadsUpdated(load_threads_cache()));
                        }
                        Err(e) => {
                            let err_note =
                                format!("Không thể giải mã gói đồng bộ ({file_name}): {e}");
                            set_last_mobile_sync_status(Some(err_note.clone()));
                            engine.emit(CoreEvent::StatusUpdate(err_note));
                        }
                    }
                });
            }
            MobileSyncControlEvent::SyncV2Batch {
                url,
                sync_id,
                batch_type,
                ..
            } => {
                let s_sync = s_snap.clone();
                let engine = Arc::clone(self);
                tokio::spawn(async move {
                    if batch_type == 2 {
                        let note =
                            "Đang giải mã thông tin phiên đồng bộ từ điện thoại…".to_string();
                        set_last_mobile_sync_status(Some(note.clone()));
                        engine.emit(CoreEvent::StatusUpdate(note));
                        let _ = import_sync_v2_meta(&s_sync, &url).await;
                    } else if batch_type == 1 {
                        let note = "Đang tải và giải mã gói tin nhắn từ điện thoại…".to_string();
                        set_last_mobile_sync_status(Some(note.clone()));
                        engine.emit(CoreEvent::StatusUpdate(note));

                        match import_sync_v2_messages(&s_sync, &url).await {
                            Ok(count) => {
                                let fin = format!(
                                    "Đã giải mã & đồng bộ thành công {count} tin nhắn từ điện thoại vào ứng dụng!"
                                );
                                set_last_mobile_sync_status(Some(fin.clone()));
                                engine.emit(CoreEvent::StatusUpdate(fin));
                                engine.emit(CoreEvent::ThreadsUpdated(load_threads_cache()));
                            }
                            Err(e) => {
                                let err = format!("Lỗi giải mã tin nhắn Sync v2 ({sync_id}): {e}");
                                set_last_mobile_sync_status(Some(err.clone()));
                                engine.emit(CoreEvent::StatusUpdate(err));
                            }
                        }
                    }
                });
            }
        }
    }

    // ------------------------------------------------------------------------
    // Direct Query / Messaging Operations (used by MCP, REST, Tauri & GUI)
    // ------------------------------------------------------------------------

    /// Return current session status and stored thread count.
    pub async fn status(&self) -> CoreStatus {
        let (logged_in, uin, display_name) = {
            let s = self.session.lock().await;
            let logged = s.login.is_some() && s.zpw_sek.is_some();
            let uin = s.login.as_ref().map(|l| l.uin.clone()).unwrap_or_default();
            let dname = s
                .login
                .as_ref()
                .map(|l| l.display_name.clone())
                .unwrap_or_default();
            (logged, uin, dname)
        };
        let db_path = self.resolve_db_path_for_uin(&uin);
        let thread_count = tokio::task::spawn_blocking(move || {
            MessageStore::open(&db_path)
                .and_then(|st| st.all_threads())
                .map(|v| v.len())
                .unwrap_or(0)
        })
        .await
        .unwrap_or(0);

        CoreStatus {
            app: "alolite",
            version: env!("CARGO_PKG_VERSION"),
            logged_in,
            uin,
            display_name,
            threads_in_db: thread_count,
        }
    }

    /// Load cached threads hydrated with up to `preview_msgs_per_thread` recent messages from SQLite.
    pub async fn list_threads(&self, preview_msgs_per_thread: usize) -> Vec<Thread> {
        let db_path = self.active_db_path().await;
        tokio::task::spawn_blocking(move || {
            let mut threads = load_threads_cache();
            load_history_into_threads(&db_path, &mut threads, preview_msgs_per_thread);
            threads
        })
        .await
        .unwrap_or_default()
    }

    /// Read stored messages for `thread_id` from SQLite (`spawn_blocking`, `INV-3`).
    pub async fn get_messages(&self, thread_id: &str, limit: usize) -> Result<Vec<StoredMessage>> {
        let db_path = self.active_db_path().await;
        let tid = thread_id.to_string();
        tokio::task::spawn_blocking(move || MessageStore::open(&db_path)?.messages_for(&tid, limit))
            .await?
    }

    /// Full-text search across stored messages in SQLite (`spawn_blocking`, `INV-3`).
    pub async fn search_messages(&self, query: &str, limit: usize) -> Result<Vec<StoredMessage>> {
        let db_path = self.active_db_path().await;
        let q = query.to_string();
        tokio::task::spawn_blocking(move || {
            MessageStore::open(&db_path)?.search_messages(&q, limit)
        })
        .await?
    }

    /// Send a plain 1-1 or group text message directly and return the typed server receipt.
    pub async fn send_text(
        &self,
        to_id: &str,
        text: &str,
        is_group: bool,
    ) -> Result<SendMessageReceipt> {
        let s = self.session.lock().await.clone();
        let client_id = next_client_message_id();
        if is_group {
            send_group_message_with_client_id(&s, to_id, text, client_id).await
        } else {
            send_message_with_client_id(&s, to_id, text, client_id).await
        }
    }

    /// Send a message from interactive UI (auto-upgrading embedded URLs to link previews) and emit events.
    pub async fn send_message_or_link(
        self: &Arc<Self>,
        to_id: String,
        text: String,
        is_group: bool,
    ) {
        let s = self.session.lock().await.clone();
        let client_id = next_client_message_id();
        if let Some(found_url) = extract_first_url(&text) {
            let meta = parse_link_metadata(&s, &found_url)
                .await
                .unwrap_or_else(|_| LinkMetadata {
                    url: found_url.clone(),
                    title: found_url.clone(),
                    desc: String::new(),
                    thumb: String::new(),
                    src: "Liên kết".into(),
                });
            self.emit(CoreEvent::LinkPreviewReady(meta.clone()));
            match send_link_message_with_client_id(&s, &to_id, &text, &meta, is_group, client_id)
                .await
            {
                Ok(receipt) => {
                    let final_id = if receipt.msg_id.is_empty() {
                        format!("out_{client_id}")
                    } else {
                        receipt.msg_id
                    };
                    let msg = ChatMessage {
                        id: final_id,
                        client_message_id: client_id.to_string(),
                        sender: "Tôi".into(),
                        text,
                        time: crate::ui::clock(),
                        is_outgoing: true,
                        ts: unix_now() * 1000,
                        link_url: Some(meta.url),
                        link_title: (!meta.title.is_empty()).then_some(meta.title),
                        link_desc: (!meta.desc.is_empty()).then_some(meta.desc),
                        link_thumb: (!meta.thumb.is_empty()).then_some(meta.thumb),
                        link_src: (!meta.src.is_empty()).then_some(meta.src),
                        ..Default::default()
                    };
                    self.emit(CoreEvent::AttachmentSent {
                        thread_id: to_id,
                        msg,
                    });
                    return;
                }
                Err(e) => {
                    let err_s = e.to_string();
                    crate::logger::log_error("core::send_link", &err_s);
                    self.emit(CoreEvent::SendFailed {
                        thread_id: to_id,
                        text,
                        error: err_s,
                    });
                    return;
                }
            }
        }

        let res = if is_group {
            send_group_message_with_client_id(&s, &to_id, &text, client_id).await
        } else {
            send_message_with_client_id(&s, &to_id, &text, client_id).await
        };
        match res {
            Ok(receipt) => {
                self.emit(CoreEvent::MessageSent {
                    client_message_id: client_id.to_string(),
                    thread_id: to_id,
                    text,
                    msg_id: receipt.msg_id,
                });
            }
            Err(e) => {
                let err_s = e.to_string();
                crate::logger::log_error("core::send_message", &err_s);
                self.emit(CoreEvent::SendFailed {
                    thread_id: to_id,
                    text,
                    error: err_s,
                });
                if SessionStatus::from_error(&e) == SessionStatus::Expired {
                    self.handle_expired_session("Không thể gửi vì phiên zpw_sek đã hết hạn")
                        .await;
                }
            }
        }
    }

    pub async fn send_quote(
        &self,
        to_id: String,
        text: String,
        quoted: ChatMessage,
        is_group: bool,
    ) {
        let s = self.session.lock().await.clone();
        let client_id = next_client_message_id();
        match send_quote_message_with_client_id(&s, &to_id, &text, &quoted, is_group, client_id)
            .await
        {
            Ok(receipt) => {
                self.emit(CoreEvent::QuoteSent {
                    client_message_id: client_id.to_string(),
                    thread_id: to_id,
                    text,
                    msg_id: receipt.msg_id,
                    quoted,
                });
            }
            Err(e) => {
                let err_s = e.to_string();
                crate::logger::log_error("core::send_quote", &err_s);
                self.emit(CoreEvent::SendFailed {
                    thread_id: to_id,
                    text,
                    error: err_s,
                });
            }
        }
    }

    pub async fn forward_message(
        &self,
        target_id: String,
        target_is_group: bool,
        msg: ChatMessage,
    ) {
        let s = self.session.lock().await.clone();
        let client_id = next_client_message_id();
        match forward_message_with_client_id(&s, &target_id, target_is_group, &msg, client_id).await
        {
            Ok(receipt) if receipt.is_ok() => {
                if let Some(sid) = msg.sticker_id.filter(|v| *v > 0) {
                    self.emit(CoreEvent::StickerSent {
                        client_message_id: client_id.to_string(),
                        thread_id: target_id,
                        msg_id: receipt.msg_id,
                        sticker_id: sid,
                        cate_id: msg.sticker_cate_id.unwrap_or(0),
                        sticker_type: msg.sticker_type.unwrap_or(7),
                    });
                } else {
                    self.emit(CoreEvent::MessageSent {
                        client_message_id: client_id.to_string(),
                        thread_id: target_id,
                        text: msg.text.clone(),
                        msg_id: receipt.msg_id,
                    });
                }
                self.emit(CoreEvent::StatusUpdate("Đã chuyển tiếp tin nhắn".into()));
            }
            Ok(receipt) => {
                let err_s = format!("server error {}", receipt.err_code().unwrap_or(0));
                crate::logger::log_error("core::forward", &err_s);
                self.emit(CoreEvent::StatusUpdate(format!(
                    "Chuyển tiếp thất bại: {err_s}"
                )));
            }
            Err(e) => {
                crate::logger::log_error("core::forward", &e.to_string());
                self.emit(CoreEvent::StatusUpdate(format!(
                    "Chuyển tiếp thất bại: {e}"
                )));
            }
        }
    }

    pub async fn react(&self, to_id: String, is_group: bool, msg: ChatMessage, reaction: String) {
        let s = self.session.lock().await.clone();
        match send_reaction(&s, &to_id, is_group, &msg, &reaction).await {
            Ok(ack) if ack.is_ok() => {
                self.emit(CoreEvent::Reacted {
                    thread_id: to_id,
                    msg_id: msg.id,
                    ts: msg.ts,
                    reaction,
                });
            }
            Ok(ack) => {
                let err_s = format!("server error {}", ack.err_code().unwrap_or(0));
                crate::logger::log_error("core::react", &err_s);
                self.emit(CoreEvent::StatusUpdate(format!(
                    "Thả cảm xúc thất bại: {err_s}"
                )));
            }
            Err(e) => {
                crate::logger::log_error("core::react", &e.to_string());
                self.emit(CoreEvent::StatusUpdate(format!(
                    "Thả cảm xúc thất bại: {e}"
                )));
            }
        }
    }

    pub async fn delete_for_me(&self, to_id: String, is_group: bool, msg: ChatMessage) {
        let s = self.session.lock().await.clone();
        let local_only =
            msg.id.is_empty() || msg.id.starts_with("out_") || msg.id.starts_with("local_");
        let res = if local_only {
            Ok(crate::protocol::ActionAck::ok())
        } else {
            delete_message_for_me(&s, &to_id, is_group, &msg).await
        };
        match res {
            Ok(ack) if ack.is_ok() => {
                let db_path = self.active_db_path().await;
                let (tid, mid, ts) = (to_id.clone(), msg.id.clone(), msg.ts);
                let _ = tokio::task::spawn_blocking(move || {
                    if let Ok(mut store) = MessageStore::open(&db_path) {
                        let _ = store.delete_message(&tid, &mid, ts);
                    }
                })
                .await;
                self.emit(CoreEvent::MessageDeleted {
                    thread_id: to_id,
                    msg_id: msg.id,
                    ts: msg.ts,
                });
            }
            Ok(ack) => {
                let err_s = format!("server error {}", ack.err_code().unwrap_or(0));
                crate::logger::log_error("core::delete", &err_s);
                self.emit(CoreEvent::StatusUpdate(format!(
                    "Xóa tin nhắn thất bại: {err_s}"
                )));
            }
            Err(e) => {
                crate::logger::log_error("core::delete", &e.to_string());
                self.emit(CoreEvent::StatusUpdate(format!(
                    "Xóa tin nhắn thất bại: {e}"
                )));
            }
        }
    }

    pub async fn recall_message(&self, to_id: String, is_group: bool, msg: ChatMessage) {
        let s = self.session.lock().await.clone();
        let local_only =
            msg.id.is_empty() || msg.id.starts_with("out_") || msg.id.starts_with("local_");
        let res = if local_only {
            Ok(crate::protocol::ActionAck::ok())
        } else {
            recall_message(&s, &to_id, is_group, &msg).await
        };
        match res {
            Ok(ack) if ack.is_ok() => {
                self.emit(CoreEvent::MessageRecalled {
                    thread_id: to_id,
                    msg_id: msg.id,
                    ts: msg.ts,
                });
            }
            Ok(ack) => {
                let err_s = format!("server error {}", ack.err_code().unwrap_or(0));
                crate::logger::log_error("core::recall", &err_s);
                self.emit(CoreEvent::StatusUpdate(format!(
                    "Đã thu hồi cục bộ (Máy chủ: {err_s})"
                )));
            }
            Err(e) => {
                crate::logger::log_error("core::recall", &e.to_string());
                self.emit(CoreEvent::StatusUpdate(format!(
                    "Không thể thu hồi trên máy chủ: {e}"
                )));
            }
        }
    }

    pub async fn send_sticker(
        self: &Arc<Self>,
        to_id: String,
        sticker_id: u64,
        cate_id: u64,
        sticker_type: u32,
        is_group: bool,
    ) {
        let s = self.session.lock().await.clone();
        let client_id = next_client_message_id();
        match send_sticker_with_client_id(
            &s,
            &to_id,
            sticker_id,
            cate_id,
            sticker_type,
            is_group,
            client_id,
        )
        .await
        {
            Ok(receipt) => {
                self.emit(CoreEvent::StickerSent {
                    client_message_id: client_id.to_string(),
                    thread_id: to_id,
                    msg_id: receipt.msg_id,
                    sticker_id,
                    cate_id,
                    sticker_type,
                });
            }
            Err(e) => {
                let err_s = e.to_string();
                crate::logger::log_error("core::send_sticker", &err_s);
                self.emit(CoreEvent::SendFailed {
                    thread_id: to_id,
                    text: String::new(),
                    error: err_s,
                });
                if SessionStatus::from_error(&e) == SessionStatus::Expired {
                    self.handle_expired_session(
                        "Không thể gửi nhãn dán vì phiên zpw_sek đã hết hạn",
                    )
                    .await;
                }
            }
        }
    }

    pub async fn send_attachment_file(
        &self,
        to_id: String,
        is_group: bool,
        path: PathBuf,
        caption: String,
    ) {
        let fname = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("tập tin")
            .to_string();
        self.emit(CoreEvent::StatusUpdate(format!(
            "⏳ Đang tải lên và gửi '{fname}'…"
        )));
        let s = self.session.lock().await.clone();
        match send_attachment(&s, &to_id, is_group, &path, &caption).await {
            Ok(cm) => {
                self.emit(CoreEvent::AttachmentSent {
                    thread_id: to_id,
                    msg: cm,
                });
            }
            Err(e) => {
                let err_s = e.to_string();
                crate::logger::log_error("core::send_attachment", &err_s);
                self.emit(CoreEvent::SendFailed {
                    thread_id: to_id,
                    text: caption,
                    error: format!("Lỗi gửi '{fname}': {err_s}"),
                });
            }
        }
    }

    pub async fn send_typing(&self, thread_id: &str, is_group: bool) {
        let s = self.session.lock().await.clone();
        let _ = send_typing_indicator(&s, thread_id, is_group).await;
    }

    pub async fn send_seen(&self, thread_id: &str, is_group: bool, last_msg_id: &str) {
        let s = self.session.lock().await.clone();
        let _ = send_seen_v2(&s, thread_id, is_group, last_msg_id).await;
    }

    // ------------------------------------------------------------------------
    // Auth, Session, and Account Switching
    // ------------------------------------------------------------------------

    async fn handle_expired_session(self: &Arc<Self>, message: &str) {
        {
            let mut sl = self.session.lock().await;
            sl.mark_expired();
            let _ = sl.save_to_file("session.json");
            sl.logout();
        }
        self.emit(CoreEvent::SessionExpired(message.to_string()));
        self.request_qr().await;
    }

    pub async fn request_qr(self: &Arc<Self>) {
        {
            let mut guard = self.current_qr_task.lock().await;
            if let Some(task) = guard.take() {
                task.abort();
            }
        }
        let engine = Arc::clone(self);
        let handle = tokio::spawn(async move {
            {
                let mut s = engine.session.lock().await;
                s.logout();
            }
            {
                let mut ps = engine.poll_state.lock().await;
                *ps = PollState::default();
            }
            {
                let mut s = engine.session.lock().await;
                let _ = login::fetch_and_apply_server_info(&mut s).await;
            }
            engine.emit(CoreEvent::StatusUpdate("Đang xin mã QR mới…".into()));

            let s_snap = engine.session.lock().await.clone();
            match qr::request_qr(&s_snap, "vi").await {
                Ok(mut q) => {
                    let Ok((w, h, pixels)) = q.rgba() else {
                        engine.emit(CoreEvent::LoginFailed(
                            "Máy chủ Zalo không trả về dữ liệu mã QR".into(),
                        ));
                        return;
                    };
                    engine.emit(CoreEvent::QrReady {
                        qr_login: q.clone(),
                        qr_image: CoreImageData {
                            width: w,
                            height: h,
                            rgba: pixels,
                        },
                    });

                    let scan_res = qr::wait_for_scan(&s_snap, &mut q).await;
                    let Ok(scanned) = scan_res else {
                        if let Err(e) = scan_res {
                            engine.emit(CoreEvent::QrExpired(format!("Mã QR đã hết hạn ({e})")));
                        }
                        return;
                    };
                    engine.emit(CoreEvent::QrScanned {
                        name: scanned.display_name,
                        avatar: scanned.avatar,
                    });

                    match qr::confirm(&s_snap, &q).await {
                        Ok(confirmed) => {
                            let mut s = engine.session.lock().await;
                            confirmed.apply_to_session(&mut s, &q);
                            let _ = s.save_to_file("session.json");
                            let _ = login::fetch_and_apply_login_info(&mut s).await;
                            let _ = s.sync_profile_me().await;
                            let _ = s.save_to_file("session.json");
                            let _ = s.save_device_identity("device.json");
                            let s_clone = s.clone();
                            drop(s);

                            engine.finish_authenticated_bootstrap(s_clone).await;
                        }
                        Err(e) => {
                            engine.emit(CoreEvent::QrExpired(format!(
                                "Xác nhận trên điện thoại thất bại: {e}"
                            )));
                        }
                    }
                }
                Err(e) => {
                    engine.emit(CoreEvent::LoginFailed(e.to_string()));
                }
            }
        });
        let mut guard = self.current_qr_task.lock().await;
        *guard = Some(handle);
    }

    pub async fn login_password(self: &Arc<Self>, phone: String, pass: String) {
        self.emit(CoreEvent::StatusUpdate("Đang xác thực tài khoản…".into()));
        let auth_res = {
            let mut s = self.session.lock().await;
            let res = login::login_by_password(&mut s, &phone, &pass, "VN").await;
            if res.is_ok() {
                let _ = login::fetch_and_apply_login_info(&mut s).await;
                let _ = s.save_to_file("session.json");
                let _ = s.save_device_identity("device.json");
            }
            res.map(|_| s.clone())
        };
        match auth_res {
            Ok(s_clone) => {
                self.finish_authenticated_bootstrap(s_clone).await;
            }
            Err(e) => {
                self.emit(CoreEvent::LoginFailed(e.to_string()));
            }
        }
    }

    async fn finish_authenticated_bootstrap(self: &Arc<Self>, s: Session) {
        if s.login.is_none() || s.zpw_sek.is_none() {
            self.emit(CoreEvent::LoginFailed(
                "Đăng nhập thất bại: Không nhận được thông tin phiên từ máy chủ Zalo.".into(),
            ));
            return;
        }
        let uin = s.login.as_ref().map(|l| l.uin.clone()).unwrap_or_default();
        let _ = save_current_account_snapshot(&uin);
        let name = s
            .login
            .as_ref()
            .map(|l| l.display_name.clone())
            .unwrap_or_default();
        let avatar = s
            .login
            .as_ref()
            .map(|l| l.avatar.clone())
            .filter(|a| !a.is_empty());
        self.emit(CoreEvent::LoginSuccess {
            name,
            uin: uin.clone(),
            avatar,
        });

        let mut list: Vec<Thread> = Vec::new();
        if let Ok(friends) = get_friends(&s).await {
            merge_thread_headers(&mut list, friends, false);
        }
        if let Ok(groups) = get_groups(&s).await {
            merge_thread_headers(&mut list, groups, true);
        }

        let db_path = self.resolve_db_path_for_uin(&uin);
        let mut list = tokio::task::spawn_blocking(move || {
            load_history_into_threads(&db_path, &mut list, MessageStore::PRELOAD_MSGS);
            list
        })
        .await
        .unwrap_or_default();

        {
            let mut ps = self.poll_state.lock().await;
            *ps = PollState::default();
            let _ = poll_new_messages(&s, &mut ps, &mut list).await;
        }
        if let Ok(mute_snap) = get_mute_snapshot(&s, unix_now()).await {
            mute_snap.merge_into(&mut list);
            self.emit(CoreEvent::MuteSnapshotSynced(mute_snap.mutes));
        }

        let sync_snapshot = list.clone();
        self.emit(CoreEvent::ThreadsUpdated(list));

        let engine_sync = Arc::clone(self);
        tokio::spawn(async move {
            engine_sync.sync_phone_messages(sync_snapshot, true).await;
        });

        let engine_kw = Arc::clone(self);
        tokio::spawn(async move {
            if let Ok(kws) = fetch_sticker_suggest_keywords(&s).await {
                if !kws.is_empty() {
                    engine_kw.emit(CoreEvent::StickerSuggestKeywordsLoaded(kws));
                }
            }
        });
    }

    pub async fn logout(&self) {
        let cur_uin = {
            let s = self.session.lock().await;
            s.login.as_ref().map(|l| l.uin.clone()).unwrap_or_default()
        };
        if !cur_uin.is_empty() {
            let _ = save_current_account_snapshot(&cur_uin);
        }
        {
            let mut guard = self.current_qr_task.lock().await;
            if let Some(task) = guard.take() {
                task.abort();
            }
        }
        let _ = std::fs::remove_file("session.json");
        let _ = std::fs::remove_file("threads_cache.json");
        let _ = std::fs::remove_file("zalo_history.db");
        {
            let mut s = self.session.lock().await;
            s.logout();
            let _ = s.save_device_identity("device.json");
        }
        {
            let mut ps = self.poll_state.lock().await;
            *ps = PollState::default();
        }
        self.emit(CoreEvent::LoggedOut);
    }

    pub async fn switch_account(&self, target_uin: &str) {
        let cur_uin = {
            let s = self.session.lock().await;
            s.login.as_ref().map(|l| l.uin.clone()).unwrap_or_default()
        };
        if let Err(e) = switch_to_account(&cur_uin, target_uin) {
            self.emit(CoreEvent::StatusUpdate(format!(
                "Lỗi chuyển tài khoản: {e}"
            )));
            return;
        }
        let (loaded, uin, name, avatar) = {
            let mut s = self.session.lock().await;
            if s.load_from_file("session.json").is_ok() {
                let uin = s.login.as_ref().map(|l| l.uin.clone()).unwrap_or_default();
                let name = s
                    .login
                    .as_ref()
                    .map(|l| l.display_name.clone())
                    .unwrap_or_default();
                let ava = s
                    .login
                    .as_ref()
                    .and_then(|l| (!l.avatar.is_empty()).then(|| l.avatar.clone()));
                (true, uin, name, ava)
            } else {
                (false, String::new(), String::new(), None)
            }
        };
        if loaded {
            let db_path = self.resolve_db_path_for_uin(&uin);
            let threads = tokio::task::spawn_blocking(move || {
                let mut threads = load_threads_cache();
                if let Ok(store) = MessageStore::open(&db_path) {
                    let _ = store.hydrate_threads(&mut threads, 50);
                }
                threads
            })
            .await
            .unwrap_or_default();
            self.emit(CoreEvent::AccountSwitched {
                uin,
                name,
                avatar,
                threads,
            });
        }
    }

    pub async fn wipe_local_data(&self) {
        let db_path = self.active_db_path().await;
        let _ = std::fs::remove_file(&db_path);
        let _ = std::fs::remove_file("zalo_history.db");
        let _ = std::fs::remove_file("threads_cache.json");
        self.emit(CoreEvent::LocalDataWiped);
    }

    // ------------------------------------------------------------------------
    // Threads & Polling
    // ------------------------------------------------------------------------

    pub async fn persist_threads(&self, threads_snapshot: Vec<Thread>) {
        let db_path = self.active_db_path().await;
        let _ = tokio::task::spawn_blocking(move || {
            if let Ok(mut store) = MessageStore::open(&db_path) {
                let _ = persist_threads(&mut store, &threads_snapshot);
            }
            save_threads_cache(&threads_snapshot);
        })
        .await;
    }

    pub async fn pin_thread(&self, thread_id: &str, is_group: bool, pin: bool) {
        let s = self.session.lock().await.clone();
        let _ = update_pinned_conversation(&s, thread_id, is_group, pin).await;
    }

    pub async fn mark_thread_unread(&self, thread_id: &str, is_group: bool, unread: bool) {
        let s = self.session.lock().await.clone();
        let _ = set_unread_mark(&s, thread_id, is_group, unread).await;
    }

    pub async fn mute_thread(
        &self,
        thread_id: &str,
        is_group: bool,
        duration_sec: i64,
        mute: bool,
    ) {
        let s = self.session.lock().await.clone();
        let _ = set_mute_conversation(&s, thread_id, is_group, duration_sec, mute).await;
        if let Ok(snap) = get_mute_snapshot(&s, unix_now()).await {
            self.emit(CoreEvent::MuteSnapshotSynced(snap.mutes));
        }
    }

    pub async fn hide_thread(&self, thread_id: &str, is_group: bool, hide: bool) {
        let s = self.session.lock().await.clone();
        let _ = set_hidden_conversation(&s, thread_id, is_group, hide).await;
    }

    pub async fn delete_thread(&self, thread_id: &str, is_group: bool, last_cli_msg_id: &str) {
        let s = self.session.lock().await.clone();
        let _ = delete_conversation(&s, thread_id, is_group, last_cli_msg_id).await;
    }

    pub async fn refresh_threads(self: &Arc<Self>, manual: bool) {
        let mut s = self.session.lock().await.clone();
        if s.login.is_none() {
            return;
        }
        match s.session_status().await {
            SessionStatus::Valid => {}
            SessionStatus::Expired => {
                self.handle_expired_session(
                    "Phiên đăng nhập (zpw_sek) đã hết hạn trên máy chủ Zalo",
                )
                .await;
                return;
            }
            SessionStatus::Unreachable => {
                crate::logger::log_error(
                    "core::refresh_threads",
                    "không kết nối được máy chủ Zalo",
                );
                self.emit(CoreEvent::SessionUnreachable(
                    "Không kết nối được máy chủ Zalo".into(),
                ));
                return;
            }
        }
        if let Some((uid, name)) = s.sync_profile_me().await {
            {
                let mut sl = self.session.lock().await;
                if let Some(ref mut li) = sl.login {
                    li.uin = uid.clone();
                    if !name.is_empty() {
                        li.display_name = name.clone();
                    }
                }
                let _ = sl.save_to_file("session.json");
            }
            let u_ava = s
                .login
                .as_ref()
                .map(|l| l.avatar.clone())
                .filter(|a| !a.is_empty());
            self.emit(CoreEvent::LoginSuccess {
                name,
                uin: uid,
                avatar: u_ava,
            });
        }

        self.emit(CoreEvent::StatusUpdate(
            "⏳ Đang làm mới danh bạ & nhóm từ máy chủ Zalo (1/3)…".into(),
        ));
        let mut list = tokio::task::spawn_blocking(load_threads_cache)
            .await
            .unwrap_or_default();
        let mut errors = Vec::new();
        match get_friends(&s).await {
            Ok(friends) => merge_thread_headers(&mut list, friends, false),
            Err(e) => {
                crate::logger::log_error("core::refresh_friends", &e.to_string());
                errors.push(format!("Danh bạ: {e}"));
            }
        }

        self.emit(CoreEvent::StatusUpdate(
            "⏳ Đang tải danh sách nhóm Zalo (2/3)…".into(),
        ));
        match get_groups(&s).await {
            Ok(groups) => merge_thread_headers(&mut list, groups, true),
            Err(e) => {
                crate::logger::log_error("core::refresh_groups", &e.to_string());
                errors.push(format!("Nhóm: {e}"));
            }
        }

        let uin = s.login.as_ref().map(|l| l.uin.as_str()).unwrap_or_default();
        let db_path = self.resolve_db_path_for_uin(uin);
        let mut list = tokio::task::spawn_blocking(move || {
            load_history_into_threads(&db_path, &mut list, MessageStore::PRELOAD_MSGS);
            list
        })
        .await
        .unwrap_or_default();

        self.emit(CoreEvent::StatusUpdate(format!(
            "⏳ Đang làm mới tin nhắn cho {} hội thoại (3/3)…",
            list.len()
        )));
        let mut fresh_ps = PollState::default();
        if let Err(e) = poll_new_messages(&s, &mut fresh_ps, &mut list).await {
            crate::logger::log_error("core::refresh_poll", &e.to_string());
            errors.push(format!("Tin nhắn: {e}"));
        }
        {
            let mut ps = self.poll_state.lock().await;
            *ps = fresh_ps;
        }
        if let Ok(mute_snap) = get_mute_snapshot(&s, unix_now()).await {
            mute_snap.merge_into(&mut list);
            self.emit(CoreEvent::MuteSnapshotSynced(mute_snap.mutes));
        }

        let total_msgs: usize = list.iter().map(|t| t.messages.len()).sum();
        let n_threads = list.len();
        self.emit(CoreEvent::ThreadsUpdated(list));
        self.emit(CoreEvent::RefreshFinished {
            threads: n_threads,
            messages: total_msgs,
            errors,
            manual,
        });
    }

    pub async fn poll_once(self: &Arc<Self>) {
        if self.is_polling.swap(true, Ordering::SeqCst) {
            return;
        }
        let s = self.session.lock().await.clone();
        if s.login.is_some() {
            let known_threads = self.known_threads_snapshot();
            let mut temp_threads: Vec<Thread> = known_threads
                .into_iter()
                .map(|(id, name, is_group)| Thread {
                    id,
                    name,
                    is_group,
                    ..Default::default()
                })
                .collect();

            let mut poll_err: Option<anyhow::Error> = None;
            for _ in 0..3 {
                let poll_res = {
                    let mut ps = self.poll_state.lock().await;
                    get_chat_from_polling(&s, &mut ps).await
                };
                match poll_res {
                    Ok(batch) => {
                        batch.merge_into(&mut temp_threads);
                        for sync_ev in batch.sync_controls {
                            self.handle_sync_control_event(&s, sync_ev).await;
                        }
                        if !batch.mute_events.is_empty() {
                            self.emit(CoreEvent::MuteEventsReceived(batch.mute_events));
                        }
                        if !batch.more && !batch.has_reset {
                            break;
                        }
                    }
                    Err(e) => {
                        poll_err = Some(e);
                        break;
                    }
                }
            }

            match poll_err {
                Some(e) => {
                    if SessionStatus::from_error(&e) == SessionStatus::Expired {
                        self.handle_expired_session("Phiên đăng nhập (zpw_sek) đã hết hạn")
                            .await;
                    } else {
                        crate::logger::log_error("core::poll_messages", &e.to_string());
                        self.emit(CoreEvent::SessionUnreachable(e.to_string()));
                    }
                }
                None => {
                    temp_threads.retain(|t| !t.messages.is_empty());
                    if !temp_threads.is_empty() {
                        resolve_thread_names(&s, &mut temp_threads).await;
                        self.dispatch_autoreplies(&s, &temp_threads).await;
                        self.emit(CoreEvent::ThreadsUpdated(temp_threads));
                    }
                }
            }
        }
        self.is_polling.store(false, Ordering::SeqCst);
    }

    async fn dispatch_autoreplies(&self, s: &Session, threads: &[Thread]) {
        let rules = load_autoreply_rules();
        if !rules.iter().any(|r| r.enabled) {
            return;
        }
        for t in threads {
            for m in &t.messages {
                if m.is_outgoing {
                    continue;
                }
                if let Some(matched) = match_autoreply_rule(&rules, &m.text, t.is_group) {
                    let reply_text = matched.reply_text;
                    let client_id = next_client_message_id();
                    let res = if t.is_group {
                        send_group_message_with_client_id(s, &t.id, &reply_text, client_id).await
                    } else {
                        send_message_with_client_id(s, &t.id, &reply_text, client_id).await
                    };
                    if let Ok(receipt) = res {
                        if receipt.is_ok() {
                            self.emit(CoreEvent::MessageSent {
                                client_message_id: client_id.to_string(),
                                thread_id: t.id.clone(),
                                text: reply_text,
                                msg_id: receipt.msg_id,
                            });
                        }
                    }
                }
            }
        }
    }

    // ------------------------------------------------------------------------
    // History & Mobile Sync
    // ------------------------------------------------------------------------

    pub async fn sync_history(&self, threads: Vec<(String, String, bool)>) {
        if crate::protocol::sync::RUNNING.swap(true, Ordering::SeqCst) {
            return;
        }
        crate::protocol::sync::CANCEL.store(false, Ordering::Relaxed);
        let s = self.session.lock().await.clone();
        let total = threads.len();
        let mut added = 0;
        let mut errors = Vec::new();
        for (i, (id, name, is_group)) in threads.into_iter().enumerate() {
            if crate::protocol::sync::CANCEL.load(Ordering::Relaxed) {
                break;
            }
            self.emit(CoreEvent::StatusUpdate(format!(
                "Đang đồng bộ {}/{total} · {name}",
                i + 1
            )));
            let mut t = Thread {
                id,
                name,
                is_group,
                ..Default::default()
            };
            match sync_thread(&s, &mut t).await {
                Ok(n) => added += n,
                Err(e) => errors.push(format!("{}: {e}", t.name)),
            }
        }
        self.emit(CoreEvent::SyncFinished {
            added,
            errors,
            cancelled: crate::protocol::sync::CANCEL.load(Ordering::Relaxed),
        });
        crate::protocol::sync::RUNNING.store(false, Ordering::SeqCst);
    }

    pub async fn load_group_history(&self, group_id: String, is_group: bool, have: usize) {
        let s = self.session.lock().await.clone();
        if s.login.is_none() {
            return;
        }
        let is_send2me = s
            .login
            .as_ref()
            .map(|l| !l.send2me_id.is_empty() && l.send2me_id == group_id)
            .unwrap_or(false);
        let uin = s.login.as_ref().map(|l| l.uin.as_str()).unwrap_or_default();
        let db_path = self.resolve_db_path_for_uin(uin);
        let gid_clone = group_id.clone();
        let mut t_opt = tokio::task::spawn_blocking(move || {
            let mut t_list = vec![Thread {
                id: gid_clone.clone(),
                is_group: is_group && !is_send2me,
                ..Default::default()
            }];
            load_history_into_threads(&db_path, &mut t_list, (have + 300).max(500));
            t_list.into_iter().find(|x| x.id == gid_clone)
        })
        .await
        .ok()
        .flatten();
        let Some(mut t) = t_opt.take() else {
            return;
        };

        if !is_group && !is_send2me {
            let mut one_list = vec![t.clone()];
            if let Ok(batch) = get_last_msgs(&s, std::slice::from_ref(&group_id)).await {
                batch.merge_into(&mut one_list);
            }
            if let Ok(batch_all) = get_last_msgs(&s, &[]).await {
                batch_all.merge_into(&mut one_list);
            }
            if let Some(updated) = one_list.into_iter().find(|x| x.id == group_id) {
                for m in updated.messages {
                    if !t.messages.iter().any(|existing| existing.id == m.id) {
                        t.messages.push(m);
                    }
                }
                if !updated.preview.is_empty() {
                    t.preview = updated.preview;
                }
            }
        }
        if is_group || is_send2me {
            if let Ok(cm_page) = get_cloud_messages(&s, &group_id, "0", false, 50).await {
                cm_page.merge_into_thread(&mut t);
            }
            let mut cursor_id = t
                .messages
                .first()
                .map(|m| m.id.as_str())
                .filter(|id| !id.is_empty() && id.chars().all(|c| c.is_ascii_digit()))
                .unwrap_or_default()
                .to_string();
            if !cursor_id.is_empty() && cursor_id != "0" {
                for page in 1..=5 {
                    let prev_len = t.messages.len();
                    let Ok(cm_old) = get_cloud_messages(&s, &group_id, &cursor_id, true, 50).await
                    else {
                        break;
                    };
                    cm_old.merge_into_thread(&mut t);
                    let new_len = t.messages.len();
                    self.emit(CoreEvent::StatusUpdate(format!(
                        "⏳ Đang đồng bộ lịch sử cũ (trang {page}/5 — {new_len} tin nhắn)…"
                    )));
                    let next_cursor = cm_old.last_msg_id.or_else(|| {
                        t.messages
                            .first()
                            .map(|m| m.id.clone())
                            .filter(|id| !id.is_empty() && id.chars().all(|c| c.is_ascii_digit()))
                    });
                    let Some(nc) = next_cursor else {
                        break;
                    };
                    if !cm_old.has_more || nc == cursor_id || new_len == prev_len {
                        break;
                    }
                    cursor_id = nc;
                }
            }
        }
        if is_group
            && !is_send2me
            && (is_generic_thread_name(&t.name) || t.avatar.as_deref().unwrap_or("").is_empty())
        {
            resolve_thread_names(&s, std::slice::from_mut(&mut t)).await;
        }
        let cnt = t.messages.len();
        if !t.messages.is_empty() || !t.name.is_empty() {
            let db_path = self.resolve_db_path_for_uin(uin);
            let t_save = t.clone();
            let _ = tokio::task::spawn_blocking(move || {
                if let Ok(mut store) = MessageStore::open(&db_path) {
                    let _ = persist_threads(&mut store, std::slice::from_ref(&t_save));
                }
            })
            .await;
            self.emit(CoreEvent::ThreadsUpdated(vec![t]));
        }
        if cnt <= have {
            self.emit(CoreEvent::HistoryReachedStart(group_id));
        } else {
            self.emit(CoreEvent::StatusUpdate(format!(
                "Đã tải thêm {} tin nhắn cũ ({cnt} tin trong hội thoại)",
                cnt.saturating_sub(have)
            )));
        }
    }

    pub async fn sync_phone_messages(&self, mut threads: Vec<Thread>, send_phone_prompt: bool) {
        if PHONE_SYNC_RUNNING.swap(true, Ordering::SeqCst) {
            return;
        }
        PHONE_SYNC_CANCEL.store(false, Ordering::Relaxed);
        let s = self.session.lock().await.clone();
        let sink = Arc::clone(&self.sink);
        let res = sync_phone_messages_opts(
            &s,
            &self.poll_state,
            &mut threads,
            send_phone_prompt,
            move |progress, status| {
                sink.emit(CoreEvent::PhoneSyncProgress { progress, status });
            },
        )
        .await;
        let cancelled = PHONE_SYNC_CANCEL.load(Ordering::Relaxed);
        match res {
            Ok((added, active_threads)) => {
                self.emit(CoreEvent::PhoneSyncFinished {
                    threads,
                    added,
                    active_threads,
                    cancelled,
                    error: None,
                });
            }
            Err(e) => {
                self.emit(CoreEvent::PhoneSyncFinished {
                    threads,
                    added: 0,
                    active_threads: 0,
                    cancelled,
                    error: Some(e.to_string()),
                });
            }
        }
        PHONE_SYNC_RUNNING.store(false, Ordering::SeqCst);
    }

    // ------------------------------------------------------------------------
    // Contacts & Friends
    // ------------------------------------------------------------------------

    pub async fn check_friend_status(&self, user_id: String) {
        let s = self.session.lock().await.clone();
        let status_res = check_friendship_status(&s, &user_id).await;
        let online_opt = get_user_online_status(&s, &user_id).await.ok();
        if let Ok(status) = status_res {
            self.emit(CoreEvent::FriendStatusChecked {
                user_id,
                status,
                online: online_opt,
            });
        }
    }

    pub async fn accept_friend(&self, user_id: String, user_name: String) {
        let s = self.session.lock().await.clone();
        match accept_friend_request(&s, &user_id).await {
            Ok(_) => {
                self.emit(CoreEvent::FriendStatusChecked {
                    user_id,
                    status: FriendshipStatus::Friend,
                    online: None,
                });
                self.emit(CoreEvent::StatusUpdate(format!(
                    "Đã đồng ý kết bạn với {user_name}!"
                )));
            }
            Err(e) => {
                self.emit(CoreEvent::StatusUpdate(format!(
                    "Không thể đồng ý kết bạn với {user_name}: {e}"
                )));
            }
        }
    }

    pub async fn reject_friend(&self, user_id: String, user_name: String) {
        let s = self.session.lock().await.clone();
        if reject_friend_request(&s, &user_id).await.is_ok() {
            self.emit(CoreEvent::FriendStatusChecked {
                user_id,
                status: FriendshipStatus::Stranger,
                online: None,
            });
            self.emit(CoreEvent::StatusUpdate(format!(
                "Đã từ chối lời mời kết bạn của {user_name}."
            )));
        }
    }

    pub async fn undo_friend(&self, user_id: String, user_name: String) {
        let s = self.session.lock().await.clone();
        if undo_friend_request(&s, &user_id).await.is_ok() {
            self.emit(CoreEvent::FriendStatusChecked {
                user_id,
                status: FriendshipStatus::Stranger,
                online: None,
            });
            self.emit(CoreEvent::StatusUpdate(format!(
                "Đã hủy lời mời kết bạn tới {user_name}."
            )));
        }
    }

    pub async fn add_friend(&self, user_id: String, user_name: String) {
        let s = self.session.lock().await.clone();
        if accept_friend_request(&s, &user_id).await.is_ok() {
            self.emit(CoreEvent::FriendStatusChecked {
                user_id,
                status: FriendshipStatus::Friend,
                online: None,
            });
            self.emit(CoreEvent::StatusUpdate(format!(
                "Đã đồng ý kết bạn với {user_name}!"
            )));
            return;
        }
        match send_friend_request(&s, &user_id, "Xin chào, kết bạn với mình nhé!").await {
            Ok(_) => {
                self.emit(CoreEvent::FriendStatusChecked {
                    user_id: user_id.clone(),
                    status: FriendshipStatus::OutgoingPending,
                    online: None,
                });
                self.emit(CoreEvent::StatusUpdate(format!(
                    "Đã gửi lời mời kết bạn tới {user_name} ({user_id})."
                )));
            }
            Err(e) => {
                self.emit(CoreEvent::StatusUpdate(format!(
                    "Không thể gửi lời mời kết bạn tới {user_name}: {e}"
                )));
            }
        }
    }

    pub async fn lookup_phone(&self, phone: String) {
        let s = self.session.lock().await.clone();
        match find_user_by_phone(&s, &phone).await {
            Ok(profile) => self.emit(CoreEvent::PhoneLookupSuccess(profile)),
            Err(e) => {
                crate::logger::log_error("core::lookup_phone", &e.to_string());
                self.emit(CoreEvent::PhoneLookupFailed(e.to_string()));
            }
        }
    }

    // ------------------------------------------------------------------------
    // Stickers, Images, Links & Host Services
    // ------------------------------------------------------------------------

    pub async fn fetch_sticker_image(&self, key: String, url: String) {
        let cache_dir = Path::new("stickers_cache");
        let _ = std::fs::create_dir_all(cache_dir);
        let cache_path = cache_dir.join(format!("{key}.png"));

        let (bytes_opt, from_network) = if let Ok(disk_bytes) = std::fs::read(&cache_path) {
            (Some(disk_bytes), false)
        } else if !url.starts_with("http://") && !url.starts_with("https://") {
            (std::fs::read(&url).ok(), false)
        } else {
            let client = { self.session.lock().await.clone().http };
            let fetch_url = jxl_to_jpg_url(&url);
            let body = match client.get(&fetch_url).send().await {
                Ok(resp) if resp.status().is_success() => resp.bytes().await.ok(),
                _ => None,
            };
            (body.map(|b| b.to_vec()), true)
        };
        let Some(bytes) = bytes_opt else {
            return;
        };

        let sink = Arc::clone(&self.sink);
        let _ = tokio::task::spawn_blocking(move || {
            let Ok(mut dyn_img) = image::load_from_memory(&bytes) else {
                if !from_network {
                    let _ = std::fs::remove_file(&cache_path);
                }
                return;
            };
            if from_network {
                let _ = std::fs::write(&cache_path, &bytes);
            }
            if !key.starts_with("ava_")
                && !key.starts_with("hd_")
                && (dyn_img.width() > 720 || dyn_img.height() > 720)
            {
                dyn_img = dyn_img.thumbnail(720, 720);
            }
            let mut rgba = dyn_img.to_rgba8();
            if key.starts_with("ava_") {
                let side = rgba.width().min(rgba.height()).clamp(32, 160);
                let x0 = (rgba.width() - side) / 2;
                let y0 = (rgba.height() - side) / 2;
                let cropped = image::imageops::crop_imm(&rgba, x0, y0, side, side).to_image();
                rgba = if side > 128 {
                    image::imageops::resize(
                        &cropped,
                        128,
                        128,
                        image::imageops::FilterType::Triangle,
                    )
                } else {
                    cropped
                };
                let w = rgba.width() as f32;
                let h = rgba.height() as f32;
                let cx = (w - 1.0) * 0.5;
                let cy = (h - 1.0) * 0.5;
                let r = w.min(h) * 0.5;
                for (x, y, px) in rgba.enumerate_pixels_mut() {
                    let dx = x as f32 - cx;
                    let dy = y as f32 - cy;
                    let dist = (dx * dx + dy * dy).sqrt();
                    let cov = (r + 0.5 - dist).clamp(0.0, 1.0);
                    px.0[3] = ((px.0[3] as f32) * cov).round() as u8;
                }
            }
            let width = rgba.width();
            let height = rgba.height();
            let pixels = rgba.into_raw();
            sink.emit(CoreEvent::StickerImageReady {
                key,
                image: CoreImageData {
                    width,
                    height,
                    rgba: pixels,
                },
            });
        })
        .await;
    }

    pub async fn fetch_sticker_store(&self) {
        let s = self.session.lock().await.clone();
        match fetch_sticker_store_packs(&s).await {
            Ok(packs) => self.emit(CoreEvent::StickerStoreLoaded(packs)),
            Err(e) => {
                crate::logger::log_error("core::fetch_sticker_store", &e.to_string());
                self.emit(CoreEvent::StickerStoreLoaded(Vec::new()));
                self.emit(CoreEvent::StatusUpdate(format!(
                    "Không tải được kho nhãn dán: {e}"
                )));
            }
        }
    }

    pub async fn fetch_sticker_pack(&self, cate_id: u64, name: String) {
        let s = self.session.lock().await.clone();
        match fetch_sticker_pack_items(&s, cate_id, &name).await {
            Ok(items) => self.emit(CoreEvent::StickerPackItemsLoaded {
                cate_id,
                name,
                items,
            }),
            Err(e) => {
                crate::logger::log_error("core::fetch_sticker_pack", &e.to_string());
                self.emit(CoreEvent::StatusUpdate(format!(
                    "Không tải được bộ nhãn \"{name}\": {e}"
                )));
            }
        }
    }

    pub async fn search_stickers(&self, keyword: String) {
        let s = self.session.lock().await.clone();
        match search_stickers_by_keyword(&s, &keyword).await {
            Ok(items) => {
                let missing = self.preload_disk_sticker_thumbnails(&items, 24);
                self.emit(CoreEvent::StickerSearchResults { keyword, items });
                self.prefetch_missing_sticker_thumbnails(&s.http, missing)
                    .await;
            }
            Err(e) => {
                crate::logger::log_error("core::search_stickers", &e.to_string());
                self.emit(CoreEvent::StickerSearchResults {
                    keyword,
                    items: Vec::new(),
                });
            }
        }
    }

    pub async fn suggest_composer_stickers(&self, keyword: String) {
        let s = self.session.lock().await.clone();
        let items = search_stickers_by_keyword(&s, &keyword)
            .await
            .unwrap_or_default();
        let missing = self.preload_disk_sticker_thumbnails(&items, 16);
        self.emit(CoreEvent::ComposerStickerSuggestions { keyword, items });
        self.prefetch_missing_sticker_thumbnails(&s.http, missing)
            .await;
    }

    fn preload_disk_sticker_thumbnails(
        &self,
        items: &[StickerItem],
        limit: usize,
    ) -> Vec<(String, String, PathBuf)> {
        let cache_dir = Path::new("stickers_cache");
        let _ = std::fs::create_dir_all(cache_dir);
        let mut missing = Vec::new();
        for item in items.iter().take(limit) {
            let key = format!("stk_{}_{}", item.cate_id, item.sticker_id);
            let cache_path = cache_dir.join(format!("{key}.png"));
            if let Ok(disk_bytes) = std::fs::read(&cache_path) {
                if let Ok(dyn_img) = image::load_from_memory(&disk_bytes) {
                    let rgba = dyn_img.to_rgba8();
                    let width = rgba.width();
                    let height = rgba.height();
                    self.emit(CoreEvent::StickerImageReady {
                        key,
                        image: CoreImageData {
                            width,
                            height,
                            rgba: rgba.into_raw(),
                        },
                    });
                    continue;
                } else {
                    let _ = std::fs::remove_file(&cache_path);
                }
            }
            let url = if item.cate_id == 0 && (1..=30).contains(&item.sticker_id) {
                format!(
                    "https://stc-chat.zdn.vn/images/stickers/default/thumb/{}.png",
                    item.sticker_id
                )
            } else {
                format!(
                    "https://api.zaloapp.com/api/emoticon/sticker/webpc?eid={}&size=130",
                    item.sticker_id
                )
            };
            missing.push((key, url, cache_path));
        }
        missing
    }

    async fn prefetch_missing_sticker_thumbnails(
        &self,
        http: &reqwest::Client,
        missing: Vec<(String, String, PathBuf)>,
    ) {
        if missing.is_empty() {
            return;
        }
        let futs = missing.into_iter().map(|(key, url, cache_path)| {
            let client = http.clone();
            let sink = Arc::clone(&self.sink);
            async move {
                if let Ok(resp) = client.get(&url).send().await {
                    if resp.status().is_success() {
                        if let Ok(b) = resp.bytes().await {
                            let vec = b.to_vec();
                            if let Ok(dyn_img) = image::load_from_memory(&vec) {
                                let _ = std::fs::write(&cache_path, &vec);
                                let rgba = dyn_img.to_rgba8();
                                let width = rgba.width();
                                let height = rgba.height();
                                sink.emit(CoreEvent::StickerImageReady {
                                    key,
                                    image: CoreImageData {
                                        width,
                                        height,
                                        rgba: rgba.into_raw(),
                                    },
                                });
                            }
                        }
                    }
                }
            }
        });
        futures_util::future::join_all(futs).await;
    }

    pub async fn parse_link_preview(&self, url: &str) {
        let s = self.session.lock().await.clone();
        if let Ok(meta) = parse_link_metadata(&s, url).await {
            self.emit(CoreEvent::LinkPreviewReady(meta));
        }
    }

    pub async fn read_notification_auth(&self, request_id: u64, request: bool) {
        let result = if request {
            request_if_user_action(true).await
        } else {
            notif_status().await
        };
        self.emit(CoreEvent::NotificationAuthorization { request_id, result });
    }

    pub async fn check_for_updates(&self, silent: bool) {
        let res = fetch_github_latest_release().await;
        self.emit(CoreEvent::UpdateCheckFinished {
            result: res,
            silent,
        });
    }

    pub async fn download_and_apply_update(
        &self,
        download_url: String,
        asset_name: String,
        target_version: String,
        expected_sha256: Option<String>,
    ) {
        self.emit(CoreEvent::UpdateDownloadStatus {
            downloading: true,
            message: format!("Đang tải bản cập nhật v{target_version}…"),
        });
        let sink = Arc::clone(&self.sink);
        match apply_github_release_update(
            &download_url,
            &asset_name,
            &target_version,
            expected_sha256.as_deref(),
            |downloaded, total| {
                sink.emit(CoreEvent::UpdateDownloadProgress { downloaded, total });
            },
        )
        .await
        {
            Ok(restarting) => {
                if restarting {
                    self.emit(CoreEvent::UpdateAppliedAndRestarting(target_version));
                } else {
                    self.emit(CoreEvent::UpdateDownloadStatus {
                        downloading: false,
                        message: format!(
                            "Đã tải và mở bộ cài đặt {asset_name} (v{target_version})"
                        ),
                    });
                }
            }
            Err(e) => {
                self.emit(CoreEvent::UpdateDownloadStatus {
                    downloading: false,
                    message: format!("Lỗi cập nhật: {e}"),
                });
            }
        }
    }

    pub async fn set_proxy(&self, proxy_url: &str) {
        let res = {
            let mut s = self.session.lock().await;
            let r = s.set_proxy(Some(proxy_url)).map_err(|e| e.to_string());
            if r.is_ok() {
                let _ = s.save_to_file("session.json");
            }
            r
        };
        self.emit(CoreEvent::ProxyUpdated(res));
    }

    pub async fn download_file(&self, url: String, file_name: String, download_dir: String) {
        let client = { self.session.lock().await.clone().http };
        let res =
            download_attachment_with_original_name(&client, &url, &file_name, &download_dir).await;
        self.emit(CoreEvent::FileDownloaded(res));
    }
}

fn merge_thread_headers(list: &mut Vec<Thread>, incoming: Vec<Thread>, force_group: bool) {
    for item in incoming {
        if let Some(ex) = list.iter_mut().find(|t| t.id == item.id) {
            if !item.name.is_empty() {
                ex.name = item.name;
            }
            if should_replace_avatar(ex.avatar.as_deref(), item.avatar.as_deref()) {
                ex.avatar = item.avatar;
            }
            ex.is_group = if force_group {
                true
            } else {
                ex.is_group || item.is_group
            };
        } else {
            list.push(item);
        }
    }
}

async fn download_attachment_with_original_name(
    client: &reqwest::Client,
    url: &str,
    file_name: &str,
    download_dir: &str,
) -> std::result::Result<(String, String), String> {
    let clean_name = crate::protocol::messaging::attachments::resolve_attachment_file_name(
        Some(file_name),
        file_name,
        url,
    );
    let resp = client
        .get(url)
        .send()
        .await
        .map_err(|e| format!("Không thể tải tập tin: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("Máy chủ trả về mã HTTP {}", resp.status()));
    }
    let bytes = resp
        .bytes()
        .await
        .map_err(|e| format!("Lỗi đọc dữ liệu tập tin: {e}"))?;

    let target_dir = if !download_dir.trim().is_empty() {
        PathBuf::from(download_dir.trim())
    } else {
        std::env::var("USERPROFILE")
            .ok()
            .map(|p| {
                PathBuf::from(p)
                    .join("Downloads")
                    .join("Zalo Received Files")
            })
            .or_else(|| {
                std::env::var("HOME").ok().map(|h| {
                    PathBuf::from(h)
                        .join("Downloads")
                        .join("Zalo Received Files")
                })
            })
            .unwrap_or_else(|| PathBuf::from("Downloads"))
    };

    tokio::task::spawn_blocking(move || {
        std::fs::create_dir_all(&target_dir)
            .map_err(|e| format!("Không thể tạo thư mục lưu file: {e}"))?;

        let (stem, ext) = match clean_name.rfind('.') {
            Some(dot_idx) if dot_idx > 0 && dot_idx + 1 < clean_name.len() => {
                (&clean_name[..dot_idx], Some(&clean_name[dot_idx + 1..]))
            }
            _ => (clean_name.as_str(), None),
        };

        let mut dest_path = target_dir.join(&clean_name);
        let mut final_name = clean_name.clone();
        let mut suffix_idx = 1_u32;
        while dest_path.exists() {
            final_name = match ext {
                Some(e) => format!("{stem} ({suffix_idx}).{e}"),
                None => format!("{stem} ({suffix_idx})"),
            };
            dest_path = target_dir.join(&final_name);
            suffix_idx += 1;
        }

        std::fs::write(&dest_path, &bytes).map_err(|e| format!("Lỗi ghi tệp xuống máy: {e}"))?;
        Ok((final_name, dest_path.display().to_string()))
    })
    .await
    .map_err(|e| format!("Lỗi tác vụ lưu file: {e}"))?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn core_engine_runs_ui_free_and_queries_sqlite_store() {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let db_file = std::env::temp_dir().join(format!("alolite-core-test-{nonce}.db"));
        let db_path = db_file.to_string_lossy().into_owned();

        {
            let mut store = MessageStore::open(&db_path).unwrap();
            store
                .upsert_thread(&crate::storage::StoredThread {
                    thread_id: "t_core".into(),
                    name: "Bob".into(),
                    is_group: false,
                    last_ts: 123_456,
                })
                .unwrap();
            store
                .upsert_message(&StoredMessage {
                    msg_id: "m_core_1".into(),
                    thread_id: "t_core".into(),
                    sender_id: "u1".into(),
                    sender_name: "Bob".into(),
                    text: "CoreEngine runs without egui".into(),
                    msg_type: "chat.text".into(),
                    ts: 123_456,
                    is_outgoing: false,
                    raw_json: String::new(),
                })
                .unwrap();
        }

        let session = Session::new(false, false, false).unwrap();
        let engine = CoreEngine::headless(session, &db_path);

        let st = engine.status().await;
        assert!(!st.logged_in);
        assert_eq!(st.threads_in_db, 1);

        let hits = engine.search_messages("CoreEngine", 10).await.unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].msg_id, "m_core_1");

        let msgs = engine.get_messages("t_core", 10).await.unwrap();
        assert_eq!(msgs.len(), 1);

        let _ = std::fs::remove_file(&db_file);
    }
}
