use std::sync::{Arc, OnceLock};

use eframe::egui;

use super::{pick_native_file_dialog, GuiEvent, GuiTx};
use crate::core::{CoreEngine, CoreEvent, CoreEventSink};
use crate::session::Session;

pub mod types;
pub use types::WorkerCmd;

pub(super) static WORKER_TX: OnceLock<tokio::sync::mpsc::UnboundedSender<WorkerCmd>> =
    OnceLock::new();
pub(super) static ENGINE: OnceLock<Arc<CoreEngine>> = OnceLock::new();

struct EguiEventSink {
    gui_tx: GuiTx,
    ctx: egui::Context,
}

impl CoreEventSink for EguiEventSink {
    fn emit(&self, ev: CoreEvent) {
        let gui_ev = match ev {
            CoreEvent::NotificationAuthorization { request_id, result } => {
                GuiEvent::NotificationAuthorization { request_id, result }
            }
            CoreEvent::QrReady { qr_login, qr_image } => GuiEvent::QrReady {
                qr_login,
                qr_image: egui::ColorImage::from_rgba_unmultiplied(
                    [qr_image.width as usize, qr_image.height as usize],
                    &qr_image.rgba,
                ),
            },
            CoreEvent::QrScanned { name, avatar } => GuiEvent::QrScanned { name, avatar },
            CoreEvent::QrExpired(msg) => GuiEvent::QrExpired(msg),
            CoreEvent::LoginSuccess { name, uin, avatar } => {
                GuiEvent::LoginSuccess { name, uin, avatar }
            }
            CoreEvent::LoginFailed(msg) => GuiEvent::LoginFailed(msg),
            CoreEvent::ThreadsUpdated(threads) => GuiEvent::ThreadsUpdated(threads),
            CoreEvent::MessageSent {
                client_message_id,
                thread_id,
                text,
                msg_id,
            } => GuiEvent::MessageSent {
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
            } => GuiEvent::StickerSent {
                client_message_id,
                thread_id,
                msg_id,
                sticker_id,
                cate_id,
                sticker_type,
            },
            CoreEvent::AttachmentSent { thread_id, msg } => {
                GuiEvent::AttachmentSent { thread_id, msg }
            }
            CoreEvent::StickerImageReady { key, image } => GuiEvent::StickerImageReady {
                key,
                image: egui::ColorImage::from_rgba_unmultiplied(
                    [image.width as usize, image.height as usize],
                    &image.rgba,
                ),
            },
            CoreEvent::StickerStoreLoaded(packs) => GuiEvent::StickerStoreLoaded(packs),
            CoreEvent::StickerPackItemsLoaded {
                cate_id,
                name,
                items,
            } => GuiEvent::StickerPackItemsLoaded {
                cate_id,
                name,
                items,
            },
            CoreEvent::StickerSearchResults { keyword, items } => {
                GuiEvent::StickerSearchResults { keyword, items }
            }
            CoreEvent::StickerSuggestKeywordsLoaded(kws) => {
                GuiEvent::StickerSuggestKeywordsLoaded(kws)
            }
            CoreEvent::ComposerStickerSuggestions { keyword, items } => {
                GuiEvent::ComposerStickerSuggestions { keyword, items }
            }
            CoreEvent::LinkPreviewReady(meta) => GuiEvent::LinkPreviewReady(meta),
            CoreEvent::PhoneLookupSuccess(res) => GuiEvent::PhoneLookupSuccess(res),
            CoreEvent::PhoneLookupFailed(err) => GuiEvent::PhoneLookupFailed(err),
            CoreEvent::FriendStatusChecked {
                user_id,
                status,
                online,
            } => GuiEvent::FriendStatusChecked {
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
            } => GuiEvent::QuoteSent {
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
            } => GuiEvent::Reacted {
                thread_id,
                msg_id,
                ts,
                reaction,
            },
            CoreEvent::MessageDeleted {
                thread_id,
                msg_id,
                ts,
            } => GuiEvent::MessageDeleted {
                thread_id,
                msg_id,
                ts,
            },
            CoreEvent::MessageRecalled {
                thread_id,
                msg_id,
                ts,
            } => GuiEvent::MessageRecalled {
                thread_id,
                msg_id,
                ts,
            },
            CoreEvent::SendFailed {
                thread_id,
                text,
                error,
            } => GuiEvent::SendFailed {
                thread_id,
                text,
                error,
            },
            CoreEvent::SessionExpired(msg) => GuiEvent::SessionExpired(msg),
            CoreEvent::SessionUnreachable(msg) => GuiEvent::SessionUnreachable(msg),
            CoreEvent::LocalDataWiped => GuiEvent::LocalDataWiped,
            CoreEvent::StatusUpdate(msg) => GuiEvent::StatusUpdate(msg),
            CoreEvent::RefreshFinished {
                threads,
                messages,
                errors,
                manual,
            } => GuiEvent::RefreshFinished {
                threads,
                messages,
                errors,
                manual,
            },
            CoreEvent::SyncFinished {
                added,
                errors,
                cancelled,
            } => GuiEvent::SyncFinished {
                added,
                errors,
                cancelled,
            },
            CoreEvent::UpdateCheckFinished { result, silent } => {
                GuiEvent::UpdateCheckFinished { result, silent }
            }
            CoreEvent::UpdateDownloadStatus {
                downloading,
                message,
            } => GuiEvent::UpdateDownloadStatus {
                downloading,
                message,
            },
            CoreEvent::UpdateDownloadProgress { downloaded, total } => {
                GuiEvent::UpdateDownloadProgress { downloaded, total }
            }
            CoreEvent::UpdateAppliedAndRestarting(ver) => GuiEvent::UpdateAppliedAndRestarting(ver),
            CoreEvent::MuteSnapshotSynced(m) => GuiEvent::MuteSnapshotSynced(m),
            CoreEvent::MuteEventsReceived(evs) => GuiEvent::MuteEventsReceived(evs),
            CoreEvent::PhoneSyncProgress { progress, status } => {
                GuiEvent::PhoneSyncProgress { progress, status }
            }
            CoreEvent::PhoneSyncFinished {
                threads,
                added,
                active_threads,
                cancelled,
                error,
            } => GuiEvent::PhoneSyncFinished {
                threads,
                added,
                active_threads,
                cancelled,
                error,
            },
            CoreEvent::ProxyUpdated(res) => GuiEvent::ProxyUpdated(res),
            CoreEvent::FileDownloaded(res) => GuiEvent::FileDownloaded(res),
            CoreEvent::LoggedOut => GuiEvent::LoggedOut,
            CoreEvent::HistoryReachedStart(id) => GuiEvent::HistoryReachedStart(id),
            CoreEvent::AccountSwitched {
                uin,
                name,
                avatar,
                threads,
            } => GuiEvent::AccountSwitched {
                uin,
                name,
                avatar,
                threads,
            },
        };
        let _ = self.gui_tx.send(gui_ev);
        self.ctx.request_repaint();
    }
}

pub(super) fn publish_known_threads_to_engine(threads: Vec<(String, String, bool)>) {
    if let Some(engine) = ENGINE.get() {
        engine.set_known_threads(threads);
    }
}

pub(super) fn spawn_session_worker(
    session: Session,
    gui_tx: GuiTx,
    egui_ctx: egui::Context,
    api_port: Option<u16>,
) {
    let (cmd_tx, mut cmd_rx) = tokio::sync::mpsc::unbounded_channel::<WorkerCmd>();
    let _ = WORKER_TX.set(cmd_tx);

    let effective_port = api_port.or_else(|| {
        std::env::var("ALOLITE_API_PORT")
            .ok()
            .and_then(|s| s.parse::<u16>().ok())
            .filter(|&p| p > 0)
    });

    let sink = Arc::new(EguiEventSink {
        gui_tx,
        ctx: egui_ctx,
    });
    let engine = CoreEngine::new(session, sink);
    let _ = ENGINE.set(Arc::clone(&engine));

    std::thread::Builder::new()
        .name("zalo-worker".into())
        .spawn(move || {
            let rt = tokio::runtime::Builder::new_multi_thread()
                .worker_threads(2)
                .enable_all()
                .build()
                .unwrap();

            rt.block_on(async move {
                engine.start_background_loops(effective_port);

                while let Some(cmd) = cmd_rx.recv().await {
                    let eng = Arc::clone(&engine);
                    tokio::spawn(async move {
                        dispatch_worker_cmd(eng, cmd).await;
                    });
                }
            });
        })
        .expect("Failed to spawn zalo-worker thread");
}

async fn dispatch_worker_cmd(engine: Arc<CoreEngine>, cmd: WorkerCmd) {
    match cmd {
        WorkerCmd::ReadNotificationAuthorization { request_id } => {
            engine.read_notification_auth(request_id, false).await;
        }
        WorkerCmd::RequestNotificationAuthorization { request_id } => {
            engine.read_notification_auth(request_id, true).await;
        }
        WorkerCmd::RequestQr => engine.request_qr().await,
        WorkerCmd::LoginPassword { phone, pass } => engine.login_password(phone, pass).await,
        WorkerCmd::Logout => engine.logout().await,
        WorkerCmd::SwitchAccount { target_uin } => engine.switch_account(&target_uin).await,
        WorkerCmd::WipeLocalData => engine.wipe_local_data().await,
        WorkerCmd::RefreshThreads => engine.refresh_threads(false).await,
        WorkerCmd::RefreshThreadsManual => engine.refresh_threads(true).await,
        WorkerCmd::PersistThreads(threads) => engine.persist_threads(threads).await,
        WorkerCmd::PinThread {
            thread_id,
            is_group,
            pin,
        } => engine.pin_thread(&thread_id, is_group, pin).await,
        WorkerCmd::MarkThreadUnread {
            thread_id,
            is_group,
            unread,
        } => {
            engine
                .mark_thread_unread(&thread_id, is_group, unread)
                .await
        }
        WorkerCmd::MuteThread {
            thread_id,
            is_group,
            duration_sec,
            mute,
        } => {
            engine
                .mute_thread(&thread_id, is_group, duration_sec, mute)
                .await
        }
        WorkerCmd::HideThread {
            thread_id,
            is_group,
            hide,
        } => engine.hide_thread(&thread_id, is_group, hide).await,
        WorkerCmd::DeleteThread {
            thread_id,
            is_group,
            last_cli_msg_id,
        } => {
            engine
                .delete_thread(&thread_id, is_group, &last_cli_msg_id)
                .await
        }
        WorkerCmd::SendMessage {
            to_id,
            text,
            is_group,
        } => engine.send_message_or_link(to_id, text, is_group).await,
        WorkerCmd::SendQuote {
            to_id,
            text,
            quoted,
            is_group,
        } => engine.send_quote(to_id, text, quoted, is_group).await,
        WorkerCmd::Forward {
            target_id,
            target_is_group,
            msg,
        } => {
            engine
                .forward_message(target_id, target_is_group, msg)
                .await
        }
        WorkerCmd::React {
            to_id,
            is_group,
            msg,
            reaction,
        } => engine.react(to_id, is_group, msg, reaction).await,
        WorkerCmd::DeleteForMe {
            to_id,
            is_group,
            msg,
        } => engine.delete_for_me(to_id, is_group, msg).await,
        WorkerCmd::RecallMessage {
            to_id,
            is_group,
            msg,
        } => engine.recall_message(to_id, is_group, msg).await,
        WorkerCmd::SendSticker {
            to_id,
            sticker_id,
            cate_id,
            sticker_type,
            is_group,
        } => {
            engine
                .send_sticker(to_id, sticker_id, cate_id, sticker_type, is_group)
                .await
        }
        WorkerCmd::PickAndSendAttachment {
            to_id,
            is_group,
            images_only,
            caption,
            preset_path,
        } => {
            let picked = if let Some(p) = preset_path {
                Some(p)
            } else {
                tokio::task::spawn_blocking(move || pick_native_file_dialog(images_only))
                    .await
                    .ok()
                    .flatten()
            };
            if let Some(path) = picked {
                engine
                    .send_attachment_file(to_id, is_group, path, caption)
                    .await;
            }
        }
        WorkerCmd::SendTyping {
            thread_id,
            is_group,
        } => engine.send_typing(&thread_id, is_group).await,
        WorkerCmd::SendSeen {
            thread_id,
            is_group,
            last_msg_id,
        } => engine.send_seen(&thread_id, is_group, &last_msg_id).await,
        WorkerCmd::FetchStickerImage { key, url } => engine.fetch_sticker_image(key, url).await,
        WorkerCmd::FetchStickerStore => engine.fetch_sticker_store().await,
        WorkerCmd::FetchStickerPack { cate_id, name } => {
            engine.fetch_sticker_pack(cate_id, name).await
        }
        WorkerCmd::SearchStickers { keyword } => engine.search_stickers(keyword).await,
        WorkerCmd::SuggestComposerStickers { keyword } => {
            engine.suggest_composer_stickers(keyword).await
        }
        WorkerCmd::ParseLinkPreview { url } => engine.parse_link_preview(&url).await,
        WorkerCmd::LookupPhone { phone } => engine.lookup_phone(phone).await,
        WorkerCmd::AddFriend { user_id, user_name } => engine.add_friend(user_id, user_name).await,
        WorkerCmd::CheckFriendStatus { user_id } => engine.check_friend_status(user_id).await,
        WorkerCmd::AcceptFriend { user_id, user_name } => {
            engine.accept_friend(user_id, user_name).await
        }
        WorkerCmd::RejectFriend { user_id, user_name } => {
            engine.reject_friend(user_id, user_name).await
        }
        WorkerCmd::UndoFriend { user_id, user_name } => {
            engine.undo_friend(user_id, user_name).await
        }
        WorkerCmd::SyncHistory { threads } => engine.sync_history(threads).await,
        WorkerCmd::LoadGroupHistory {
            group_id,
            is_group,
            have,
        } => engine.load_group_history(group_id, is_group, have).await,
        WorkerCmd::SyncPhoneMessages {
            threads,
            send_phone_prompt,
        } => engine.sync_phone_messages(threads, send_phone_prompt).await,
        WorkerCmd::CheckForUpdates { silent } => engine.check_for_updates(silent).await,
        WorkerCmd::DownloadAndApplyUpdate {
            download_url,
            asset_name,
            target_version,
            expected_sha256,
        } => {
            engine
                .download_and_apply_update(
                    download_url,
                    asset_name,
                    target_version,
                    expected_sha256,
                )
                .await
        }
        WorkerCmd::SetProxy { proxy_url } => engine.set_proxy(&proxy_url).await,
        WorkerCmd::DownloadFile {
            url,
            file_name,
            download_dir,
        } => engine.download_file(url, file_name, download_dir).await,
    }
}
