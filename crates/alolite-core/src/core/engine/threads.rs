//! Thread management, thread persistence, periodic/manual poll execution,
//! cloud message history sync, group history loading, and mobile phone sync.

use std::sync::atomic::Ordering;
use std::sync::Arc;

use anyhow::Result;

use super::merge_thread_headers;
use super::CoreEngine;
use crate::clock::unix_now;
use crate::core::events::CoreEvent;
use crate::protocol::messaging::polling::{
    get_chat_from_polling, get_cloud_messages, get_last_msgs, poll_new_messages, PollState,
};
use crate::protocol::messaging::threads::{
    delete_conversation, is_generic_thread_name, set_hidden_conversation,
    update_pinned_conversation,
};
use crate::protocol::messaging::unread::{
    get_mute_snapshot, set_mute_conversation, set_unread_mark,
};
use crate::protocol::messaging::Thread;
use crate::protocol::session::{Session, SessionStatus};
use crate::protocol::social::friends::get_friends_with;
use crate::protocol::social::groups::{get_groups, resolve_thread_names};
use crate::protocol::sync::{
    fetch_phone_messages, fetch_thread_history, format_phone_sync_final_message, PHONE_SYNC_CANCEL,
    PHONE_SYNC_RUNNING,
};
use crate::storage::{
    commit_phone_sync_to_store, db_path_for_account, load_history_into_threads, load_threads_cache,
    oldest_message_id, persist_account_threads, persist_thread_snapshot,
    persist_threads as storage_persist_threads, preload_account_history, save_threads_cache,
    MessageStore,
};

async fn compose_sync_thread(s: &Session, t: &mut Thread) -> Result<usize> {
    let id = t.id.clone();
    let uin = s.login.as_ref().map(|l| l.uin.clone()).unwrap_or_default();
    let db_path = db_path_for_account(&uin);
    let db_path_init = db_path.clone();
    let cursor = tokio::task::spawn_blocking(move || oldest_message_id(&db_path_init, &id))
        .await?
        .unwrap_or_default();
    let db_path_page = db_path.clone();
    fetch_thread_history(s, t, &cursor, move |snapshot| {
        let db_path_p = db_path_page.clone();
        let snap = snapshot.clone();
        async move {
            tokio::task::spawn_blocking(move || persist_thread_snapshot(&db_path_p, &snap)).await?
        }
    })
    .await
}

impl CoreEngine {
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

    pub async fn persist_threads(&self, threads_snapshot: Vec<Thread>) {
        let db_path = self.active_db_path().await;
        let _ = tokio::task::spawn_blocking(move || {
            if let Ok(mut store) = MessageStore::open(&db_path) {
                let _ = storage_persist_threads(&mut store, &threads_snapshot);
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
            SessionStatus::Valid => self.set_session_verified(true),
            SessionStatus::Expired => {
                self.set_session_verified(false);
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
                if let Some(li) = &mut sl.login {
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
        let uin = s.login.as_ref().map(|l| l.uin.as_str()).unwrap_or_default();
        match get_friends_with(&s, |t| preload_account_history(uin, t)).await {
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
        match poll_new_messages(&s, &mut fresh_ps, &mut list).await {
            Ok(()) => persist_account_threads(uin, &list),
            Err(e) => {
                crate::logger::log_error("core::refresh_poll", &e.to_string());
                errors.push(format!("Tin nhắn: {e}"));
            }
        }
        {
            let mut ps = self.poll_state.lock().await;
            *ps = fresh_ps;
        }
        if let Ok(mute_snap) = get_mute_snapshot(&s, unix_now()).await {
            mute_snap.merge_into(&mut list);
            self.emit(CoreEvent::MuteSnapshotSynced(mute_snap.mutes));
        }
        resolve_thread_names(&s, &mut list).await;
        let cache_copy = list.clone();
        let _ = tokio::task::spawn_blocking(move || save_threads_cache(&cache_copy)).await;

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
        if (unix_now().max(0) as u64) < self.poll_paused_until.load(Ordering::Relaxed) {
            self.is_polling.store(false, Ordering::SeqCst);
            return;
        }
        let s = self.session.lock().await.clone();
        if s.has_live_credentials() && self.is_session_verified() {
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
                        self.poll_failures.store(0, Ordering::Relaxed);
                        self.poll_paused_until.store(0, Ordering::Relaxed);
                        self.handle_expired_session("Phiên đăng nhập (zpw_sek) đã hết hạn")
                            .await;
                    } else {
                        // A bare `HTTP 400` from the long-poll edge is not an auth failure, so
                        // treat it as transient: report it once and back off instead of
                        // re-sending the same request every 1200ms.
                        let failures = self.poll_failures.fetch_add(1, Ordering::Relaxed) + 1;
                        crate::logger::log_error("core::poll_messages", &e.to_string());
                        if failures == 1 {
                            self.emit(CoreEvent::SessionUnreachable(e.to_string()));
                        }
                        if failures >= 3 {
                            let backoff_secs = 5u64 << (failures.saturating_sub(3)).min(3);
                            let pause_until = unix_now().max(0) as u64 + backoff_secs.min(60);
                            self.poll_paused_until.store(pause_until, Ordering::Relaxed);
                        }
                    }
                }
                None => {
                    self.poll_failures.store(0, Ordering::Relaxed);
                    self.poll_paused_until.store(0, Ordering::Relaxed);
                    temp_threads.retain(|t| !t.messages.is_empty());
                    if !temp_threads.is_empty() {
                        resolve_thread_names(&s, &mut temp_threads).await;
                        self.dispatch_autoreplies(&s, &temp_threads).await;
                        self.emit(CoreEvent::ThreadsUpdated(temp_threads));
                    }
                }
            }
        } else {
            self.poll_failures.store(0, Ordering::Relaxed);
            self.poll_paused_until.store(0, Ordering::Relaxed);
        }
        self.is_polling.store(false, Ordering::SeqCst);
    }

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
            match compose_sync_thread(&s, &mut t).await {
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
                    let _ = storage_persist_threads(&mut store, std::slice::from_ref(&t_save));
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
        let sink_progress = Arc::clone(&sink);
        let fetch_res = fetch_phone_messages(
            &s,
            &self.poll_state,
            &mut threads,
            send_phone_prompt,
            move |progress, status| {
                sink_progress.emit(CoreEvent::PhoneSyncProgress { progress, status });
            },
        )
        .await;

        let res = match fetch_res {
            Ok(outcome) if outcome.cancelled => Ok((0, 0)),
            Ok(outcome) => {
                let uin = s.login.as_ref().map(|l| l.uin.clone()).unwrap_or_default();
                let db_path = db_path_for_account(&uin);
                let snapshot = threads.clone();
                let db_added = tokio::task::spawn_blocking(move || {
                    commit_phone_sync_to_store(&db_path, &snapshot)
                })
                .await;
                match db_added {
                    Ok(Ok(n)) => {
                        let after_total: usize = threads.iter().map(|t| t.messages.len()).sum();
                        let added = n.max(after_total.saturating_sub(outcome.before_total));
                        let active_threads =
                            threads.iter().filter(|t| !t.messages.is_empty()).count();
                        let final_msg = format_phone_sync_final_message(added, active_threads);
                        sink.emit(CoreEvent::PhoneSyncProgress {
                            progress: 1.0,
                            status: final_msg,
                        });
                        Ok((added, active_threads))
                    }
                    Ok(Err(e)) => Err(e),
                    Err(e) => Err(anyhow::anyhow!("JoinError: {e}")),
                }
            }
            Err(e) => Err(e),
        };
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

    /// Cancel an in-progress cloud history sync or phone message sync.
    pub fn cancel_cloud_sync(&self) {
        crate::protocol::sync::CANCEL.store(true, Ordering::Relaxed);
    }

    pub fn cancel_phone_sync(&self) {
        PHONE_SYNC_CANCEL.store(true, Ordering::Relaxed);
    }
}
