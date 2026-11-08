use super::flags::SettingsActionFlags;
use crate::gui::theme::*;
use crate::gui::*;
use eframe::egui;

impl ZaloGuiApp {
    pub(super) fn render_tab_account(
        &mut self,
        ui: &mut egui::Ui,
        pal: &ThemePalette,
        flags: &mut SettingsActionFlags,
        _inner_w: f32,
    ) {
        ui.label(
            RichText::new("Tài khoản đang đăng nhập")
                .color(pal.text_primary)
                .size(13.5)
                .strong(),
        );
        ui.add_space(6.0);
        egui::Frame::none()
            .fill(pal.bg_elevated)
            .stroke(Stroke::new(1.0_f32, pal.border))
            .rounding(Rounding::same(10.0))
            .inner_margin(Margin::same(14.0))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    let (av_r, _) = ui.allocate_exact_size(Vec2::splat(46.0), Sense::hover());
                    let user_disp = if self.user_name.is_empty() {
                        "Người dùng Zalo"
                    } else {
                        &self.user_name
                    };
                    paint_user_avatar(
                        ui.painter(),
                        av_r.center(),
                        23.0,
                        user_disp,
                        false,
                        self.user_avatar.as_deref(),
                        &self.sticker_textures,
                        &mut self.loading_stickers,
                    );
                    ui.add_space(10.0);
                    ui.vertical(|ui| {
                        ui.label(
                            RichText::new(user_disp)
                                .size(15.0)
                                .color(pal.text_primary)
                                .strong(),
                        );
                        ui.horizontal(|ui| {
                            let (dr, _) = ui.allocate_exact_size(Vec2::splat(8.0), Sense::hover());
                            ui.painter()
                                .circle_filled(dr.center(), 3.5, pal.accent_online);
                            ui.label(
                                RichText::new("Đang hoạt động")
                                    .size(11.5)
                                    .color(pal.accent_online),
                            );
                            if !self.user_uin.is_empty() {
                                ui.label(
                                    RichText::new(format!("· UIN: {}", self.user_uin))
                                        .size(11.0)
                                        .color(pal.text_muted),
                                );
                            }
                        });
                    });
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        let lo_btn = egui::Button::new(
                            RichText::new("Đăng xuất tài khoản này")
                                .size(12.0)
                                .color(pal.danger)
                                .strong(),
                        )
                        .fill(pal.danger.linear_multiply(0.12))
                        .stroke(Stroke::new(1.0_f32, pal.danger.linear_multiply(0.35)))
                        .rounding(Rounding::same(7.0));
                        if ui
                            .add(lo_btn)
                            .on_hover_cursor(CursorIcon::PointingHand)
                            .clicked()
                        {
                            flags.do_logout = true;
                        }
                    });
                });
            });

        ui.add_space(18.0);
        ui.horizontal(|ui| {
            ui.label(
                RichText::new("Danh sách tài khoản (Đa tài khoản)")
                    .color(pal.text_primary)
                    .size(13.5)
                    .strong(),
            );
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let add_btn = egui::Button::new(
                    RichText::new(format!("{} Thêm tài khoản mới (Quét QR)", lucide::PLUS))
                        .size(11.8)
                        .color(pal.primary_hover)
                        .strong(),
                )
                .fill(pal.primary.linear_multiply(0.14))
                .stroke(Stroke::new(1.0_f32, pal.primary.linear_multiply(0.4)))
                .rounding(Rounding::same(7.0));
                if ui
                    .add(add_btn)
                    .on_hover_cursor(CursorIcon::PointingHand)
                    .clicked()
                {
                    flags.add_new_account = true;
                }
            });
        });
        ui.add_space(2.0);
        ui.label(
                                                RichText::new("Chuyển đổi tức thì giữa các tài khoản Zalo đã lưu trên máy mà không cần quét lại mã QR.")
                                                    .size(11.8)
                                                    .color(pal.text_secondary),
                                            );
        ui.add_space(10.0);

        let saved = crate::account_manager::list_saved_accounts(&self.user_uin);
        let mut delete_target: Option<String> = None;

        for acc in &saved {
            egui::Frame::none()
                .fill(if acc.is_current {
                    pal.bg_selected
                } else {
                    pal.bg_elevated
                })
                .stroke(Stroke::new(
                    1.0_f32,
                    if acc.is_current {
                        pal.primary
                    } else {
                        pal.border
                    },
                ))
                .rounding(Rounding::same(8.0))
                .inner_margin(Margin::symmetric(14.0, 10.0))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        let (av_r, _) = ui.allocate_exact_size(Vec2::splat(34.0), Sense::hover());
                        paint_user_avatar(
                            ui.painter(),
                            av_r.center(),
                            17.0,
                            &acc.display_name,
                            false,
                            acc.avatar.as_deref(),
                            &self.sticker_textures,
                            &mut self.loading_stickers,
                        );
                        ui.add_space(8.0);
                        ui.vertical(|ui| {
                            ui.label(
                                RichText::new(&acc.display_name)
                                    .size(13.2)
                                    .color(pal.text_primary)
                                    .strong(),
                            );
                            ui.label(
                                RichText::new(format!("UIN: {}", acc.uin))
                                    .size(10.8)
                                    .color(pal.text_muted),
                            );
                        });
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            if acc.is_current {
                                ui.label(
                                    RichText::new("✓ Đang sử dụng")
                                        .size(11.5)
                                        .color(pal.accent_online)
                                        .strong(),
                                );
                            } else {
                                let del_btn = egui::Button::new(
                                    RichText::new(lucide::X).size(12.0).color(pal.text_muted),
                                )
                                .fill(Color32::TRANSPARENT);
                                if ui
                                    .add(del_btn)
                                    .on_hover_cursor(CursorIcon::PointingHand)
                                    .on_hover_text("Xoá tài khoản này khỏi danh sách lưu")
                                    .clicked()
                                {
                                    delete_target = Some(acc.uin.clone());
                                }
                                let sw_btn = egui::Button::new(
                                    RichText::new(format!("{} Chuyển sang", lucide::REFRESH))
                                        .size(11.5)
                                        .color(pal.primary_hover)
                                        .strong(),
                                )
                                .fill(pal.primary.linear_multiply(0.12))
                                .stroke(Stroke::new(1.0_f32, pal.primary.linear_multiply(0.35)))
                                .rounding(Rounding::same(6.0));
                                if ui
                                    .add(sw_btn)
                                    .on_hover_cursor(CursorIcon::PointingHand)
                                    .clicked()
                                {
                                    flags.switch_to_uin = Some(acc.uin.clone());
                                }
                            }
                        });
                    });
                });
            ui.add_space(6.0);
        }

        if let Some(target) = delete_target {
            let _ = crate::account_manager::delete_saved_account(&target);
        }

        ui.add_space(14.0);
        ui.separator();
        ui.add_space(10.0);
        ui.label(
            RichText::new(format!(
                "{} Chạy song song nhiều cửa sổ (Multi-Instance)",
                lucide::EXTERNAL_LINK
            ))
            .color(pal.text_primary)
            .size(13.0)
            .strong(),
        );
        ui.add_space(2.0);
        ui.label(RichText::new("Mở thêm một cửa sổ Zalo độc lập với hồ sơ riêng (hoạt động song song cùng lúc trên màn hình).").color(pal.text_secondary).size(11.8));
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            ui.label(
                RichText::new("Tên hồ sơ:")
                    .color(pal.text_secondary)
                    .size(12.0),
            );
            ui.add(
                egui::TextEdit::singleline(&mut self.settings.multi_profile_name)
                    .hint_text("acc2, work, shop…")
                    .desired_width(130.0),
            );
            if ui
                .button(RichText::new("➕ Mở cửa sổ Zalo mới").strong())
                .clicked()
            {
                flags.spawn_multi_zalo = true;
            }
        });
    }
}
