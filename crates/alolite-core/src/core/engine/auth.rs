//! Authentication flows (QR code login, password login), session expiration handling,
//! logout, and multi-account switching.

use std::sync::Arc;

use super::merge_thread_headers;
use super::CoreEngine;
use crate::clock::unix_now;
use crate::core::events::{CoreEvent, CoreImageData};
use crate::protocol::auth::{login, qr};
use crate::protocol::messaging::polling::{poll_new_messages, PollState};
use crate::protocol::messaging::stickers::fetch_sticker_suggest_keywords;
use crate::protocol::messaging::unread::get_mute_snapshot;
use crate::protocol::messaging::Thread;
use crate::protocol::session::Session;
use crate::protocol::social::friends::get_friends_with;
use crate::protocol::social::groups::get_groups;
use crate::services::account_manager::{
    delete_saved_account as svc_delete_saved_account,
    list_saved_accounts as svc_list_saved_accounts, save_current_account_snapshot,
    switch_to_account, AccountProfile,
};
use crate::storage::{
    load_history_into_threads, load_threads_cache, persist_account_threads,
    preload_account_history, MessageStore,
};

impl CoreEngine {
    pub(super) async fn handle_expired_session(self: &Arc<Self>, message: &str) {
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

    pub async fn login_password(
        self: &Arc<Self>,
        phone: String,
        pass: String,
        captcha_token: Option<String>,
        captcha_answer: Option<String>,
    ) {
        self.emit(CoreEvent::StatusUpdate("Đang xác thực tài khoản…".into()));
        let mut s = self.session.lock().await;
        match login::login_by_password(
            &mut s,
            &phone,
            &pass,
            "VN",
            captcha_token.as_deref(),
            captcha_answer.as_deref(),
        )
        .await
        {
            Ok(login::LoginPasswordOutcome::Success(_)) => {
                let _ = login::fetch_and_apply_login_info(&mut s).await;
                let _ = s.save_to_file("session.json");
                let _ = s.save_device_identity("device.json");
                let s_clone = s.clone();
                drop(s);
                self.finish_authenticated_bootstrap(s_clone).await;
            }
            Ok(login::LoginPasswordOutcome::CaptchaRequired(challenge)) => {
                self.emit(CoreEvent::CaptchaRequired(challenge));
            }
            Err(e) => {
                self.emit(CoreEvent::LoginFailed(e.to_string()));
            }
        }
    }

    pub async fn refresh_captcha(self: &Arc<Self>) {
        let s = self.session.lock().await.clone();
        match login::get_login_captcha(&s).await {
            Ok(challenge) => {
                self.emit(CoreEvent::CaptchaRequired(challenge));
            }
            Err(e) => {
                self.emit(CoreEvent::LoginFailed(format!("Lỗi làm mới captcha: {e}")));
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
        if let Ok(friends) = get_friends_with(&s, |t| preload_account_history(&uin, t)).await {
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
            if poll_new_messages(&s, &mut ps, &mut list).await.is_ok() {
                persist_account_threads(&uin, &list);
            }
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

    /// Save the active account snapshot, log out, and request a fresh QR code to add another account.
    pub async fn add_new_account(self: &Arc<Self>) {
        let cur_uin = {
            let s = self.session.lock().await;
            s.login.as_ref().map(|l| l.uin.clone()).unwrap_or_default()
        };
        if !cur_uin.is_empty() {
            let _ = save_current_account_snapshot(&cur_uin);
        }
        self.logout().await;
        self.request_qr().await;
    }

    /// List saved multi-account profiles (`accounts/<uin>.json`).
    pub async fn list_saved_accounts(&self) -> Vec<AccountProfile> {
        let cur_uin = {
            let s = self.session.lock().await;
            s.login.as_ref().map(|l| l.uin.clone()).unwrap_or_default()
        };
        svc_list_saved_accounts(&cur_uin)
    }

    /// Remove a saved non-active account snapshot from `accounts/`.
    pub fn delete_saved_account(&self, uin: &str) -> Result<(), String> {
        svc_delete_saved_account(uin).map_err(|e| e.to_string())
    }
}
