use super::*;

pub(super) fn handle_chat_cmd(
    cmd: WorkerCmd,
    session: &std::sync::Arc<tokio::sync::Mutex<Session>>,
    gui_tx: &GuiTx,
    egui_ctx: &egui::Context,
) {
    match cmd {
        WorkerCmd::SendMessage {
            to_id,
            text,
            is_group,
        } => {
            let session_clone = session.clone();
            let gui_tx_clone = gui_tx.clone();
            let ctx_clone = egui_ctx.clone();
            tokio::spawn(async move {
                let s = session_clone.lock().await.clone();
                let client_id = crate::wire::next_client_message_id();
                if let Some(found_url) = crate::protocol::messaging::links::extract_first_url(&text)
                {
                    let meta =
                        crate::protocol::messaging::links::parse_link_metadata(&s, &found_url)
                            .await
                            .unwrap_or_else(|_| crate::protocol::messaging::LinkMetadata {
                                url: found_url.clone(),
                                title: found_url.clone(),
                                desc: String::new(),
                                thumb: String::new(),
                                src: "Liên kết".into(),
                            });
                    let _ = gui_tx_clone.send(GuiEvent::LinkPreviewReady(meta.clone()));
                    match crate::protocol::messaging::links::send_link_message_with_client_id(
                        &s, &to_id, &text, &meta, is_group, client_id,
                    )
                    .await
                    {
                        Ok(resp) => {
                            let msg_id = extract_msg_id(&resp);
                            let now = crate::ui::clock();
                            let final_id = if msg_id.is_empty() {
                                format!("out_{client_id}")
                            } else {
                                msg_id
                            };
                            let msg = ChatMessage {
                                id: final_id,
                                client_message_id: client_id.to_string(),
                                sender: "Tôi".into(),
                                text,
                                time: now,
                                is_outgoing: true,
                                ts: (crate::clock::unix_now() * 1000),
                                link_url: Some(meta.url),
                                link_title: if meta.title.is_empty() {
                                    None
                                } else {
                                    Some(meta.title)
                                },
                                link_desc: if meta.desc.is_empty() {
                                    None
                                } else {
                                    Some(meta.desc)
                                },
                                link_thumb: if meta.thumb.is_empty() {
                                    None
                                } else {
                                    Some(meta.thumb)
                                },
                                link_src: if meta.src.is_empty() {
                                    None
                                } else {
                                    Some(meta.src)
                                },
                                ..Default::default()
                            };
                            let _ = gui_tx_clone.send(GuiEvent::AttachmentSent {
                                thread_id: to_id,
                                msg,
                            });
                            ctx_clone.request_repaint();
                            return;
                        }
                        Err(e) => {
                            let err_s = e.to_string();
                            crate::logger::log_error("gui::send_link", &err_s);
                            let _ = gui_tx_clone.send(GuiEvent::SendFailed {
                                thread_id: to_id,
                                text,
                                error: err_s,
                            });
                            ctx_clone.request_repaint();
                            return;
                        }
                    }
                }

                let res = if is_group {
                    crate::protocol::messaging::send::send_group_message_with_client_id(
                        &s, &to_id, &text, client_id,
                    )
                    .await
                } else {
                    crate::protocol::messaging::send::send_message_with_client_id(
                        &s, &to_id, &text, client_id,
                    )
                    .await
                };
                match res {
                    Ok(resp) => {
                        let msg_id = extract_msg_id(&resp);
                        let _ = gui_tx_clone.send(GuiEvent::MessageSent {
                            client_message_id: client_id.to_string(),
                            thread_id: to_id,
                            text,
                            msg_id,
                        });
                        ctx_clone.request_repaint();
                    }
                    Err(e) => {
                        let err_s = e.to_string();
                        crate::logger::log_error("gui::send_message", &err_s);
                        let _ = gui_tx_clone.send(GuiEvent::SendFailed {
                            thread_id: to_id,
                            text,
                            error: err_s.clone(),
                        });
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
                                "Không thể gửi vì phiên zpw_sek đã hết hạn".into(),
                            ));
                            if let Some(tx) = WORKER_TX.get() {
                                let _ = tx.send(WorkerCmd::RequestQr);
                            }
                        }
                        ctx_clone.request_repaint();
                    }
                }
            });
        }
        WorkerCmd::SendQuote {
            to_id,
            text,
            quoted,
            is_group,
        } => {
            let session_clone = session.clone();
            let gui_tx_clone = gui_tx.clone();
            let ctx_clone = egui_ctx.clone();
            tokio::spawn(async move {
                let s = session_clone.lock().await.clone();
                let client_id = crate::wire::next_client_message_id();
                match crate::protocol::messaging::send::send_quote_message_with_client_id(
                    &s, &to_id, &text, &quoted, is_group, client_id,
                )
                .await
                {
                    Ok(resp) => {
                        let _ = gui_tx_clone.send(GuiEvent::QuoteSent {
                            client_message_id: client_id.to_string(),
                            thread_id: to_id,
                            text,
                            msg_id: extract_msg_id(&resp),
                            quoted,
                        });
                    }
                    Err(e) => {
                        let err_s = e.to_string();
                        crate::logger::log_error("gui::send_quote", &err_s);
                        let _ = gui_tx_clone.send(GuiEvent::SendFailed {
                            thread_id: to_id,
                            text,
                            error: err_s,
                        });
                    }
                }
                ctx_clone.request_repaint();
            });
        }
        WorkerCmd::Forward {
            target_id,
            target_is_group,
            msg,
        } => {
            let session_clone = session.clone();
            let gui_tx_clone = gui_tx.clone();
            let ctx_clone = egui_ctx.clone();
            tokio::spawn(async move {
                let s = session_clone.lock().await.clone();
                let client_id = crate::wire::next_client_message_id();
                let res = crate::protocol::messaging::send::forward_message_with_client_id(
                    &s,
                    &target_id,
                    target_is_group,
                    &msg,
                    client_id,
                )
                .await;
                match res {
                    Ok(resp) if resp_error_code(&resp).is_none() => {
                        let msg_id = extract_msg_id(&resp);
                        if let Some(sid) = msg.sticker_id.filter(|v| *v > 0) {
                            let _ = gui_tx_clone.send(GuiEvent::StickerSent {
                                client_message_id: client_id.to_string(),
                                thread_id: target_id,
                                msg_id,
                                sticker_id: sid,
                                cate_id: msg.sticker_cate_id.unwrap_or(0),
                                sticker_type: msg.sticker_type.unwrap_or(7),
                            });
                        } else {
                            let _ = gui_tx_clone.send(GuiEvent::MessageSent {
                                client_message_id: client_id.to_string(),
                                thread_id: target_id,
                                text: msg.text.clone(),
                                msg_id,
                            });
                        }
                        let _ = gui_tx_clone
                            .send(GuiEvent::StatusUpdate("Đã chuyển tiếp tin nhắn".into()));
                    }
                    Ok(resp) => {
                        let err_s = format!("server error {}", resp_error_code(&resp).unwrap_or(0));
                        crate::logger::log_error("gui::forward", &err_s);
                        let _ = gui_tx_clone.send(GuiEvent::StatusUpdate(format!(
                            "Chuyển tiếp thất bại: {err_s}"
                        )));
                    }
                    Err(e) => {
                        crate::logger::log_error("gui::forward", &e.to_string());
                        let _ = gui_tx_clone
                            .send(GuiEvent::StatusUpdate(format!("Chuyển tiếp thất bại: {e}")));
                    }
                }
                ctx_clone.request_repaint();
            });
        }
        WorkerCmd::React {
            to_id,
            is_group,
            msg,
            reaction,
        } => {
            let session_clone = session.clone();
            let gui_tx_clone = gui_tx.clone();
            let ctx_clone = egui_ctx.clone();
            tokio::spawn(async move {
                let s = session_clone.lock().await.clone();
                let res = crate::protocol::messaging::reactions::send_reaction(
                    &s, &to_id, is_group, &msg, &reaction,
                )
                .await;
                match res {
                    Ok(resp) if resp_error_code(&resp).is_none() => {
                        let _ = gui_tx_clone.send(GuiEvent::Reacted {
                            thread_id: to_id,
                            msg_id: msg.id.clone(),
                            ts: msg.ts,
                            reaction,
                        });
                    }
                    Ok(resp) => {
                        let err_s = format!("server error {}", resp_error_code(&resp).unwrap_or(0));
                        crate::logger::log_error("gui::react", &err_s);
                        let _ = gui_tx_clone.send(GuiEvent::StatusUpdate(format!(
                            "Thả cảm xúc thất bại: {err_s}"
                        )));
                    }
                    Err(e) => {
                        crate::logger::log_error("gui::react", &e.to_string());
                        let _ = gui_tx_clone
                            .send(GuiEvent::StatusUpdate(format!("Thả cảm xúc thất bại: {e}")));
                    }
                }
                ctx_clone.request_repaint();
            });
        }
        WorkerCmd::DeleteForMe {
            to_id,
            is_group,
            msg,
        } => {
            let session_clone = session.clone();
            let gui_tx_clone = gui_tx.clone();
            let ctx_clone = egui_ctx.clone();
            tokio::spawn(async move {
                let s = session_clone.lock().await.clone();
                let local_only =
                    msg.id.is_empty() || msg.id.starts_with("out_") || msg.id.starts_with("local_");
                let res: anyhow::Result<serde_json::Value> = if local_only {
                    Ok(serde_json::json!({}))
                } else {
                    crate::protocol::messaging::send::delete_message_for_me(
                        &s, &to_id, is_group, &msg,
                    )
                    .await
                };
                match res {
                    Ok(resp) if resp_error_code(&resp).is_none() => {
                        let (tid, mid, ts) = (to_id.clone(), msg.id.clone(), msg.ts);
                        let _ = tokio::task::spawn_blocking(move || {
                            if let Ok(mut store) =
                                crate::store::MessageStore::open("zalo_history.db")
                            {
                                let _ = store.delete_message(&tid, &mid, ts);
                            }
                        })
                        .await;
                        let _ = gui_tx_clone.send(GuiEvent::MessageDeleted {
                            thread_id: to_id,
                            msg_id: msg.id.clone(),
                            ts: msg.ts,
                        });
                    }
                    Ok(resp) => {
                        let err_s = format!("server error {}", resp_error_code(&resp).unwrap_or(0));
                        crate::logger::log_error("gui::delete", &err_s);
                        let _ = gui_tx_clone.send(GuiEvent::StatusUpdate(format!(
                            "Xóa tin nhắn thất bại: {err_s}"
                        )));
                    }
                    Err(e) => {
                        crate::logger::log_error("gui::delete", &e.to_string());
                        let _ = gui_tx_clone.send(GuiEvent::StatusUpdate(format!(
                            "Xóa tin nhắn thất bại: {e}"
                        )));
                    }
                }
                ctx_clone.request_repaint();
            });
        }
        WorkerCmd::RecallMessage {
            to_id,
            is_group,
            msg,
        } => {
            let session_clone = session.clone();
            let gui_tx_clone = gui_tx.clone();
            let ctx_clone = egui_ctx.clone();
            tokio::spawn(async move {
                let s = session_clone.lock().await.clone();
                let local_only =
                    msg.id.is_empty() || msg.id.starts_with("out_") || msg.id.starts_with("local_");
                let res: anyhow::Result<serde_json::Value> = if local_only {
                    Ok(serde_json::json!({}))
                } else {
                    crate::protocol::messaging::send::recall_message(&s, &to_id, is_group, &msg)
                        .await
                };
                match res {
                    Ok(resp) if resp_error_code(&resp).is_none() => {
                        let _ = gui_tx_clone.send(GuiEvent::MessageRecalled {
                            thread_id: to_id,
                            msg_id: msg.id.clone(),
                            ts: msg.ts,
                        });
                    }
                    Ok(resp) => {
                        let err_s = format!("server error {}", resp_error_code(&resp).unwrap_or(0));
                        crate::logger::log_error("gui::recall", &err_s);
                        let _ = gui_tx_clone.send(GuiEvent::StatusUpdate(format!(
                            "Đã thu hồi cục bộ (Máy chủ: {err_s})"
                        )));
                    }
                    Err(e) => {
                        crate::logger::log_error("gui::recall", &e.to_string());
                        let _ = gui_tx_clone.send(GuiEvent::StatusUpdate(format!(
                            "Không thể thu hồi trên máy chủ: {e}"
                        )));
                    }
                }
                ctx_clone.request_repaint();
            });
        }
        WorkerCmd::SendSticker {
            to_id,
            sticker_id,
            cate_id,
            sticker_type,
            is_group,
        } => {
            let session_clone = session.clone();
            let gui_tx_clone = gui_tx.clone();
            let ctx_clone = egui_ctx.clone();
            if cate_id == AMI_FAT_CAT_CATE_ID {
                if let Some((_lbl, png_bytes, gif_bytes)) = get_ami_fat_cat_gif(sticker_id) {
                    tokio::spawn(async move {
                        let idx = sticker_id.saturating_sub(999000);
                        let _ = std::fs::create_dir_all("stickers_cache");
                        let gif_path = std::path::PathBuf::from(format!(
                            "stickers_cache/ami_fat_cat_{idx:02}.gif"
                        ));
                        if !gif_path.exists() {
                            let _ = std::fs::write(&gif_path, gif_bytes);
                        }
                        let key = format!("stk_{cate_id}_{sticker_id}");
                        if let Ok(dyn_img) = image::load_from_memory(png_bytes) {
                            let rgba = dyn_img.to_rgba8();
                            let size = [rgba.width() as usize, rgba.height() as usize];
                            let pixels = rgba.as_flat_samples();
                            let color_img =
                                egui::ColorImage::from_rgba_unmultiplied(size, pixels.as_slice());
                            let _ = gui_tx_clone.send(GuiEvent::StickerImageReady {
                                key,
                                image: color_img,
                            });
                        }
                        let s = session_clone.lock().await.clone();
                        let client_id = crate::wire::next_client_message_id();
                        let ami_tag = format!("#ami_fat_cat_{idx:02}");
                        match crate::protocol::messaging::attachments::send_attachment(
                            &s, &to_id, is_group, &gif_path, &ami_tag,
                        )
                        .await
                        {
                            Ok((_resp, cm)) => {
                                let _ = gui_tx_clone.send(GuiEvent::StickerSent {
                                    client_message_id: cm.client_message_id,
                                    thread_id: to_id,
                                    msg_id: cm.id,
                                    sticker_id,
                                    cate_id,
                                    sticker_type,
                                });
                            }
                            Err(e) => {
                                let err_s = e.to_string();
                                crate::logger::log_error("gui::send_ami_sticker", &err_s);
                                let _ = gui_tx_clone.send(GuiEvent::StickerSent {
                                    client_message_id: client_id.to_string(),
                                    thread_id: to_id,
                                    msg_id: format!("out_{client_id}"),
                                    sticker_id,
                                    cate_id,
                                    sticker_type,
                                });
                            }
                        }
                        ctx_clone.request_repaint();
                    });
                    return;
                }
            }
            tokio::spawn(async move {
                let s = session_clone.lock().await.clone();
                let client_id = crate::wire::next_client_message_id();
                let res = crate::protocol::messaging::stickers::send_sticker_with_client_id(
                    &s,
                    &to_id,
                    sticker_id,
                    cate_id,
                    sticker_type,
                    is_group,
                    client_id,
                )
                .await;
                match res {
                    Ok(resp) => {
                        let msg_id = resp
                            .get("data")
                            .and_then(|d| d.get("msgId"))
                            .and_then(|v| match v {
                                serde_json::Value::String(s) => Some(s.clone()),
                                serde_json::Value::Number(n) => Some(n.to_string()),
                                _ => None,
                            })
                            .unwrap_or_default();
                        let _ = gui_tx_clone.send(GuiEvent::StickerSent {
                            client_message_id: client_id.to_string(),
                            thread_id: to_id,
                            msg_id,
                            sticker_id,
                            cate_id,
                            sticker_type,
                        });
                        ctx_clone.request_repaint();
                    }
                    Err(e) => {
                        let err_s = e.to_string();
                        crate::logger::log_error("gui::send_sticker", &err_s);
                        let _ = gui_tx_clone.send(GuiEvent::SendFailed {
                            thread_id: to_id,
                            text: String::new(),
                            error: err_s.clone(),
                        });
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
                                "Không thể gửi nhãn dán vì phiên zpw_sek đã hết hạn".into(),
                            ));
                            if let Some(tx) = WORKER_TX.get() {
                                let _ = tx.send(WorkerCmd::RequestQr);
                            }
                        }
                        ctx_clone.request_repaint();
                    }
                }
            });
        }
        WorkerCmd::PickAndSendAttachment {
            to_id,
            is_group,
            images_only,
            caption,
            preset_path,
        } => {
            let session_clone = session.clone();
            let gui_tx_clone = gui_tx.clone();
            let ctx_clone = egui_ctx.clone();
            tokio::spawn(async move {
                let picked_path = if let Some(p) = preset_path {
                    Some(p)
                } else {
                    tokio::task::spawn_blocking(move || pick_native_file_dialog(images_only))
                        .await
                        .ok()
                        .flatten()
                };

                let Some(path) = picked_path else {
                    return;
                };
                let fname = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("tập tin")
                    .to_string();
                let _ = gui_tx_clone.send(GuiEvent::StatusUpdate(format!(
                    "⏳ Đang tải lên và gửi '{fname}'…"
                )));
                ctx_clone.request_repaint();

                let s = session_clone.lock().await.clone();
                match crate::protocol::messaging::attachments::send_attachment(
                    &s, &to_id, is_group, &path, &caption,
                )
                .await
                {
                    Ok((_resp, cm)) => {
                        let _ = gui_tx_clone.send(GuiEvent::AttachmentSent {
                            thread_id: to_id,
                            msg: cm,
                        });
                    }
                    Err(e) => {
                        let err_s = e.to_string();
                        crate::logger::log_error("gui::send_attachment", &err_s);
                        let _ = gui_tx_clone.send(GuiEvent::SendFailed {
                            thread_id: to_id,
                            text: caption,
                            error: format!("Lỗi gửi '{fname}': {err_s}"),
                        });
                    }
                }
                ctx_clone.request_repaint();
            });
        }
        WorkerCmd::SendTyping {
            thread_id,
            is_group,
        } => {
            let session_clone = session.clone();
            tokio::spawn(async move {
                let s = session_clone.lock().await.clone();
                let _ = crate::protocol::messaging::send::send_typing_indicator(
                    &s, &thread_id, is_group,
                )
                .await;
            });
        }
        WorkerCmd::SendSeen {
            thread_id,
            is_group,
            last_msg_id,
        } => {
            let session_clone = session.clone();
            tokio::spawn(async move {
                let s = session_clone.lock().await.clone();
                let _ = crate::protocol::messaging::unread::send_seen_v2(
                    &s,
                    &thread_id,
                    is_group,
                    &last_msg_id,
                )
                .await;
            });
        }
        _ => unreachable!(),
    }
}
