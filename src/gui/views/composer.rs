use crate::gui::theme::*;
use crate::gui::*;
use eframe::egui;

impl ZaloGuiApp {
    pub(in crate::gui) fn render_composer_bar(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        pal: &ThemePalette,
        thread_id: &str,
        thread_name: &str,
        is_group: bool,
    ) {
        let is_narrow_w = false;
        let contextual_stickers = self.resolve_contextual_sticker_suggestions(ctx);
        let composer_link_preview = self.resolve_composer_link_preview(ctx);

        let composer_frame = egui::Frame::none()
            .fill(pal.bg_sidebar)
            .inner_margin(Margin::symmetric(14.0, 10.0))
            .stroke(Stroke::new(1.0_f32, pal.border))
            .show(ui, |ui| {
                // Handle drag-and-drop files onto the chat window
                let dropped: Vec<std::path::PathBuf> = ui.ctx().input(|i| {
                    i.raw
                        .dropped_files
                        .iter()
                        .filter_map(|df| df.path.clone())
                        .collect()
                });
                for dpath in dropped {
                    if let Some(tx) = WORKER_TX.get() {
                        let _ = tx.send(WorkerCmd::PickAndSendAttachment {
                            to_id: thread_id.to_string(),
                            is_group,
                            images_only: false,
                            caption: String::new(),
                            preset_path: Some(dpath),
                        });
                    }
                }

                let is_ime_busy = self.ime_active || self.ime_just_committed;
                let is_cmd_enter_mode = self.settings.send_message_key == "cmd_enter";
                let (send_via_enter, insert_newline_via_enter) = ui.input(|i| {
                    let ent = i.key_pressed(Key::Enter) && !is_ime_busy;
                    let cmd_or_ctrl = i.modifiers.command || i.modifiers.ctrl;
                    if is_cmd_enter_mode {
                        (ent && cmd_or_ctrl, ent && !cmd_or_ctrl)
                    } else {
                        (ent && !i.modifiers.shift, ent && i.modifiers.shift)
                    }
                });
                if send_via_enter || insert_newline_via_enter {
                    ui.input_mut(|i| {
                        i.events.retain(|ev| {
                            !matches!(
                                ev,
                                egui::Event::Key {
                                    key: Key::Enter,
                                    pressed: true,
                                    ..
                                }
                            ) && !matches!(
                                ev,
                                egui::Event::Text(t)
                                    if t == "\n" || t == "\r" || t == "\r\n"
                            )
                        });
                    });
                }

                egui::Frame::none()
                    .fill(pal.bg_input)
                    .rounding(Rounding::same(11.0))
                    .stroke(Stroke::new(1.0_f32, pal.border))
                    .inner_margin(Margin::symmetric(8.0, 5.0))
                    .show(ui, |ui| {
                        let mut send_clicked = false;
                        let mut send_selected_sticker: Option<
                            crate::protocol::messaging::StickerItem,
                        > = None;
                        let composer_id = Id::new(("chat_composer_input", &thread_id));

                        let response = ui
                            .horizontal(|ui| {
                                ui.spacing_mut().item_spacing.x = 4.0;

                                // 1) Compact Sticker Icon Button
                                let (stk_rect, stk_resp) =
                                    ui.allocate_exact_size(Vec2::new(34.0, 34.0), Sense::click());
                                if self.sticker_picker_open || stk_resp.hovered() {
                                    ui.painter().rect_filled(
                                        stk_rect,
                                        Rounding::same(8.0),
                                        pal.bg_hover,
                                    );
                                }
                                draw_icon_sticker(
                                    ui.painter(),
                                    stk_rect.center(),
                                    18.0,
                                    if self.sticker_picker_open || stk_resp.hovered() {
                                        Color32::from_rgb(251, 191, 36)
                                    } else {
                                        pal.text_secondary
                                    },
                                );
                                if stk_resp
                                    .on_hover_cursor(CursorIcon::PointingHand)
                                    .on_hover_text("Nhãn dán (Sticker)")
                                    .clicked()
                                {
                                    self.sticker_picker_open = !self.sticker_picker_open;
                                }

                                // 2) Compact Photo Upload Icon Button
                                let (img_rect, img_resp) =
                                    ui.allocate_exact_size(Vec2::new(34.0, 34.0), Sense::click());
                                if img_resp.hovered() {
                                    ui.painter().rect_filled(
                                        img_rect,
                                        Rounding::same(8.0),
                                        pal.bg_hover,
                                    );
                                }
                                draw_icon_image(
                                    ui.painter(),
                                    img_rect.center(),
                                    18.0,
                                    if img_resp.hovered() {
                                        pal.accent_online
                                    } else {
                                        pal.text_secondary
                                    },
                                );
                                if img_resp
                                    .on_hover_cursor(CursorIcon::PointingHand)
                                    .on_hover_text("Gửi hình ảnh")
                                    .clicked()
                                {
                                    let cap = self.input_message.trim().to_string();
                                    self.input_message.clear();
                                    if let Some(tx) = WORKER_TX.get() {
                                        let _ = tx.send(WorkerCmd::PickAndSendAttachment {
                                            to_id: thread_id.to_string(),
                                            is_group,
                                            images_only: true,
                                            caption: cap,
                                            preset_path: None,
                                        });
                                    }
                                }

                                // 3) Compact File Upload Icon Button
                                let (att_rect, att_resp) =
                                    ui.allocate_exact_size(Vec2::new(34.0, 34.0), Sense::click());
                                if att_resp.hovered() {
                                    ui.painter().rect_filled(
                                        att_rect,
                                        Rounding::same(8.0),
                                        pal.bg_hover,
                                    );
                                }
                                draw_icon_attachment(
                                    ui.painter(),
                                    att_rect.center(),
                                    17.0,
                                    if att_resp.hovered() {
                                        pal.primary_hover
                                    } else {
                                        pal.text_secondary
                                    },
                                );
                                if att_resp
                                    .on_hover_cursor(CursorIcon::PointingHand)
                                    .on_hover_text("Đính kèm File / Video")
                                    .clicked()
                                {
                                    let cap = self.input_message.trim().to_string();
                                    self.input_message.clear();
                                    if let Some(tx) = WORKER_TX.get() {
                                        let _ = tx.send(WorkerCmd::PickAndSendAttachment {
                                            to_id: thread_id.to_string(),
                                            is_group,
                                            images_only: false,
                                            caption: cap,
                                            preset_path: None,
                                        });
                                    }
                                }

                                let is_multiline = self.input_message.contains('\n');
                                let vis_lines = if is_multiline {
                                    self.input_message.split('\n').count().clamp(2, 5)
                                } else {
                                    1
                                };
                                let box_h = if is_multiline {
                                    14.0 + (vis_lines as f32 * 20.0)
                                } else {
                                    34.0
                                };

                                // Vertical hairline separator
                                let (sep_r, _) = ui.allocate_exact_size(
                                    Vec2::new(8.0, (box_h - 10.0).clamp(20.0, 72.0)),
                                    Sense::hover(),
                                );
                                ui.painter().line_segment(
                                    [sep_r.center_top(), sep_r.center_bottom()],
                                    Stroke::new(1.0_f32, pal.border),
                                );

                                let send_btn_w = if is_narrow_w { 52.0 } else { 76.0 };
                                let sched_btn_w = 34.0;
                                let input_w =
                                    (ui.available_width() - send_btn_w - sched_btn_w - 16.0)
                                        .max(50.0);
                                let hint_str = format!(
                                    "Nhập tin nhắn tới {}...",
                                    truncate_chars(thread_name, 18)
                                );
                                let resp = if is_multiline {
                                    ui.allocate_ui_with_layout(
                                        Vec2::new(input_w, box_h),
                                        Layout::top_down(Align::Min),
                                        |ui| {
                                            ui.set_min_size(Vec2::new(input_w, box_h));
                                            ui.set_max_size(Vec2::new(input_w, box_h));
                                            egui::ScrollArea::vertical()
                                                .id_salt(("composer_fixed_scroll", &thread_id))
                                                .max_width(input_w)
                                                .max_height(box_h)
                                                .auto_shrink([false, false])
                                                .stick_to_bottom(true)
                                                .show(ui, |ui| {
                                                    ui.add(
                                                        egui::TextEdit::multiline(
                                                            &mut self.input_message,
                                                        )
                                                        .id(composer_id)
                                                        .desired_rows(vis_lines)
                                                        .frame(false)
                                                        .hint_text(hint_str)
                                                        .margin(Margin::symmetric(6.0, 6.0))
                                                        .desired_width(input_w - 10.0),
                                                    )
                                                })
                                                .inner
                                        },
                                    )
                                    .inner
                                } else {
                                    ui.add_sized(
                                        [input_w, 34.0],
                                        egui::TextEdit::singleline(&mut self.input_message)
                                            .id(composer_id)
                                            .frame(false)
                                            .hint_text(hint_str)
                                            .margin(Margin::symmetric(6.0, 7.0))
                                            .desired_width(input_w),
                                    )
                                };
                                if resp.changed()
                                    && !self.input_message.trim().is_empty()
                                    && !self.settings.block_typing
                                {
                                    let should_emit = self
                                        .last_typing_sent_at
                                        .as_ref()
                                        .map(|(last_tid, t)| {
                                            last_tid != thread_id
                                                || t.elapsed()
                                                    >= std::time::Duration::from_millis(3200)
                                        })
                                        .unwrap_or(true);
                                    if should_emit {
                                        self.last_typing_sent_at = Some((
                                            thread_id.to_string(),
                                            std::time::Instant::now(),
                                        ));
                                        if let Some(tx) = WORKER_TX.get() {
                                            let _ = tx.send(WorkerCmd::SendTyping {
                                                thread_id: thread_id.to_string(),
                                                is_group,
                                            });
                                        }
                                    }
                                }

                                // Schedule Send Button (`🕒`)
                                let (sched_rect, sched_resp) =
                                    ui.allocate_exact_size(Vec2::new(32.0, 34.0), Sense::click());
                                if self.schedule_modal_open || sched_resp.hovered() {
                                    ui.painter().rect_filled(
                                        sched_rect,
                                        Rounding::same(8.0),
                                        pal.bg_hover,
                                    );
                                }
                                ui.painter().text(
                                    sched_rect.center(),
                                    egui::Align2::CENTER_CENTER,
                                    "🕒",
                                    FontId::proportional(14.5),
                                    if self.schedule_modal_open || sched_resp.hovered() {
                                        pal.primary_hover
                                    } else {
                                        pal.text_secondary
                                    },
                                );
                                if sched_resp
                                    .on_hover_cursor(CursorIcon::PointingHand)
                                    .on_hover_text("Hẹn giờ gửi tin nhắn & Nhắc hẹn")
                                    .clicked()
                                {
                                    self.schedule_modal_open = !self.schedule_modal_open;
                                }

                                // Send Button
                                let can_send = !self.input_message.trim().is_empty()
                                    || self.pending_clipboard_image.is_some();
                                let (send_rect, send_resp) = ui.allocate_exact_size(
                                    Vec2::new(send_btn_w, 34.0),
                                    Sense::click(),
                                );
                                if send_resp.secondary_clicked() {
                                    self.schedule_modal_open = true;
                                }
                                let send_bg = if !can_send {
                                    pal.primary.linear_multiply(0.42)
                                } else if send_resp.hovered() {
                                    pal.primary_hover
                                } else {
                                    pal.primary
                                };
                                ui.painter()
                                    .rect_filled(send_rect, Rounding::same(8.0), send_bg);
                                if !is_narrow_w {
                                    ui.painter().text(
                                        Pos2::new(send_rect.left() + 15.0, send_rect.center().y),
                                        egui::Align2::LEFT_CENTER,
                                        "Gửi",
                                        FontId::proportional(13.0),
                                        Color32::WHITE,
                                    );
                                    draw_icon_send(
                                        ui.painter(),
                                        Pos2::new(send_rect.right() - 17.0, send_rect.center().y),
                                        14.0,
                                        Color32::WHITE,
                                    );
                                } else {
                                    draw_icon_send(
                                        ui.painter(),
                                        send_rect.center(),
                                        16.0,
                                        Color32::WHITE,
                                    );
                                }
                                send_clicked = send_resp
                                    .on_hover_cursor(if can_send {
                                        CursorIcon::PointingHand
                                    } else {
                                        CursorIcon::Default
                                    })
                                    .clicked();
                                resp
                            })
                            .inner;

                        if is_ime_busy && response.lost_focus() {
                            response.request_focus();
                        }
                        if insert_newline_via_enter
                            && (response.has_focus() || response.lost_focus())
                        {
                            if let Some(mut st) = egui::TextEdit::load_state(ui.ctx(), composer_id)
                            {
                                let char_idx = st
                                    .cursor
                                    .char_range()
                                    .map(|r| r.primary.index)
                                    .unwrap_or_else(|| self.input_message.chars().count());
                                let byte_idx = self
                                    .input_message
                                    .char_indices()
                                    .nth(char_idx)
                                    .map(|(i, _)| i)
                                    .unwrap_or(self.input_message.len());
                                self.input_message.insert(byte_idx, '\n');
                                let next_c = egui::text::CCursor::new(char_idx + 1);
                                st.cursor
                                    .set_char_range(Some(egui::text::CCursorRange::one(next_c)));
                                st.store(ui.ctx(), composer_id);
                            } else {
                                self.input_message.push('\n');
                            }
                            response.request_focus();
                        }
                        if response.changed() {
                            ensure_cjk_font(ui.ctx(), std::iter::once(self.input_message.as_str()));
                        }
                        if self.composer_focus_pending {
                            response.request_focus();
                            self.composer_focus_pending = false;
                        }
                        response.context_menu(|ui| {
                            if ui
                                .button(format!(
                                    "{} Dán hình ảnh từ Clipboard (Ctrl+V)",
                                    lucide::IMAGE
                                ))
                                .clicked()
                            {
                                ui.close_menu();
                                if let Some((path, w, h, col_img)) = read_clipboard_image_to_png() {
                                    let tex = ctx.load_texture(
                                        format!("clip_preview_{}", crate::clock::unix_now()),
                                        col_img,
                                        TextureOptions::LINEAR,
                                    );
                                    self.pending_clipboard_image = Some((path, w, h, tex));
                                    self.composer_focus_pending = true;
                                    self.show_toast(format!(
                                        "Đã dán ảnh ({w}×{h} px) từ Clipboard — nhấn Enter để gửi"
                                    ));
                                } else {
                                    self.show_toast("Không tìm thấy hình ảnh nào trong Clipboard");
                                }
                            }
                        });

                        if !contextual_stickers.is_empty() && response.has_focus() {
                            let len = contextual_stickers.len();
                            if ui.input(|i| i.key_pressed(Key::ArrowUp)) {
                                self.composer_sticker_selected = Some(0);
                            } else if let Some(sel) = self.composer_sticker_selected {
                                if ui.input(|i| i.key_pressed(Key::ArrowRight)) {
                                    self.composer_sticker_selected = Some((sel + 1) % len);
                                } else if ui.input(|i| i.key_pressed(Key::ArrowLeft)) {
                                    self.composer_sticker_selected = Some((sel + len - 1) % len);
                                } else if ui.input(|i| i.key_pressed(Key::ArrowDown)) {
                                    self.composer_sticker_selected = None;
                                }
                            }
                        } else {
                            self.composer_sticker_selected = None;
                        }

                        let enter_pressed = (response.lost_focus() || response.has_focus())
                            && send_via_enter
                            && !is_ime_busy;
                        if enter_pressed {
                            if let Some(sel) = self.composer_sticker_selected {
                                if let Some(st) = contextual_stickers.get(sel) {
                                    send_selected_sticker = Some(st.clone());
                                }
                            }
                        }
                        let can_send = !self.input_message.trim().is_empty()
                            || self.pending_clipboard_image.is_some();

                        if let Some(chosen) = send_selected_sticker {
                            self.input_message.clear();
                            self.dismissed_sticker_suggest_for.clear();
                            self.composer_sticker_selected = None;
                            response.request_focus();
                            if let Some(tx) = WORKER_TX.get() {
                                let _ = tx.send(WorkerCmd::SendSticker {
                                    to_id: thread_id.to_string(),
                                    sticker_id: chosen.sticker_id,
                                    cate_id: chosen.cate_id,
                                    sticker_type: chosen.sticker_type,
                                    is_group,
                                });
                            }
                        } else if (send_clicked || enter_pressed)
                            && self.pending_clipboard_image.is_some()
                        {
                            if let Some((clip_path, _, _, _)) = self.pending_clipboard_image.take()
                            {
                                let cap = self.input_message.trim().to_string();
                                self.input_message.clear();
                                self.dismissed_sticker_suggest_for.clear();
                                self.composer_sticker_selected = None;
                                response.request_focus();
                                if let Some(tx) = WORKER_TX.get() {
                                    let _ = tx.send(WorkerCmd::PickAndSendAttachment {
                                        to_id: thread_id.to_string(),
                                        is_group,
                                        images_only: true,
                                        caption: cap,
                                        preset_path: Some(clip_path),
                                    });
                                }
                            }
                        } else if (send_clicked || enter_pressed) && can_send {
                            let text = self.input_message.trim().to_string();
                            self.input_message.clear();
                            self.dismissed_sticker_suggest_for.clear();
                            self.composer_sticker_selected = None;
                            response.request_focus();
                            let quoted = self.replying_to.take();
                            if let Some(tx) = WORKER_TX.get() {
                                if let Some(quoted) = quoted {
                                    let _ = tx.send(WorkerCmd::SendQuote {
                                        to_id: thread_id.to_string(),
                                        text,
                                        quoted,
                                        is_group,
                                    });
                                } else {
                                    let _ = tx.send(WorkerCmd::SendMessage {
                                        to_id: thread_id.to_string(),
                                        text,
                                        is_group,
                                    });
                                }
                            }
                        } else if enter_pressed {
                            response.request_focus();
                        }
                    });
            });

        // 4b. Floating Contextual Sticker Suggestion Popover (`egui::Area`, rendered AFTER composer bar)
        if !contextual_stickers.is_empty() && !self.sticker_picker_open {
            self.render_contextual_sticker_popover(
                ctx,
                composer_frame.response.rect,
                &contextual_stickers,
                thread_id,
                is_group,
                pal,
            );
        }
        if let Some(ref link_meta) = composer_link_preview {
            if contextual_stickers.is_empty() && !self.sticker_picker_open {
                self.render_composer_link_preview_popover(
                    ctx,
                    composer_frame.response.rect,
                    link_meta,
                    pal,
                );
            }
        }
    }
}
