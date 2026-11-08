//! Login view for AloLite (Streamlined QR Code login).

use crate::gui::components::{buttons, cards, inputs};
use crate::gui::theme::{lucide, ThemePalette};
use crate::gui::worker::{WorkerCmd, WORKER_TX};
use crate::gui::{LoginTab, ZaloGuiApp};
use eframe::egui::{
    self, Align, Align2, Color32, CursorIcon, FontId, Layout, Margin, RichText, Rounding, Sense,
    Stroke, Ui, Vec2,
};

impl ZaloGuiApp {
    /// Renders the entire login screen (Streamlined QR Code experience).
    pub(super) fn render_login_screen(&mut self, ui: &mut Ui, pal: &ThemePalette) {
        ui.vertical_centered(|ui| {
            let avail_h = ui.available_height();
            // Optical vertical center: keep the block balanced when window is tall
            let content_height = 550.0;
            let top_space = if avail_h > content_height {
                ((avail_h - content_height) * 0.42).max(14.0)
            } else {
                12.0
            };
            ui.add_space(top_space);

            // App Logo
            if let Some(tex) = &self.app_logo_texture {
                ui.image((tex.id(), Vec2::new(60.0, 60.0)));
                ui.add_space(8.0);
            }

            ui.label(
                RichText::new("Đăng nhập AloLite")
                    .color(pal.primary)
                    .size(24.0)
                    .strong(),
            );
            ui.add_space(3.0);
            ui.label(
                RichText::new("Kết nối tài khoản Zalo của bạn qua mã QR an toàn")
                    .color(pal.text_secondary)
                    .size(13.0),
            );
            ui.add_space(16.0);

            // Elevated Card Container
            cards::elevated_card(ui, pal, Some(350.0), (22.0, 22.0), |ui| {
                ui.vertical_centered(|ui| {
                    if self.login_tab == LoginTab::Password {
                        self.render_password_tab(ui, pal);
                    } else {
                        self.render_qr_tab(ui, pal);
                    }
                });
            });

            ui.add_space(12.0);

            // Subtle alternative login link at bottom
            if self.login_tab == LoginTab::Password {
                if ui
                    .link(
                        RichText::new(format!("{} Quay lại Quét mã QR", lucide::REFRESH))
                            .size(12.5)
                            .color(pal.primary),
                    )
                    .clicked()
                {
                    self.login_tab = LoginTab::QrCode;
                }
            } else if ui
                .link(
                    RichText::new("Đăng nhập bằng số điện thoại (Thử nghiệm)")
                        .size(12.0)
                        .color(pal.text_muted),
                )
                .clicked()
            {
                self.login_tab = LoginTab::Password;
            }

            ui.add_space(8.0);
            ui.label(
                RichText::new("AloLite là dự án độc lập, không trực thuộc VNG Corporation.")
                    .size(10.5)
                    .color(pal.text_muted.linear_multiply(0.8)),
            );
        });
    }

    fn render_qr_tab(&mut self, ui: &mut Ui, pal: &ThemePalette) {
        // STATE 1: Phone scanned the QR code -> Show user profile & waiting for confirmation
        if let Some(name) = &self.qr_scanned_name {
            ui.add_space(10.0);

            // User Avatar Circle
            // Render real avatar image or dynamic colored initials badge
            let (ava_rect, _) = ui.allocate_exact_size(Vec2::new(76.0, 76.0), Sense::hover());
            crate::gui::theme::paint_user_avatar(
                ui.painter(),
                ava_rect.center(),
                38.0,
                name,
                false,
                self.qr_scanned_avatar.as_deref(),
                &self.sticker_textures,
                &mut self.loading_stickers,
            );

            ui.add_space(12.0);
            ui.label(
                RichText::new(name)
                    .size(18.0)
                    .color(pal.text_primary)
                    .strong(),
            );

            ui.add_space(6.0);
            ui.label(
                RichText::new(format!("{}  Đã quét thành công", lucide::CHECK))
                    .size(13.0)
                    .color(pal.accent_online)
                    .strong(),
            );

            ui.add_space(14.0);
            egui::Frame::none()
                .fill(pal.bg_input)
                .rounding(Rounding::same(10.0))
                .stroke(Stroke::new(1.0, pal.border))
                .inner_margin(Margin::symmetric(16.0, 12.0))
                .show(ui, |ui| {
                    ui.vertical_centered(|ui| {
                        ui.label(
                            RichText::new("Vui lòng nhấn 'Đăng nhập' trên ứng dụng Zalo\nđiện thoại để tiếp tục.")
                                .size(12.5)
                                .color(pal.text_secondary),
                        );
                    });
                });

            ui.add_space(14.0);
            if ui
                .add(
                    egui::Button::new(
                        RichText::new("Hủy / Quét lại tài khoản khác")
                            .size(12.0)
                            .color(pal.text_muted),
                    )
                    .frame(false),
                )
                .on_hover_cursor(CursorIcon::PointingHand)
                .clicked()
            {
                self.login_in_progress = true;
                self.qr_scanned_name = None;
                self.qr_status = "Đang tạo mã QR mới…".into();
                if let Some(tx) = WORKER_TX.get() {
                    let _ = tx.send(WorkerCmd::RequestQr);
                }
            }
            return;
        }

        // STATE 2: Active QR code is available
        if let Some(tex) = &self.qr_texture {
            egui::Frame::none()
                .fill(Color32::WHITE)
                .rounding(Rounding::same(12.0))
                .stroke(Stroke::new(1.0, pal.border))
                .inner_margin(Margin::same(14.0))
                .show(ui, |ui| {
                    ui.image((tex.id(), Vec2::new(230.0, 230.0)));
                });

            ui.add_space(14.0);
            ui.label(
                RichText::new("Mở ứng dụng Zalo trên điện thoại")
                    .color(pal.text_primary)
                    .size(13.5)
                    .strong(),
            );
            ui.add_space(2.0);
            ui.label(
                RichText::new("Chọn biểu tượng Quét mã QR để đăng nhập")
                    .color(pal.text_secondary)
                    .size(12.5),
            );

            ui.add_space(10.0);
            // Subtle refresh link instead of giant button
            if ui
                .add(
                    egui::Button::new(
                        RichText::new(format!("{} Làm mới mã", lucide::REFRESH))
                            .size(12.0)
                            .color(pal.text_muted),
                    )
                    .frame(false),
                )
                .on_hover_cursor(CursorIcon::PointingHand)
                .clicked()
            {
                self.login_in_progress = true;
                self.qr_status = "Đang xin mã QR…".into();
                if let Some(tx) = WORKER_TX.get() {
                    let _ = tx.send(WorkerCmd::RequestQr);
                }
            }
        } else {
            // STATE 3: Expired / Loading / Error
            let (rect, _) = ui.allocate_exact_size(Vec2::new(258.0, 258.0), Sense::hover());
            ui.painter()
                .rect_filled(rect, Rounding::same(12.0), pal.bg_input);
            ui.painter()
                .rect_stroke(rect, Rounding::same(12.0), Stroke::new(1.0, pal.border));

            if self.login_in_progress {
                ui.painter().text(
                    rect.center(),
                    Align2::CENTER_CENTER,
                    "Đang tạo mã QR…",
                    FontId::proportional(14.0),
                    pal.text_secondary,
                );
            } else {
                let center_y = rect.center().y;
                ui.painter().text(
                    Vec2::new(rect.center().x, center_y - 20.0).to_pos2(),
                    Align2::CENTER_CENTER,
                    lucide::ALERT_TRIANGLE,
                    FontId::proportional(36.0),
                    pal.warning,
                );
                ui.painter().text(
                    Vec2::new(rect.center().x, center_y + 16.0).to_pos2(),
                    Align2::CENTER_CENTER,
                    "Mã QR đã hết hạn",
                    FontId::proportional(14.0),
                    pal.warning,
                );
            }

            ui.add_space(14.0);
            if !self.login_in_progress {
                ui.label(
                    RichText::new("Mã QR đã hết hạn vì lý do bảo mật.")
                        .color(pal.text_secondary)
                        .size(12.5),
                );
                ui.add_space(10.0);
                if buttons::primary_button(ui, "Tạo mã QR mới", pal, Some(180.0), true).clicked()
                {
                    self.login_in_progress = true;
                    self.qr_status = "Đang xin mã QR…".into();
                    if let Some(tx) = WORKER_TX.get() {
                        let _ = tx.send(WorkerCmd::RequestQr);
                    }
                }
            }
        }
    }

    /// Renders the Phone / Password login tab with perfectly centered inputs.
    fn render_password_tab(&mut self, ui: &mut Ui, pal: &ThemePalette) {
        ui.with_layout(Layout::top_down(Align::Min), |ui| {
            let phone_resp = inputs::labeled_text_input(
                ui,
                "Số điện thoại",
                &mut self.phone_input,
                "Ví dụ: 0912345678",
                false,
                pal,
            );

            let pass_resp = inputs::labeled_text_input(
                ui,
                "Mật khẩu",
                &mut self.password_input,
                "Nhập mật khẩu Zalo",
                true,
                pal,
            );

            let enter_pressed = ui.input(|i| i.key_pressed(egui::Key::Enter));
            let mut submit_requested = false;

            if enter_pressed {
                if pass_resp.has_focus() || pass_resp.lost_focus() {
                    submit_requested = true;
                } else if phone_resp.has_focus() || phone_resp.lost_focus() {
                    if self.password_input.trim().is_empty() {
                        pass_resp.request_focus();
                    } else {
                        submit_requested = true;
                    }
                }
            }

            if !self.login_status_msg.is_empty() {
                ui.label(
                    RichText::new(&self.login_status_msg)
                        .color(pal.danger)
                        .size(12.5),
                );
                ui.add_space(8.0);
            }

            ui.add_space(4.0);
            let btn_clicked =
                buttons::primary_button(ui, "Đăng nhập", pal, None, !self.login_in_progress)
                    .clicked();
            if (btn_clicked || submit_requested) && !self.login_in_progress {
                if self.phone_input.trim().is_empty() {
                    self.login_status_msg = "Vui lòng nhập số điện thoại".into();
                } else if self.password_input.is_empty() {
                    self.login_status_msg = "Vui lòng nhập mật khẩu".into();
                } else {
                    self.login_in_progress = true;
                    self.login_status_msg = "Đang kết nối…".into();
                    if let Some(tx) = WORKER_TX.get() {
                        let _ = tx.send(WorkerCmd::LoginPassword {
                            phone: self.phone_input.clone(),
                            pass: self.password_input.clone(),
                        });
                    }
                }
            }
        });
    }
}
