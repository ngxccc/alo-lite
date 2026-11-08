use super::*;

impl ZaloGuiApp {
    pub(super) fn handle_chat_event(&mut self, ctx: &egui::Context, event: GuiEvent) {
        match event {
            GuiEvent::MessageSent {
                client_message_id,
                thread_id,
                text,
                msg_id,
            } => {
                if let Some(t) = self.threads.iter_mut().find(|t| t.id == thread_id) {
                    t.preview = text.clone();
                    let now = crate::ui::clock();
                    t.time = now.clone();
                    let final_id = if msg_id.is_empty() {
                        format!("out_{client_message_id}")
                    } else {
                        msg_id
                    };
                    merge_sent_message(
                        t,
                        ChatMessage {
                            id: final_id,
                            client_message_id,
                            sender: "Tôi".into(),
                            text,
                            time: now,
                            is_outgoing: true,
                            ts: (crate::clock::unix_now() * 1000),
                            ..Default::default()
                        },
                    );
                    self.persist_threads_async();
                }
                self.status_msg = format!("Đã gửi tin nhắn lúc {}", crate::ui::clock());
            }
            GuiEvent::StickerSent {
                client_message_id,
                thread_id,
                msg_id,
                sticker_id,
                cate_id,
                sticker_type,
            } => {
                if let Some(t) = self.threads.iter_mut().find(|t| t.id == thread_id) {
                    t.preview = "[Nhãn dán]".into();
                    let now = crate::ui::clock();
                    t.time = now.clone();
                    let final_id = if msg_id.is_empty() {
                        format!("out_{client_message_id}")
                    } else {
                        msg_id
                    };
                    merge_sent_message(
                        t,
                        ChatMessage {
                            id: final_id,
                            client_message_id,
                            sender: "Tôi".into(),
                            text: "[Nhãn dán]".into(),
                            time: now,
                            is_outgoing: true,
                            ts: (crate::clock::unix_now() * 1000),
                            sticker_id: Some(sticker_id),
                            sticker_cate_id: Some(cate_id),
                            sticker_type: Some(sticker_type),
                            ..Default::default()
                        },
                    );
                    self.persist_threads_async();
                }
                self.status_msg = format!("Đã gửi nhãn dán lúc {}", crate::ui::clock());
            }
            GuiEvent::AttachmentSent { thread_id, msg } => {
                if let Some(t) = self.threads.iter_mut().find(|t| t.id == thread_id) {
                    t.preview = msg.text.clone();
                    t.time = msg.time.clone();
                    merge_sent_message(t, msg);
                    self.persist_threads_async();
                }
                self.status_msg = format!("Đã gửi đính kèm lúc {}", crate::ui::clock());
            }
            GuiEvent::QuoteSent {
                client_message_id,
                thread_id,
                text,
                msg_id,
                quoted,
            } => {
                if let Some(t) = self.threads.iter_mut().find(|t| t.id == thread_id) {
                    t.preview = text.clone();
                    let now = crate::ui::clock();
                    t.time = now.clone();
                    let final_id = if msg_id.is_empty() {
                        format!("out_{client_message_id}")
                    } else {
                        msg_id
                    };
                    merge_sent_message(
                        t,
                        ChatMessage {
                            id: final_id,
                            client_message_id,
                            sender: "Tôi".into(),
                            text,
                            time: now,
                            is_outgoing: true,
                            ts: (crate::clock::unix_now() * 1000),
                            reply_to_id: Some(quoted.id.clone()),
                            reply_to_sender: Some(quoted.sender.clone()),
                            reply_to_text: Some(quoted.text.clone()),
                            ..Default::default()
                        },
                    );
                    self.persist_threads_async();
                }
                self.status_msg = format!("Đã gửi trả lời lúc {}", crate::ui::clock());
            }
            GuiEvent::Reacted {
                thread_id,
                msg_id,
                ts,
                reaction,
            } => {
                if let Some(t) = self.threads.iter_mut().find(|t| t.id == thread_id) {
                    if let Some(m) = t.messages.iter_mut().find(|m| {
                        (!m.id.is_empty() && m.id == msg_id) || (m.id.is_empty() && m.ts == ts)
                    }) {
                        m.reactions = vec![reaction];
                    }
                    self.persist_threads_async();
                }
                self.status_msg = "Đã thả cảm xúc".into();
            }
            GuiEvent::MessageDeleted {
                thread_id,
                msg_id,
                ts,
            } => {
                if let Some(t) = self.threads.iter_mut().find(|t| t.id == thread_id) {
                    t.messages.retain(|m| {
                        !((!msg_id.is_empty() && m.id == msg_id)
                            || (msg_id.is_empty() && m.ts == ts))
                    });
                    if let Some(last) = t.messages.last() {
                        t.preview = last.text.clone();
                    }
                    self.persist_threads_async();
                }
                self.status_msg = "Đã xóa tin nhắn ở phía bạn".into();
            }
            GuiEvent::MessageRecalled {
                thread_id,
                msg_id,
                ts,
            } => {
                if let Some(t) = self.threads.iter_mut().find(|t| t.id == thread_id) {
                    if let Some(m) = t.messages.iter_mut().find(|m| {
                        (!msg_id.is_empty() && m.id == msg_id) || (msg_id.is_empty() && m.ts == ts)
                    }) {
                        m.text = "[Tin nhắn đã thu hồi]".into();
                        m.sticker_id = None;
                        m.link_url = None;
                        m.link_thumb = None;
                    }
                    if let Some(last) = t.messages.last() {
                        t.preview = last.text.clone();
                    }
                    self.persist_threads_async();
                }
                self.status_msg = "Đã thu hồi tin nhắn với mọi người".into();
            }
            GuiEvent::SendFailed {
                thread_id: _,
                text,
                error,
            } => {
                if self.input_message.is_empty() && !text.is_empty() {
                    self.input_message = text;
                }
                self.status_msg = format!("Không gửi được: {error}");
            }
            GuiEvent::StickerImageReady { key, image } => {
                self.loading_stickers.remove(&key);
                let tex = ctx.load_texture(&key, image, TextureOptions::LINEAR);
                self.sticker_textures.insert(key, tex);
            }
            GuiEvent::StickerStoreLoaded(packs) => {
                self.sticker_store_loading = false;
                let count = packs.len();
                self.sticker_store_packs = packs;
                self.status_msg = format!("Đã tải Kho Nhãn Dán Zalo ({count} bộ nhãn)");
            }
            GuiEvent::StickerPackItemsLoaded {
                cate_id,
                name,
                items,
            } => {
                self.loading_sticker_packs.remove(&cate_id);
                let count = items.len();
                if !self
                    .pinned_sticker_packs
                    .iter()
                    .any(|(id, _)| *id == cate_id)
                {
                    self.pinned_sticker_packs.insert(0, (cate_id, name.clone()));
                }
                self.sticker_pack_items.insert(cate_id, items);
                self.status_msg = format!("Đã mở bộ nhãn \"{name}\" ({count} nhãn dán)");
            }
            GuiEvent::StickerSearchResults { keyword, items } => {
                self.sticker_search_loading = false;
                let count = items.len();
                self.sticker_search_results = items;
                self.status_msg = format!("Tìm thấy {count} nhãn dán cho từ khóa \"{keyword}\"");
            }
            GuiEvent::StickerSuggestKeywordsLoaded(kws) => {
                for k in kws {
                    self.sticker_suggest_keywords.insert(k);
                }
            }
            GuiEvent::ComposerStickerSuggestions { keyword, items } => {
                self.composer_sticker_loading.remove(&keyword);
                self.composer_sticker_cache.insert(keyword, items);
            }
            GuiEvent::LinkPreviewReady(meta) => {
                self.loading_link_previews.remove(&meta.url);
                if !meta.thumb.is_empty() {
                    let key = format!("thumb_{}", crate::crypto::md5_hex(&meta.thumb));
                    if !self.sticker_textures.contains_key(&key)
                        && self.loading_stickers.insert(key.clone())
                    {
                        if let Some(tx) = WORKER_TX.get() {
                            let _ = tx.send(WorkerCmd::FetchStickerImage {
                                key,
                                url: meta.thumb.clone(),
                            });
                        }
                    }
                }
                let mut updated_any = false;
                for t in &mut self.threads {
                    for m in &mut t.messages {
                        let matches_url = m.link_url.as_deref() == Some(meta.url.as_str())
                            || crate::protocol::messaging::links::extract_first_url(&m.text)
                                .as_deref()
                                == Some(meta.url.as_str());
                        if matches_url {
                            if m.link_url.is_none() {
                                m.link_url = Some(meta.url.clone());
                                updated_any = true;
                            }
                            if (m.link_title.is_none()
                                || m.link_title.as_deref() == Some(meta.url.as_str()))
                                && !meta.title.is_empty()
                            {
                                m.link_title = Some(meta.title.clone());
                                updated_any = true;
                            }
                            if m.link_desc.is_none() && !meta.desc.is_empty() {
                                m.link_desc = Some(meta.desc.clone());
                                updated_any = true;
                            }
                            let old_thumb_expired = m
                                .link_thumb
                                .as_deref()
                                .map(|th| {
                                    th.is_empty()
                                        || (th.contains("zpcloud.zdn.vn")
                                            && !self.sticker_textures.contains_key(&format!(
                                                "thumb_{}",
                                                crate::crypto::md5_hex(th)
                                            )))
                                })
                                .unwrap_or(true);
                            if old_thumb_expired && !meta.thumb.is_empty() {
                                m.link_thumb = Some(meta.thumb.clone());
                                updated_any = true;
                            }
                            if m.link_src.is_none() && !meta.src.is_empty() {
                                m.link_src = Some(meta.src.clone());
                                updated_any = true;
                            }
                        }
                    }
                }
                self.link_metadata_cache.insert(meta.url.clone(), meta);
                if updated_any {
                    self.persist_threads_async();
                }
            }
            _ => unreachable!(),
        }
    }
}
