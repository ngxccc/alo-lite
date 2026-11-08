use crate::gui::theme::*;
use crate::gui::*;
use eframe::egui;

impl ZaloGuiApp {
    pub(in crate::gui) fn render_profile_popover(
        &mut self,
        ctx: &egui::Context,
        pal: &ThemePalette,
    ) {
        if !self.profile_menu_open {
            return;
        }
        if ctx.input(|i| i.key_pressed(Key::Escape)) {
            self.profile_menu_open = false;
            return;
        }

        let mut close_menu = false;
        let mut do_logout = false;
        let mut do_wipe = false;
        let area_resp = egui::Area::new(Id::new("left_rail_profile_popover"))
            .order(egui::Order::Foreground)
            .fixed_pos(Pos2::new(68.0, 44.0))
            .show(ctx, |ui| {
                egui::Frame::none()
                    .fill(pal.bg_elevated)
                    .stroke(Stroke::new(1.0_f32, pal.border))
                    .rounding(Rounding::same(12.0))
                    .inner_margin(Margin::same(10.0))
                    .show(ui, |ui| {
                        ui.set_width(220.0);
                        let display_user = if self.user_name.is_empty() {
                            "Người dùng Zalo"
                        } else {
                            &self.user_name
                        };
                        ui.label(
                            RichText::new(display_user)
                                .size(14.0)
                                .color(pal.text_primary)
                                .strong(),
                        );
                        ui.horizontal(|ui| {
                            let (dr, _) = ui.allocate_exact_size(Vec2::splat(8.0), Sense::hover());
                            ui.painter()
                                .circle_filled(dr.center(), 3.8, pal.accent_online);
                            ui.label(
                                RichText::new("Đang hoạt động")
                                    .size(11.5)
                                    .color(pal.text_secondary),
                            );
                        });
                        if !self.user_uin.is_empty() {
                            ui.label(
                                RichText::new(format!("UIN: {}", self.user_uin))
                                    .size(10.5)
                                    .color(pal.text_muted),
                            );
                        }

                        ui.add_space(6.0);
                        ui.separator();
                        ui.add_space(4.0);
                        if ui
                            .add_sized(
                                [200.0, 28.0],
                                egui::Button::new(
                                    RichText::new(format!("{}  Quản lý & Đổi tài khoản", lucide::USER))
                                        .size(12.2)
                                        .color(pal.text_primary),
                                )
                                .fill(Color32::TRANSPARENT),
                            )
                            .on_hover_cursor(CursorIcon::PointingHand)
                            .clicked()
                        {
                            self.settings_modal_open = true;
                            self.settings_tab = SettingsTab::Account;
                            close_menu = true;
                        }

                        if ui
                            .add_sized(
                                [200.0, 28.0],
                                egui::Button::new(
                                    RichText::new(format!("{}  Làm mới hội thoại", lucide::REFRESH))
                                        .size(12.2)
                                        .color(pal.text_primary),
                                )
                                .fill(Color32::TRANSPARENT),
                            )
                            .on_hover_cursor(CursorIcon::PointingHand)
                            .on_hover_text("Làm mới danh bạ, nhóm và tin nhắn mới từ máy chủ Zalo")
                            .clicked()
                        {
                            if let Some(tx) = WORKER_TX.get() {
                                let _ = tx.send(WorkerCmd::RefreshThreadsManual);
                            }
                            self.status_msg = "⏳ Đang làm mới danh sách hội thoại…".into();
                            close_menu = true;
                        }

                        let theme_lbl = if self.dark_mode {
                            format!("{}  Chuyển giao diện Sáng", lucide::SUN)
                        } else {
                            format!("{}  Chuyển giao diện Tối", lucide::MOON)
                        };
                        if ui
                            .add_sized(
                                [200.0, 28.0],
                                egui::Button::new(
                                    RichText::new(theme_lbl).size(12.2).color(pal.text_primary),
                                )
                                .fill(Color32::TRANSPARENT),
                            )
                            .on_hover_cursor(CursorIcon::PointingHand)
                            .clicked()
                        {
                            self.dark_mode = !self.dark_mode;
                            self.theme_dirty = true;
                            close_menu = true;
                        }
                        let has_upd = self
                            .update_info
                            .as_ref()
                            .map(|u| u.has_update)
                            .unwrap_or(false);
                        let upd_lbl = if has_upd {
                            format!(
                                "{}  Cập nhật mới (v{})",
                                lucide::SPARKLES,
                                self.update_info
                                    .as_ref()
                                    .map(|u| u.latest_version.as_str())
                                    .unwrap_or("")
                            )
                        } else {
                            format!(
                                "{}  Kiểm tra cập nhật (v{})",
                                lucide::REFRESH,
                                env!("CARGO_PKG_VERSION")
                            )
                        };
                        if ui
                            .add_sized(
                                [200.0, 28.0],
                                egui::Button::new(
                                    RichText::new(upd_lbl).size(12.2).color(if has_upd {
                                        pal.accent_online
                                    } else {
                                        pal.text_primary
                                    }),
                                )
                                .fill(Color32::TRANSPARENT),
                            )
                            .on_hover_cursor(CursorIcon::PointingHand)
                            .clicked()
                        {
                            self.settings_modal_open = true;
                            self.settings_tab = SettingsTab::Update;
                            self.update_checking = true;
                            self.update_status =
                                "⏳ Đang kiểm tra bản cập nhật mới trên GitHub Releases…".into();
                            if let Some(tx) = WORKER_TX.get() {
                                let _ = tx.send(WorkerCmd::CheckForUpdates { silent: false });
                            }
                            close_menu = true;
                        }

                        if self.dev_mode {
                            let dev_lbl = if self.dev_panel_open {
                                "Đóng Dev Inspector"
                            } else {
                                "Mở Dev Inspector"
                            };
                            if ui
                                .add_sized(
                                    [200.0, 28.0],
                                    egui::Button::new(
                                        RichText::new(dev_lbl).size(12.2).color(pal.warning),
                                    )
                                    .fill(Color32::TRANSPARENT),
                                )
                                .on_hover_cursor(CursorIcon::PointingHand)
                                .clicked()
                            {
                                self.dev_panel_open = !self.dev_panel_open;
                                if self.dev_panel_open {
                                    self.scanner_findings = logger::scan_logs();
                                }
                                close_menu = true;
                            }
                        }

                        ui.add_space(4.0);
                        ui.separator();
                        ui.add_space(4.0);

                        if ui
                            .add_sized(
                                [200.0, 30.0],
                                egui::Button::new(
                                    RichText::new("Xoá dữ liệu tin nhắn cục bộ")
                                        .size(12.0)
                                        .color(pal.text_secondary),
                                )
                                .fill(pal.bg_input)
                                .stroke(Stroke::new(1.0_f32, pal.border))
                                .rounding(Rounding::same(7.0)),
                            )
                            .on_hover_text(
                                "Xoá zalo_history.db và threads_cache.json. Phiên đăng nhập giữ nguyên.",
                            )
                            .on_hover_cursor(CursorIcon::PointingHand)
                            .clicked()
                        {
                            do_wipe = true;
                            close_menu = true;
                        }
                        ui.add_space(4.0);
                        ui.separator();
                        ui.add_space(4.0);

                        if ui
                            .add_sized(
                                [200.0, 30.0],
                                egui::Button::new(
                                    RichText::new("Đăng xuất tài khoản")
                                        .size(12.5)
                                        .color(pal.danger)
                                        .strong(),
                                )
                                .fill(pal.danger.linear_multiply(0.12))
                                .rounding(Rounding::same(7.0)),
                            )
                            .on_hover_cursor(CursorIcon::PointingHand)
                            .clicked()
                        {
                            do_logout = true;
                            close_menu = true;
                        }
                        ui.add_space(4.0);
                        if ui
                            .add_sized(
                                [200.0, 28.0],
                                egui::Button::new(
                                    RichText::new(format!(
                                        "{}  Thoát hoàn toàn (Ctrl+Q)",
                                        lucide::X
                                    ))
                                    .size(12.0)
                                    .color(pal.text_secondary),
                                )
                                .fill(pal.bg_input)
                                .stroke(Stroke::new(1.0_f32, pal.border))
                                .rounding(Rounding::same(7.0)),
                            )
                            .on_hover_cursor(CursorIcon::PointingHand)
                            .clicked()
                        {
                            self.allow_quit = true;
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                            std::process::exit(0);
                        }
                    });
            });

        if ctx.input(|i| i.pointer.any_click())
            && !area_resp.response.hovered()
            && ctx
                .input(|i| i.pointer.interact_pos())
                .map(|p| p.x > 64.0 || p.y > 60.0)
                .unwrap_or(false)
        {
            close_menu = true;
        }

        if do_logout {
            self.is_logged_in = false;
            self.user_name.clear();
            self.user_uin.clear();
            self.threads.clear();
            self.selected_thread_index = 0;
            self.qr_texture = None;
            self.login_status_msg.clear();
            self.qr_status = "Đang xin mã QR mới…".into();
            self.status_msg = "Đã đăng xuất".into();
            // `WorkerCmd::Logout` owns the file side: it removes `session.json` and
            // `threads_cache.json` but keeps `zalo_history.db` (#21).
            if let Some(tx) = WORKER_TX.get() {
                let _ = tx.send(WorkerCmd::Logout);
                let _ = tx.send(WorkerCmd::RequestQr);
            }
        }
        if do_wipe {
            // Destructive and irreversible, so it asks first.
            self.wipe_confirm_open = true;
        }
        if close_menu {
            self.profile_menu_open = false;
        }
    }
}
