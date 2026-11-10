//! System maintenance, updates, notification permissions, settings, and diagnostic tools.

use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::sync::Arc;

use super::CoreEngine;
use crate::clock::unix_now;
use crate::core::events::CoreEvent;
use crate::core::ipc::{
    detect_inline_video_support, embedded_app_logo_data_url, CoreBootstrapSnapshot, DevCryptoInfo,
    DevDiagnosticsSnapshot, ImportSummaryDto, PinnedPackMeta,
};
use crate::core::settings::AppSettings;
use crate::core::stickers::{
    builtin_pack_stickers, load_custom_sticker_packs, FEATURED_PACK_TABS, ONION_STICKERS,
    POPULAR_STICKERS,
};
use crate::protocol::messaging::Thread;
use crate::protocol::sync::PHONE_SYNC_RUNNING;
use crate::services::native_notifications::{request_if_user_action, status as notif_status};
use crate::services::updater::{apply_github_release_update, fetch_github_latest_release};
use crate::storage::MessageStore;

impl CoreEngine {
    pub async fn read_notification_auth(&self, request_id: u64, request: bool) {
        let result = if request {
            request_if_user_action(true).await
        } else {
            notif_status().await
        };
        self.emit(CoreEvent::NotificationAuthorization { request_id, result });
    }

    pub async fn check_for_updates(&self, silent: bool) {
        let res = fetch_github_latest_release().await;
        self.emit(CoreEvent::UpdateCheckFinished {
            result: res,
            silent,
        });
    }

    pub async fn download_and_apply_update(
        &self,
        download_url: String,
        asset_name: String,
        target_version: String,
        expected_sha256: Option<String>,
    ) {
        self.emit(CoreEvent::UpdateDownloadStatus {
            downloading: true,
            message: format!("Đang tải bản cập nhật v{target_version}…"),
        });
        let sink = Arc::clone(&self.sink);
        match apply_github_release_update(
            &download_url,
            &asset_name,
            &target_version,
            expected_sha256.as_deref(),
            |downloaded, total| {
                sink.emit(CoreEvent::UpdateDownloadProgress { downloaded, total });
            },
        )
        .await
        {
            Ok(restarting) => {
                if restarting {
                    self.emit(CoreEvent::UpdateAppliedAndRestarting(target_version));
                } else {
                    self.emit(CoreEvent::UpdateDownloadStatus {
                        downloading: false,
                        message: format!(
                            "Đã tải và mở bộ cài đặt {asset_name} (v{target_version})"
                        ),
                    });
                }
            }
            Err(e) => {
                self.emit(CoreEvent::UpdateDownloadStatus {
                    downloading: false,
                    message: format!("Lỗi cập nhật: {e}"),
                });
            }
        }
    }

    /// Build the full initial snapshot required by desktop UI hosts (Tauri WebView, etc.).
    pub async fn bootstrap_snapshot(
        &self,
        preloaded_threads: Vec<Thread>,
    ) -> CoreBootstrapSnapshot {
        let settings = AppSettings::load();
        settings.apply_privacy_blockers();

        let (logged_in, uin, display_name, avatar, send2me_id) = {
            let s = self.session.lock().await;
            let logged_in = s.login.is_some() && !s.is_expired_marked();
            let (uin, display_name, avatar, send2me_id) = s
                .login
                .as_ref()
                .map(|l| {
                    (
                        l.uin.clone(),
                        l.display_name.clone(),
                        (!l.avatar.is_empty()).then_some(l.avatar.clone()),
                        l.send2me_id.clone(),
                    )
                })
                .unwrap_or_default();
            (logged_in, uin, display_name, avatar, send2me_id)
        };

        let mut threads = if preloaded_threads.is_empty() && logged_in {
            self.list_threads(MessageStore::PRELOAD_MSGS).await
        } else {
            preloaded_threads
        };
        if logged_in && !threads.is_empty() {
            let s_snap = self.session.lock().await.clone();
            crate::protocol::social::groups::resolve_thread_names(&s_snap, &mut threads).await;
        }
        if !send2me_id.is_empty() {
            for t in threads.iter_mut() {
                if !t.is_group && t.name.contains("Cloud của tôi") {
                    t.id = send2me_id.clone();
                }
            }
        }
        let now_ts = unix_now();
        for t in threads.iter_mut() {
            if let Some(&until) = settings.muted_threads.get(&t.id) {
                if until == -1 || until > now_ts {
                    t.presentation.muted = true;
                    t.unread.muted = true;
                    t.unread.muted_until = (until > 0).then_some(until);
                }
            } else if t.unread.is_muted() {
                t.presentation.muted = true;
            }
        }
        crate::protocol::messaging::threads::deduplicate_threads(&mut threads, None);
        self.set_known_threads(
            threads
                .iter()
                .map(|t| (t.id.clone(), t.name.clone(), t.is_group))
                .collect(),
        );

        let saved_accounts = crate::services::account_manager::list_saved_accounts(&uin);
        let db_path = self.resolve_db_path_for_uin(&uin);
        let popular_stickers = POPULAR_STICKERS.iter().map(|p| p.to_item()).collect();
        let onion_stickers = ONION_STICKERS.iter().map(|p| p.to_item()).collect();
        let mut pinned_packs = Vec::new();
        for custom in load_custom_sticker_packs() {
            if let Some(items) = builtin_pack_stickers(custom.cate_id) {
                pinned_packs.push(PinnedPackMeta {
                    cate_id: custom.cate_id,
                    name: custom.name,
                    items,
                });
            }
        }
        for &(cid, title) in FEATURED_PACK_TABS {
            if pinned_packs.iter().any(|p| p.cate_id == cid) {
                continue;
            }
            if let Some(items) = builtin_pack_stickers(cid) {
                pinned_packs.push(PinnedPackMeta {
                    cate_id: cid,
                    name: title.to_string(),
                    items,
                });
            }
        }

        let (inline_video_supported, inline_video_reason) =
            tokio::task::spawn_blocking(detect_inline_video_support)
                .await
                .unwrap_or((true, None));

        CoreBootstrapSnapshot {
            app_version: env!("CARGO_PKG_VERSION"),
            dev_mode: crate::logger::is_dev_mode(),
            logged_in,
            uin,
            display_name,
            avatar,
            threads,
            settings,
            saved_accounts,
            popular_stickers,
            onion_stickers,
            pinned_packs,
            app_logo_data_url: embedded_app_logo_data_url(),
            active_db_path: db_path,
            sync_running: crate::protocol::sync::RUNNING.load(Ordering::Relaxed),
            phone_sync_running: PHONE_SYNC_RUNNING.load(Ordering::Relaxed),
            inline_video_supported,
            inline_video_reason,
        }
    }

    /// Save UI settings, synchronize runtime privacy blockers, and return the persisted copy.
    pub fn save_settings(&self, settings: AppSettings) -> AppSettings {
        settings.apply_privacy_blockers();
        settings.save();
        settings
    }

    /// Import local SQLite/JSON Zalo history backup into the active account's SQLite DB.
    pub async fn import_history_from_path(
        &self,
        src_path: PathBuf,
    ) -> std::result::Result<ImportSummaryDto, String> {
        if !src_path.exists() {
            return Err(format!("Đường dẫn không tồn tại: {}", src_path.display()));
        }
        let db_path = self.active_db_path().await;
        let src_display = src_path.display().to_string();
        let report = tokio::task::spawn_blocking(move || {
            crate::storage::import_history_from_path(&src_path, &db_path)
        })
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())?;

        let threads = self.list_threads(MessageStore::PRELOAD_MSGS).await;
        self.emit(CoreEvent::ThreadsUpdated(threads));
        let dto = ImportSummaryDto {
            threads_imported: report.threads_imported,
            messages_imported: report.messages_imported,
            source_path: src_display,
        };
        self.emit(CoreEvent::StatusUpdate(format!(
            "Đã nhập thành công {} tin nhắn từ {} hội thoại!",
            dto.messages_imported, dto.threads_imported
        )));
        Ok(dto)
    }

    /// Scan common Linux/Wine/Flatpak Zalo directories and import the first existing store.
    pub async fn auto_detect_and_import_history(
        &self,
    ) -> std::result::Result<ImportSummaryDto, String> {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
        let home_p = PathBuf::from(home);
        let candidates = [
            home_p.join(".local/share/zalo"),
            home_p.join(".local/share/Zalo"),
            home_p.join(".config/zalo"),
            home_p.join(".config/Zalo"),
            home_p.join(".var/app/com.vng.zalo/data/zalo"),
            home_p.join(".var/app/com.vng.zalo/config/zalo"),
            home_p
                .join(".wine/drive_c/users")
                .join(std::env::var("USER").unwrap_or_default())
                .join("AppData/Roaming/Zalo"),
            home_p.join("zalo_history.db"),
            home_p.join("threads_cache.json"),
        ];
        for cand in candidates {
            if cand.exists() {
                return self.import_history_from_path(cand).await;
            }
        }
        Err("Không tìm thấy thư mục Zalo mặc định. Vui lòng nhập đường dẫn thủ công.".into())
    }

    /// Return diagnostic HTTP logs, scan findings, and public protocol parameters (never secrets).
    pub fn dev_diagnostics(&self) -> DevDiagnosticsSnapshot {
        DevDiagnosticsSnapshot {
            dev_mode: crate::logger::is_dev_mode(),
            logs: crate::logger::get_entries(),
            findings: crate::logger::scan_logs(),
            crypto: DevCryptoInfo {
                zpw_ver: crate::session::ZPW_VER,
                zpw_type: crate::session::ZPW_TYPE,
                cipher_algo: "AES-128-CBC / AES-256-CBC (PKCS7, Zero IV)",
                sign_key_formula: "md5('zsecure' + tag + sorted_values)",
                param_cipher_key_len: crate::param_cipher::PARAM_CIPHER_KEY.len(),
            },
        }
    }
}
