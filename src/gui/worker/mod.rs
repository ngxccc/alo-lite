use super::*;

pub mod auth;
pub mod chat;
pub mod contacts;
pub mod history;
pub mod services;
pub mod stickers;
pub mod threads;
pub mod types;

pub use crate::gui::UpdateCheckResult;
pub use services::fetch_github_latest_release;
pub use types::*;

/// Thread (id, name, is_group) list the poll timer maps incoming messages onto; the GUI
/// republishes it whenever its threads change.
pub(super) static KNOWN_THREADS: std::sync::Mutex<Vec<(String, String, bool)>> =
    std::sync::Mutex::new(Vec::new());

pub(super) static WORKER_TX: std::sync::OnceLock<tokio::sync::mpsc::UnboundedSender<WorkerCmd>> =
    std::sync::OnceLock::new();

pub(super) fn spawn_session_worker(session: Session, gui_tx: GuiTx, egui_ctx: egui::Context) {
    let (cmd_tx, mut cmd_rx) = tokio::sync::mpsc::unbounded_channel::<WorkerCmd>();
    let _ = WORKER_TX.set(cmd_tx);

    let session = std::sync::Arc::new(tokio::sync::Mutex::new(session));
    let poll_state = std::sync::Arc::new(tokio::sync::Mutex::new(
        crate::protocol::messaging::PollState::default(),
    ));
    let ctrl_poll_state = std::sync::Arc::new(tokio::sync::Mutex::new(
        crate::call::CtrlPollState::default(),
    ));

    std::thread::Builder::new()
        .name("zalo-worker".into())
        .spawn(move || {
            // Lightweight 2-thread async runtime to keep memory & CPU footprint minimal
            let rt = tokio::runtime::Builder::new_multi_thread()
                .worker_threads(2)
                .enable_all()
                .build()
                .unwrap();

            rt.block_on(async move {
                {
                    let mut s = session.lock().await;
                    if let Ok(resp) = crate::protocol::auth::login::get_server_info(&s).await {
                        if let Some(d) = resp.get("data") {
                            s.apply_server_info(d);
                        }
                    }
                }
                // Message poll timer. It used to live in the GUI's update(), which forced a full
                // repaint every 1.2 s just to fire it. The PollMessages handler skips while a
                // long-poll is in flight and when logged out.
                tokio::spawn(async {
                    let mut tick = tokio::time::interval(Duration::from_millis(1200));
                    tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
                    loop {
                        tick.tick().await;
                        let known_threads =
                            KNOWN_THREADS.lock().map(|k| k.clone()).unwrap_or_default();
                        if let Some(tx) = WORKER_TX.get() {
                            let _ = tx.send(WorkerCmd::PollMessages { known_threads });
                        }
                    }
                });

                // Scheduled messages worker (#38) + optional HTTP REST API server (#35)
                if let Ok(port_str) = std::env::var("ALOLITE_API_PORT") {
                    if let Ok(port) = port_str.parse::<u16>() {
                        if port > 0 {
                            let s_api = session.clone();
                            tokio::spawn(async move {
                                let _ = crate::api_mcp::run_local_http_server(
                                    s_api,
                                    port,
                                    "zalo_history.db".into(),
                                )
                                .await;
                            });
                        }
                    }
                }
                {
                    let s_sched = session.clone();
                    let gui_tx_sched = gui_tx.clone();
                    let ctx_sched = egui_ctx.clone();
                    tokio::spawn(async move {
                        let mut tick = tokio::time::interval(Duration::from_secs(2));
                        tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
                        loop {
                            tick.tick().await;
                            let now_ts = crate::clock::unix_now();
                            let due = crate::scheduler::claim_due_scheduled_messages(
                                "zalo_history.db",
                                now_ts,
                            )
                            .unwrap_or_default();
                            if due.is_empty() {
                                continue;
                            }
                            let s = s_sched.lock().await.clone();
                            if s.login.is_none() {
                                for item in due {
                                    let _ = crate::scheduler::mark_scheduled_failed(
                                        "zalo_history.db",
                                        item.id,
                                        "Chưa đăng nhập phiên Zalo",
                                    );
                                }
                                let _ = gui_tx_sched.send(GuiEvent::ScheduledListUpdated);
                                ctx_sched.request_repaint();
                                continue;
                            }
                            for item in due {
                                let client_id = crate::wire::next_client_message_id();
                                let send_res = if item.is_group {
                                    crate::protocol::messaging::send::send_group_message_with_client_id(&s, &item.thread_id, &item.text, client_id)
                                        .await
                                } else {
                                    crate::protocol::messaging::send::send_message_with_client_id(&s, &item.thread_id, &item.text, client_id).await
                                };
                                match send_res {
                                    Ok(resp) => {
                                        if let Some(code) = resp_error_code(&resp) {
                                            let _ = crate::scheduler::mark_scheduled_failed(
                                                "zalo_history.db",
                                                item.id,
                                                &format!("Mã lỗi Zalo {code}"),
                                            );
                                        } else {
                                            let msg_id = extract_msg_id(&resp);
                                            let _ = crate::scheduler::mark_scheduled_sent(
                                                "zalo_history.db",
                                                item.id,
                                            );
                                            let _ = gui_tx_sched.send(GuiEvent::MessageSent {
                                                client_message_id: client_id.to_string(),
                                                thread_id: item.thread_id.clone(),
                                                text: item.text.clone(),
                                                msg_id,
                                            });
                                        }
                                    }
                                    Err(e) => {
                                        let _ = crate::scheduler::mark_scheduled_failed(
                                            "zalo_history.db",
                                            item.id,
                                            &e.to_string(),
                                        );
                                    }
                                }
                            }
                            let _ = gui_tx_sched.send(GuiEvent::ScheduledListUpdated);
                            ctx_sched.request_repaint();
                        }
                    });
                }
                {
                    let s_kw = session.clone();
                    let gui_tx_kw = gui_tx.clone();
                    let ctx_kw = egui_ctx.clone();
                    tokio::spawn(async move {
                        let s = s_kw.lock().await.clone();
                        if s.login.is_some() {
                            if let Some(tx) = WORKER_TX.get() {
                                let _ = tx.send(WorkerCmd::RefreshThreads);
                            }
                            if let Ok(kws) = crate::protocol::messaging::stickers::fetch_sticker_suggest_keywords(&s).await {
                                if !kws.is_empty() {
                                    let _ = gui_tx_kw
                                        .send(GuiEvent::StickerSuggestKeywordsLoaded(kws));
                                    ctx_kw.request_repaint();
                                }
                            }
                        }
                    });
                }

                let call_bridge = std::sync::Arc::new(crate::call::CallBridge::spawn(
                    session.clone(),
                    gui_tx.clone(),
                    egui_ctx.clone(),
                ));

                // 1) Real-time `zpw_ws` WebSocket loop for instant chat messages + VoIP events
                {
                    let s_ws = session.clone();
                    let cb = call_bridge.clone();
                    let gui_tx_ws = gui_tx.clone();
                    let ctx_ws = egui_ctx.clone();
                    tokio::spawn(async move {
                        let mut ws_rx: Option<
                            tokio::sync::mpsc::UnboundedReceiver<crate::socket::WsEvent>,
                        > = None;
                        let mut ws_sek = String::new();
                        loop {
                            let s_snap = s_ws.lock().await.clone();
                            if s_snap.login.is_some() {
                                let cur_sek = s_snap.zpw_sek.clone().unwrap_or_default();
                                if !cur_sek.is_empty() && (ws_rx.is_none() || ws_sek != cur_sek) {
                                    ws_sek = cur_sek;
                                    ws_rx = Some(crate::socket::spawn(s_snap.clone()));
                                }
                                if let Some(rx) = ws_rx.as_mut() {
                                    match tokio::time::timeout(
                                        Duration::from_millis(1500),
                                        rx.recv(),
                                    )
                                    .await
                                    {
                                        Ok(Some(ev)) => {
                                            let data_node = ev
                                                .payload
                                                .get("data")
                                                .unwrap_or(&ev.payload);
                                            for voip_ev in
                                                crate::call::extract_voip_controls(data_node)
                                            {
                                                cb.handle_control_event(voip_ev);
                                            }
                                            crate::protocol::messaging::attachments::dispatch_file_done_controls(data_node);
                                            let mute_evs = crate::protocol::messaging::unread::extract_mute_controls(
                                                data_node,
                                                crate::clock::unix_now(),
                                            );
                                            if !mute_evs.is_empty() {
                                                let _ = gui_tx_ws
                                                    .send(GuiEvent::MuteEventsReceived(mute_evs));
                                                ctx_ws.request_repaint();
                                            }
                                            for sync_ev in
                                                crate::sync::extract_mobile_sync_controls(data_node)
                                            {
                                                match sync_ev {
                                                    crate::sync::MobileSyncControlEvent::TransferAfterLogin {
                                                        temp_key,
                                                    } => {
                                                        crate::sync::set_latest_mobile_temp_key(&temp_key);
                                                        let known = crate::store::load_threads_cache();
                                                        if let Some(wtx) = WORKER_TX.get() {
                                                            let _ = wtx.send(
                                                                WorkerCmd::SyncPhoneMessages {
                                                                    threads: known,
                                                                    send_phone_prompt: false,
                                                                },
                                                            );
                                                        }
                                                    }
                                                    crate::sync::MobileSyncControlEvent::WaitingConfirmOnPhone { .. } => {
                                                        let note = "Đang chờ bạn bấm 'Đồng bộ' trên màn hình Zalo điện thoại…".to_string();
                                                        crate::sync::set_last_mobile_sync_status(Some(note.clone()));
                                                        let _ = gui_tx_ws.send(GuiEvent::StatusUpdate(note));
                                                        ctx_ws.request_repaint();
                                                    }
                                                    crate::sync::MobileSyncControlEvent::MobileBusyRestoring => {
                                                        let note = "Điện thoại đang bận sao lưu/khôi phục phiên trước — hãy mở Zalo trên điện thoại xác nhận hoặc thử lại sau ít phút".to_string();
                                                        crate::sync::set_last_mobile_sync_status(Some(note.clone()));
                                                        let _ = gui_tx_ws.send(GuiEvent::StatusUpdate(note));
                                                        ctx_ws.request_repaint();
                                                    }
                                                    crate::sync::MobileSyncControlEvent::UserConfirmed => {
                                                        let note = "Điện thoại đã xác nhận đồng bộ — đang nén và đóng gói tin nhắn…".to_string();
                                                        crate::sync::set_last_mobile_sync_status(Some(note.clone()));
                                                        let _ = gui_tx_ws.send(GuiEvent::StatusUpdate(note));
                                                        ctx_ws.request_repaint();
                                                    }
                                                    crate::sync::MobileSyncControlEvent::UserRejected => {
                                                        let note = "Bạn đã từ chối yêu cầu đồng bộ tin nhắn trên điện thoại".to_string();
                                                        crate::sync::set_last_mobile_sync_status(Some(note.clone()));
                                                        let _ = gui_tx_ws.send(GuiEvent::StatusUpdate(note));
                                                        ctx_ws.request_repaint();
                                                    }
                                                    crate::sync::MobileSyncControlEvent::BackupReady {
                                                        url,
                                                        file_name,
                                                        encrypted_key,
                                                        raw_uid,
                                                        ..
                                                    } => {
                                                        let s_sync = s_snap.clone();
                                                        let gui_tx_sync = gui_tx_ws.clone();
                                                        let ctx_sync = ctx_ws.clone();
                                                        tokio::spawn(async move {
                                                            let status_dl = format!("Đang tải và giải mã gói đồng bộ tin nhắn ({file_name})…");
                                                            crate::sync::set_last_mobile_sync_status(Some(status_dl.clone()));
                                                            let _ = gui_tx_sync.send(GuiEvent::StatusUpdate(status_dl));
                                                            ctx_sync.request_repaint();

                                                            match crate::store::import_mobile_backup_container_with_uid(&s_sync, &url, &encrypted_key, raw_uid).await {
                                                                Ok(count) => {
                                                                    let note = format!("Đã giải mã & đồng bộ thành công {count} tin nhắn từ điện thoại vào ứng dụng!");
                                                                    crate::sync::set_last_mobile_sync_status(Some(note.clone()));
                                                                    let _ = gui_tx_sync.send(GuiEvent::StatusUpdate(note));
                                                                    let updated = crate::store::load_threads_cache();
                                                                    let _ = gui_tx_sync.send(GuiEvent::ThreadsUpdated(updated));
                                                                }
                                                                Err(e) => {
                                                                    let err_note = format!("Không thể giải mã gói đồng bộ ({file_name}): {e}");
                                                                    crate::sync::set_last_mobile_sync_status(Some(err_note.clone()));
                                                                    let _ = gui_tx_sync.send(GuiEvent::StatusUpdate(err_note));
                                                                }
                                                            }
                                                            ctx_sync.request_repaint();
                                                        });
                                                    }
                                                    crate::sync::MobileSyncControlEvent::SyncV2Batch {
                                                        url,
                                                        sync_id,
                                                        batch_type,
                                                        is_last: _,
                                                    } => {
                                                        let s_sync = s_snap.clone();
                                                        let gui_tx_sync = gui_tx_ws.clone();
                                                        let ctx_sync = ctx_ws.clone();
                                                        tokio::spawn(async move {
                                                            if batch_type == 2 {
                                                                let note = "Đang giải mã thông tin phiên đồng bộ từ điện thoại…".to_string();
                                                                crate::sync::set_last_mobile_sync_status(Some(note.clone()));
                                                                let _ = gui_tx_sync.send(GuiEvent::StatusUpdate(note));
                                                                ctx_sync.request_repaint();
                                                                let _ = crate::store::import_sync_v2_meta(&s_sync, &url).await;
                                                            } else if batch_type == 1 {
                                                                let note = "Đang tải và giải mã gói tin nhắn từ điện thoại…".to_string();
                                                                crate::sync::set_last_mobile_sync_status(Some(note.clone()));
                                                                let _ = gui_tx_sync.send(GuiEvent::StatusUpdate(note));
                                                                ctx_sync.request_repaint();

                                                                match crate::store::import_sync_v2_messages(&s_sync, &url).await {
                                                                    Ok(count) => {
                                                                        let fin = format!("Đã giải mã & đồng bộ thành công {count} tin nhắn từ điện thoại vào ứng dụng!");
                                                                        crate::sync::set_last_mobile_sync_status(Some(fin.clone()));
                                                                        let _ = gui_tx_sync.send(GuiEvent::StatusUpdate(fin));
                                                                        let updated = crate::store::load_threads_cache();
                                                                        let _ = gui_tx_sync.send(GuiEvent::ThreadsUpdated(updated));
                                                                    }
                                                                    Err(e) => {
                                                                        let err = format!("Lỗi giải mã tin nhắn Sync v2 ({sync_id}): {e}");
                                                                        crate::sync::set_last_mobile_sync_status(Some(err.clone()));
                                                                        let _ = gui_tx_sync.send(GuiEvent::StatusUpdate(err));
                                                                    }
                                                                }
                                                                ctx_sync.request_repaint();
                                                            }
                                                        });
                                                    }
                                                }
                                            }
                                            if ev.is_chat_ish() {
                                                let mut ws_threads = Vec::new();
                                                if let Some(li) = s_snap.login.as_ref() {
                                                    if !li.send2me_id.is_empty() {
                                                        ws_threads.push(Thread {
                                                            presentation: Default::default(),
                                                            unread: Default::default(),
                                                            id: li.send2me_id.clone(),
                                                            name: "Cloud của tôi (Lưu trữ)"
                                                                .into(),
                                                            preview: String::new(),
                                                            time: String::new(),
                                                            avatar: None,
                                                            is_group: false,
                                                            messages: Vec::new(),
                                                        });
                                                    }
                                                }
                                                crate::protocol::messaging::polling::merge_previews(
                                                    &mut ws_threads,
                                                    &ev.payload,
                                                );
                                                ws_threads.retain(|t| !t.messages.is_empty());
                                                if !ws_threads.is_empty() {
                                                    crate::protocol::social::groups::resolve_thread_names(
                                                        &s_snap,
                                                        &mut ws_threads,
                                                    )
                                                    .await;
                                                    let _ = gui_tx_ws.send(
                                                        GuiEvent::ThreadsUpdated(ws_threads),
                                                    );
                                                    ctx_ws.request_repaint();
                                                }
                                            }
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

                // 2) Background HTTP long-poll fallback for VoIP & mute control events (`lp_control`)
                {
                    let s_ctrl = session.clone();
                    let cps = ctrl_poll_state.clone();
                    let cb = call_bridge.clone();
                    let gui_tx_ctrl = gui_tx.clone();
                    let ctx_ctrl = egui_ctx.clone();
                    tokio::spawn(async move {
                        loop {
                            let s_snap = s_ctrl.lock().await.clone();
                            if s_snap.login.is_some() {
                                let ctrl_res = {
                                    let mut st = cps.lock().await;
                                    crate::call::poll_control_events(&s_snap, &mut st).await
                                };
                                if let Ok((events, mute_evs)) = ctrl_res {
                                    for ev in events {
                                        cb.handle_control_event(ev);
                                    }
                                    if !mute_evs.is_empty() {
                                        let _ = gui_tx_ctrl
                                            .send(GuiEvent::MuteEventsReceived(mute_evs));
                                        ctx_ctrl.request_repaint();
                                    }
                                } else {
                                    tokio::time::sleep(Duration::from_secs(5)).await;
                                }
                            } else {
                                tokio::time::sleep(Duration::from_secs(3)).await;
                            }
                        }
                    });
                }

                // 3) Periodic GitHub Releases API update checker (polls every 30 minutes)
                {
                    let gui_tx_upd = gui_tx.clone();
                    let ctx_upd = egui_ctx.clone();
                    tokio::spawn(async move {
                        let mut ticker = tokio::time::interval(Duration::from_secs(30 * 60));
                        // First tick fires immediately; skip it because ZaloGuiApp::new already
                        // dispatches an initial CheckForUpdates on startup.
                        ticker.tick().await;
                        loop {
                            ticker.tick().await;
                            let res = fetch_github_latest_release().await;
                            let _ = gui_tx_upd.send(GuiEvent::UpdateCheckFinished {
                                result: res,
                                silent: true,
                            });
                            ctx_upd.request_repaint();
                        }
                    });
                }

                let mut current_qr_task: Option<tokio::task::JoinHandle<()>> = None;

                while let Some(cmd) = cmd_rx.recv().await {
                    match cmd {
                        WorkerCmd::RequestQr
                        | WorkerCmd::LoginPassword { .. }
                        | WorkerCmd::Logout
                        | WorkerCmd::SwitchAccount { .. }
                        | WorkerCmd::WipeLocalData => {
                            auth::handle_auth_cmd(cmd, &session, &poll_state, &gui_tx, &egui_ctx, &mut current_qr_task).await;
                        }
                        WorkerCmd::SendMessage { .. }
                        | WorkerCmd::SendQuote { .. }
                        | WorkerCmd::Forward { .. }
                        | WorkerCmd::React { .. }
                        | WorkerCmd::DeleteForMe { .. }
                        | WorkerCmd::RecallMessage { .. }
                        | WorkerCmd::SendSticker { .. }
                        | WorkerCmd::PickAndSendAttachment { .. }
                        | WorkerCmd::SendTyping { .. }
                        | WorkerCmd::SendSeen { .. } => {
                            chat::handle_chat_cmd(cmd, &session, &gui_tx, &egui_ctx);
                        }
                        WorkerCmd::PersistThreads(..)
                        | WorkerCmd::RefreshThreads
                        | WorkerCmd::RefreshThreadsManual
                        | WorkerCmd::PollMessages { .. }
                        | WorkerCmd::PinThread { .. }
                        | WorkerCmd::MarkThreadUnread { .. }
                        | WorkerCmd::MuteThread { .. }
                        | WorkerCmd::HideThread { .. }
                        | WorkerCmd::DeleteThread { .. } => {
                            threads::handle_threads_cmd(cmd, &session, &poll_state, &gui_tx, &egui_ctx);
                        }
                        WorkerCmd::FetchStickerImage { .. }
                        | WorkerCmd::FetchStickerStore
                        | WorkerCmd::FetchStickerPack { .. }
                        | WorkerCmd::SearchStickers { .. }
                        | WorkerCmd::SuggestComposerStickers { .. } => {
                            stickers::handle_stickers_cmd(cmd, &session, &gui_tx, &egui_ctx);
                        }
                        WorkerCmd::CheckFriendStatus { .. }
                        | WorkerCmd::AcceptFriend { .. }
                        | WorkerCmd::RejectFriend { .. }
                        | WorkerCmd::UndoFriend { .. }
                        | WorkerCmd::AddFriend { .. }
                        | WorkerCmd::LookupPhone { .. }
                        | WorkerCmd::FetchSocialProfile { .. }
                        | WorkerCmd::FetchMySocialAvatars { .. } => {
                            contacts::handle_contacts_cmd(cmd, &session, &gui_tx, &egui_ctx);
                        }
                        WorkerCmd::SyncHistory { .. }
                        | WorkerCmd::LoadGroupHistory { .. }
                        | WorkerCmd::SyncPhoneMessages { .. } => {
                            history::handle_history_cmd(cmd, &session, &poll_state, &gui_tx, &egui_ctx);
                        }
                        WorkerCmd::ReadNotificationAuthorization { .. }
                        | WorkerCmd::RequestNotificationAuthorization { .. }
                        | WorkerCmd::CheckForUpdates { .. }
                        | WorkerCmd::DownloadAndApplyUpdate { .. }
                        | WorkerCmd::StartCall { .. }
                        | WorkerCmd::AnswerCall { .. }
                        | WorkerCmd::SetCallMediaState { .. }
                        | WorkerCmd::EndCall
                        | WorkerCmd::FetchTimelineFeed { .. }
                        | WorkerCmd::SetProxy { .. }
                        | WorkerCmd::DownloadFile { .. }
                        | WorkerCmd::ParseLinkPreview { .. } => {
                            services::handle_services_cmd(cmd, &session, &call_bridge, &gui_tx, &egui_ctx);
                        }
                    }
                }
            });
        })
        .expect("Failed to spawn zalo-worker thread");
}
