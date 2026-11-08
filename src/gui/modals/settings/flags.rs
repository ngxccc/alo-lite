use crate::gui::theme::*;
use crate::gui::*;

#[derive(Default)]
pub(super) struct SettingsActionFlags {
    pub do_logout: bool,
    pub pick_dl_dir: bool,
    pub open_dl_dir: bool,
    pub pick_migrate_file: bool,
    pub run_migrate_input: bool,
    pub auto_detect_zalo_dir: bool,
    pub export_theme_tpl: bool,
    pub reload_theme_json: bool,
    pub export_sticker_tpl: bool,
    pub reload_sticker_json: bool,
    pub export_autoreply_tpl: bool,
    pub spawn_multi_zalo: bool,
    pub trigger_sync: bool,
    pub trigger_phone_sync: bool,
    pub apply_proxy_now: bool,
    pub clear_proxy_now: bool,
    pub add_new_account: bool,
    pub switch_to_uin: Option<String>,
    pub theme_switched: bool,
}

impl ZaloGuiApp {
    pub(super) fn handle_settings_actions(&mut self, flags: SettingsActionFlags) {
        let do_logout = flags.do_logout;
        let trigger_sync = flags.trigger_sync;
        let trigger_phone_sync = flags.trigger_phone_sync;
        let apply_proxy_now = flags.apply_proxy_now;
        let clear_proxy_now = flags.clear_proxy_now;
        let spawn_multi_zalo = flags.spawn_multi_zalo;
        let pick_dl_dir = flags.pick_dl_dir;
        let open_dl_dir = flags.open_dl_dir;
        let pick_migrate_file = flags.pick_migrate_file;
        let run_migrate_input = flags.run_migrate_input;
        let auto_detect_zalo_dir = flags.auto_detect_zalo_dir;
        let export_theme_tpl = flags.export_theme_tpl;
        let reload_theme_json = flags.reload_theme_json;
        let export_sticker_tpl = flags.export_sticker_tpl;
        let reload_sticker_json = flags.reload_sticker_json;
        let export_autoreply_tpl = flags.export_autoreply_tpl;
        let add_new_account = flags.add_new_account;
        let switch_to_uin = flags.switch_to_uin;
        let theme_switched = flags.theme_switched;

        if do_logout {
            self.settings_modal_open = false;
            self.is_logged_in = false;
            self.user_name.clear();
            self.user_uin.clear();
            self.threads.clear();
            self.selected_thread_index = 0;
            self.qr_texture = None;
            self.login_status_msg.clear();
            self.qr_status = "Đang xin mã QR mới…".into();
            self.status_msg = "Đã đăng xuất".into();
            if let Some(tx) = WORKER_TX.get() {
                let _ = tx.send(WorkerCmd::Logout);
                let _ = tx.send(WorkerCmd::RequestQr);
            }
        }
        if let Some(target) = switch_to_uin {
            self.settings_modal_open = false;
            if let Some(tx) = WORKER_TX.get() {
                let _ = tx.send(WorkerCmd::SwitchAccount { target_uin: target });
            }
        }
        if add_new_account {
            self.settings_modal_open = false;
            let _ = crate::account_manager::save_current_account_snapshot(&self.user_uin);
            self.is_logged_in = false;
            self.user_name.clear();
            self.user_uin.clear();
            self.threads.clear();
            self.selected_thread_index = 0;
            self.qr_texture = None;
            self.login_status_msg.clear();
            self.qr_status = "Đang xin mã QR mới…".into();
            self.status_msg = "Đang xin mã QR đăng nhập tài khoản mới…".into();
            if let Some(tx) = WORKER_TX.get() {
                let _ = tx.send(WorkerCmd::Logout);
                let _ = tx.send(WorkerCmd::RequestQr);
            }
        }
        if theme_switched {
            self.theme_dirty = true;
            self.settings.save();
            self.show_toast(if self.dark_mode {
                "Đã chuyển sang giao diện Tối (Atom One Dark)"
            } else {
                "Đã chuyển sang giao diện Sáng (Zalo Light)"
            });
        }
        if trigger_sync {
            let threads: Vec<(String, String, bool)> = self
                .threads
                .iter()
                .filter(|t| t.is_group || t.name.contains("Cloud của tôi"))
                .map(|t| (t.id.clone(), t.name.clone(), t.is_group))
                .collect();
            let n = threads.len();
            if let Some(tx) = WORKER_TX.get() {
                let _ = tx.send(WorkerCmd::SyncHistory { threads });
            }
            self.show_toast(format!("Bắt đầu đồng bộ lịch sử {n} hội thoại nhóm/Cloud…"));
        }
        if trigger_phone_sync {
            self.phone_sync_progress = 0.05;
            self.phone_sync_status = "Đang gửi yêu cầu đồng bộ tin nhắn tới điện thoại…".into();
            let threads = self.threads.clone();
            if let Some(tx) = WORKER_TX.get() {
                let _ = tx.send(WorkerCmd::SyncPhoneMessages {
                    threads,
                    send_phone_prompt: true,
                });
            }
            self.show_toast("Đang yêu cầu đồng bộ tin nhắn từ điện thoại…");
        }
        if apply_proxy_now {
            let proxy_url = self.settings.proxy_url.trim().to_string();
            if let Some(tx) = WORKER_TX.get() {
                let _ = tx.send(WorkerCmd::SetProxy { proxy_url });
            }
        }
        if clear_proxy_now {
            self.settings.proxy_url.clear();
            self.settings.save();
            if let Some(tx) = WORKER_TX.get() {
                let _ = tx.send(WorkerCmd::SetProxy {
                    proxy_url: String::new(),
                });
            }
        }
        if spawn_multi_zalo {
            let profile = self.settings.multi_profile_name.trim().to_string();
            if profile.is_empty() {
                self.show_toast("Vui lòng nhập tên Profile (ví dụ: acc2)");
            } else if let Ok(exe) = std::env::current_exe() {
                let mut cmd = std::process::Command::new(exe);
                cmd.arg("--gui").arg("--profile").arg(&profile);
                if !self.settings.proxy_url.trim().is_empty() {
                    cmd.arg("--proxy").arg(self.settings.proxy_url.trim());
                }
                match cmd.spawn() {
                    Ok(_) => {
                        self.settings.save();
                        self.show_toast(format!("Đã mở cửa sổ Zalo mới (Profile: {profile})"));
                    }
                    Err(e) => {
                        self.show_toast(format!("Không mở được cửa sổ mới: {e}"));
                    }
                }
            }
        }
        if pick_dl_dir {
            if let Some(folder) = pick_native_file_dialog(false) {
                let dir_path = folder.parent().unwrap_or(&folder).to_path_buf();
                self.settings.download_dir = dir_path.display().to_string();
                self.settings.save();
                self.show_toast(format!(
                    "Đã đổi thư mục lưu file: {}",
                    self.settings.download_dir
                ));
            }
        }
        if open_dl_dir {
            let p = std::path::PathBuf::from(&self.settings.download_dir);
            let _ = std::fs::create_dir_all(&p);
            #[cfg(target_os = "windows")]
            {
                let _ = std::process::Command::new("explorer").arg(&p).spawn();
            }
            #[cfg(target_os = "macos")]
            {
                let _ = std::process::Command::new("open").arg(&p).spawn();
            }
            #[cfg(not(any(target_os = "windows", target_os = "macos")))]
            {
                let _ = std::process::Command::new("xdg-open").arg(&p).spawn();
            }
        }

        let active_db = crate::store::db_path_for_account(&self.user_uin);
        if pick_migrate_file {
            if let Some(src_path) = pick_native_file_dialog(false) {
                self.migrate_path_input = src_path.display().to_string();
                self.execute_import_path(&src_path, &active_db);
            }
        }
        if run_migrate_input {
            let p = self.migrate_path_input.trim().to_string();
            if !p.is_empty() {
                let src_path = std::path::PathBuf::from(&p);
                self.execute_import_path(&src_path, &active_db);
            } else {
                self.show_toast("Vui lòng nhập hoặc chọn đường dẫn file/thư mục sao lưu");
            }
        }
        if auto_detect_zalo_dir {
            self.auto_detect_and_import(&active_db);
        }
        if export_theme_tpl {
            match ThemePalette::export_template("theme.json") {
                Ok(_) => self.show_toast("Đã xuất mẫu theme.json tại thư mục ứng dụng"),
                Err(e) => self.show_toast(format!("Lỗi xuất theme.json: {e}")),
            }
        }
        if reload_theme_json {
            self.theme_dirty = true;
            self.show_toast("Đã áp dụng lại bảng màu từ theme.json");
        }
        if export_sticker_tpl {
            match export_custom_stickers_template("custom_stickers.json") {
                Ok(_) => self.show_toast("Đã xuất mẫu custom_stickers.json tại thư mục ứng dụng"),
                Err(e) => self.show_toast(format!("Lỗi xuất custom_stickers.json: {e}")),
            }
        }
        if reload_sticker_json {
            let n = self.reload_custom_sticker_packs();
            self.show_toast(format!(
                "Đã nạp {n} bộ Sticker Pack tùy chỉnh từ custom_stickers.json"
            ));
        }
        if export_autoreply_tpl {
            let mut rules = crate::api_mcp::load_autoreply_rules();
            if rules.is_empty() {
                rules.push(crate::api_mcp::AutoReplyRule {
                    id: "default_busy".into(),
                    keyword: "/busy".into(),
                    match_mode: "contains".into(),
                    reply_text: "Hiện mình đang bận, mình sẽ phản hồi sớm nhé!".into(),
                    enabled: false,
                    scope: "direct".into(),
                });
            }
            let _ = crate::api_mcp::save_autoreply_rules(&rules);
            self.show_toast("Đã lưu cấu hình mẫu autoreply_rules.json");
        }
    }

    fn execute_import_path(&mut self, src_path: &std::path::Path, active_db: &str) {
        if !src_path.exists() {
            self.show_toast(format!("Đường dẫn không tồn tại: {}", src_path.display()));
            return;
        }
        match crate::store::import_history_from_path(src_path, active_db) {
            Ok(summary) => {
                let t_cnt = summary.threads_imported;
                let m_cnt = summary.messages_imported;
                crate::store::load_history_into_threads(
                    active_db,
                    &mut self.threads,
                    crate::store::MessageStore::PRELOAD_MSGS,
                );
                self.persist_threads_async();
                self.show_toast(format!(
                    "Đã nhập thành công {m_cnt} tin nhắn từ {t_cnt} hội thoại!"
                ));
            }
            Err(e) => {
                self.show_toast(format!("Lỗi nhập lịch sử: {e}"));
            }
        }
    }

    fn auto_detect_and_import(&mut self, active_db: &str) {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
        let home_p = std::path::PathBuf::from(home);
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
        for cand in &candidates {
            if cand.exists() {
                self.migrate_path_input = cand.display().to_string();
                self.execute_import_path(cand, active_db);
                return;
            }
        }
        self.show_toast("Không tìm thấy thư mục Zalo mặc định. Vui lòng nhập đường dẫn thủ công.");
    }
}
