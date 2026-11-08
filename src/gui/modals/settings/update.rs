use super::flags::SettingsActionFlags;
use crate::gui::modals::updates::{extract_concise_changelog, render_markdown_bullet_line};
use crate::gui::theme::*;
use crate::gui::*;
use eframe::egui;

impl ZaloGuiApp {
    pub(super) fn render_tab_update(
        &mut self,
        ui: &mut egui::Ui,
        pal: &ThemePalette,
        _flags: &mut SettingsActionFlags,
        inner_w: f32,
        _ctx: &egui::Context,
    ) {
        let cur_ver = env!("CARGO_PKG_VERSION");
        let info_opt = self.update_info.clone();
        let has_update = info_opt.as_ref().map(|u| u.has_update).unwrap_or(false);
        let border_col = if has_update {
            pal.accent_online
        } else {
            pal.border
        };

        egui::Frame::none()
                                                .fill(pal.bg_elevated)
                                                .stroke(Stroke::new(1.0_f32, border_col))
                                                .rounding(Rounding::same(10.0))
                                                .inner_margin(Margin::symmetric(18.0, 16.0))
                                                .show(ui, |ui| {
                                                    let card_avail_w =
                                                        (inner_w - 36.0).max(280.0);
                                                    ui.set_min_width(card_avail_w);
                                                    ui.set_max_width(card_avail_w);

                                                    // 1) Header Row: App Icon + Title/Status + Action Button on the right
                                                    ui.horizontal(|ui| {
                                                        let (icon_r, _) = ui.allocate_exact_size(
                                                            Vec2::splat(44.0),
                                                            Sense::hover(),
                                                        );
                                                        if let Some(ref logo) =
                                                            self.app_logo_texture
                                                        {
                                                            ui.painter().image(
                                                                logo.id(),
                                                                icon_r,
                                                                Rect::from_min_max(
                                                                    Pos2::ZERO,
                                                                    Pos2::new(1.0, 1.0),
                                                                ),
                                                                Color32::WHITE,
                                                            );
                                                        } else {
                                                            ui.painter().rect_filled(
                                                                icon_r,
                                                                Rounding::same(11.0),
                                                                pal.primary.linear_multiply(0.20),
                                                            );
                                                            draw_lucide_icon(
                                                                ui.painter(),
                                                                icon_r.center(),
                                                                lucide::SPARKLES,
                                                                20.0,
                                                                pal.primary_hover,
                                                            );
                                                        }

                                                        ui.add_space(6.0);
                                                        ui.vertical(|ui| {
                                                            ui.horizontal(|ui| {
                                                                ui.label(
                                                                    RichText::new("Zalo Lite")
                                                                        .size(15.0)
                                                                        .color(pal.text_primary)
                                                                        .strong(),
                                                                );
                                                                egui::Frame::none()
                                                                    .fill(
                                                                        pal.primary
                                                                            .linear_multiply(0.18),
                                                                    )
                                                                    .rounding(Rounding::same(6.0))
                                                                    .inner_margin(
                                                                        Margin::symmetric(7.0, 2.0),
                                                                    )
                                                                    .show(ui, |ui| {
                                                                        ui.label(
                                                                            RichText::new(format!(
                                                                                "v{cur_ver}"
                                                                            ))
                                                                            .size(11.2)
                                                                            .color(
                                                                                pal.primary_hover,
                                                                            )
                                                                            .strong(),
                                                                        );
                                                                    });
                                                            });
                                                            ui.add_space(2.0);
                                                            let sub_txt = if self.update_checking {
                                                                "Đang kiểm tra bản cập nhật mới…"
                                                                    .to_string()
                                                            } else if let Some(ref info) = info_opt
                                                            {
                                                                if info.has_update {
                                                                    let mb = (info.asset_size
                                                                        as f64)
                                                                        / (1024.0 * 1024.0);
                                                                    if mb > 0.1 {
                                                                        format!(
                                                                            "Đã có bản cập nhật mới v{} ({mb:.1} MB)",
                                                                            info.latest_version
                                                                        )
                                                                    } else {
                                                                        format!(
                                                                            "Đã có bản cập nhật mới v{}",
                                                                            info.latest_version
                                                                        )
                                                                    }
                                                                } else {
                                                                    "Bạn đang sử dụng phiên bản mới nhất"
                                                                        .to_string()
                                                                }
                                                            } else {
                                                                "Tự động nhận bản cập nhật qua GitHub Releases"
                                                                    .to_string()
                                                            };
                                                            let sub_col = if has_update {
                                                                pal.accent_online
                                                            } else {
                                                                pal.text_secondary
                                                            };
                                                            ui.label(
                                                                RichText::new(sub_txt)
                                                                    .size(12.2)
                                                                    .color(sub_col),
                                                            );
                                                        });

                                                        ui.with_layout(
                                                            Layout::right_to_left(Align::Center),
                                                            |ui| {
                                                                if let Some(ref info) = info_opt {
                                                                    if info.has_update {
                                                                        if let (
                                                                            Some(dl_url),
                                                                            Some(asset_name),
                                                                        ) = (
                                                                            info.download_url
                                                                                .clone(),
                                                                            info.asset_name.clone(),
                                                                        ) {
                                                                            let upd_lbl = if self
                                                                                .update_downloading
                                                                            {
                                                                                "Đang cập nhật…"
                                                                                    .to_string()
                                                                            } else {
                                                                                format!(
                                                                                    "Cập nhật v{}",
                                                                                    info.latest_version
                                                                                )
                                                                            };
                                                                            let upd_btn =
                                                                                egui::Button::new(
                                                                                    RichText::new(
                                                                                        upd_lbl,
                                                                                    )
                                                                                    .size(12.5)
                                                                                    .color(
                                                                                        Color32::WHITE,
                                                                                    )
                                                                                    .strong(),
                                                                                )
                                                                                .fill(
                                                                                    pal.accent_online,
                                                                                )
                                                                                .rounding(
                                                                                    Rounding::same(
                                                                                        8.0,
                                                                                    ),
                                                                                )
                                                                                .min_size(
                                                                                    Vec2::new(
                                                                                        132.0, 34.0,
                                                                                    ),
                                                                                );
                                                                            if ui
                                                                                .add_enabled(
                                                                                    !self.update_downloading,
                                                                                    upd_btn,
                                                                                )
                                                                                .on_hover_cursor(
                                                                                    CursorIcon::PointingHand,
                                                                                )
                                                                                .clicked()
                                                                            {
                                                                                self.update_downloading =
                                                                                    true;
                                                                                self.update_progress =
                                                                                    Some((
                                                                                        0,
                                                                                        info.asset_size,
                                                                                    ));
                                                                                if let Some(tx) =
                                                                                    WORKER_TX.get()
                                                                                {
                                                                                    let _ = tx.send(
                                                                                        WorkerCmd::DownloadAndApplyUpdate {
                                                                                            download_url: dl_url,
                                                                                            asset_name,
                                                                                            target_version: info.latest_version.clone(),
                                                                                            expected_sha256: info.expected_sha256.clone(),
                                                                                        },
                                                                                    );
                                                                                }
                                                                            }
                                                                            return;
                                                                        }
                                                                    }
                                                                }

                                                                let chk_lbl = if self.update_checking
                                                                {
                                                                    "Đang kiểm tra…".to_string()
                                                                } else {
                                                                    format!(
                                                                        "{} Kiểm tra cập nhật",
                                                                        lucide::REFRESH
                                                                    )
                                                                };
                                                                let chk_btn = egui::Button::new(
                                                                    RichText::new(chk_lbl)
                                                                        .size(12.2)
                                                                        .color(pal.text_primary),
                                                                )
                                                                .fill(pal.bg_input)
                                                                .stroke(Stroke::new(
                                                                    1.0_f32, pal.border,
                                                                ))
                                                                .rounding(Rounding::same(8.0))
                                                                .min_size(Vec2::new(142.0, 32.0));
                                                                if ui
                                                                    .add_enabled(
                                                                        !self.update_checking
                                                                            && !self
                                                                                .update_downloading,
                                                                        chk_btn,
                                                                    )
                                                                    .on_hover_cursor(
                                                                        CursorIcon::PointingHand,
                                                                    )
                                                                    .clicked()
                                                                {
                                                                    self.update_checking = true;
                                                                    self.update_status.clear();
                                                                    if let Some(tx) =
                                                                        WORKER_TX.get()
                                                                    {
                                                                        let _ = tx.send(
                                                                            WorkerCmd::CheckForUpdates {
                                                                                silent: false,
                                                                            },
                                                                        );
                                                                    }
                                                                }
                                                            },
                                                        );
                                                    });

                                                    // 2) Live Download Progress Bar
                                                    if self.update_downloading {
                                                        ui.add_space(12.0);
                                                        let (dl_bytes, tot_bytes) =
                                                            self.update_progress.unwrap_or((0, 0));
                                                        let frac = if tot_bytes > 0 {
                                                            (dl_bytes as f32 / tot_bytes as f32)
                                                                .clamp(0.0, 1.0)
                                                        } else {
                                                            0.35
                                                        };
                                                        let dl_mb =
                                                            dl_bytes as f64 / (1024.0 * 1024.0);
                                                        let tot_mb =
                                                            tot_bytes as f64 / (1024.0 * 1024.0);
                                                        let p_txt = if tot_bytes > 0 {
                                                            format!(
                                                                "Đang tải gói cập nhật: {dl_mb:.1} / {tot_mb:.1} MB ({:.0}%)",
                                                                frac * 100.0
                                                            )
                                                        } else {
                                                            format!(
                                                                "Đang tải gói cập nhật: {dl_mb:.1} MB…"
                                                            )
                                                        };
                                                        ui.add(
                                                            egui::ProgressBar::new(frac)
                                                                .text(p_txt)
                                                                .animate(true),
                                                        );
                                                        ui.add_space(4.0);
                                                        ui.label(
                                                            RichText::new(
                                                                "Ứng dụng sẽ tự động xác thực SHA-256 và khởi động lại ngay khi tải xong.",
                                                            )
                                                            .size(11.2)
                                                            .color(pal.text_muted),
                                                        );
                                                    } else if self.update_status.starts_with("Lỗi")
                                                        || self.update_status.starts_with("Không")
                                                    {
                                                        ui.add_space(10.0);
                                                        ui.label(
                                                            RichText::new(&self.update_status)
                                                                .size(12.0)
                                                                .color(pal.danger),
                                                        );
                                                    }

                                                    // 3) Concise Changelog Summary (bullet items only, no images/tables)
                                                    if let Some(ref info) = info_opt {
                                                        let bullets = extract_concise_changelog(
                                                            &info.release_notes,
                                                        );
                                                        if !bullets.is_empty() {
                                                            ui.add_space(12.0);
                                                            egui::Frame::none()
                                                                .fill(pal.bg_input)
                                                                .stroke(Stroke::new(
                                                                    1.0_f32, pal.border,
                                                                ))
                                                                .rounding(Rounding::same(8.0))
                                                                .inner_margin(Margin::symmetric(
                                                                    14.0, 10.0,
                                                                ))
                                                                .show(ui, |ui| {
                                                                    ui.set_min_width(
                                                                        card_avail_w - 28.0,
                                                                    );
                                                                    ui.set_max_width(
                                                                        card_avail_w - 28.0,
                                                                    );
                                                                    let hdr = if info.has_update {
                                                                        format!(
                                                                            "Điểm mới trong phiên bản v{}:",
                                                                            info.latest_version
                                                                        )
                                                                    } else {
                                                                        format!(
                                                                            "Có gì mới ở phiên bản v{}:",
                                                                            info.latest_version
                                                                        )
                                                                    };
                                                                    ui.label(
                                                                        RichText::new(hdr)
                                                                            .size(12.2)
                                                                            .color(pal.text_primary)
                                                                            .strong(),
                                                                    );
                                                                    ui.add_space(4.0);
                                                                    for b in bullets.iter().take(16)
                                                                    {
                                                                        render_markdown_bullet_line(
                                                                            ui,
                                                                            b,
                                                                            11.8,
                                                                            pal.primary_hover,
                                                                            pal.text_secondary,
                                                                            pal,
                                                                        );
                                                                        ui.add_space(3.0);
                                                                    }
                                                                });
                                                        }

                                                        // 4) Clean Footer Row
                                                        ui.add_space(10.0);
                                                        ui.horizontal(|ui| {
                                                            if !info.published_at.is_empty() {
                                                                ui.label(
                                                                    RichText::new(format!(
                                                                        "Phát hành: {}",
                                                                        info.published_at
                                                                    ))
                                                                    .size(11.2)
                                                                    .color(pal.text_muted),
                                                                );
                                                            }
                                                            ui.with_layout(
                                                                Layout::right_to_left(
                                                                    Align::Center,
                                                                ),
                                                                |ui| {
                                                                    let link_resp = ui.add(
                                                                        egui::Label::new(
                                                                            RichText::new(format!(
                                                                                "Xem trên GitHub {}",
                                                                                lucide::EXTERNAL_LINK
                                                                            ))
                                                                            .size(11.5)
                                                                            .color(
                                                                                pal.primary_hover,
                                                                            ),
                                                                        )
                                                                        .sense(Sense::click()),
                                                                    );
                                                                    if link_resp
                                                                        .on_hover_cursor(
                                                                            CursorIcon::PointingHand,
                                                                        )
                                                                        .clicked()
                                                                    {
                                                                        open_external_url(
                                                                            &info.html_url,
                                                                        );
                                                                    }
                                                                },
                                                            );
                                                        });
                                                    }
                                                });
    }
}
