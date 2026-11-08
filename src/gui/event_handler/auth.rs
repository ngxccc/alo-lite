use super::*;

impl ZaloGuiApp {
    pub(super) fn handle_auth_event(&mut self, ctx: &egui::Context, event: GuiEvent) {
        match event {
            GuiEvent::QrReady { qr_image, .. } => {
                self.qr_texture =
                    Some(ctx.load_texture("qr_code_tex", qr_image, TextureOptions::NEAREST));
                self.qr_status = "Mở ứng dụng Zalo trên điện thoại và quét mã QR".into();
                self.qr_scanned_name = None;
                self.login_in_progress = false;
            }
            GuiEvent::QrScanned { name, avatar } => {
                self.qr_scanned_name = Some(name.clone());
                self.qr_scanned_avatar = avatar;
                self.qr_status =
                    format!("Đã quét bởi {name} — Vui lòng bấm 'Đăng nhập' trên điện thoại…");
            }
            GuiEvent::QrExpired(reason) => {
                self.login_in_progress = false;
                self.qr_texture = None;
                self.qr_scanned_name = None;
                self.qr_scanned_avatar = None;
                self.qr_status = format!("{reason}.");
            }
            GuiEvent::LoginSuccess { name, uin, avatar } => {
                self.is_logged_in = true;
                self.qr_scanned_name = None;
                self.qr_scanned_avatar = None;
                ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(Vec2::new(1020.0, 720.0)));
                ctx.send_viewport_cmd(egui::ViewportCommand::MinInnerSize(Vec2::new(720.0, 500.0)));
                self.user_name = name;
                self.user_uin = uin;
                if avatar.is_some() {
                    self.user_avatar = avatar;
                }
                self.threads.clear();
                self.unread_counts.clear();
                self.history_reached_start.clear();
                self.selected_thread_index = 0;
                self.login_in_progress = false;
                self.composer_sticker_cache.retain(|_, v| !v.is_empty());
                self.status_msg = "Trực tuyến · Đồng bộ thời gian thực".into();
            }
            GuiEvent::LoginFailed(err) => {
                self.login_in_progress = false;
                self.login_status_msg = format_user_friendly_login_error(&err);
            }
            GuiEvent::LoggedOut => {
                self.is_logged_in = false;
                self.user_name.clear();
                self.user_uin.clear();
                self.threads.clear();
                self.selected_thread_index = 0;
                self.qr_texture = None;
                self.qr_status = "Đang xin mã QR mới…".into();
                self.status_msg = "Đã đăng xuất".into();
            }
            GuiEvent::AccountSwitched {
                uin,
                name,
                avatar,
                threads,
            } => {
                self.is_logged_in = true;
                self.user_uin = uin;
                self.user_name = name.clone();
                self.user_avatar = avatar;
                self.threads = threads;
                self.selected_thread_index = 0;
                self.status_msg = format!("Đã chuyển sang tài khoản {name}");
                self.show_toast(format!("Đã chuyển sang tài khoản {name}"));
                self.history_reached_start.clear();
            }
            GuiEvent::SessionExpired(reason) => {
                self.is_logged_in = false;
                self.qr_texture = None;
                self.qr_status =
                    format!("{reason} — Quét mã QR bên dưới để khôi phục phiên trực tuyến.");
                self.status_msg = format!("{reason} (Lịch sử tin nhắn vẫn được bảo toàn)");
            }
            GuiEvent::SessionUnreachable(reason) => {
                // Only the network failed: stay logged in, keep the thread list, and
                // let the next poll retry (#21).
                self.status_msg = format!("{reason} — đang thử lại…");
            }
            GuiEvent::LocalDataWiped => {
                self.threads.clear();
                self.selected_thread_index = 0;
                self.status_msg = "Đã xoá dữ liệu tin nhắn cục bộ (session giữ nguyên)".into();
            }
            _ => unreachable!(),
        }
    }
}
