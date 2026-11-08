use crate::gui::theme::*;
use crate::gui::*;
use eframe::egui;

impl ZaloGuiApp {
    pub(in crate::gui) fn render_update_notification_modal(
        &mut self,
        ctx: &egui::Context,
        pal: &ThemePalette,
    ) {
        if !self.update_modal_open {
            return;
        }
        let Some(info) = self.update_info.clone().filter(|u| u.has_update) else {
            self.update_modal_open = false;
            return;
        };
        if ctx.input(|i| i.key_pressed(Key::Escape)) && !self.update_downloading {
            self.update_modal_open = false;
            return;
        }

        let mut open = true;
        let mut trigger_download = false;
        let mut skip_this_version = false;
        let mut remind_later = false;

        let screen = ctx.screen_rect();
        egui::Area::new(Id::new("update_modal_backdrop"))
            .order(egui::Order::Foreground)
            .fixed_pos(screen.min)
            .show(ctx, |ui| {
                let (r, resp) = ui.allocate_exact_size(screen.size(), Sense::click());
                ui.painter().rect_filled(
                    r,
                    Rounding::ZERO,
                    Color32::from_rgba_unmultiplied(0, 0, 0, if pal.is_dark { 155 } else { 95 }),
                );
                if resp.clicked() && !self.update_downloading {
                    open = false;
                }
            });

        let card_w = (screen.width() - 48.0).clamp(420.0, 560.0);
        egui::Area::new(Id::new("update_modal_card"))
            .order(egui::Order::Tooltip)
            .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
            .show(ctx, |ui| {
                egui::Frame::none()
                    .fill(pal.bg_elevated)
                    .rounding(Rounding::same(14.0))
                    .stroke(Stroke::new(1.2_f32, pal.border))
                    .shadow(egui::epaint::Shadow {
                        offset: Vec2::new(0.0, 12.0),
                        blur: 32.0,
                        spread: 0.0,
                        color: Color32::from_rgba_unmultiplied(0, 0, 0, 130),
                    })
                    .inner_margin(Margin::symmetric(22.0, 18.0))
                    .show(ui, |ui| {
                        let inner_w = card_w - 44.0;
                        ui.set_width(inner_w);

                        // 1. Top Header Bar: Title + Close button
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new(format!(
                                    "{}  Có bản cập nhật mới từ GitHub Releases",
                                    lucide::SPARKLES
                                ))
                                .size(15.5)
                                .color(pal.text_primary)
                                .strong(),
                            );
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                let (x_r, x_resp) =
                                    ui.allocate_exact_size(Vec2::splat(28.0), Sense::click());
                                if x_resp.hovered() {
                                    ui.painter().rect_filled(
                                        x_r,
                                        Rounding::same(6.0),
                                        pal.bg_hover,
                                    );
                                }
                                draw_lucide_icon(
                                    ui.painter(),
                                    x_r.center(),
                                    lucide::X,
                                    15.0,
                                    if x_resp.hovered() {
                                        pal.text_primary
                                    } else {
                                        pal.text_secondary
                                    },
                                );
                                if x_resp
                                    .on_hover_cursor(CursorIcon::PointingHand)
                                    .on_hover_text("Đóng (Esc)")
                                    .clicked()
                                    && !self.update_downloading
                                {
                                    open = false;
                                }
                            });
                        });

                        ui.add_space(8.0);
                        ui.separator();
                        ui.add_space(10.0);

                        // 2. App Logo + Release Title + Version Transition Pill + Metadata
                        ui.horizontal(|ui| {
                            let (icon_r, _) =
                                ui.allocate_exact_size(Vec2::splat(46.0), Sense::hover());
                            if let Some(ref logo) = self.app_logo_texture {
                                ui.painter().image(
                                    logo.id(),
                                    icon_r,
                                    Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                                    Color32::WHITE,
                                );
                            } else {
                                ui.painter().rect_filled(
                                    icon_r,
                                    Rounding::same(11.0),
                                    pal.primary.linear_multiply(0.20),
                                );
                            }
                            ui.add_space(8.0);
                            ui.vertical(|ui| {
                                ui.set_max_width((inner_w - 62.0).max(200.0));
                                ui.add(
                                    egui::Label::new(
                                        RichText::new(&info.release_title)
                                            .size(14.2)
                                            .color(pal.text_primary)
                                            .strong(),
                                    )
                                    .wrap(),
                                );
                                ui.add_space(5.0);
                                ui.horizontal_wrapped(|ui| {
                                    ui.spacing_mut().item_spacing.x = 6.0;
                                    egui::Frame::none()
                                        .fill(pal.primary.linear_multiply(0.18))
                                        .stroke(Stroke::new(
                                            1.0_f32,
                                            pal.primary.linear_multiply(0.45),
                                        ))
                                        .rounding(Rounding::same(6.0))
                                        .inner_margin(Margin::symmetric(8.0, 2.0))
                                        .show(ui, |ui| {
                                            ui.label(
                                                RichText::new(format!(
                                                    "v{}  ->  v{}",
                                                    info.current_version, info.latest_version
                                                ))
                                                .size(11.5)
                                                .color(pal.primary_hover)
                                                .strong(),
                                            );
                                        });

                                    let mut meta_parts = Vec::new();
                                    if !info.published_at.is_empty() {
                                        meta_parts.push(format!("Phát hành: {}", info.published_at));
                                    }
                                    if info.asset_size > 0 {
                                        let mb = (info.asset_size as f64) / (1024.0 * 1024.0);
                                        meta_parts.push(format!("Dung lượng: {mb:.1} MB"));
                                    }
                                    if info.expected_sha256.is_some() {
                                        meta_parts.push(format!(
                                            "{} Đã xác thực SHA-256",
                                            lucide::SHIELD_CHECK
                                        ));
                                    }
                                    if !meta_parts.is_empty() {
                                        ui.label(
                                            RichText::new(meta_parts.join("  ·  "))
                                                .size(11.5)
                                                .color(pal.text_secondary),
                                        );
                                    }
                                });
                            });
                        });

                        // 3. Scrollable Release Notes Box (padded top & bottom so text is never clipped)
                        let bullets = extract_concise_changelog(&info.release_notes);
                        if !bullets.is_empty() {
                            ui.add_space(12.0);
                            egui::Frame::none()
                                .fill(pal.bg_input)
                                .stroke(Stroke::new(1.0_f32, pal.border))
                                .rounding(Rounding::same(10.0))
                                .inner_margin(Margin::symmetric(14.0, 12.0))
                                .show(ui, |ui| {
                                    ui.set_width(inner_w - 28.0);
                                    ui.label(
                                        RichText::new(format!(
                                            "Chi tiết cập nhật & sửa lỗi trong v{} (từ GitHub Releases API):",
                                            info.latest_version
                                        ))
                                        .size(12.2)
                                        .color(pal.text_primary)
                                        .strong(),
                                    );
                                    ui.add_space(8.0);
                                    egui::ScrollArea::vertical()
                                        .id_salt("update_modal_notes_scroll")
                                        .max_height(215.0)
                                        .auto_shrink([false, false])
                                        .show(ui, |ui| {
                                            ui.add_space(2.0);
                                            for b in bullets.iter().take(20) {
                                                render_markdown_bullet_line(
                                                    ui,
                                                    b,
                                                    12.2,
                                                    pal.accent_online,
                                                    pal.text_secondary,
                                                    pal,
                                                );
                                                ui.add_space(6.0);
                                            }
                                            ui.add_space(2.0);
                                        });
                                });
                        }

                        if self.update_downloading {
                            ui.add_space(12.0);
                            let (dl_bytes, tot_bytes) = self.update_progress.unwrap_or((0, 0));
                            let frac = if tot_bytes > 0 {
                                (dl_bytes as f32 / tot_bytes as f32).clamp(0.0, 1.0)
                            } else {
                                0.35
                            };
                            let dl_mb = dl_bytes as f64 / (1024.0 * 1024.0);
                            let tot_mb = tot_bytes as f64 / (1024.0 * 1024.0);
                            let p_txt = if tot_bytes > 0 {
                                format!(
                                    "Đang tải bản cập nhật: {dl_mb:.1} / {tot_mb:.1} MB ({:.0}%)",
                                    frac * 100.0
                                )
                            } else {
                                format!("Đang tải bản cập nhật: {dl_mb:.1} MB…")
                            };
                            ui.add(egui::ProgressBar::new(frac).text(p_txt).animate(true));
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
                            ui.add_space(8.0);
                            ui.label(
                                RichText::new(&self.update_status)
                                    .size(12.0)
                                    .color(pal.danger),
                            );
                        }

                        // 4. Bottom Action Bar (Crisp high-contrast emerald primary button)
                        ui.add_space(14.0);
                        ui.horizontal(|ui| {
                            if info.download_url.is_some() && info.asset_name.is_some() {
                                let btn_lbl = if self.update_downloading {
                                    "Đang cập nhật…".to_string()
                                } else {
                                    format!("Cập nhật ngay (v{})", info.latest_version)
                                };
                                let upd_btn = egui::Button::new(
                                    RichText::new(btn_lbl)
                                        .size(12.8)
                                        .color(Color32::WHITE)
                                        .strong(),
                                )
                                .fill(Color32::from_rgb(22, 163, 74))
                                .stroke(Stroke::new(1.0_f32, Color32::from_rgb(34, 197, 94)))
                                .rounding(Rounding::same(8.0))
                                .min_size(Vec2::new(168.0, 36.0));
                                if ui
                                    .add_enabled(!self.update_downloading, upd_btn)
                                    .on_hover_cursor(CursorIcon::PointingHand)
                                    .clicked()
                                {
                                    trigger_download = true;
                                }
                            }

                            if ui
                                .add(
                                    egui::Button::new(
                                        RichText::new(format!("{}  Xem trên GitHub", lucide::GLOBE))
                                            .size(12.2)
                                            .color(pal.text_primary),
                                    )
                                    .fill(pal.bg_input)
                                    .stroke(Stroke::new(1.0_f32, pal.border))
                                    .rounding(Rounding::same(8.0))
                                    .min_size(Vec2::new(134.0, 36.0)),
                                )
                                .on_hover_cursor(CursorIcon::PointingHand)
                                .clicked()
                            {
                                open_external_url(&info.html_url);
                            }

                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                if ui
                                    .add_enabled(
                                        !self.update_downloading,
                                        egui::Button::new(
                                            RichText::new("Để sau")
                                                .size(12.2)
                                                .color(pal.text_primary),
                                        )
                                        .fill(pal.bg_input)
                                        .stroke(Stroke::new(1.0_f32, pal.border))
                                        .rounding(Rounding::same(8.0))
                                        .min_size(Vec2::new(68.0, 34.0)),
                                    )
                                    .on_hover_cursor(CursorIcon::PointingHand)
                                    .clicked()
                                {
                                    remind_later = true;
                                }
                                if ui
                                    .add_enabled(
                                        !self.update_downloading,
                                        egui::Button::new(
                                            RichText::new("Bỏ qua bản này")
                                                .size(11.8)
                                                .color(pal.text_secondary),
                                        )
                                        .fill(Color32::TRANSPARENT)
                                        .rounding(Rounding::same(8.0))
                                        .min_size(Vec2::new(104.0, 34.0)),
                                    )
                                    .on_hover_cursor(CursorIcon::PointingHand)
                                    .clicked()
                                {
                                    skip_this_version = true;
                                }
                            });
                        });
                    });
            });
        if trigger_download {
            if let (Some(dl_url), Some(asset_name)) =
                (info.download_url.clone(), info.asset_name.clone())
            {
                self.update_downloading = true;
                self.update_progress = Some((0, info.asset_size));
                if let Some(tx) = WORKER_TX.get() {
                    let _ = tx.send(WorkerCmd::DownloadAndApplyUpdate {
                        download_url: dl_url,
                        asset_name,
                        target_version: info.latest_version.clone(),
                        expected_sha256: info.expected_sha256.clone(),
                    });
                }
            }
        }
        if skip_this_version {
            self.settings.skipped_update_version = info.latest_version;
            self.settings.save();
            self.update_modal_open = false;
            self.update_banner_dismissed = true;
        }
        if remind_later || !open {
            self.update_modal_open = false;
        }
    }
}

pub(in crate::gui) fn extract_concise_changelog(markdown: &str) -> Vec<String> {
    let mut bullets: Vec<String> = Vec::new();
    let mut in_code = false;
    let mut in_install_section = false;

    for line in markdown.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("```") {
            in_code = !in_code;
            continue;
        }
        if in_code || trimmed.is_empty() {
            continue;
        }
        if trimmed.starts_with('#') {
            let h_low = trimmed.to_lowercase();
            in_install_section = h_low.contains("bản cài đặt")
                || h_low.contains("hướng dẫn cài")
                || h_low.contains("installation");
            continue;
        }
        if in_install_section {
            continue;
        }
        if let Some(b) = trimmed
            .strip_prefix("- ")
            .or_else(|| trimmed.strip_prefix("* "))
        {
            let b_clean = b.trim();
            let b_low = b_clean.to_lowercase();
            if b_clean.is_empty()
                || b_low.starts_with("bộ cài đặt:")
                || b_low.starts_with("bản chạy ngay")
                || b_low.contains("sudo apt")
                || b_low.contains("sudo dnf")
                || b_low.contains(".deb")
                || b_low.contains(".rpm")
                || b_low.contains(".dmg")
                || b_low.contains(".appimage")
            {
                continue;
            }
            bullets.push(b_clean.to_string());
        }
    }
    bullets
}

/// Render a single bullet item containing `**bold**`, `` `code` ``, and `[label](url)` links
/// into one unified wrapped `LayoutJob` so the bullet `• ` and text never collide or clip.
pub(in crate::gui) fn render_markdown_bullet_line(
    ui: &mut egui::Ui,
    text: &str,
    size: f32,
    bullet_color: Color32,
    base_color: Color32,
    pal: &ThemePalette,
) {
    let mut job = egui::text::LayoutJob::default();
    job.append(
        "•  ",
        0.0,
        egui::TextFormat {
            font_id: FontId::proportional(size + 0.5),
            color: bullet_color,
            ..Default::default()
        },
    );
    let first_link = append_markdown_inline_to_job(&mut job, text, size, base_color, pal);
    if let Some(url) = first_link {
        let resp = ui.add(egui::Label::new(job).wrap().sense(Sense::click()));
        if resp
            .on_hover_cursor(CursorIcon::PointingHand)
            .on_hover_text(&url)
            .clicked()
        {
            open_external_url(&url);
        }
    } else {
        ui.add(egui::Label::new(job).wrap());
    }
}

fn find_next_markdown_link(s: &str) -> Option<(usize, usize, usize)> {
    let mut search_from = 0;
    while let Some(rel_lb) = s[search_from..].find('[') {
        let lb = search_from + rel_lb;
        let after_lb = &s[lb + 1..];
        if let Some(rb) = after_lb.find("](") {
            let label_part = &after_lb[..rb];
            if !label_part.contains('[') && !label_part.contains('\n') {
                let after_rb = &after_lb[rb + 2..];
                if let Some(rp) = after_rb.find(')') {
                    let url_part = &after_rb[..rp];
                    if !url_part.contains(' ') && !url_part.contains('\n') {
                        return Some((lb, lb + 1 + rb, lb + 1 + rb + 2 + rp));
                    }
                }
            }
        }
        search_from = lb + 1;
    }
    None
}

fn append_markdown_inline_to_job(
    job: &mut egui::text::LayoutJob,
    text: &str,
    size: f32,
    base_color: Color32,
    pal: &ThemePalette,
) -> Option<String> {
    let mut first_link: Option<String> = None;
    let mut rest = text;

    while !rest.is_empty() {
        let next_bold = rest.find("**");
        let next_code = rest.find('`');
        let next_link = find_next_markdown_link(rest);

        let min_pos = [next_bold, next_code, next_link.map(|(lb, _, _)| lb)]
            .into_iter()
            .flatten()
            .min();

        let Some(pos) = min_pos else {
            job.append(
                rest,
                0.0,
                egui::TextFormat {
                    font_id: FontId::proportional(size),
                    color: base_color,
                    ..Default::default()
                },
            );
            break;
        };

        if pos > 0 {
            job.append(
                &rest[..pos],
                0.0,
                egui::TextFormat {
                    font_id: FontId::proportional(size),
                    color: base_color,
                    ..Default::default()
                },
            );
            rest = &rest[pos..];
            continue;
        }

        if let Some((0, rb_idx, rp_idx)) = next_link {
            let label = &rest[1..rb_idx];
            let url = &rest[rb_idx + 2..rp_idx];
            if first_link.is_none() && (url.starts_with("http://") || url.starts_with("https://")) {
                first_link = Some(url.to_string());
            }
            let clean_lbl = label.trim_matches('*').trim_matches('`');
            job.append(
                clean_lbl,
                0.0,
                egui::TextFormat {
                    font_id: FontId::proportional(size),
                    color: pal.primary_hover,
                    underline: Stroke::new(1.0_f32, pal.primary_hover),
                    ..Default::default()
                },
            );
            rest = &rest[rp_idx + 1..];
            continue;
        }

        if rest.starts_with("**") {
            let after = &rest[2..];
            if let Some(end_b) = after.find("**") {
                let bold_txt = &after[..end_b];
                let clean_bold = extract_markdown_links_inline(bold_txt);
                job.append(
                    &clean_bold,
                    0.0,
                    egui::TextFormat {
                        font_id: FontId::proportional(size + 0.3),
                        color: pal.text_primary,
                        ..Default::default()
                    },
                );
                rest = &after[end_b + 2..];
                continue;
            }
        }

        if rest.starts_with('`') {
            let after = &rest[1..];
            if let Some(end_c) = after.find('`') {
                let code_txt = &after[..end_c];
                job.append(
                    code_txt,
                    0.0,
                    egui::TextFormat {
                        font_id: FontId::monospace((size - 0.6).max(10.5)),
                        color: pal.primary_hover,
                        background: pal.bg_input,
                        ..Default::default()
                    },
                );
                rest = &after[end_c + 1..];
                continue;
            }
        }

        let mut chars = rest.chars();
        if let Some(ch) = chars.next() {
            let mut buf = [0u8; 4];
            job.append(
                ch.encode_utf8(&mut buf),
                0.0,
                egui::TextFormat {
                    font_id: FontId::proportional(size),
                    color: base_color,
                    ..Default::default()
                },
            );
            rest = chars.as_str();
        } else {
            break;
        }
    }

    first_link
}

/// Collapse `[label](url)` inside a bold segment into `label` so raw URLs never leak.
fn extract_markdown_links_inline(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while !rest.is_empty() {
        if let Some((lb, rb, rp)) = find_next_markdown_link(rest) {
            out.push_str(&rest[..lb]);
            out.push_str(&rest[lb + 1..rb]);
            rest = &rest[rp + 1..];
        } else {
            out.push_str(rest);
            break;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn markdown_links_inside_bold_are_stripped_cleanly() {
        let raw = "Hẹn giờ gửi tin nhắn ([#38](https://github.com/ngxccc/alolite/issues/38)), REST API ([#35](https://github.com/ngxccc/alolite/issues/35))";
        let cleaned = extract_markdown_links_inline(raw);
        assert_eq!(cleaned, "Hẹn giờ gửi tin nhắn (#38), REST API (#35)");
    }
}
