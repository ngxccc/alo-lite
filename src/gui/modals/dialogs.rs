use crate::gui::theme::*;
use crate::gui::*;
use eframe::egui;

impl ZaloGuiApp {
    pub(in crate::gui) fn render_toast_overlay(&mut self, ctx: &egui::Context, pal: &ThemePalette) {
        let Some((ref text, ts)) = self.toast_notice else {
            return;
        };
        if ts.elapsed() > Duration::from_millis(2500) {
            self.toast_notice = None;
            return;
        }
        ctx.request_repaint_after(Duration::from_millis(200));
        let screen = ctx.screen_rect();
        egui::Area::new(Id::new("floating_toast_notice"))
            .order(egui::Order::Tooltip)
            .fixed_pos(Pos2::new(
                (screen.right() - 320.0).max(80.0),
                screen.bottom() - 54.0,
            ))
            .interactable(false)
            .show(ctx, |ui| {
                egui::Frame::none()
                    .fill(pal.bg_elevated)
                    .stroke(Stroke::new(1.0_f32, pal.border))
                    .rounding(Rounding::same(10.0))
                    .inner_margin(Margin::symmetric(14.0, 9.0))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            let (dr, _) = ui.allocate_exact_size(Vec2::splat(8.0), Sense::hover());
                            ui.painter()
                                .circle_filled(dr.center(), 4.0, pal.accent_online);
                            ui.label(
                                RichText::new(text)
                                    .size(12.0)
                                    .color(pal.text_primary)
                                    .strong(),
                            );
                        });
                    });
            });
    }

    pub(in crate::gui) fn render_wipe_confirm(&mut self, ctx: &egui::Context, pal: &ThemePalette) {
        if !self.wipe_confirm_open {
            return;
        }
        let mut open = true;
        let mut confirm = false;
        let mut cancel = false;
        egui::Window::new("Xoá dữ liệu tin nhắn cục bộ")
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
            .fixed_size(Vec2::new(380.0, 150.0))
            .show(ctx, |ui| {
                ui.label(
                    RichText::new(
                        "Xoá zalo_history.db và threads_cache.json trên máy này. \
                         Không thể hoàn tác, và tin nhắn cũ chỉ tải lại được từ đám mây Zalo.",
                    )
                    .size(12.5)
                    .color(pal.text_secondary),
                );
                ui.add_space(4.0);
                ui.label(
                    RichText::new("Phiên đăng nhập trong session.json giữ nguyên.")
                        .size(12.0)
                        .color(pal.text_muted),
                );
                ui.add_space(12.0);
                ui.horizontal(|ui| {
                    if ui
                        .add_sized(
                            [140.0, 30.0],
                            egui::Button::new(
                                RichText::new("Xoá dữ liệu")
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
                        confirm = true;
                    }
                    if ui
                        .add_sized(
                            [100.0, 30.0],
                            egui::Button::new(
                                RichText::new("Huỷ").size(12.5).color(pal.text_secondary),
                            )
                            .fill(pal.bg_input)
                            .stroke(Stroke::new(1.0_f32, pal.border))
                            .rounding(Rounding::same(7.0)),
                        )
                        .on_hover_cursor(CursorIcon::PointingHand)
                        .clicked()
                    {
                        cancel = true;
                    }
                });
            });
        if confirm {
            self.wipe_confirm_open = false;
            if let Some(tx) = WORKER_TX.get() {
                let _ = tx.send(WorkerCmd::WipeLocalData);
            }
        } else if cancel || !open {
            self.wipe_confirm_open = false;
        }
    }
    pub(in crate::gui) fn render_phone_sync_modal(
        &mut self,
        ctx: &egui::Context,
        pal: &ThemePalette,
    ) {
        if !self.phone_sync_modal_open {
            return;
        }
        if ctx.input(|i| i.key_pressed(Key::Escape)) {
            crate::sync::PHONE_SYNC_CANCEL.store(true, std::sync::atomic::Ordering::Relaxed);
            self.phone_sync_modal_open = false;
            return;
        }
        let is_running = crate::sync::PHONE_SYNC_RUNNING.load(std::sync::atomic::Ordering::Relaxed);
        let mut open = true;
        let mut cancel = false;
        let mut go_to_local_import = false;

        egui::Window::new(format!(
            "{} Đồng bộ tin nhắn từ điện thoại",
            lucide::REFRESH
        ))
        .open(&mut open)
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
        .fixed_size(Vec2::new(440.0, 240.0))
        .show(ctx, |ui| {
            ui.add_space(4.0);
            egui::Frame::none()
                .fill(pal.bg_elevated)
                .stroke(Stroke::new(1.0, pal.border))
                .rounding(Rounding::same(8.0))
                .inner_margin(Margin::same(12.0))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(lucide::PHONE)
                                .size(22.0)
                                .color(pal.primary_hover),
                        );
                        ui.vertical(|ui| {
                            ui.label(
                                RichText::new("Các bước cần thực hiện trên điện thoại:")
                                    .color(pal.text_primary)
                                    .size(13.0)
                                    .strong(),
                            );
                            ui.add_space(2.0);
                            ui.label(
                                RichText::new("1. Mở ứng dụng Zalo trên điện thoại.")
                                    .color(pal.text_secondary)
                                    .size(12.0),
                            );
                            ui.label(
                                RichText::new(
                                    "2. Khi màn hình hiện 'Đồng bộ tin nhắn?', bấm 'Đồng bộ ngay'.",
                                )
                                .color(pal.primary_hover)
                                .size(12.0)
                                .strong(),
                            );
                            ui.label(
                                RichText::new(
                                    "3. Xác thực bằng FaceID / Vân tay / Mã PIN trên điện thoại.",
                                )
                                .color(pal.accent_online)
                                .size(12.0)
                                .strong(),
                            );
                        });
                    });
                });

            ui.add_space(10.0);
            let prog = self.phone_sync_progress.clamp(0.05, 1.0);
            ui.add(egui::ProgressBar::new(prog).animate(is_running));
            ui.add_space(8.0);
            let status_display = if self.phone_sync_status.is_empty() {
                "Đang gửi tín hiệu đánh thức tới Zalo trên điện thoại…"
            } else {
                &self.phone_sync_status
            };
            let is_err = status_display.starts_with("Không")
                || status_display.starts_with("Lỗi")
                || status_display.starts_with("Chưa")
                || status_display.starts_with("Điện thoại chưa");
            let is_ok = status_display.starts_with("Đã") || status_display.starts_with("Hoàn tất");
            let text_color = if is_err {
                pal.danger
            } else if is_ok {
                pal.accent_online
            } else {
                pal.text_secondary
            };
            let icon = if is_err {
                lucide::ALERT_CIRCLE
            } else if is_ok {
                lucide::CHECK
            } else {
                lucide::REFRESH
            };
            ui.label(
                RichText::new(format!("{} {}", icon, status_display))
                    .color(text_color)
                    .size(12.0),
            );
            ui.add_space(14.0);
            ui.horizontal(|ui| {
                if ui.button(RichText::new("Hủy bỏ").size(12.5)).clicked() {
                    cancel = true;
                }
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ui
                        .button(
                            RichText::new(format!(
                                "{} Nhập file sao lưu (.db / .json)…",
                                lucide::FOLDER_OPEN
                            ))
                            .size(12.0)
                            .color(pal.primary_hover),
                        )
                        .clicked()
                    {
                        go_to_local_import = true;
                    }
                });
            });
        });

        if !open || cancel {
            crate::sync::PHONE_SYNC_CANCEL.store(true, std::sync::atomic::Ordering::Relaxed);
            self.phone_sync_modal_open = false;
        }
        if go_to_local_import {
            crate::sync::PHONE_SYNC_CANCEL.store(true, std::sync::atomic::Ordering::Relaxed);
            self.phone_sync_modal_open = false;
            self.settings_modal_open = true;
            self.settings_tab = SettingsTab::Sync;
        }
    }

    pub(in crate::gui) fn render_forward_dialog(
        &mut self,
        ctx: &egui::Context,
        pal: &ThemePalette,
    ) {
        if self.forward_msg.is_none() {
            return;
        }
        let preview = self
            .forward_msg
            .as_ref()
            .map(|m| {
                if m.sticker_id.is_some() {
                    "[Nhãn dán]".to_string()
                } else {
                    truncate_chars(&m.text, 80).into_owned()
                }
            })
            .unwrap_or_default();
        let query = self.forward_filter.to_lowercase();
        let candidates: Vec<(String, String, bool)> = self
            .threads
            .iter()
            .filter(|t| query.is_empty() || t.name.to_lowercase().contains(&query))
            .map(|t| (t.id.clone(), t.name.clone(), t.is_group))
            .collect();
        let mut open = true;
        let mut chosen: Option<(String, bool)> = None;
        egui::Window::new("Chuyển tiếp tin nhắn")
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
            .fixed_size(Vec2::new(360.0, 420.0))
            .show(ctx, |ui| {
                ui.label(
                    RichText::new(format!("Nội dung: {preview}"))
                        .size(12.5)
                        .color(pal.text_secondary),
                );
                ui.add_space(6.0);
                ui.add(
                    egui::TextEdit::singleline(&mut self.forward_filter)
                        .hint_text("Tìm hội thoại…")
                        .desired_width(f32::INFINITY),
                );
                ui.add_space(6.0);
                egui::ScrollArea::vertical()
                    .max_height(320.0)
                    .show(ui, |ui| {
                        for (id, name, is_group) in &candidates {
                            let label = if *is_group {
                                format!("[Nhóm] {name}")
                            } else {
                                name.clone()
                            };
                            let btn = egui::Button::new(RichText::new(label).size(13.5))
                                .fill(pal.bg_input);
                            if ui
                                .add_sized([ui.available_width(), 30.0], btn)
                                .on_hover_cursor(CursorIcon::PointingHand)
                                .clicked()
                            {
                                chosen = Some((id.clone(), *is_group));
                            }
                        }
                    });
            });
        if let Some((target_id, target_is_group)) = chosen {
            if let (Some(msg), Some(tx)) = (self.forward_msg.take(), WORKER_TX.get()) {
                let _ = tx.send(WorkerCmd::Forward {
                    target_id,
                    target_is_group,
                    msg,
                });
                self.status_msg = "Đang chuyển tiếp…".into();
            }
        } else if !open {
            self.forward_msg = None;
        }
    }

    /// Detect an `http://` or `https://` URL in `self.input_message` and asynchronously fetch its
    /// OpenGraph / Zalo `parselink` metadata (`title`, `desc`, `thumb`, `src`).
    pub(in crate::gui) fn resolve_composer_link_preview(
        &mut self,
        ctx: &egui::Context,
    ) -> Option<crate::protocol::messaging::LinkMetadata> {
        let Some(url) = crate::protocol::messaging::links::extract_first_url(&self.input_message)
        else {
            self.dismissed_link_preview_for.clear();
            return None;
        };
        if self.dismissed_link_preview_for == url {
            return None;
        }
        let elapsed = self.last_composer_edit_at.elapsed();
        let debounce = Duration::from_millis(200);
        if elapsed < debounce {
            ctx.request_repaint_after(debounce - elapsed + Duration::from_millis(15));
            return self.link_metadata_cache.get(&url).cloned();
        }
        if let Some(cached) = self.link_metadata_cache.get(&url).cloned() {
            return Some(cached);
        }
        if self.loading_link_previews.insert(url.clone()) {
            if let Some(tx) = WORKER_TX.get() {
                let _ = tx.send(WorkerCmd::ParseLinkPreview { url: url.clone() });
            }
        }
        let domain = url
            .split("://")
            .nth(1)
            .and_then(|r| r.split('/').next())
            .unwrap_or("Liên kết")
            .to_string();
        Some(crate::protocol::messaging::LinkMetadata {
            url,
            title: "Đang tải thông tin liên kết…".into(),
            desc: String::new(),
            thumb: String::new(),
            src: domain,
        })
    }

    /// Render a floating Link Preview card (`egui::Area`) anchored directly above `composer_rect`
    /// when the user types or pastes a URL into the composer input.
    pub(in crate::gui) fn render_composer_link_preview_popover(
        &mut self,
        ctx: &egui::Context,
        composer_rect: Rect,
        meta: &crate::protocol::messaging::LinkMetadata,
        pal: &ThemePalette,
    ) {
        let strip_h = 72.0_f32;
        let strip_w = composer_rect.width().max(220.0);
        let pos = Pos2::new(
            composer_rect.left(),
            (composer_rect.top() - strip_h).max(48.0),
        );
        let mut dismiss_clicked = false;

        egui::Area::new(Id::new("zalo_composer_link_preview_popover"))
            .order(egui::Order::Foreground)
            .fixed_pos(pos)
            .show(ctx, |ui| {
                egui::Frame::none()
                    .fill(if pal.is_dark {
                        Color32::from_rgba_unmultiplied(33, 37, 43, 248)
                    } else {
                        Color32::from_rgba_unmultiplied(255, 255, 255, 250)
                    })
                    .stroke(Stroke::new(1.0_f32, pal.border))
                    .rounding(Rounding {
                        nw: 12.0,
                        ne: 12.0,
                        sw: 0.0,
                        se: 0.0,
                    })
                    .shadow(egui::epaint::Shadow {
                        offset: Vec2::new(0.0, -3.0),
                        blur: 12.0,
                        spread: 0.0,
                        color: Color32::from_black_alpha(if pal.is_dark { 85 } else { 24 }),
                    })
                    .inner_margin(Margin::symmetric(14.0, 8.0))
                    .show(ui, |ui| {
                        let inner_w = (strip_w - 28.0).max(160.0);
                        ui.set_min_width(inner_w);
                        ui.set_max_width(inner_w);
                        ui.set_max_height(54.0);
                        ui.horizontal(|ui| {
                            let thumb_key = if !meta.thumb.is_empty() {
                                Some(format!("thumb_{}", crate::crypto::md5_hex(&meta.thumb)))
                            } else {
                                None
                            };
                            if let Some(tkey) = thumb_key.as_deref() {
                                if let Some(tex) = self.sticker_textures.get(tkey) {
                                    ui.add(
                                        egui::Image::new(tex)
                                            .fit_to_exact_size(Vec2::new(48.0, 48.0))
                                            .rounding(Rounding::same(7.0)),
                                    );
                                    ui.add_space(6.0);
                                }
                            }
                            let text_w = (inner_w - 94.0).max(100.0);
                            ui.vertical(|ui| {
                                ui.set_max_width(text_w);
                                ui.label(
                                    RichText::new(format!(
                                        "{} {}",
                                        lucide::GLOBE,
                                        truncate_chars(&meta.title, 54)
                                    ))
                                    .size(12.5)
                                    .color(pal.primary_hover)
                                    .strong(),
                                );
                                let sub = if !meta.desc.is_empty() {
                                    truncate_chars(&meta.desc, 68).into_owned()
                                } else {
                                    clean_display_url(&meta.url, 48)
                                };
                                ui.label(RichText::new(sub).size(11.3).color(pal.text_secondary));
                            });
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                if ui
                                    .small_button(lucide::X)
                                    .on_hover_cursor(CursorIcon::PointingHand)
                                    .on_hover_text("Ẩn xem trước liên kết")
                                    .clicked()
                                {
                                    dismiss_clicked = true;
                                }
                            });
                        });
                    });
            });

        if dismiss_clicked {
            self.dismissed_link_preview_for = meta.url.clone();
        }
    }

    pub(in crate::gui) fn render_dev_logs(&mut self, ui: &mut egui::Ui) {
        let entries = logger::get_entries();
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.set_width(360.0);
                egui::ScrollArea::vertical()
                    .id_salt("dev_logs_list_scroll")
                    .show(ui, |ui| {
                        for e in entries.iter().rev() {
                            let st_str = e
                                .status
                                .map(|s| s.to_string())
                                .unwrap_or_else(|| "ERR".into());
                            let row = ui.selectable_label(
                                self.selected_log
                                    .as_ref()
                                    .map(|s| s.id == e.id)
                                    .unwrap_or(false),
                                format!(
                                    "[{}] {} {} ({}ms)",
                                    e.timestamp, e.method, st_str, e.latency_ms
                                ),
                            );
                            if row.clicked() {
                                self.selected_log = Some(e.clone());
                            }
                        }
                    });
            });

            ui.separator();

            ui.vertical(|ui| {
                if let Some(e) = &self.selected_log {
                    ui.strong(format!("{} {}", e.method, e.url));
                    ui.label(format!(
                        "Thời gian: {} · Độ trễ: {}ms · HTTP: {:?}",
                        e.timestamp, e.latency_ms, e.status
                    ));
                    if let Some(err) = &e.error_msg {
                        ui.colored_label(Color32::RED, format!("Lỗi: {err}"));
                    }
                    ui.separator();

                    egui::ScrollArea::vertical()
                        .id_salt("dev_logs_detail_scroll")
                        .show(ui, |ui| {
                            if let Some(dec) = &e.decrypted_preview {
                                ui.label("Dữ liệu giải mã (Decrypted Payload):");
                                ui.code(dec);
                            } else {
                                ui.label("Payload:");
                                ui.code(&e.payload_preview);
                            }
                        });
                } else {
                    ui.label("Chọn một request bên trái để kiểm tra chi tiết.");
                }
            });
        });
    }

    pub(in crate::gui) fn render_dev_scanner(&mut self, ui: &mut egui::Ui) {
        egui::ScrollArea::vertical()
            .id_salt("dev_scanner_scroll")
            .show(ui, |ui| {
                ui.heading("Báo cáo quét chẩn đoán tự động:");
                ui.add_space(6.0);

                if self.scanner_findings.is_empty() {
                    ui.label("Bấm 'Làm mới quét' để chạy kiểm tra.");
                }

                for f in &self.scanner_findings {
                    let (badge_text, color) = match f.level {
                        FindingLevel::Info => ("INFO", Color32::from_rgb(40, 167, 69)),
                        FindingLevel::Warning => ("CẢNH BÁO", Color32::from_rgb(255, 193, 7)),
                        FindingLevel::Critical => ("NGHIÊM TRỌNG", Color32::RED),
                    };

                    egui::Frame::group(ui.style())
                        .stroke(Stroke::new(1.0_f32, color))
                        .inner_margin(8.0)
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.colored_label(color, format!("[{badge_text}]"));
                                ui.strong(&f.title);
                            });
                            ui.label(&f.detail);
                            ui.label(
                                RichText::new(format!("Khắc phục: {}", f.suggestion))
                                    .color(Color32::from_rgb(0, 104, 255)),
                            );
                        });
                    ui.add_space(6.0);
                }
            });
    }

    pub(in crate::gui) fn render_dev_crypto(&mut self, ui: &mut egui::Ui) {
        egui::ScrollArea::vertical()
            .id_salt("dev_crypto_scroll")
            .show(ui, |ui| {
                ui.heading("Thông số mật mã & Khung giao thức:");
                ui.add_space(8.0);

                egui::Grid::new("crypto_grid").striped(true).show(ui, |ui| {
                    ui.label("API Version (zpw_ver):");
                    ui.code(crate::session::ZPW_VER);
                    ui.end_row();

                    ui.label("API Type (zpw_type):");
                    ui.code(crate::session::ZPW_TYPE);
                    ui.end_row();

                    ui.label("ParamCipher Master Key:");
                    ui.code(format!("{:02X?}", crate::param_cipher::PARAM_CIPHER_KEY));
                    ui.end_row();

                    ui.label("Zero IV:");
                    ui.code("00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 (16 zero bytes)");
                    ui.end_row();

                    ui.label("Thuật toán mã hoá:");
                    ui.label("AES-128-CBC / AES-256-CBC with PKCS7 padding");
                    ui.end_row();

                    ui.label("Chữ ký SignKey:");
                    ui.code("md5('zsecure' + tag + sorted_values)");
                    ui.end_row();
                });
            });
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn unread_badge_icon_and_device_filter_behave_correctly() {
        let icon0 = crate::gui::theme::load_app_icon_with_badge(0).unwrap();
        assert_eq!(icon0.rgba.len(), (icon0.width * icon0.height * 4) as usize);

        let icon5 = crate::gui::theme::load_app_icon_with_badge(5).unwrap();
        assert_eq!((icon5.width, icon5.height), (64, 64));
        assert_eq!(icon5.rgba.len(), 64 * 64 * 4);

        let icon99 = crate::gui::theme::load_app_icon_with_badge(125).unwrap();
        assert_eq!((icon99.width, icon99.height), (64, 64));

        assert!(crate::media::is_ignored_virtual_or_ir_device(
            "Integrated IR Camera"
        ));
        assert!(crate::media::is_ignored_virtual_or_ir_device(
            "Windows Hello Depth Camera"
        ));
        assert!(crate::media::is_ignored_virtual_or_ir_device(
            "Monitor of Built-in Audio"
        ));
        assert!(!crate::media::is_ignored_virtual_or_ir_device(
            "Integrated Webcam"
        ));
    }
}
