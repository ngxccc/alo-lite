use super::*;

pub(super) fn handle_contacts_cmd(
    cmd: WorkerCmd,
    session: &std::sync::Arc<tokio::sync::Mutex<Session>>,
    gui_tx: &GuiTx,
    egui_ctx: &egui::Context,
) {
    match cmd {
        WorkerCmd::CheckFriendStatus { user_id } => {
            let client = crate::protocol::ZaloClient::new(session.clone());
            let gui_tx_clone = gui_tx.clone();
            let ctx_clone = egui_ctx.clone();
            tokio::spawn(async move {
                let status_res = client.check_friendship_status(&user_id).await;
                let s = client.snapshot_session().await;
                let online_opt =
                    crate::protocol::social::friends::get_user_online_status(&s, &user_id)
                        .await
                        .ok();
                if let Ok(status) = status_res {
                    let _ = gui_tx_clone.send(GuiEvent::FriendStatusChecked {
                        user_id,
                        status,
                        online: online_opt,
                    });
                    ctx_clone.request_repaint();
                }
            });
        }
        WorkerCmd::AcceptFriend { user_id, user_name } => {
            let client = crate::protocol::ZaloClient::new(session.clone());
            let gui_tx_clone = gui_tx.clone();
            let ctx_clone = egui_ctx.clone();
            tokio::spawn(async move {
                match client.accept_friend_request(&user_id).await {
                    Ok(_) => {
                        let _ = gui_tx_clone.send(GuiEvent::FriendStatusChecked {
                            user_id,
                            status: crate::protocol::social::FriendshipStatus::Friend,
                            online: None,
                        });
                        let _ = gui_tx_clone.send(GuiEvent::StatusUpdate(format!(
                            "Đã đồng ý kết bạn với {user_name}!"
                        )));
                    }
                    Err(e) => {
                        let _ = gui_tx_clone.send(GuiEvent::StatusUpdate(format!(
                            "Không thể đồng ý kết bạn với {user_name}: {e}"
                        )));
                    }
                }
                ctx_clone.request_repaint();
            });
        }
        WorkerCmd::RejectFriend { user_id, user_name } => {
            let client = crate::protocol::ZaloClient::new(session.clone());
            let gui_tx_clone = gui_tx.clone();
            let ctx_clone = egui_ctx.clone();
            tokio::spawn(async move {
                if client.reject_friend_request(&user_id).await.is_ok() {
                    let _ = gui_tx_clone.send(GuiEvent::FriendStatusChecked {
                        user_id,
                        status: crate::protocol::social::FriendshipStatus::Stranger,
                        online: None,
                    });
                    let _ = gui_tx_clone.send(GuiEvent::StatusUpdate(format!(
                        "Đã từ chối lời mời kết bạn của {user_name}."
                    )));
                    ctx_clone.request_repaint();
                }
            });
        }
        WorkerCmd::UndoFriend { user_id, user_name } => {
            let session_clone = session.clone();
            let gui_tx_clone = gui_tx.clone();
            let ctx_clone = egui_ctx.clone();
            tokio::spawn(async move {
                let s = session_clone.lock().await.clone();
                if crate::protocol::social::friends::undo_friend_request(&s, &user_id)
                    .await
                    .is_ok()
                {
                    let _ = gui_tx_clone.send(GuiEvent::FriendStatusChecked {
                        user_id,
                        status: crate::protocol::social::FriendshipStatus::Stranger,
                        online: None,
                    });
                    let _ = gui_tx_clone.send(GuiEvent::StatusUpdate(format!(
                        "Đã hủy lời mời kết bạn tới {user_name}."
                    )));
                    ctx_clone.request_repaint();
                }
            });
        }
        WorkerCmd::AddFriend { user_id, user_name } => {
            let session_clone = session.clone();
            let gui_tx_clone = gui_tx.clone();
            let ctx_clone = egui_ctx.clone();
            tokio::spawn(async move {
                let s = session_clone.lock().await.clone();
                // Check if there is an incoming request we can accept directly
                if crate::protocol::social::friends::accept_friend_request(&s, &user_id)
                    .await
                    .is_ok()
                {
                    let _ = gui_tx_clone.send(GuiEvent::FriendStatusChecked {
                        user_id,
                        status: crate::protocol::social::FriendshipStatus::Friend,
                        online: None,
                    });
                    let _ = gui_tx_clone.send(GuiEvent::StatusUpdate(format!(
                        "Đã đồng ý kết bạn với {user_name}! Bạn có thể gọi lại ngay."
                    )));
                    ctx_clone.request_repaint();
                    return;
                }
                match crate::protocol::social::friends::send_friend_request(
                    &s,
                    &user_id,
                    "Xin chào, kết bạn với mình nhé!",
                )
                .await
                {
                    Ok(_) => {
                        let _ = gui_tx_clone.send(GuiEvent::FriendStatusChecked {
                            user_id: user_id.clone(),
                            status: crate::protocol::social::FriendshipStatus::OutgoingPending,
                            online: None,
                        });
                        let _ = gui_tx_clone.send(GuiEvent::StatusUpdate(format!(
                            "Đã gửi lời mời kết bạn tới {user_name} ({user_id})."
                        )));
                    }
                    Err(e) => {
                        let _ = gui_tx_clone.send(GuiEvent::StatusUpdate(format!(
                            "Không thể gửi lời mời kết bạn tới {user_name}: {e}"
                        )));
                    }
                }
                ctx_clone.request_repaint();
            });
        }
        WorkerCmd::LookupPhone { phone } => {
            let client = crate::protocol::ZaloClient::new(session.clone());
            let gui_tx_clone = gui_tx.clone();
            let ctx_clone = egui_ctx.clone();
            tokio::spawn(async move {
                match client.find_user_by_phone(&phone).await {
                    Ok(profile) => {
                        let _ = gui_tx_clone.send(GuiEvent::PhoneLookupSuccess(profile));
                    }
                    Err(e) => {
                        crate::logger::log_error("gui::lookup_phone", &e.to_string());
                        let _ = gui_tx_clone.send(GuiEvent::PhoneLookupFailed(e.to_string()));
                    }
                }
                ctx_clone.request_repaint();
            });
        }
        WorkerCmd::FetchSocialProfile { target_uid } => {
            let session_clone = session.clone();
            let gui_tx_clone = gui_tx.clone();
            let ctx_clone = egui_ctx.clone();
            tokio::spawn(async move {
                let s = session_clone.lock().await.clone();
                let res = s
                    .get_friend_social_profile(&target_uid)
                    .await
                    .map_err(|e| e.to_string());
                let _ = gui_tx_clone.send(GuiEvent::SocialProfileLoaded(res));
                ctx_clone.request_repaint();
            });
        }
        WorkerCmd::FetchMySocialAvatars { page, count } => {
            let session_clone = session.clone();
            let gui_tx_clone = gui_tx.clone();
            let ctx_clone = egui_ctx.clone();
            tokio::spawn(async move {
                let s = session_clone.lock().await.clone();
                let res = s
                    .get_my_social_avatar_list(page, count)
                    .await
                    .map_err(|e| e.to_string());
                let _ = gui_tx_clone.send(GuiEvent::SocialAvatarsLoaded(res));
                ctx_clone.request_repaint();
            });
        }
        _ => unreachable!(),
    }
}
