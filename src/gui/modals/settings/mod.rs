use crate::gui::theme::*;
use crate::gui::*;
use eframe::egui;

pub mod account;
pub mod appearance;
pub mod backdrop;
pub mod flags;
pub mod general;
pub mod notif_messages;
pub mod privacy_sync;
pub mod update;
pub mod utilities;

pub(crate) use backdrop::build_blurred_backdrop_image;
use flags::SettingsActionFlags;

impl ZaloGuiApp {
    pub(in crate::gui) fn render_settings_modal(
        &mut self,
        ctx: &egui::Context,
        pal: &ThemePalette,
    ) {
        if !self.settings_modal_open {
            self.settings_blur_tex = None;
            self.settings_blur_req_at = None;
            return;
        }
        if ctx.input(|i| i.key_pressed(Key::Escape)) {
            self.settings_modal_open = false;
            self.settings_blur_tex = None;
            self.settings_blur_req_at = None;
            self.settings.save();
            return;
        }

        // Capture & blur the underlying app viewport so the background is genuinely Gaussian-blurred
        let captured_shot = ctx.input(|i| {
            for ev in &i.events {
                if let egui::Event::Screenshot { image, .. } = ev {
                    return Some(image.clone());
                }
            }
            None
        });
        if let Some(shot) = captured_shot {
            if self.settings_blur_tex.is_none() {
                let blurred = build_blurred_backdrop_image(&shot, pal.is_dark);
                self.settings_blur_tex = Some(ctx.load_texture(
                    "settings_frosted_backdrop_tex",
                    blurred,
                    TextureOptions::LINEAR,
                ));
            }
        }
        if self.settings_blur_tex.is_none() {
            if self.settings_blur_req_at.is_none() {
                self.settings_blur_req_at = Some(std::time::Instant::now());
                ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot);
                ctx.request_repaint();
                return;
            } else if self
                .settings_blur_req_at
                .map(|t| t.elapsed() < Duration::from_millis(65))
                .unwrap_or(false)
            {
                ctx.request_repaint();
                return;
            }
        }

        let mut open = true;
        let screen = ctx.screen_rect();
        let win_w = (screen.width() * 0.88).clamp(620.0, 780.0);
        let win_h = (screen.height() * 0.82).clamp(480.0, 600.0);
        let modal_rect = Rect::from_center_size(screen.center(), Vec2::new(win_w, win_h));

        // Full-screen blurred & dimmed backdrop behind the Settings modal (blocks background clicks)
        egui::Area::new(Id::new("settings_modal_frosted_backdrop"))
            .order(egui::Order::Middle)
            .fixed_pos(screen.min)
            .show(ctx, |ui| {
                let (bg_rect, bg_resp) =
                    ui.allocate_exact_size(screen.size(), Sense::click_and_drag());
                if let Some(ref btex) = self.settings_blur_tex {
                    ui.painter().image(
                        btex.id(),
                        bg_rect,
                        Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                        Color32::WHITE,
                    );
                } else {
                    ui.painter().rect_filled(
                        bg_rect,
                        Rounding::ZERO,
                        if pal.is_dark {
                            Color32::from_rgba_unmultiplied(6, 9, 16, 224)
                        } else {
                            Color32::from_rgba_unmultiplied(15, 23, 42, 185)
                        },
                    );
                }
                if bg_resp.clicked() {
                    if let Some(pos) = bg_resp.interact_pointer_pos() {
                        if !modal_rect.contains(pos) {
                            open = false;
                        }
                    }
                }
            });

        egui::Window::new("settings_window_modal")
            .order(egui::Order::Foreground)
            .title_bar(false)
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
            .fixed_size(Vec2::new(win_w, win_h))
            .frame(
                egui::Frame::none()
                    .fill(pal.bg_chat_canvas)
                    .stroke(Stroke::new(1.2_f32, pal.border))
                    .rounding(Rounding::same(12.0))
                    .shadow(egui::epaint::Shadow {
                        offset: Vec2::new(0.0, 14.0),
                        blur: 40.0,
                        spread: 6.0,
                        color: Color32::from_black_alpha(160),
                    })
                    .inner_margin(Margin::ZERO),
            )
            .show(ctx, |ui| {
                ui.visuals_mut().widgets.inactive.bg_stroke = Stroke::new(1.4_f32, pal.text_muted);
                ui.visuals_mut().widgets.hovered.bg_stroke =
                    Stroke::new(1.6_f32, pal.primary_hover);
                let mut flags = SettingsActionFlags::default();
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 0.0;
                    // Left Tabs Navigation Sidebar (200px)
                    egui::Frame::none()
                        .fill(pal.bg_sidebar)
                        .inner_margin(Margin::symmetric(10.0, 16.0))
                        .show(ui, |ui| {
                            ui.set_width(200.0);
                            ui.set_height(win_h - 32.0);
                            ui.vertical(|ui| {
                                ui.add_space(4.0);
                                ui.label(
                                    RichText::new("Cài đặt")
                                        .size(17.5)
                                        .color(pal.text_primary)
                                        .strong(),
                                );
                                ui.add_space(14.0);

                                let tabs = [
                                    (SettingsTab::General, lucide::SETTINGS, "Cài đặt chung"),
                                    (SettingsTab::Account, lucide::USER, "Tài khoản"),
                                    (SettingsTab::Privacy, lucide::LOCK, "Quyền riêng tư"),
                                    (SettingsTab::Sync, lucide::REFRESH, "Đồng bộ tin nhắn"),
                                    (SettingsTab::Appearance, lucide::PAINTBRUSH, "Giao diện"),
                                    (SettingsTab::Notifications, lucide::BELL, "Thông báo"),
                                    (SettingsTab::Messages, lucide::MESSAGE_SQUARE, "Tin nhắn"),
                                    (SettingsTab::Utilities, lucide::SLIDERS, "Tiện ích"),
                                    (SettingsTab::Update, lucide::SPARKLES, "Cập nhật"),
                                ];
                                for (tab, icon, title) in tabs {
                                    let is_active = self.settings_tab == tab;
                                    let text_col = if is_active {
                                        pal.primary_hover
                                    } else {
                                        pal.text_secondary
                                    };

                                    let (rect, resp) = ui.allocate_exact_size(
                                        Vec2::new(196.0, 36.0),
                                        Sense::click(),
                                    );
                                    if is_active || resp.hovered() {
                                        ui.painter().rect_filled(
                                            rect,
                                            Rounding::same(8.0),
                                            if is_active {
                                                pal.bg_selected
                                            } else {
                                                pal.bg_hover
                                            },
                                        );
                                    }
                                    let icon_pt = Pos2::new(rect.left() + 18.0, rect.center().y);
                                    draw_lucide_icon(ui.painter(), icon_pt, icon, 14.5, text_col);
                                    let text_pt = Pos2::new(rect.left() + 38.0, rect.center().y);
                                    ui.painter().text(
                                        text_pt,
                                        egui::Align2::LEFT_CENTER,
                                        title,
                                        FontId::proportional(13.2),
                                        if is_active {
                                            pal.text_primary
                                        } else {
                                            text_col
                                        },
                                    );

                                    if resp.on_hover_cursor(CursorIcon::PointingHand).clicked() {
                                        self.settings_tab = tab;
                                    }
                                    ui.add_space(2.0);
                                }
                                ui.add_space(8.0);
                                ui.separator();
                                ui.add_space(8.0);

                                let (lo_rect, lo_resp) =
                                    ui.allocate_exact_size(Vec2::new(196.0, 36.0), Sense::click());
                                if lo_resp.hovered() {
                                    ui.painter().rect_filled(
                                        lo_rect,
                                        Rounding::same(8.0),
                                        pal.danger.linear_multiply(0.18),
                                    );
                                }
                                let lo_icon_pt =
                                    Pos2::new(lo_rect.left() + 18.0, lo_rect.center().y);
                                draw_lucide_icon(
                                    ui.painter(),
                                    lo_icon_pt,
                                    lucide::LOG_OUT,
                                    14.5,
                                    pal.danger,
                                );
                                let lo_text_pt =
                                    Pos2::new(lo_rect.left() + 38.0, lo_rect.center().y);
                                ui.painter().text(
                                    lo_text_pt,
                                    egui::Align2::LEFT_CENTER,
                                    "Đăng xuất",
                                    FontId::proportional(13.2),
                                    pal.danger,
                                );
                                if lo_resp
                                    .on_hover_cursor(CursorIcon::PointingHand)
                                    .on_hover_text("Đăng xuất khỏi tài khoản Zalo hiện tại")
                                    .clicked()
                                {
                                    flags.do_logout = true;
                                }
                            });
                        });

                    // Right Content Panel
                    let content_w = (win_w - 268.0).max(350.0);
                    egui::Frame::none()
                        .fill(pal.bg_chat_canvas)
                        .inner_margin(Margin::symmetric(24.0, 16.0))
                        .show(ui, |ui| {
                            ui.vertical(|ui| {
                                ui.set_min_width(content_w);
                                ui.set_max_width(content_w);
                                ui.set_height(win_h - 32.0);

                                ui.horizontal(|ui| {
                                    let tab_title = match self.settings_tab {
                                        SettingsTab::General => "Cài đặt chung",
                                        SettingsTab::Account => "Tài khoản & Đa tài khoản",
                                        SettingsTab::Privacy => "Quyền riêng tư",
                                        SettingsTab::Sync => "Đồng bộ tin nhắn",
                                        SettingsTab::Appearance => "Cài đặt giao diện",
                                        SettingsTab::Notifications => "Cài đặt thông báo",
                                        SettingsTab::Messages => "Cài đặt tin nhắn",
                                        SettingsTab::Utilities => "Tiện ích & Gợi ý",
                                        SettingsTab::Update => "Cập nhật phần mềm",
                                    };
                                    ui.label(
                                        RichText::new(tab_title)
                                            .size(15.5)
                                            .color(pal.text_primary)
                                            .strong(),
                                    );
                                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                        let (cr, c_resp) = ui
                                            .allocate_exact_size(Vec2::splat(26.0), Sense::click());
                                        if c_resp.hovered() {
                                            ui.painter().rect_filled(
                                                cr,
                                                Rounding::same(6.0),
                                                pal.bg_hover,
                                            );
                                        }
                                        draw_lucide_icon(
                                            ui.painter(),
                                            cr.center(),
                                            lucide::X,
                                            14.0,
                                            if c_resp.hovered() {
                                                pal.text_primary
                                            } else {
                                                pal.text_secondary
                                            },
                                        );
                                        if c_resp
                                            .on_hover_cursor(CursorIcon::PointingHand)
                                            .clicked()
                                        {
                                            open = false;
                                        }
                                    });
                                });

                                ui.add_space(8.0);
                                ui.separator();
                                ui.add_space(8.0);

                                let inner_w = (content_w - 8.0).max(320.0);
                                egui::ScrollArea::vertical()
                                    .max_width(content_w)
                                    .max_height((win_h - 86.0).max(240.0))
                                    .auto_shrink([false, false])
                                    .show(ui, |ui| {
                                        ui.vertical(|ui| {
                                            ui.set_min_width(inner_w);
                                            ui.set_max_width(inner_w);

                                            match self.settings_tab {
                                                SettingsTab::General => self.render_tab_general(
                                                    ui, pal, &mut flags, inner_w,
                                                ),
                                                SettingsTab::Account => self.render_tab_account(
                                                    ui, pal, &mut flags, inner_w,
                                                ),
                                                SettingsTab::Privacy => self.render_tab_privacy(
                                                    ui, pal, &mut flags, inner_w,
                                                ),
                                                SettingsTab::Sync => self
                                                    .render_tab_sync(ui, pal, &mut flags, inner_w),
                                                SettingsTab::Appearance => self
                                                    .render_tab_appearance(
                                                        ui, pal, &mut flags, inner_w,
                                                    ),
                                                SettingsTab::Notifications => self
                                                    .render_tab_notifications(
                                                        ui, pal, &mut flags, inner_w,
                                                    ),
                                                SettingsTab::Messages => self.render_tab_messages(
                                                    ui, pal, &mut flags, inner_w,
                                                ),
                                                SettingsTab::Utilities => self
                                                    .render_tab_utilities(
                                                        ui, pal, &mut flags, inner_w,
                                                    ),
                                                SettingsTab::Update => self.render_tab_update(
                                                    ui, pal, &mut flags, inner_w, ctx,
                                                ),
                                            }
                                            self.handle_settings_actions(flags);
                                        });
                                    });
                            });
                        });
                });
            });

        if !open {
            self.settings_modal_open = false;
            self.settings_blur_tex = None;
            self.settings_blur_req_at = None;
            self.settings.save();
        }
    }
}
