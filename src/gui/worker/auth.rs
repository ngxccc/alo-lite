use super::*;

pub(super) async fn handle_auth_cmd(
    cmd: WorkerCmd,
    session: &std::sync::Arc<tokio::sync::Mutex<Session>>,
    poll_state: &std::sync::Arc<tokio::sync::Mutex<crate::protocol::messaging::PollState>>,
    gui_tx: &GuiTx,
    egui_ctx: &egui::Context,
    current_qr_task: &mut Option<tokio::task::JoinHandle<()>>,
) {
    match cmd {
        WorkerCmd::RequestQr => {
            if let Some(task) = current_qr_task.take() {
                task.abort();
            }
            let session_clone = session.clone();
            let poll_clone = poll_state.clone();
            let gui_tx_clone = gui_tx.clone();
            let ctx_clone = egui_ctx.clone();

            let handle = tokio::spawn(async move {
                {
                    let mut s = session_clone.lock().await;
                    s.logout();
                }
                {
                    let mut ps = poll_clone.lock().await;
                    *ps = crate::protocol::messaging::PollState::default();
                }
                {
                    // The QR payload format follows the login-time server flag
                    // (`setttings.authen.trusted_device.qr_format_trust_device`): QR2
                    // companion linking + mobile biometrics only when the server enables it.
                    let mut s = session_clone.lock().await;
                    if let Ok(resp) = crate::protocol::auth::login::get_server_info(&s).await {
                        if let Some(d) = resp.get("data") {
                            s.apply_server_info(d);
                        }
                    }
                }
                let _ = gui_tx_clone.send(GuiEvent::StatusUpdate("Đang xin mã QR mới…".into()));
                ctx_clone.request_repaint();

                let client = crate::protocol::ZaloClient::new(session_clone.clone());
                let q_res = client.request_qr("vi").await;
                match q_res {
                    Ok(mut q) => {
                        if let Ok((w, h, pixels)) = q.rgba() {
                            let img = egui::ColorImage::from_rgba_unmultiplied(
                                [w as usize, h as usize],
                                &pixels,
                            );
                            let _ = gui_tx_clone.send(GuiEvent::QrReady {
                                qr_login: q.clone(),
                                qr_image: img,
                            });
                            ctx_clone.request_repaint();

                            let scan_res = client.wait_for_qr_scan(&mut q).await;

                            if let Ok(scanned) = scan_res {
                                let name = scanned
                                    .get("display_name")
                                    .or_else(|| scanned.get("displayName"))
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("Zalo User");
                                let avatar = scanned
                                    .get("avatar")
                                    .or_else(|| scanned.get("avatar_url"))
                                    .or_else(|| scanned.get("thumb"))
                                    .or_else(|| scanned.get("avt"))
                                    .and_then(|v| v.as_str())
                                    .filter(|s| !s.is_empty())
                                    .map(|s| s.to_string());
                                let _ = gui_tx_clone.send(GuiEvent::QrScanned {
                                    name: name.to_string(),
                                    avatar,
                                });
                                ctx_clone.request_repaint();

                                let confirm_res = client.confirm_qr(&q).await;

                                if let Ok(resp) = confirm_res {
                                    let Some(sek) =
                                        crate::protocol::auth::qr::extract_zpw_sek(&resp)
                                    else {
                                        let _ = gui_tx_clone.send(GuiEvent::QrExpired(
                                                                "Chưa nhận được xác nhận từ điện thoại. Vui lòng quét lại.".into(),
                                                            ));
                                        ctx_clone.request_repaint();
                                        return;
                                    };
                                    let mut s = session_clone.lock().await;
                                    s.set_zpw_sek(sek);
                                    if let Some(cs) = &q.companion_state {
                                        if let Some(proof) = &cs.proof {
                                            s.companion_ik =
                                                Some(cs.identity_key.public_key().to_vec());
                                            s.companion_priv =
                                                Some(cs.identity_key.private_key().to_vec());
                                            s.companion_master_id = Some(proof.master_id.clone());
                                            s.companion_link_id = Some(proof.link_id);
                                        }
                                    }
                                    if let Some(data) = resp.get("data") {
                                        crate::protocol::auth::login::apply_login_config(
                                            &mut s, data,
                                        );
                                    }
                                    let _ = s.save_to_file("session.json");

                                    if let Ok(info) =
                                        crate::protocol::auth::login::get_login_info(&s).await
                                    {
                                        if let Some(cfg) = info.get("data") {
                                            crate::protocol::auth::login::apply_login_config(
                                                &mut s, cfg,
                                            );
                                            s.apply_server_info(cfg);
                                        }
                                    }
                                    let _ = s.sync_profile_me().await;
                                    let _ = s.save_to_file("session.json");
                                    let _ = s.save_device_identity("device.json");

                                    // Guarantee: Never navigate to main workspace without an authenticated UIN
                                    if s.login.is_none() || s.zpw_sek.is_none() {
                                        let _ = gui_tx_clone.send(GuiEvent::LoginFailed(
                                                                "Đăng nhập thất bại: Không nhận được thông tin phiên từ máy chủ Zalo.".into(),
                                                            ));
                                        ctx_clone.request_repaint();
                                        return;
                                    }

                                    let uin =
                                        s.login.as_ref().map(|l| l.uin.clone()).unwrap_or_default();
                                    let _ =
                                        crate::account_manager::save_current_account_snapshot(&uin);
                                    let dname = s
                                        .login
                                        .as_ref()
                                        .map(|l| l.display_name.clone())
                                        .unwrap_or_default();
                                    let u_ava = s
                                        .login
                                        .as_ref()
                                        .map(|l| l.avatar.clone())
                                        .filter(|a| !a.is_empty());
                                    let _ = gui_tx_clone.send(GuiEvent::LoginSuccess {
                                        name: dname,
                                        uin: uin.clone(),
                                        avatar: u_ava,
                                    });
                                    ctx_clone.request_repaint();

                                    let mut list: Vec<Thread> = Vec::new();
                                    if let Ok(friends) =
                                        crate::protocol::social::friends::get_friends(&s).await
                                    {
                                        for ft in friends {
                                            if let Some(ex) =
                                                list.iter_mut().find(|t| t.id == ft.id)
                                            {
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
                                    if let Ok(groups) =
                                        crate::protocol::social::groups::get_groups(&s).await
                                    {
                                        for gt in groups {
                                            if let Some(ex) =
                                                list.iter_mut().find(|t| t.id == gt.id)
                                            {
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
                                    let db_path = crate::store::db_path_for_account(&uin);
                                    crate::store::load_history_into_threads(
                                        &db_path,
                                        &mut list,
                                        crate::store::MessageStore::PRELOAD_MSGS,
                                    );
                                    {
                                        let mut ps = poll_clone.lock().await;
                                        let _ =
                                            crate::protocol::messaging::polling::poll_new_messages(
                                                &s, &mut ps, &mut list,
                                            )
                                            .await;
                                    }
                                    if let Ok(mute_resp) =
                                        crate::protocol::messaging::unread::get_mute_conversations(
                                            &s,
                                        )
                                        .await
                                    {
                                        crate::protocol::messaging::unread::merge_mute_metadata(
                                            &mut list, &mute_resp,
                                        );
                                        if let Some(mute_map) =
                                            crate::protocol::messaging::unread::extract_mute_map(
                                                &mute_resp,
                                                crate::clock::unix_now(),
                                            )
                                        {
                                            let _ = gui_tx_clone
                                                .send(GuiEvent::MuteSnapshotSynced(mute_map));
                                        }
                                    }
                                    let sync_snapshot = list.clone();
                                    let _ = gui_tx_clone.send(GuiEvent::ThreadsUpdated(list));
                                    ctx_clone.request_repaint();

                                    // Ask the phone for the message keys right after login, like the
                                    // reference client: that request is what makes Zalo Mobile show
                                    // its confirmation (with biometrics when the account requires it,
                                    // see `re-notes/10` §1.2.2). #73 suppressed the prompt; we now
                                    // want it so the session can actually decrypt phone messages.
                                    if let Some(wtx) = WORKER_TX.get() {
                                        let _ = wtx.send(WorkerCmd::SyncPhoneMessages {
                                            threads: sync_snapshot,
                                            send_phone_prompt: true,
                                        });
                                    }

                                    let s_kw = s.clone();
                                    let tx_kw = gui_tx_clone.clone();
                                    let ctx_kw = ctx_clone.clone();
                                    tokio::spawn(async move {
                                        if let Ok(kws) =
                                            crate::protocol::messaging::stickers::fetch_sticker_suggest_keywords(&s_kw)
                                                .await
                                        {
                                            if !kws.is_empty() {
                                                let _ = tx_kw.send(
                                                    GuiEvent::StickerSuggestKeywordsLoaded(kws),
                                                );
                                                ctx_kw.request_repaint();
                                            }
                                        }
                                    });
                                } else if let Err(e) = confirm_res {
                                    let _ = gui_tx_clone.send(GuiEvent::QrExpired(format!(
                                        "Xác nhận trên điện thoại thất bại: {e}"
                                    )));
                                    ctx_clone.request_repaint();
                                }
                            } else if let Err(e) = scan_res {
                                let _ = gui_tx_clone
                                    .send(GuiEvent::QrExpired(format!("Mã QR đã hết hạn ({e})")));
                                ctx_clone.request_repaint();
                            }
                        } else {
                            let _ = gui_tx_clone.send(GuiEvent::LoginFailed(
                                "Máy chủ Zalo không trả về dữ liệu mã QR".into(),
                            ));
                            ctx_clone.request_repaint();
                        }
                    }
                    Err(e) => {
                        let _ = gui_tx_clone.send(GuiEvent::LoginFailed(e.to_string()));
                        ctx_clone.request_repaint();
                    }
                }
            });
            *current_qr_task = Some(handle);
        }
        WorkerCmd::LoginPassword { phone, pass } => {
            let session_clone = session.clone();
            let poll_clone = poll_state.clone();
            let gui_tx_clone = gui_tx.clone();
            let ctx_clone = egui_ctx.clone();
            tokio::spawn(async move {
                let _ =
                    gui_tx_clone.send(GuiEvent::StatusUpdate("Đang xác thực tài khoản…".into()));
                ctx_clone.request_repaint();
                let mut s = session_clone.lock().await;
                match crate::protocol::auth::login::login_by_password(&mut s, &phone, &pass, "VN")
                    .await
                {
                    Ok(_) => {
                        if let Ok(info) = crate::protocol::auth::login::get_login_info(&s).await {
                            if let Some(cfg) = info.get("data") {
                                crate::protocol::auth::login::apply_login_config(&mut s, cfg);
                                s.apply_server_info(cfg);
                                let _ = s.save_to_file("session.json");
                                let _ = s.save_device_identity("device.json");

                                let uin =
                                    s.login.as_ref().map(|l| l.uin.clone()).unwrap_or_default();
                                let _ = crate::account_manager::save_current_account_snapshot(&uin);
                                let name = s
                                    .login
                                    .as_ref()
                                    .map(|l| l.display_name.clone())
                                    .unwrap_or_default();
                                let u_ava = s
                                    .login
                                    .as_ref()
                                    .map(|l| l.avatar.clone())
                                    .filter(|a| !a.is_empty());
                                let _ = gui_tx_clone.send(GuiEvent::LoginSuccess {
                                    name,
                                    uin: uin.clone(),
                                    avatar: u_ava,
                                });
                                ctx_clone.request_repaint();

                                let mut list: Vec<Thread> = Vec::new();
                                if let Ok(friends) =
                                    crate::protocol::social::friends::get_friends(&s).await
                                {
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
                                if let Ok(groups) =
                                    crate::protocol::social::groups::get_groups(&s).await
                                {
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
                                let db_path = crate::store::db_path_for_account(&uin);
                                crate::store::load_history_into_threads(
                                    &db_path,
                                    &mut list,
                                    crate::store::MessageStore::PRELOAD_MSGS,
                                );
                                {
                                    let mut ps = poll_clone.lock().await;
                                    *ps = crate::protocol::messaging::PollState::default();
                                    let _ = crate::protocol::messaging::polling::poll_new_messages(
                                        &s, &mut ps, &mut list,
                                    )
                                    .await;
                                }
                                if let Ok(mute_resp) =
                                    crate::protocol::messaging::unread::get_mute_conversations(&s)
                                        .await
                                {
                                    crate::protocol::messaging::unread::merge_mute_metadata(
                                        &mut list, &mute_resp,
                                    );
                                    if let Some(mute_map) =
                                        crate::protocol::messaging::unread::extract_mute_map(
                                            &mute_resp,
                                            crate::clock::unix_now(),
                                        )
                                    {
                                        let _ = gui_tx_clone
                                            .send(GuiEvent::MuteSnapshotSynced(mute_map));
                                    }
                                }
                                let sync_snapshot = list.clone();
                                let _ = gui_tx_clone.send(GuiEvent::ThreadsUpdated(list));
                                ctx_clone.request_repaint();

                                // Same as the QR path: request the phone's message keys so the
                                // device can decrypt the transferred history (`re-notes/10` §1.2.2).
                                if let Some(wtx) = WORKER_TX.get() {
                                    let _ = wtx.send(WorkerCmd::SyncPhoneMessages {
                                        threads: sync_snapshot,
                                        send_phone_prompt: true,
                                    });
                                }
                            }
                        }
                    }
                    Err(e) => {
                        let _ = gui_tx_clone.send(GuiEvent::LoginFailed(e.to_string()));
                        ctx_clone.request_repaint();
                    }
                }
            });
        }
        WorkerCmd::Logout => {
            let cur_uin = {
                let s = session.lock().await;
                s.login.as_ref().map(|l| l.uin.clone()).unwrap_or_default()
            };
            if !cur_uin.is_empty() {
                let _ = crate::account_manager::save_current_account_snapshot(&cur_uin);
            }
            if let Some(task) = current_qr_task.take() {
                task.abort();
            }
            let _ = std::fs::remove_file("session.json");
            let _ = std::fs::remove_file("threads_cache.json");
            let _ = std::fs::remove_file("zalo_history.db");
            {
                let mut s = session.lock().await;
                s.logout();
                let _ = s.save_device_identity("device.json");
            }
            {
                let mut ps = poll_state.lock().await;
                *ps = crate::protocol::messaging::PollState::default();
            }
            let _ = gui_tx.send(GuiEvent::LoggedOut);
            egui_ctx.request_repaint();
        }
        WorkerCmd::SwitchAccount { target_uin } => {
            let cur_uin = {
                let s = session.lock().await;
                s.login.as_ref().map(|l| l.uin.clone()).unwrap_or_default()
            };
            if let Err(e) = crate::account_manager::switch_to_account(&cur_uin, &target_uin) {
                let _ = gui_tx.send(GuiEvent::StatusUpdate(format!("Lỗi chuyển tài khoản: {e}")));
                egui_ctx.request_repaint();
            } else {
                let mut s = session.lock().await;
                if s.load_from_file("session.json").is_ok() {
                    let uin = s.login.as_ref().map(|l| l.uin.clone()).unwrap_or_default();
                    let name = s
                        .login
                        .as_ref()
                        .map(|l| l.display_name.clone())
                        .unwrap_or_default();
                    let ava = s.login.as_ref().and_then(|l| {
                        if l.avatar.is_empty() {
                            None
                        } else {
                            Some(l.avatar.clone())
                        }
                    });
                    let mut threads = crate::store::load_threads_cache();
                    if let Ok(store) = crate::store::MessageStore::open("zalo_history.db") {
                        let _ = store.hydrate_threads(&mut threads, 50);
                    }
                    let _ = gui_tx.send(GuiEvent::AccountSwitched {
                        uin,
                        name,
                        avatar: ava,
                        threads,
                    });
                }
                egui_ctx.request_repaint();
            }
        }
        WorkerCmd::WipeLocalData => {
            let _ = std::fs::remove_file("zalo_history.db");
            let _ = std::fs::remove_file("threads_cache.json");
            let _ = gui_tx.send(GuiEvent::LocalDataWiped);
            egui_ctx.request_repaint();
        }
        _ => unreachable!(),
    }
}
