use super::*;

pub(super) fn handle_history_cmd(
    cmd: WorkerCmd,
    session: &std::sync::Arc<tokio::sync::Mutex<Session>>,
    poll_state: &std::sync::Arc<tokio::sync::Mutex<crate::protocol::messaging::PollState>>,
    gui_tx: &GuiTx,
    egui_ctx: &egui::Context,
) {
    match cmd {
        WorkerCmd::SyncHistory { threads } => {
            use std::sync::atomic::Ordering;
            if crate::sync::RUNNING.swap(true, Ordering::SeqCst) {
                return;
            }
            crate::sync::CANCEL.store(false, Ordering::Relaxed);
            let session_clone = session.clone();
            let gui_tx_clone = gui_tx.clone();
            let ctx_clone = egui_ctx.clone();
            tokio::spawn(async move {
                let s = session_clone.lock().await.clone();
                let total = threads.len();
                let mut added = 0;
                let mut errors = Vec::new();
                for (i, (id, name, is_group)) in threads.into_iter().enumerate() {
                    if crate::sync::CANCEL.load(Ordering::Relaxed) {
                        break;
                    }
                    let _ = gui_tx_clone.send(GuiEvent::StatusUpdate(format!(
                        "Đang đồng bộ {}/{total} · {name}",
                        i + 1
                    )));
                    ctx_clone.request_repaint();
                    let mut t = Thread {
                        id,
                        name,
                        preview: String::new(),
                        time: String::new(),
                        avatar: None,
                        is_group,
                        messages: Vec::new(),
                        ..Default::default()
                    };
                    match crate::store::sync_thread(&s, &mut t).await {
                        Ok(n) => added += n,
                        Err(e) => errors.push(format!("{}: {e}", t.name)),
                    }
                }
                let _ = gui_tx_clone.send(GuiEvent::SyncFinished {
                    added,
                    errors,
                    cancelled: crate::sync::CANCEL.load(Ordering::Relaxed),
                });
                ctx_clone.request_repaint();
                crate::sync::RUNNING.store(false, Ordering::SeqCst);
            });
        }
        WorkerCmd::LoadGroupHistory {
            group_id,
            is_group,
            have,
        } => {
            let session_clone = session.clone();
            let gui_tx_clone = gui_tx.clone();
            let ctx_clone = egui_ctx.clone();
            tokio::spawn(async move {
                let s = session_clone.lock().await.clone();
                if s.login.is_some() {
                    let is_send2me = s
                        .login
                        .as_ref()
                        .map(|l| !l.send2me_id.is_empty() && l.send2me_id == group_id)
                        .unwrap_or(false);
                    let t = Thread {
                        presentation: Default::default(),
                        unread: Default::default(),
                        id: group_id.clone(),
                        name: String::new(),
                        preview: String::new(),
                        time: String::new(),
                        avatar: None,
                        is_group: is_group && !is_send2me,
                        messages: Vec::new(),
                    };
                    let mut t_list = vec![t];
                    let uin = s.login.as_ref().map(|l| l.uin.as_str()).unwrap_or_default();
                    let db_path = crate::store::db_path_for_account(uin);
                    crate::store::load_history_into_threads(
                        &db_path,
                        &mut t_list,
                        (have + 300).max(500),
                    );
                    let Some(mut t) = t_list.into_iter().find(|x| x.id == group_id) else {
                        return;
                    };
                    if !is_group && !is_send2me {
                        let mut one_list = vec![t.clone()];
                        if let Ok(resp) = crate::protocol::messaging::polling::get_last_msgs(
                            &s,
                            std::slice::from_ref(&group_id),
                        )
                        .await
                        {
                            crate::protocol::messaging::polling::merge_previews(
                                &mut one_list,
                                &resp,
                            );
                        }
                        if let Ok(resp_all) =
                            crate::protocol::messaging::polling::get_last_msgs(&s, &[]).await
                        {
                            crate::protocol::messaging::polling::merge_previews(
                                &mut one_list,
                                &resp_all,
                            );
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
                        if let Ok(cm_resp) =
                            crate::protocol::messaging::polling::get_cloud_messages(
                                &s, &group_id, "0", false, 50,
                            )
                            .await
                        {
                            crate::protocol::messaging::polling::merge_group_history(
                                &mut t, &cm_resp,
                            );
                        }
                        // Backfill up to 5 pages (250 messages) of older cloud history
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
                                let Ok(cm_old) =
                                    crate::protocol::messaging::polling::get_cloud_messages(
                                        &s, &group_id, &cursor_id, true, 50,
                                    )
                                    .await
                                else {
                                    break;
                                };
                                crate::protocol::messaging::polling::merge_group_history(
                                    &mut t, &cm_old,
                                );
                                let new_len = t.messages.len();
                                let _ = gui_tx_clone.send(GuiEvent::StatusUpdate(format!(
                                "⏳ Đang đồng bộ lịch sử cũ (trang {page}/5 — {new_len} tin nhắn)…"
                            )));
                                ctx_clone.request_repaint();
                                let data_node = cm_old
                                    .pointer("/data/data")
                                    .or_else(|| cm_old.get("data"))
                                    .unwrap_or(&cm_old);
                                let has_more = data_node
                                    .get("hasMore")
                                    .and_then(|v| v.as_i64())
                                    .unwrap_or(0);
                                let next_cursor = data_node
                                    .get("lastMsgId")
                                    .and_then(|v| {
                                        v.as_str()
                                            .map(str::to_string)
                                            .or_else(|| v.as_u64().map(|n| n.to_string()))
                                    })
                                    .filter(|s| {
                                        !s.is_empty()
                                            && s != "0"
                                            && s.chars().all(|c| c.is_ascii_digit())
                                    })
                                    .or_else(|| {
                                        t.messages.first().map(|m| m.id.clone()).filter(|id| {
                                            !id.is_empty() && id.chars().all(|c| c.is_ascii_digit())
                                        })
                                    });
                                let Some(nc) = next_cursor else {
                                    break;
                                };
                                if has_more == 0 || nc == cursor_id || new_len == prev_len {
                                    break;
                                }
                                cursor_id = nc;
                            }
                        }
                    }
                    if is_group
                        && !is_send2me
                        && (crate::protocol::messaging::threads::is_generic_thread_name(&t.name)
                            || t.avatar.as_deref().unwrap_or("").is_empty())
                    {
                        crate::protocol::social::groups::resolve_thread_names(
                            &s,
                            std::slice::from_mut(&mut t),
                        )
                        .await;
                    }
                    let cnt = t.messages.len();
                    if !t.messages.is_empty() || !t.name.is_empty() {
                        let uin = s.login.as_ref().map(|l| l.uin.as_str()).unwrap_or_default();
                        let db_path = crate::store::db_path_for_account(uin);
                        if let Ok(mut store) = crate::store::MessageStore::open(&db_path) {
                            let _ =
                                crate::store::persist_threads(&mut store, std::slice::from_ref(&t));
                        }
                        let _ = gui_tx_clone.send(GuiEvent::ThreadsUpdated(vec![t]));
                    }
                    if cnt <= have {
                        let _ = gui_tx_clone.send(GuiEvent::HistoryReachedStart(group_id.clone()));
                    } else {
                        let _ = gui_tx_clone.send(GuiEvent::StatusUpdate(format!(
                            "Đã tải thêm {} tin nhắn cũ ({cnt} tin trong hội thoại)",
                            cnt.saturating_sub(have)
                        )));
                    }
                    ctx_clone.request_repaint();
                }
            });
        }
        WorkerCmd::SyncPhoneMessages {
            mut threads,
            send_phone_prompt,
        } => {
            use std::sync::atomic::Ordering;
            if crate::sync::PHONE_SYNC_RUNNING.swap(true, Ordering::SeqCst) {
                return;
            }
            crate::sync::PHONE_SYNC_CANCEL.store(false, Ordering::Relaxed);
            let session_clone = session.clone();
            let poll_clone = poll_state.clone();
            let gui_tx_clone = gui_tx.clone();
            let ctx_clone = egui_ctx.clone();
            tokio::spawn(async move {
                let s = session_clone.lock().await.clone();
                let gui_tx_prog = gui_tx_clone.clone();
                let ctx_prog = ctx_clone.clone();
                let res = crate::store::sync_phone_messages_opts(
                    &s,
                    &poll_clone,
                    &mut threads,
                    send_phone_prompt,
                    move |progress, status| {
                        let _ = gui_tx_prog.send(GuiEvent::PhoneSyncProgress { progress, status });
                        ctx_prog.request_repaint();
                    },
                )
                .await;
                let cancelled = crate::sync::PHONE_SYNC_CANCEL.load(Ordering::Relaxed);
                match res {
                    Ok((added, active_threads)) => {
                        let _ = gui_tx_clone.send(GuiEvent::PhoneSyncFinished {
                            threads,
                            added,
                            active_threads,
                            cancelled,
                            error: None,
                        });
                    }
                    Err(e) => {
                        let _ = gui_tx_clone.send(GuiEvent::PhoneSyncFinished {
                            threads,
                            added: 0,
                            active_threads: 0,
                            cancelled,
                            error: Some(e.to_string()),
                        });
                    }
                }
                ctx_clone.request_repaint();
                crate::sync::PHONE_SYNC_RUNNING.store(false, Ordering::SeqCst);
            });
        }
        _ => unreachable!(),
    }
}
