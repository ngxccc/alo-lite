use super::*;

pub(super) fn handle_threads_cmd(
    cmd: WorkerCmd,
    session: &std::sync::Arc<tokio::sync::Mutex<Session>>,
    poll_state: &std::sync::Arc<tokio::sync::Mutex<crate::protocol::messaging::PollState>>,
    gui_tx: &GuiTx,
    egui_ctx: &egui::Context,
) {
    match cmd {
        WorkerCmd::PersistThreads(threads_snapshot) => {
            tokio::task::spawn_blocking(move || {
                if let Ok(mut store) = crate::store::MessageStore::open("zalo_history.db") {
                    let _ = crate::store::persist_threads(&mut store, &threads_snapshot);
                }
                crate::store::save_threads_cache(&threads_snapshot);
            });
        }
        WorkerCmd::PinThread {
            thread_id,
            is_group,
            pin,
        } => {
            let session_clone = session.clone();
            tokio::spawn(async move {
                let s = session_clone.lock().await.clone();
                let _ = crate::protocol::messaging::threads::update_pinned_conversation(
                    &s, &thread_id, is_group, pin,
                )
                .await;
            });
        }
        WorkerCmd::MarkThreadUnread {
            thread_id,
            is_group,
            unread,
        } => {
            let session_clone = session.clone();
            tokio::spawn(async move {
                let s = session_clone.lock().await.clone();
                let _ = crate::protocol::messaging::unread::set_unread_mark(
                    &s, &thread_id, is_group, unread,
                )
                .await;
            });
        }
        WorkerCmd::MuteThread {
            thread_id,
            is_group,
            duration_sec,
            mute,
        } => {
            let client = crate::protocol::ZaloClient::new(session.clone());
            let session_clone = session.clone();
            let gui_tx_clone = gui_tx.clone();
            let ctx_clone = egui_ctx.clone();
            tokio::spawn(async move {
                let _ = client
                    .set_mute_conversation(&thread_id, is_group, duration_sec, mute)
                    .await;
                let s = session_clone.lock().await.clone();
                if let Ok(mute_resp) =
                    crate::protocol::messaging::unread::get_mute_conversations(&s).await
                {
                    if let Some(mute_map) = crate::protocol::messaging::unread::extract_mute_map(
                        &mute_resp,
                        crate::clock::unix_now(),
                    ) {
                        let _ = gui_tx_clone.send(GuiEvent::MuteSnapshotSynced(mute_map));
                        ctx_clone.request_repaint();
                    }
                }
            });
        }
        WorkerCmd::HideThread {
            thread_id,
            is_group,
            hide,
        } => {
            let session_clone = session.clone();
            tokio::spawn(async move {
                let s = session_clone.lock().await.clone();
                let _ = crate::protocol::messaging::threads::set_hidden_conversation(
                    &s, &thread_id, is_group, hide,
                )
                .await;
            });
        }
        WorkerCmd::DeleteThread {
            thread_id,
            is_group,
            last_cli_msg_id,
        } => {
            let session_clone = session.clone();
            tokio::spawn(async move {
                let s = session_clone.lock().await.clone();
                let _ = crate::protocol::messaging::threads::delete_conversation(
                    &s,
                    &thread_id,
                    is_group,
                    &last_cli_msg_id,
                )
                .await;
            });
        }
        ref cmd @ (WorkerCmd::RefreshThreads | WorkerCmd::RefreshThreadsManual) => {
            let manual = matches!(cmd, WorkerCmd::RefreshThreadsManual);
            let session_clone = session.clone();
            let poll_clone = poll_state.clone();
            let gui_tx_clone = gui_tx.clone();
            let ctx_clone = egui_ctx.clone();
            tokio::spawn(async move {
                let mut s = session_clone.lock().await.clone();
                if s.login.is_some() {
                    match s.session_status().await {
                        crate::session::SessionStatus::Valid => {}
                        crate::session::SessionStatus::Expired => {
                            // Keep `session.json`: the old credentials stay
                            // readable until a new login replaces the file (#21).
                            {
                                let mut sl = session_clone.lock().await;
                                sl.mark_expired();
                                let _ = sl.save_to_file("session.json");
                                sl.logout();
                            }
                            let _ = gui_tx_clone.send(GuiEvent::SessionExpired(
                                "Phiên đăng nhập (zpw_sek) đã hết hạn trên máy chủ Zalo".into(),
                            ));
                            ctx_clone.request_repaint();
                            if let Some(tx) = WORKER_TX.get() {
                                let _ = tx.send(WorkerCmd::RequestQr);
                            }
                            return;
                        }
                        crate::session::SessionStatus::Unreachable => {
                            // Offline: keep the session, say so, and let the
                            // next poll retry.
                            crate::logger::log_error(
                                "gui::refresh_threads",
                                "không kết nối được máy chủ Zalo",
                            );
                            let _ = gui_tx_clone.send(GuiEvent::SessionUnreachable(
                                "Không kết nối được máy chủ Zalo".into(),
                            ));
                            ctx_clone.request_repaint();
                            return;
                        }
                    }
                    if let Some((uid, name)) = s.sync_profile_me().await {
                        {
                            let mut sl = session_clone.lock().await;
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
                        let _ = gui_tx_clone.send(GuiEvent::LoginSuccess {
                            name,
                            uin: uid,
                            avatar: u_ava,
                        });
                    }
                    let _ = gui_tx_clone.send(GuiEvent::StatusUpdate(
                        "⏳ Đang làm mới danh bạ & nhóm từ máy chủ Zalo (1/3)…".into(),
                    ));
                    ctx_clone.request_repaint();
                    let mut list = crate::store::load_threads_cache();
                    let mut errors = Vec::new();
                    let client = crate::protocol::ZaloClient::new(session_clone.clone());
                    match client.get_friends().await {
                        Ok(friends) => {
                            for ft in friends {
                                if let Some(ex) = list.iter_mut().find(|t| t.id == ft.id) {
                                    if !ft.name.is_empty() {
                                        ex.name = ft.name;
                                    }
                                    if crate::protocol::messaging::threads::should_replace_avatar(
                                        ex.avatar.as_deref(),
                                        ft.avatar.as_deref(),
                                    ) {
                                        ex.avatar = ft.avatar;
                                    }
                                    ex.is_group = ex.is_group || ft.is_group;
                                } else {
                                    list.push(ft);
                                }
                            }
                        }
                        Err(e) => {
                            crate::logger::log_error("gui::refresh_friends", &e.to_string());
                            errors.push(format!("Danh bạ: {e}"));
                        }
                    }
                    let _ = gui_tx_clone.send(GuiEvent::StatusUpdate(
                        "⏳ Đang tải danh sách nhóm Zalo (2/3)…".into(),
                    ));
                    ctx_clone.request_repaint();
                    match client.get_groups().await {
                        Ok(groups) => {
                            for gt in groups {
                                if let Some(ex) = list.iter_mut().find(|t| t.id == gt.id) {
                                    if !gt.name.is_empty() {
                                        ex.name = gt.name;
                                    }
                                    if crate::protocol::messaging::threads::should_replace_avatar(
                                        ex.avatar.as_deref(),
                                        gt.avatar.as_deref(),
                                    ) {
                                        ex.avatar = gt.avatar;
                                    }
                                    ex.is_group = true;
                                } else {
                                    list.push(gt);
                                }
                            }
                        }
                        Err(e) => {
                            crate::logger::log_error("gui::refresh_groups", &e.to_string());
                            errors.push(format!("Nhóm: {e}"));
                        }
                    }
                    let uin = s.login.as_ref().map(|l| l.uin.as_str()).unwrap_or_default();
                    let db_path = crate::store::db_path_for_account(uin);
                    crate::store::load_history_into_threads(
                        &db_path,
                        &mut list,
                        crate::store::MessageStore::PRELOAD_MSGS,
                    );
                    let _ = gui_tx_clone.send(GuiEvent::StatusUpdate(format!(
                        "⏳ Đang làm mới tin nhắn cho {} hội thoại (3/3)…",
                        list.len()
                    )));
                    ctx_clone.request_repaint();
                    let mut fresh_ps = crate::protocol::messaging::PollState::default();
                    if let Err(e) = crate::protocol::messaging::polling::poll_new_messages(
                        &s,
                        &mut fresh_ps,
                        &mut list,
                    )
                    .await
                    {
                        crate::logger::log_error("gui::refresh_poll", &e.to_string());
                        errors.push(format!("Tin nhắn: {e}"));
                    }
                    {
                        let mut ps = poll_clone.lock().await;
                        *ps = fresh_ps;
                    }
                    if let Ok(mute_resp) =
                        crate::protocol::messaging::unread::get_mute_conversations(&s).await
                    {
                        crate::protocol::messaging::unread::merge_mute_metadata(
                            &mut list, &mute_resp,
                        );
                        if let Some(mute_map) = crate::protocol::messaging::unread::extract_mute_map(
                            &mute_resp,
                            crate::clock::unix_now(),
                        ) {
                            let _ = gui_tx_clone.send(GuiEvent::MuteSnapshotSynced(mute_map));
                        }
                    }
                    let total_msgs: usize = list.iter().map(|t| t.messages.len()).sum();
                    let n_threads = list.len();
                    let _ = gui_tx_clone.send(GuiEvent::ThreadsUpdated(list));
                    let _ = gui_tx_clone.send(GuiEvent::RefreshFinished {
                        threads: n_threads,
                        messages: total_msgs,
                        errors,
                        manual,
                    });
                    ctx_clone.request_repaint();
                }
            });
        }
        WorkerCmd::PollMessages { known_threads } => {
            static IS_POLLING: std::sync::atomic::AtomicBool =
                std::sync::atomic::AtomicBool::new(false);
            if IS_POLLING.swap(true, std::sync::atomic::Ordering::SeqCst) {
                return;
            }

            let session_clone = session.clone();
            let poll_clone = poll_state.clone();
            let gui_tx_clone = gui_tx.clone();
            let ctx_clone = egui_ctx.clone();
            tokio::spawn(async move {
                let s = session_clone.lock().await.clone();
                if s.login.is_some() {
                    let mut temp_threads: Vec<Thread> = known_threads
                        .into_iter()
                        .map(|(id, name, is_group)| Thread {
                            presentation: Default::default(),
                            unread: Default::default(),
                            id,
                            name,
                            preview: String::new(),
                            time: String::new(),
                            avatar: None,
                            is_group,
                            messages: Vec::new(),
                        })
                        .collect();

                    let mut poll_err: Option<anyhow::Error> = None;
                    for _ in 0..3 {
                        let poll_res = {
                            let mut ps = poll_clone.lock().await;
                            crate::protocol::messaging::polling::get_chat_from_polling(&s, &mut ps)
                                .await
                        };
                        match poll_res {
                            Ok(lp_resp) => {
                                crate::protocol::messaging::polling::merge_previews(
                                    &mut temp_threads,
                                    &lp_resp,
                                );
                                let data_node = lp_resp.get("data").unwrap_or(&lp_resp);
                                for sync_ev in crate::sync::extract_mobile_sync_controls(data_node)
                                {
                                    match sync_ev {
                                                        crate::sync::MobileSyncControlEvent::TransferAfterLogin { temp_key } => {
                                                            crate::sync::set_latest_mobile_temp_key(&temp_key);
                                                        }
                                                        crate::sync::MobileSyncControlEvent::WaitingConfirmOnPhone { .. } => {
                                                            let note = "Đang chờ bạn bấm 'Đồng bộ' trên màn hình Zalo điện thoại…".to_string();
                                                            crate::sync::set_last_mobile_sync_status(Some(note.clone()));
                                                            let _ = gui_tx_clone.send(GuiEvent::StatusUpdate(note));
                                                            ctx_clone.request_repaint();
                                                        }
                                                        crate::sync::MobileSyncControlEvent::MobileBusyRestoring => {
                                                            let note = "Điện thoại đang bận sao lưu/khôi phục phiên trước — hãy mở Zalo trên điện thoại xác nhận hoặc thử lại sau ít phút".to_string();
                                                            crate::sync::set_last_mobile_sync_status(Some(note.clone()));
                                                            let _ = gui_tx_clone.send(GuiEvent::StatusUpdate(note));
                                                            ctx_clone.request_repaint();
                                                        }
                                                        crate::sync::MobileSyncControlEvent::UserConfirmed => {
                                                            let note = "Điện thoại đã xác nhận đồng bộ — đang nén và đóng gói tin nhắn…".to_string();
                                                            crate::sync::set_last_mobile_sync_status(Some(note.clone()));
                                                            let _ = gui_tx_clone.send(GuiEvent::StatusUpdate(note));
                                                            ctx_clone.request_repaint();
                                                        }
                                                        crate::sync::MobileSyncControlEvent::UserRejected => {
                                                            let note = "Bạn đã từ chối yêu cầu đồng bộ tin nhắn trên điện thoại".to_string();
                                                            crate::sync::set_last_mobile_sync_status(Some(note.clone()));
                                                            let _ = gui_tx_clone.send(GuiEvent::StatusUpdate(note));
                                                            ctx_clone.request_repaint();
                                                        }
                                                        crate::sync::MobileSyncControlEvent::BackupReady {
                                                            url,
                                                            file_name,
                                                            encrypted_key,
                                                            raw_uid,
                                                            ..
                                                        } => {
                                                            let s_sync = s.clone();
                                                            let gui_tx_sync = gui_tx_clone.clone();
                                                            let ctx_sync = ctx_clone.clone();
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
                                                            let s_sync = s.clone();
                                                            let gui_tx_sync = gui_tx_clone.clone();
                                                            let ctx_sync = ctx_clone.clone();
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
                                let more = data_node
                                    .get("more")
                                    .and_then(serde_json::Value::as_i64)
                                    .unwrap_or(0);
                                let has_reset = data_node
                                    .get("resetLastActionId")
                                    .and_then(serde_json::Value::as_str)
                                    .map(|r| r != "0" && !r.is_empty())
                                    .unwrap_or(false);
                                if more == 0 && !has_reset {
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
                            if crate::session::SessionStatus::from_error(&e)
                                == crate::session::SessionStatus::Expired
                            {
                                {
                                    let mut sl = session_clone.lock().await;
                                    sl.mark_expired();
                                    let _ = sl.save_to_file("session.json");
                                    sl.logout();
                                }
                                let _ = gui_tx_clone.send(GuiEvent::SessionExpired(
                                    "Phiên đăng nhập (zpw_sek) đã hết hạn".into(),
                                ));
                                ctx_clone.request_repaint();
                                if let Some(tx) = WORKER_TX.get() {
                                    let _ = tx.send(WorkerCmd::RequestQr);
                                }
                            } else {
                                // The session is untouched: report and retry on
                                // the next poll instead of logging the user out.
                                crate::logger::log_error("gui::poll_messages", &e.to_string());
                                let _ =
                                    gui_tx_clone.send(GuiEvent::SessionUnreachable(e.to_string()));
                                ctx_clone.request_repaint();
                            }
                        }
                        None => {
                            temp_threads.retain(|t| !t.messages.is_empty());
                            if !temp_threads.is_empty() {
                                crate::protocol::social::groups::resolve_thread_names(
                                    &s,
                                    &mut temp_threads,
                                )
                                .await;
                                let rules = crate::api_mcp::load_autoreply_rules();
                                if rules.iter().any(|r| r.enabled) {
                                    for t in &temp_threads {
                                        for m in &t.messages {
                                            if !m.is_outgoing {
                                                if let Some(matched) =
                                                    crate::api_mcp::match_autoreply_rule(
                                                        &rules, &m.text, t.is_group,
                                                    )
                                                {
                                                    let reply_text = matched.reply_text;
                                                    let client_id =
                                                        crate::wire::next_client_message_id();
                                                    let res = if t.is_group {
                                                        crate::protocol::messaging::send::send_group_message_with_client_id(&s, &t.id, &reply_text, client_id).await
                                                    } else {
                                                        crate::protocol::messaging::send::send_message_with_client_id(
                                                            &s,
                                                            &t.id,
                                                            &reply_text,
                                                            client_id,
                                                        )
                                                        .await
                                                    };
                                                    if let Ok(resp) = res {
                                                        if resp_error_code(&resp).is_none() {
                                                            let msg_id = extract_msg_id(&resp);
                                                            let _ = gui_tx_clone.send(
                                                                GuiEvent::MessageSent {
                                                                    client_message_id: client_id
                                                                        .to_string(),
                                                                    thread_id: t.id.clone(),
                                                                    text: reply_text,
                                                                    msg_id,
                                                                },
                                                            );
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                                let _ = gui_tx_clone.send(GuiEvent::ThreadsUpdated(temp_threads));
                                ctx_clone.request_repaint();
                            }
                        }
                    }
                }
                IS_POLLING.store(false, std::sync::atomic::Ordering::SeqCst);
            });
        }
        _ => unreachable!(),
    }
}
