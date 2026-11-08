//! Background loops (WebSocket, periodic polling, update checks), event handling, and auto-reply dispatch.

use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;

use super::CoreEngine;
use crate::clock::unix_now;
use crate::core::events::CoreEvent;
use crate::mcp::{load_autoreply_rules, match_autoreply_rule};
use crate::protocol::auth::login;
use crate::protocol::messaging::attachments::dispatch_file_done_controls;
use crate::protocol::messaging::send::{
    send_group_message_with_client_id, send_message_with_client_id,
};
use crate::protocol::messaging::stickers::fetch_sticker_suggest_keywords;
use crate::protocol::messaging::unread::extract_mute_controls;
use crate::protocol::messaging::Thread;
use crate::protocol::session::Session;
use crate::protocol::social::groups::resolve_thread_names;
use crate::protocol::sync::{
    clean_sync, clear_sync_keys, fetch_mobile_backup_container, fetch_sync_v2_messages,
    fetch_sync_v2_meta, get_sync_public_key, resolve_znoise_ids, set_last_mobile_sync_status,
    set_latest_mobile_temp_key, MobileSyncControlEvent,
};
use crate::protocol::wire::next_client_message_id;
use crate::services::updater::fetch_github_latest_release;
use crate::storage::{
    commit_mobile_backup_to_store, commit_sync_v2_messages_to_store, db_path_for_account,
    extract_conv_archives_from_entries, load_threads_cache, persist_resolved_thread_names,
};

async fn compose_import_mobile_backup(
    s: &Session,
    url: &str,
    encrypted_key: &str,
    raw_uid: Option<i64>,
) -> Result<usize> {
    let downloaded = fetch_mobile_backup_container(s, url, encrypted_key).await?;
    let (conv_archives, all_fids, all_gids) = tokio::task::spawn_blocking(move || {
        extract_conv_archives_from_entries(downloaded.entries, raw_uid)
    })
    .await??;
    let (fid_map, gid_map) = resolve_znoise_ids(s, &all_fids, &all_gids)
        .await
        .unwrap_or_default();
    let uin = s.login.as_ref().map(|l| l.uin.clone()).unwrap_or_default();
    let my_name = s
        .login
        .as_ref()
        .map(|l| l.display_name.clone())
        .unwrap_or_else(|| "Tôi".into());
    let db_path = db_path_for_account(&uin);
    let my_uid = uin.clone();
    let (total_imported, mut hydrated_threads) = tokio::task::spawn_blocking(move || {
        commit_mobile_backup_to_store(
            &db_path,
            &my_uid,
            &my_name,
            conv_archives,
            &fid_map,
            &gid_map,
            raw_uid,
        )
    })
    .await??;
    resolve_thread_names(s, &mut hydrated_threads).await;
    let db_path_names = db_path_for_account(&uin);
    let resolved_snapshot = hydrated_threads;
    let _ = tokio::task::spawn_blocking(move || {
        persist_resolved_thread_names(&db_path_names, &resolved_snapshot)
    })
    .await;
    let _ = clean_sync(s, &downloaded.pub_key_b64).await;
    clear_sync_keys();
    Ok(total_imported)
}

async fn compose_import_sync_v2_messages(s: &Session, url: &str) -> Result<usize> {
    let partitions = fetch_sync_v2_messages(s, url).await?;
    let uin = s.login.as_ref().map(|l| l.uin.clone()).unwrap_or_default();
    let my_name = s
        .login
        .as_ref()
        .map(|l| l.display_name.clone())
        .unwrap_or_else(|| "Tôi".into());
    let db_path = db_path_for_account(&uin);
    let count = tokio::task::spawn_blocking(move || {
        commit_sync_v2_messages_to_store(&db_path, &uin, &my_name, partitions)
    })
    .await??;
    if let Some(pub_k) = get_sync_public_key() {
        let _ = clean_sync(s, &pub_k).await;
    }
    Ok(count)
}

impl CoreEngine {
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

    pub(super) async fn handle_sync_control_event(
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

                    match compose_import_mobile_backup(&s_sync, &url, &encrypted_key, raw_uid).await
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
                        let _ = fetch_sync_v2_meta(&s_sync, &url).await;
                    } else if batch_type == 1 {
                        let note = "Đang tải và giải mã gói tin nhắn từ điện thoại…".to_string();
                        set_last_mobile_sync_status(Some(note.clone()));
                        engine.emit(CoreEvent::StatusUpdate(note));

                        match compose_import_sync_v2_messages(&s_sync, &url).await {
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

    pub(super) async fn dispatch_autoreplies(&self, s: &Session, threads: &[Thread]) {
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
}
