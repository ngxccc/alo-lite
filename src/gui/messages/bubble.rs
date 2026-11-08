use super::cards::*;
use super::date_divider::*;
use super::markdown::*;
use super::os_utils::*;
use crate::gui::theme::*;
use crate::gui::*;
use eframe::egui::{self};

pub fn same_message_cluster(a: &ChatMessage, b: &ChatMessage) -> bool {
    if is_system_notice_message(a) || is_system_notice_message(b) {
        return false;
    }
    if a.is_outgoing != b.is_outgoing || a.sender != b.sender {
        return false;
    }
    if should_show_time_divider(Some(a), b) {
        return false;
    }
    let a_ms = normalize_ts_ms(a.ts);
    let b_ms = normalize_ts_ms(b.ts);
    if a_ms > 0 && b_ms > 0 && (b_ms - a_ms).abs() > 10 * 60 * 1000 {
        return false;
    }
    true
}

pub fn render_message_body(
    ui: &mut egui::Ui,
    msg: &ChatMessage,
    thread_name: &str,
    pal: &ThemePalette,
    outgoing: bool,
    max_text_width: f32,
    sticker_textures: &HashMap<String, TextureHandle>,
    loading_stickers: &mut HashSet<String>,
) -> Option<BubbleAction> {
    let mut body_action = None;
    let (text_col, meta_col) = if outgoing {
        (pal.bubble_out_text, pal.bubble_out_meta)
    } else {
        (pal.bubble_in_text, pal.bubble_in_meta)
    };

    let trimmed_text = msg.text.trim();
    if trimmed_text == "[Tin nhắn đã thu hồi]" {
        ui.label(
            RichText::new(format!("{} Tin nhắn đã được thu hồi", lucide::BAN))
                .color(meta_col)
                .italics()
                .size(13.8),
        );
        return None;
    }

    let sender_label = if msg.is_outgoing {
        "Bạn".to_string()
    } else if !msg.sender.is_empty() && msg.sender != "0" {
        msg.sender.clone()
    } else {
        thread_name.to_string()
    };

    // Dedicated inline Photo Card when the message is an uploaded/received Zalo photo (#72)
    let is_zalo_photo = is_zalo_photo_message(msg);
    if is_zalo_photo {
        if let Some(thumb_url) = photo_primary_url(msg) {
            let key = format!("thumb_{}", crate::crypto::md5_hex(thumb_url));
            if !sticker_textures.contains_key(&key) && loading_stickers.insert(key.clone()) {
                if let Some(tx) = WORKER_TX.get() {
                    let _ = tx.send(WorkerCmd::FetchStickerImage {
                        key: key.clone(),
                        url: thumb_url.to_string(),
                    });
                }
            }
            let open_target = msg
                .link_url
                .as_deref()
                .filter(|u| !u.is_empty())
                .unwrap_or(thumb_url)
                .to_string();
            let title_label = msg
                .link_title
                .as_deref()
                .map(str::trim)
                .filter(|t| {
                    !t.is_empty() && !t.starts_with("http://") && !t.starts_with("https://")
                })
                .unwrap_or("Hình ảnh")
                .to_string();
            if let Some(tex) = sticker_textures.get(&key) {
                let [tw, th] = tex.size();
                let tw_f = (tw.max(1)) as f32;
                let th_f = (th.max(1)) as f32;
                let max_w = max_text_width.clamp(220.0, 360.0);
                let max_h = 340.0_f32;
                let scale = (max_w / tw_f).min(max_h / th_f).min(1.0);
                let disp_w = (tw_f * scale).clamp(160.0, max_w);
                let disp_h = (th_f * scale).clamp(120.0, max_h);
                let img = egui::Image::new(tex)
                    .fit_to_exact_size(Vec2::new(disp_w, disp_h))
                    .rounding(Rounding::same(12.0))
                    .sense(Sense::click());
                let resp = ui
                    .add(img)
                    .on_hover_cursor(CursorIcon::PointingHand)
                    .on_hover_text("Bấm để xem ảnh trong ứng dụng");
                ui.painter().rect_stroke(
                    resp.rect,
                    Rounding::same(12.0),
                    Stroke::new(
                        if resp.hovered() { 1.6_f32 } else { 1.0_f32 },
                        if resp.hovered() {
                            pal.primary
                        } else {
                            pal.border
                        },
                    ),
                );
                if resp.hovered() {
                    ui.painter().rect_filled(
                        resp.rect,
                        Rounding::same(12.0),
                        Color32::from_rgba_unmultiplied(6, 10, 20, 88),
                    );
                    let pill_r = Rect::from_center_size(resp.rect.center(), Vec2::new(108.0, 26.0));
                    ui.painter().rect_filled(
                        pill_r,
                        Rounding::same(13.0),
                        Color32::from_rgba_unmultiplied(15, 23, 42, 225),
                    );
                    ui.painter().text(
                        pill_r.center(),
                        egui::Align2::CENTER_CENTER,
                        "Phóng to ảnh",
                        FontId::proportional(11.5),
                        Color32::WHITE,
                    );
                }
                if resp.clicked() {
                    body_action = Some(BubbleAction::OpenImagePopup {
                        tex_key: key,
                        full_url: open_target,
                        title: title_label,
                        sender: sender_label,
                        time: msg.time.clone(),
                    });
                }
            } else {
                let ph_frame = egui::Frame::none()
                    .fill(text_col.linear_multiply(0.08))
                    .rounding(Rounding::same(10.0))
                    .inner_margin(Margin::symmetric(12.0, 10.0))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            let (ir, _) = ui.allocate_exact_size(Vec2::splat(22.0), Sense::hover());
                            draw_icon_image(ui.painter(), ir.center(), 18.0, text_col);
                            ui.label(
                                RichText::new(&title_label)
                                    .size(13.5)
                                    .color(text_col)
                                    .strong(),
                            );
                        });
                    });
                if ui
                    .interact(
                        ph_frame.response.rect,
                        Id::new(("photo_ph_click", &msg.id, msg.ts)),
                        Sense::click(),
                    )
                    .on_hover_cursor(CursorIcon::PointingHand)
                    .on_hover_text("Bấm để xem hình ảnh")
                    .clicked()
                {
                    body_action = Some(BubbleAction::OpenImagePopup {
                        tex_key: key,
                        full_url: open_target,
                        title: title_label,
                        sender: sender_label,
                        time: msg.time.clone(),
                    });
                }
            }
            if let Some(cap) = extract_photo_user_caption(msg) {
                ui.add_space(5.0);
                render_chat_markdown_block(ui, cap, text_col, pal);
            }
            return body_action;
        }
    }
    // Dedicated File / Video Attachment Card (#74)
    let is_zalo_file_or_video = is_zalo_file_or_video_message(msg);
    if is_zalo_file_or_video {
        let is_vid =
            msg.link_src.as_deref() == Some("Video Zalo") || trimmed_text.starts_with("[Video]");
        let url_str = msg.link_url.as_deref().unwrap_or_default();
        let resolved_file_name =
            crate::protocol::messaging::attachments::resolve_attachment_file_name(
                msg.link_title.as_deref(),
                trimmed_text,
                url_str,
            );
        let title_str = if is_vid {
            msg.link_title
                .as_deref()
                .filter(|s| !s.is_empty())
                .unwrap_or(trimmed_text)
                .to_string()
        } else {
            resolved_file_name.clone()
        };
        let sub_str = if is_vid {
            "Video đính kèm · Bấm để mở"
        } else {
            "Tập tin Zalo · Bấm để lưu về máy"
        };
        let link_col = if pal.is_dark {
            Color32::from_rgb(97, 175, 239)
        } else {
            pal.primary
        };
        let resp = ui
            .add(
                egui::Label::new(
                    RichText::new(&title_str)
                        .size(13.8)
                        .color(text_col)
                        .strong(),
                )
                .wrap()
                .sense(Sense::click()),
            )
            .on_hover_cursor(CursorIcon::PointingHand)
            .on_hover_text(if is_vid {
                "Bấm để mở video"
            } else {
                "Bấm để tải và lưu tập tin với tên gốc"
            });
        ui.add_space(2.0);
        let dom_resp = ui
            .add(
                egui::Label::new(RichText::new(sub_str).size(12.5).color(link_col))
                    .sense(Sense::click()),
            )
            .on_hover_cursor(CursorIcon::PointingHand);
        if resp.clicked() || dom_resp.clicked() {
            if let Some(url) = msg.link_url.as_deref().filter(|u| !u.is_empty()) {
                if is_vid && !url.to_ascii_lowercase().contains("/renewmedia") {
                    open_external_url(url);
                } else {
                    return Some(BubbleAction::DownloadFile {
                        url: url.to_string(),
                        file_name: resolved_file_name,
                    });
                }
            }
        }
        return None;
    }

    let Some((caption, url, domain)) = extract_message_link_view(msg) else {
        render_chat_markdown_block(ui, &msg.text, text_col, pal);
        return None;
    };

    let link_col = if pal.is_dark {
        Color32::from_rgb(97, 175, 239) // #61AFEF Zalo/Atom clean link blue
    } else {
        pal.primary
    };

    // Auto-enrich link metadata if title or thumbnail is missing/expired
    let needs_meta_enrich = msg.link_title.is_none()
        || msg.link_thumb.is_none()
        || msg
            .link_thumb
            .as_deref()
            .map(|th| {
                th.contains("zpcloud.zdn.vn")
                    && !sticker_textures
                        .contains_key(&format!("thumb_{}", crate::crypto::md5_hex(th)))
            })
            .unwrap_or(false);
    if needs_meta_enrich {
        let meta_key = format!("linkmeta_{}", crate::crypto::md5_hex(&url));
        if loading_stickers.insert(meta_key) {
            if let Some(tx) = WORKER_TX.get() {
                let _ = tx.send(WorkerCmd::ParseLinkPreview { url: url.clone() });
            }
        }
    }

    // 1. Optional user comment above the link
    if !caption.is_empty() && caption != url {
        render_chat_markdown_block(ui, &caption, text_col, pal);
        ui.add_space(4.0);
    }

    // 2. Optional compact thumbnail directly inside the bubble (no nested card box)
    if let Some(thumb_url) = msg.link_thumb.as_deref().filter(|u| !u.is_empty()) {
        let key = format!("thumb_{}", crate::crypto::md5_hex(thumb_url));
        if !sticker_textures.contains_key(&key)
            && (thumb_url.starts_with("http://") || thumb_url.starts_with("https://"))
            && loading_stickers.insert(key.clone())
        {
            if let Some(tx) = WORKER_TX.get() {
                let _ = tx.send(WorkerCmd::FetchStickerImage {
                    key: key.clone(),
                    url: thumb_url.to_string(),
                });
            }
        }
        if let Some(tex) = sticker_textures.get(&key) {
            let [tw, th] = tex.size();
            let aspect = if tw > 0 && th > 0 {
                (th as f32 / tw as f32).clamp(0.45, 0.85)
            } else {
                0.56
            };
            let thumb_w = max_text_width.min(260.0);
            let img_h = (thumb_w * aspect).min(132.0);
            let img = egui::Image::new(tex)
                .fit_to_exact_size(Vec2::new(thumb_w, img_h))
                .rounding(Rounding::same(6.0))
                .sense(Sense::click());
            if ui
                .add(img)
                .on_hover_cursor(CursorIcon::PointingHand)
                .on_hover_text(&url)
                .clicked()
            {
                open_external_url(&url);
            }
            ui.add_space(5.0);
        }
    }

    // 3. Bold Title (exact Zalo PC style: no globe icon, no vertical accent bar, no nested border)
    let header_title = msg
        .link_title
        .as_deref()
        .map(str::trim)
        .filter(|t| !t.is_empty() && *t != domain)
        .unwrap_or_else(|| {
            url.strip_prefix("https://")
                .or_else(|| url.strip_prefix("http://"))
                .unwrap_or(&url)
        });
    let title_resp = ui
        .add(
            egui::Label::new(
                RichText::new(truncate_chars(header_title, 95))
                    .size(13.8)
                    .color(text_col)
                    .strong(),
            )
            .wrap()
            .sense(Sense::click()),
        )
        .on_hover_cursor(CursorIcon::PointingHand)
        .on_hover_text(&url);
    if title_resp.clicked() {
        open_external_url(&url);
    }

    // 4. Optional short description (only if concise and distinct)
    let desc_opt = msg
        .link_desc
        .as_deref()
        .map(str::trim)
        .filter(|d| !d.is_empty() && *d != header_title && *d != url && caption.is_empty());
    if let Some(desc) = desc_opt {
        ui.add_space(2.0);
        let d_resp = ui
            .add(
                egui::Label::new(
                    RichText::new(truncate_chars(desc, 110))
                        .size(12.3)
                        .color(meta_col),
                )
                .wrap()
                .sense(Sense::click()),
            )
            .on_hover_cursor(CursorIcon::PointingHand)
            .on_hover_text(&url);
        if d_resp.clicked() {
            open_external_url(&url);
        }
    }

    // 5. Clean Domain line (`docs.google.com`, `drive.google.com`, `tiktok.com`)
    ui.add_space(2.5);
    let dom_resp = ui
        .add(
            egui::Label::new(RichText::new(&domain).size(12.6).color(link_col))
                .sense(Sense::click()),
        )
        .on_hover_cursor(CursorIcon::PointingHand)
        .on_hover_text(&url);
    if dom_resp.clicked() {
        open_external_url(&url);
    }

    body_action
}

pub fn is_zalo_file_or_video_message(msg: &ChatMessage) -> bool {
    let trimmed = msg.text.trim();
    matches!(
        msg.link_src.as_deref(),
        Some("Tập tin Zalo") | Some("Video Zalo")
    ) || trimmed.starts_with("[Tập tin]")
        || trimmed.starts_with("[Video]")
        || trimmed.starts_with("[Tin nhắn thoại]")
        || msg
            .link_url
            .as_deref()
            .is_some_and(|u| u.to_ascii_lowercase().contains("/renewmedia"))
}

pub fn photo_primary_url(msg: &ChatMessage) -> Option<&str> {
    msg.link_thumb
        .as_deref()
        .filter(|u| !u.is_empty())
        .or_else(|| msg.link_url.as_deref().filter(|u| !u.is_empty()))
        .or_else(|| {
            let t = msg.text.trim();
            crate::protocol::messaging::attachments::is_zalo_photo_cdn_url(t).then_some(t)
        })
}

pub fn is_zalo_photo_message(msg: &ChatMessage) -> bool {
    if is_sticker_message(msg).is_some() || is_zalo_file_or_video_message(msg) {
        return false;
    }
    let trimmed = msg.text.trim();
    msg.link_src.as_deref() == Some("Hình ảnh Zalo")
        || trimmed == "[Hình ảnh]"
        || trimmed.starts_with("[Hình ảnh] ")
        || msg
            .link_url
            .as_deref()
            .is_some_and(crate::protocol::messaging::attachments::is_zalo_photo_cdn_url)
        || msg
            .link_thumb
            .as_deref()
            .is_some_and(crate::protocol::messaging::attachments::is_zalo_photo_cdn_url)
        || crate::protocol::messaging::attachments::is_zalo_photo_cdn_url(trimmed)
}

pub fn extract_photo_user_caption(msg: &ChatMessage) -> Option<&str> {
    let is_valid_caption = |s: &str| -> bool {
        let t = s.trim();
        !t.is_empty()
            && t != "[Hình ảnh]"
            && !t.starts_with("http://")
            && !t.starts_with("https://")
            && Some(t) != msg.link_src.as_deref()
    };
    if let Some(desc) = msg.link_desc.as_deref().map(str::trim) {
        if is_valid_caption(desc) {
            return Some(desc);
        }
    }
    let trimmed = msg.text.trim();
    let stripped = trimmed
        .strip_prefix("[Hình ảnh]")
        .map(str::trim)
        .unwrap_or(trimmed);
    if is_valid_caption(stripped) {
        Some(stripped)
    } else {
        None
    }
}

pub fn render_chat_bubble(
    ui: &mut egui::Ui,
    msg: &ChatMessage,
    prev_same_sender: bool,
    next_same_sender: bool,
    is_latest_outgoing: bool,
    is_group: bool,
    thread_name: &str,
    sender_avatar: Option<&str>,
    pal: &ThemePalette,
    sticker_textures: &HashMap<String, TextureHandle>,
    loading_stickers: &mut HashSet<String>,
    emoji_textures: &HashMap<String, TextureHandle>,
    idx: usize,
) -> Option<BubbleAction> {
    let avail_w = ui.available_width();
    let max_bubble_width = (avail_w * 0.72).clamp(140.0, 520.0);
    let max_text_width = (max_bubble_width - 24.0).max(90.0);
    let has_sticker = is_sticker_message(msg).is_some();
    let call_card_opt = extract_call_card_view(msg);
    let is_call_card = call_card_opt.is_some();
    let is_photo_card = !has_sticker && is_zalo_photo_message(msg);
    let is_file_card = !has_sticker && is_zalo_file_or_video_message(msg);
    let has_link_card =
        !has_sticker && !is_photo_card && !is_file_card && extract_message_link_view(msg).is_some();
    let has_interactive_card = is_call_card || has_link_card || is_photo_card || is_file_card;
    if !has_sticker && !has_interactive_card && msg.text.trim().is_empty() {
        return None;
    }

    let show_timestamp = !msg.time.is_empty()
        && (!next_same_sender
            || is_latest_outgoing
            || has_interactive_card
            || !msg.reactions.is_empty()
            || msg.is_pinned
            || msg.is_starred);

    let photo_caption = if is_photo_card {
        extract_photo_user_caption(msg)
    } else {
        None
    };
    let has_photo_caption = photo_caption.is_some();
    let is_frameless_photo = is_photo_card
        && !has_photo_caption
        && msg.reply_to_text.is_none()
        && (msg.is_outgoing || !is_group);

    let sender_is_name = !msg.sender.is_empty()
        && msg.sender != "0"
        && !msg.sender.chars().all(|c| c.is_ascii_digit());
    let show_group_sender = !msg.is_outgoing && !prev_same_sender && is_group && sender_is_name;

    // Pre-measure exact content width for both outgoing and incoming bubbles so every bubble,
    // photo, and call card hugs its content tightly without horizontal overflow or inner/outer frame mismatch.
    let content_width = if has_sticker {
        116.0_f32
    } else if is_call_card {
        max_text_width.min(256.0)
    } else if is_photo_card {
        let thumb_u = photo_primary_url(msg).unwrap_or("");
        let tkey = format!("thumb_{}", crate::crypto::md5_hex(thumb_u));
        let photo_w = if let Some(tex) = sticker_textures.get(&tkey) {
            let [tw, th] = tex.size();
            let tw_f = (tw.max(1)) as f32;
            let th_f = (th.max(1)) as f32;
            let max_w = max_text_width.clamp(220.0, 360.0);
            let max_h = 340.0_f32;
            let scale = (max_w / tw_f).min(max_h / th_f).min(1.0);
            (tw_f * scale).clamp(160.0, max_w).ceil()
        } else {
            240.0_f32
        };
        if let Some(cap_str) = photo_caption {
            let cap_w = ui.fonts(|f| {
                f.layout(
                    cap_str.to_string(),
                    FontId::proportional(14.2),
                    Color32::WHITE,
                    max_text_width,
                )
                .size()
                .x
            });
            photo_w.max(cap_w + 4.0).min(max_text_width).ceil()
        } else {
            photo_w
        }
    } else if is_file_card || has_link_card {
        let has_loaded_thumb = msg
            .link_thumb
            .as_deref()
            .filter(|u| !u.is_empty())
            .map(|u| sticker_textures.contains_key(&format!("thumb_{}", crate::crypto::md5_hex(u))))
            .unwrap_or(false);
        if has_loaded_thumb {
            max_text_width.min(260.0)
        } else {
            let (cap, url, dom) = extract_message_link_view(msg).unwrap_or_default();
            let title = msg
                .link_title
                .as_deref()
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| {
                    if !url.is_empty() {
                        &url
                    } else {
                        msg.text.trim()
                    }
                });
            let desc = msg.link_desc.as_deref().unwrap_or("");
            let measure = |s: &str, sz: f32| -> f32 {
                if s.is_empty() {
                    0.0
                } else {
                    ui.fonts(|f| {
                        f.layout(
                            truncate_chars(s, 110).into_owned(),
                            FontId::proportional(sz),
                            Color32::WHITE,
                            max_text_width.min(340.0),
                        )
                        .size()
                        .x
                    })
                }
            };
            let w = measure(title, 13.8)
                .max(measure(&cap, 13.8))
                .max(measure(desc, 12.3))
                .max(measure(&dom, 12.6))
                + 8.0;
            w.clamp(135.0, max_text_width.min(340.0)).ceil()
        }
    } else {
        let display_str = if msg.text.trim() == "[Tin nhắn đã thu hồi]" {
            "  Tin nhắn đã được thu hồi".to_string()
        } else {
            strip_markdown_syntax_for_width(&msg.text)
        };
        let text_w = ui.fonts(|f| {
            f.layout(
                display_str,
                FontId::proportional(14.2),
                Color32::WHITE,
                max_text_width,
            )
            .size()
            .x
        });
        let quote_w = match msg.reply_to_text.as_ref() {
            Some(qt) => {
                let q_body_w = ui.fonts(|f| {
                    f.layout(
                        truncate_chars(qt, 80).into_owned(),
                        FontId::proportional(12.0),
                        Color32::WHITE,
                        (max_text_width - 28.0).max(80.0),
                    )
                    .size()
                    .x
                });
                let who = msg
                    .reply_to_sender
                    .as_deref()
                    .filter(|sn| !sn.is_empty() && !sn.chars().all(|c| c.is_ascii_digit()))
                    .unwrap_or("Tin nhắn trích dẫn");
                let q_who_w = ui.fonts(|f| {
                    f.layout_no_wrap(who.to_string(), FontId::proportional(11.5), Color32::WHITE)
                        .size()
                        .x
                });
                q_body_w.max(q_who_w) + 30.0
            }
            None => 0.0_f32,
        };
        let sender_w = if show_group_sender {
            ui.fonts(|f| {
                f.layout_no_wrap(
                    truncate_chars(&msg.sender, 22).into_owned(),
                    FontId::proportional(12.0),
                    Color32::WHITE,
                )
                .size()
                .x
            }) + 8.0
        } else {
            0.0_f32
        };
        let marks_w = if !msg.reactions.is_empty() || msg.is_pinned || msg.is_starred {
            78.0_f32
        } else {
            0.0_f32
        };
        let ts_min_w = if show_timestamp && is_latest_outgoing && msg.is_outgoing {
            96.0_f32
        } else {
            44.0_f32
        };
        text_w
            .max(quote_w)
            .max(sender_w)
            .max(marks_w)
            .max(ts_min_w)
            .min(max_text_width)
            .ceil()
            + 4.0
    };

    let h_margin = if has_sticker || is_frameless_photo {
        2.0_f32
    } else {
        12.0_f32
    };
    let v_margin = if has_sticker || is_frameless_photo {
        2.0_f32
    } else {
        8.5_f32
    };
    let total_bubble_w = content_width + h_margin * 2.0;

    if msg.is_outgoing {
        let out_resp = ui.vertical(|ui| {
            let row_resp = ui.horizontal(|ui| {
                let left_pad = (ui.available_width() - total_bubble_w - 6.0).max(0.0);
                if left_pad > 0.0 {
                    ui.add_space(left_pad);
                }

                let fill = if has_sticker || is_frameless_photo {
                    Color32::TRANSPARENT
                } else if let Some(card) = &call_card_opt {
                    if card.is_missed_or_failed {
                        if pal.is_dark {
                            Color32::from_rgb(38, 23, 30)
                        } else {
                            Color32::from_rgb(254, 242, 242)
                        }
                    } else {
                        pal.bg_elevated
                    }
                } else {
                    pal.bubble_out_bg
                };
                let stroke = if has_sticker || is_frameless_photo {
                    Stroke::NONE
                } else if let Some(card) = &call_card_opt {
                    let border_col = if card.is_missed_or_failed {
                        Color32::from_rgb(239, 68, 68).linear_multiply(0.55)
                    } else {
                        Color32::from_rgb(34, 197, 94).linear_multiply(0.55)
                    };
                    Stroke::new(1.2_f32, border_col)
                } else if !pal.is_dark {
                    Stroke::new(1.0_f32, Color32::from_rgb(186, 214, 255))
                } else {
                    Stroke::new(1.0_f32, Color32::from_rgb(53, 76, 104))
                };
                let rounding = Rounding::same(8.0);

                let mut card_action = None;
                let mut quote_action = None;
                let fr = egui::Frame::none()
                    .fill(fill)
                    .stroke(stroke)
                    .rounding(rounding)
                    .inner_margin(Margin::symmetric(h_margin, v_margin))
                    .show(ui, |ui| {
                        ui.set_min_width(content_width);
                        ui.set_max_width(content_width);
                        ui.with_layout(Layout::top_down(Align::Min), |ui| {
                            quote_action = render_quote_block(ui, msg, pal, true, content_width);
                            if let Some(card) = &call_card_opt {
                                card_action = render_call_card_widget(ui, card, pal, content_width);
                            } else if !render_sticker_widget(
                                ui,
                                msg,
                                sticker_textures,
                                loading_stickers,
                            ) {
                                card_action = render_message_body(
                                    ui,
                                    msg,
                                    thread_name,
                                    pal,
                                    true,
                                    content_width,
                                    sticker_textures,
                                    loading_stickers,
                                );
                            }
                            render_reactions(ui, msg, pal, emoji_textures);
                            if show_timestamp && !has_sticker && !is_frameless_photo {
                                ui.add_space(3.5);
                                let ts_txt = if is_latest_outgoing {
                                    format!("{} · Đã nhận", msg.time)
                                } else {
                                    msg.time.clone()
                                };
                                ui.label(
                                    RichText::new(ts_txt).size(11.0).color(pal.bubble_out_meta),
                                );
                            }
                        });
                    });
                if fr.response.double_clicked() {
                    return Some(BubbleAction::Reply);
                }
                quote_action.or(card_action).or_else(|| {
                    bubble_context_menu(
                        ui,
                        &fr.response,
                        msg,
                        idx,
                        pal,
                        emoji_textures,
                        has_interactive_card,
                    )
                })
            });

            row_resp.inner
        });
        out_resp.inner
    } else {
        ui.horizontal_top(|ui| {
            let display_sender = if sender_is_name {
                &msg.sender
            } else {
                thread_name
            };
            let sender_accent = avatar_color_for(display_sender, false);

            // Sender Avatar (only on first message of a consecutive group)
            if !prev_same_sender {
                let (av_rect, _) = ui.allocate_exact_size(Vec2::new(30.0, 30.0), Sense::hover());
                paint_user_avatar(
                    ui.painter(),
                    av_rect.center(),
                    15.0,
                    display_sender,
                    false,
                    sender_avatar,
                    sticker_textures,
                    loading_stickers,
                );
            } else {
                ui.add_space(30.0);
            }

            let fill = if has_sticker || is_frameless_photo {
                Color32::TRANSPARENT
            } else if let Some(card) = &call_card_opt {
                if card.is_missed_or_failed {
                    if pal.is_dark {
                        Color32::from_rgb(38, 23, 30)
                    } else {
                        Color32::from_rgb(254, 242, 242)
                    }
                } else {
                    pal.bg_elevated
                }
            } else {
                pal.bubble_in_bg
            };
            let stroke = if has_sticker || is_frameless_photo {
                Stroke::NONE
            } else if let Some(card) = &call_card_opt {
                let border_col = if card.is_missed_or_failed {
                    Color32::from_rgb(239, 68, 68).linear_multiply(0.55)
                } else {
                    Color32::from_rgb(34, 197, 94).linear_multiply(0.55)
                };
                Stroke::new(1.2_f32, border_col)
            } else {
                Stroke::new(1.0_f32, pal.bubble_in_border)
            };
            let rounding = Rounding::same(8.0);

            ui.vertical(|ui| {
                ui.set_max_width(total_bubble_w);
                let mut card_action = None;
                let mut quote_action = None;
                let fr = egui::Frame::none()
                    .fill(fill)
                    .stroke(stroke)
                    .rounding(rounding)
                    .inner_margin(Margin::symmetric(h_margin, v_margin))
                    .show(ui, |ui| {
                        ui.set_min_width(content_width);
                        ui.set_max_width(content_width);
                        ui.with_layout(Layout::top_down(Align::Min), |ui| {
                            if show_group_sender {
                                ui.label(
                                    RichText::new(truncate_chars(&msg.sender, 22))
                                        .color(sender_accent)
                                        .size(12.0)
                                        .strong(),
                                );
                                ui.add_space(2.0);
                            }
                            quote_action = render_quote_block(ui, msg, pal, false, content_width);
                            if let Some(card) = &call_card_opt {
                                card_action = render_call_card_widget(ui, card, pal, content_width);
                            } else if !render_sticker_widget(
                                ui,
                                msg,
                                sticker_textures,
                                loading_stickers,
                            ) {
                                card_action = render_message_body(
                                    ui,
                                    msg,
                                    thread_name,
                                    pal,
                                    false,
                                    content_width,
                                    sticker_textures,
                                    loading_stickers,
                                );
                            }
                            render_reactions(ui, msg, pal, emoji_textures);
                            if show_timestamp && !has_sticker && !is_frameless_photo {
                                ui.add_space(3.5);
                                ui.label(
                                    RichText::new(&msg.time)
                                        .color(pal.bubble_in_meta)
                                        .size(11.0),
                                );
                            }
                        });
                    });
                if fr.response.double_clicked() {
                    return Some(BubbleAction::Reply);
                }
                quote_action.or(card_action).or_else(|| {
                    bubble_context_menu(
                        ui,
                        &fr.response,
                        msg,
                        idx,
                        pal,
                        emoji_textures,
                        has_interactive_card,
                    )
                })
            })
            .inner
        })
        .inner
    }
}

#[derive(Clone, PartialEq)]
pub enum BubbleAction {
    Reply,
    JumpToQuoted(String),
    Forward,
    React(&'static str),
    TogglePin,
    ToggleStar,
    DeleteForMe,
    RecallForEveryone,
    OpenImagePopup {
        tex_key: String,
        full_url: String,
        title: String,
        sender: String,
        time: String,
    },
    DownloadFile {
        url: String,
        file_name: String,
    },
}

/// Right-click menu on a chat bubble: reactions, reply, forward, copy, delete-for-me.
pub fn bubble_context_menu(
    ui: &mut egui::Ui,
    bubble: &egui::Response,
    msg: &ChatMessage,
    idx: usize,
    _pal: &ThemePalette,
    emoji_textures: &HashMap<String, TextureHandle>,
    _has_interactive_card: bool,
) -> Option<BubbleAction> {
    let secondary_or_popup = ui.input(|i| {
        i.pointer.button_down(egui::PointerButton::Secondary)
            || i.pointer.button_clicked(egui::PointerButton::Secondary)
            || i.pointer.button_released(egui::PointerButton::Secondary)
    }) || ui.memory(|m| m.any_popup_open());
    if !secondary_or_popup {
        return None;
    }
    let resp = ui.interact(
        bubble.rect,
        Id::new(("bubble_ctx", idx, msg.ts, &msg.id)),
        Sense::click(),
    );
    let mut action = None;

    resp.context_menu(|ui| {
        ui.set_min_width(210.0);
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing = Vec2::new(4.0, 0.0);
            for &(code, key, tooltip) in &REACTION_LIST {
                if let Some(tex) = emoji_textures.get(key) {
                    let img = egui::Image::new(tex).fit_to_exact_size(Vec2::splat(24.0));
                    let btn = egui::ImageButton::new(img).rounding(Rounding::same(6.0));
                    if ui.add(btn).on_hover_text(tooltip).clicked() {
                        action = Some(BubbleAction::React(code));
                        ui.close_menu();
                    }
                } else if ui.button(RichText::new(code).size(18.0)).clicked() {
                    action = Some(BubbleAction::React(code));
                    ui.close_menu();
                }
            }
        });
        ui.separator();
        if is_zalo_photo_message(msg) {
            if let Some(thumb_url) = photo_primary_url(msg) {
                if ui
                    .button(format!("{} Xem ảnh phóng to", lucide::SEARCH))
                    .clicked()
                {
                    action = Some(BubbleAction::OpenImagePopup {
                        tex_key: format!("thumb_{}", crate::crypto::md5_hex(thumb_url)),
                        full_url: msg
                            .link_url
                            .clone()
                            .filter(|u| !u.is_empty())
                            .unwrap_or_else(|| thumb_url.to_string()),
                        title: msg
                            .link_title
                            .clone()
                            .filter(|t| {
                                !t.is_empty()
                                    && !t.starts_with("http://")
                                    && !t.starts_with("https://")
                            })
                            .unwrap_or_else(|| "Hình ảnh".to_string()),
                        sender: if msg.is_outgoing {
                            "Bạn".to_string()
                        } else {
                            msg.sender.clone()
                        },
                        time: msg.time.clone(),
                    });
                    ui.close_menu();
                }
            }
        }
        if is_zalo_file_or_video_message(msg) {
            if let Some(url) = msg.link_url.as_deref().filter(|u| !u.is_empty()) {
                let resolved_name =
                    crate::protocol::messaging::attachments::resolve_attachment_file_name(
                        msg.link_title.as_deref(),
                        msg.text.trim(),
                        url,
                    );
                if ui
                    .button(format!("📥 Lưu tập tin \"{resolved_name}\""))
                    .clicked()
                {
                    action = Some(BubbleAction::DownloadFile {
                        url: url.to_string(),
                        file_name: resolved_name,
                    });
                    ui.close_menu();
                }
            }
        }
        if ui.button("Trả lời").clicked() {
            action = Some(BubbleAction::Reply);
            ui.close_menu();
        }
        if ui.button("Chuyển tiếp").clicked() {
            action = Some(BubbleAction::Forward);
            ui.close_menu();
        }
        if ui.button("Copy tin nhắn").clicked() {
            ui.ctx().copy_text(msg.text.clone());
            ui.close_menu();
        }
        if ui
            .button(if msg.is_pinned {
                "Bỏ ghim tin nhắn"
            } else {
                "Ghim tin nhắn"
            })
            .clicked()
        {
            action = Some(BubbleAction::TogglePin);
            ui.close_menu();
        }
        if ui
            .button(if msg.is_starred {
                "Bỏ đánh dấu tin nhắn"
            } else {
                "Đánh dấu tin nhắn"
            })
            .clicked()
        {
            action = Some(BubbleAction::ToggleStar);
            ui.close_menu();
        }
        ui.separator();
        if msg.is_outgoing
            && msg.text.trim() != "[Tin nhắn đã thu hồi]"
            && ui
                .button(
                    RichText::new("Thu hồi với mọi người").color(Color32::from_rgb(249, 115, 22)),
                )
                .clicked()
        {
            action = Some(BubbleAction::RecallForEveryone);
            ui.close_menu();
        }
        if ui
            .button(RichText::new("Xóa chỉ ở phía tôi").color(Color32::from_rgb(220, 60, 60)))
            .clicked()
        {
            action = Some(BubbleAction::DeleteForMe);
            ui.close_menu();
        }
    });
    action
}

/// Quoted-message block shown at the top of a reply bubble.
pub fn render_quote_block(
    ui: &mut egui::Ui,
    msg: &ChatMessage,
    pal: &ThemePalette,
    outgoing: bool,
    max_w: f32,
) -> Option<BubbleAction> {
    let qt = msg.reply_to_text.as_ref()?;
    let (text_col, meta_col) = if outgoing {
        (pal.bubble_out_text, pal.bubble_out_meta)
    } else {
        (pal.bubble_in_text, pal.bubble_in_meta)
    };
    let bar_col = if outgoing && pal.is_dark {
        Color32::from_rgb(147, 197, 253)
    } else {
        pal.primary
    };
    let bg_col = if outgoing && pal.is_dark {
        Color32::from_rgba_unmultiplied(0, 0, 0, 42)
    } else if outgoing {
        Color32::from_rgba_unmultiplied(255, 255, 255, 150)
    } else {
        text_col.linear_multiply(0.06)
    };
    let inner_col_w = (max_w - 28.0).max(72.0);
    let frame = egui::Frame::none()
        .fill(bg_col)
        .rounding(Rounding::same(7.0))
        .inner_margin(Margin::symmetric(8.0, 5.0))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                let (bar_rect, _) = ui.allocate_exact_size(Vec2::new(3.0, 28.0), Sense::hover());
                ui.painter()
                    .rect_filled(bar_rect, Rounding::same(1.5), bar_col);
                ui.vertical(|ui| {
                    ui.set_width(inner_col_w);
                    let who = msg
                        .reply_to_sender
                        .as_deref()
                        .filter(|sn| !sn.is_empty() && !sn.chars().all(|c| c.is_ascii_digit()))
                        .unwrap_or("Tin nhắn trích dẫn");
                    ui.label(RichText::new(who).color(bar_col).size(11.5).strong());
                    ui.add(
                        egui::Label::new(
                            RichText::new(truncate_chars(qt, 84))
                                .color(meta_col)
                                .size(12.0),
                        )
                        .wrap(),
                    );
                });
            });
        });
    let reference = crate::message_navigation::message_reply_reference(msg);
    let response = ui.interact(
        frame.response.rect,
        frame.response.id.with("quote_jump"),
        Sense::click(),
    );
    let clicked = response
        .on_hover_cursor(CursorIcon::PointingHand)
        .on_hover_text("Đi tới tin nhắn gốc đã tải")
        .clicked();
    ui.add_space(5.0);
    clicked.then(|| BubbleAction::JumpToQuoted(reference.unwrap_or_default()))
}

pub fn render_reactions(
    ui: &mut egui::Ui,
    msg: &ChatMessage,
    pal: &ThemePalette,
    emoji_textures: &HashMap<String, TextureHandle>,
) {
    let has_pin = msg.is_pinned;
    let has_star = msg.is_starred;
    let has_react = !msg.reactions.is_empty();

    if !has_pin && !has_star && !has_react {
        return;
    }

    ui.add_space(3.0);
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = Vec2::new(4.0, 2.0);

        if has_pin {
            egui::Frame::none()
                .fill(pal.primary.linear_multiply(0.18))
                .stroke(Stroke::new(1.0_f32, pal.primary))
                .rounding(Rounding::same(8.0))
                .inner_margin(Margin::symmetric(5.0, 2.0))
                .show(ui, |ui| {
                    ui.label(
                        RichText::new(format!("{} Đã ghim", lucide::PIN))
                            .size(11.0)
                            .color(pal.primary)
                            .strong(),
                    );
                });
        }

        if has_star {
            egui::Frame::none()
                .fill(Color32::from_rgb(234, 179, 8).linear_multiply(0.2))
                .stroke(Stroke::new(1.0_f32, Color32::from_rgb(234, 179, 8)))
                .rounding(Rounding::same(8.0))
                .inner_margin(Margin::symmetric(5.0, 2.0))
                .show(ui, |ui| {
                    ui.label(
                        RichText::new(format!("{} Đã lưu", lucide::STAR))
                            .size(11.0)
                            .color(Color32::from_rgb(234, 179, 8))
                            .strong(),
                    );
                });
        }

        if has_react {
            egui::Frame::none()
                .fill(pal.bg_elevated)
                .stroke(Stroke::new(1.0_f32, pal.border))
                .rounding(Rounding::same(12.0))
                .inner_margin(Margin::symmetric(6.0, 2.0))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing = Vec2::new(3.0, 0.0);
                        for r in &msg.reactions {
                            let key = emoji_to_tex_key(r);
                            if let Some(tex) = emoji_textures.get(key) {
                                ui.add(egui::Image::new(tex).fit_to_exact_size(Vec2::splat(18.0)));
                            } else {
                                ui.label(RichText::new(r).size(13.0));
                            }
                        }
                    });
                });
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quote_navigation_bubble_propagates_action_for_both_directions() {
        for is_outgoing in [false, true] {
            let ctx = egui::Context::default();
            let msg = ChatMessage {
                id: "reply-1".into(),
                text: "Reply body".into(),
                sender: "Sender".into(),
                reply_to_id: Some("original-1".into()),
                reply_to_text: Some("Original text".into()),
                is_outgoing,
                ..Default::default()
            };
            let mut position = egui::pos2(30.0, 30.0);
            let mut action = None;
            for frame in 0..4 {
                let mut events = vec![egui::Event::PointerMoved(position)];
                if frame >= 2 {
                    events.push(egui::Event::PointerButton {
                        pos: position,
                        button: egui::PointerButton::Primary,
                        pressed: frame == 2,
                        modifiers: egui::Modifiers::NONE,
                    });
                }
                let output = ctx.run(
                    egui::RawInput {
                        screen_rect: Some(egui::Rect::from_min_size(
                            egui::Pos2::ZERO,
                            Vec2::new(600.0, 400.0),
                        )),
                        events,
                        ..Default::default()
                    },
                    |ctx| {
                        egui::CentralPanel::default().show(ctx, |ui| {
                            action = render_chat_bubble(
                                ui,
                                &msg,
                                false,
                                false,
                                false,
                                false,
                                "Thread",
                                None,
                                &ThemePalette::light(),
                                &HashMap::new(),
                                &mut HashSet::new(),
                                &HashMap::new(),
                                0,
                            );
                        });
                    },
                );
                if frame == 0 {
                    position = output
                        .shapes
                        .iter()
                        .find_map(|shape| match &shape.shape {
                            egui::Shape::Text(text) if text.galley.job.text == "Original text" => {
                                Some(text.pos + Vec2::new(4.0, 4.0))
                            }
                            _ => None,
                        })
                        .expect("Quote must be rendered in the bubble");
                }
            }
            assert!(
                matches!(action, Some(BubbleAction::JumpToQuoted(reference)) if reference == "original-1")
            );
        }
    }

    #[test]
    fn quote_navigation_emits_exact_reference_when_clicked() {
        let ctx = egui::Context::default();
        let msg = ChatMessage {
            reply_to_id: Some("00123".into()),
            reply_to_text: Some("Original text".into()),
            ..Default::default()
        };
        let mut action = None;
        for pressed in [None, Some(true), Some(false)] {
            let mut events = vec![egui::Event::PointerMoved(egui::pos2(30.0, 30.0))];
            if let Some(pressed) = pressed {
                events.push(egui::Event::PointerButton {
                    pos: egui::pos2(30.0, 30.0),
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: egui::Modifiers::NONE,
                });
            }
            let _ = ctx.run(
                egui::RawInput {
                    events,
                    ..Default::default()
                },
                |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        action = render_quote_block(ui, &msg, &ThemePalette::light(), false, 300.0);
                    });
                },
            );
        }
        assert!(
            matches!(action, Some(BubbleAction::JumpToQuoted(reference)) if reference == "00123")
        );
    }

    #[test]
    fn quote_navigation_advertises_click_when_reference_exists() {
        let ctx = egui::Context::default();
        let msg = ChatMessage {
            reply_to_id: Some("original-1".into()),
            reply_to_text: Some("Original text".into()),
            ..Default::default()
        };
        for _ in 0..2 {
            let output = ctx.run(
                egui::RawInput {
                    events: vec![egui::Event::PointerMoved(egui::pos2(30.0, 30.0))],
                    ..Default::default()
                },
                |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        render_quote_block(ui, &msg, &ThemePalette::light(), false, 300.0);
                    });
                },
            );
            if output.platform_output.cursor_icon == egui::CursorIcon::PointingHand {
                return;
            }
        }
        panic!("Quote with an exact reference must advertise a clickable target");
    }
}
