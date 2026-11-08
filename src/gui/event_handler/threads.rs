use super::*;

impl ZaloGuiApp {
    pub(super) fn handle_threads_event(&mut self, ctx: &egui::Context, event: GuiEvent) {
        match event {
            GuiEvent::ThreadsUpdated(updates) => {
                ensure_cjk_font(ctx, thread_texts(&updates));
                if self.is_logged_in && !updates.is_empty() {
                    let prev_selected = self.threads.get(self.selected_thread_index).cloned();
                    let live_ids: HashSet<String> = self
                        .threads
                        .iter()
                        .chain(updates.iter())
                        .filter(|u| !u.id.is_empty() && u.id != "0")
                        .map(|u| u.id.clone())
                        .collect();
                    let update_ids: HashSet<String> = updates
                        .iter()
                        .filter(|u| !u.id.is_empty() && u.id != "0")
                        .map(|u| u.id.clone())
                        .collect();

                    let active_tid = prev_selected
                        .as_ref()
                        .map(|t| t.id.clone())
                        .unwrap_or_default();
                    let is_focused = ctx.input(|i| i.focused);
                    let mut new_incoming_alert = false;

                    let now_ts = crate::clock::unix_now();
                    for mut nt in updates {
                        let settings_muted_until = self
                            .settings
                            .muted_threads
                            .get(&nt.id)
                            .copied()
                            .filter(|&u| u == -1 || u > now_ts);
                        if let Some(until) = settings_muted_until {
                            nt.presentation.muted = true;
                            nt.unread.muted = true;
                            nt.unread.muted_until = (until > 0).then_some(until);
                        } else if nt.unread.is_muted() {
                            nt.presentation.muted = true;
                        }
                        if let Some(existing) = self.threads.iter_mut().find(|t| t.id == nt.id) {
                            let is_thread_muted = settings_muted_until.is_some()
                                || existing.presentation.muted
                                || existing.unread.is_muted()
                                || nt.presentation.muted
                                || nt.unread.is_muted();
                            let old_len = existing.messages.len();
                            crate::protocol::messaging::threads::merge_thread_into(existing, nt, true);
                            if is_thread_muted {
                                existing.presentation.muted = true;
                                existing.unread.muted = true;
                                if let Some(until) = settings_muted_until {
                                    existing.unread.muted_until = (until > 0).then_some(until);
                                }
                            }
                            let added = existing.messages.len().saturating_sub(old_len);
                            if added > 0 {
                                let inc_cnt = existing
                                    .messages
                                    .iter()
                                    .rev()
                                    .take(added)
                                    .filter(|m| !m.is_outgoing)
                                    .count();
                                if inc_cnt > 0 && (existing.id != active_tid || !is_focused) {
                                    *self.unread_counts.entry(existing.id.clone()).or_insert(0) +=
                                        inc_cnt;
                                    if !is_thread_muted {
                                        new_incoming_alert = true;
                                    }
                                }
                                if existing.id == self.chat_render_window.0 {
                                    self.chat_render_window.1 += added;
                                    self.show_toast(format!("Đã tải thêm {added} tin nhắn cũ"));
                                }
                            }
                        } else if let Some(existing) = self.threads.iter_mut().find(|t| {
                            !update_ids.contains(&t.id)
                                && crate::protocol::messaging::threads::threads_represent_same_contact(t, &nt)
                        }) {
                            crate::protocol::messaging::threads::merge_thread_into(existing, nt, true);
                        } else if !nt.id.is_empty() && nt.id != "0" {
                            self.threads.push(nt);
                        }
                    }

                    if new_incoming_alert && self.settings.enable_notifications {
                        let total_unread: usize = self.unread_counts.values().sum();
                        let title = if total_unread > 0 {
                            format!("({total_unread}) AloLite")
                        } else {
                            "AloLite".to_string()
                        };
                        ctx.send_viewport_cmd(egui::ViewportCommand::Title(title));
                        if !is_focused {
                            ctx.send_viewport_cmd(egui::ViewportCommand::RequestUserAttention(
                                egui::UserAttentionType::Informational,
                            ));
                        }
                    }
                    crate::protocol::messaging::threads::deduplicate_threads(
                        &mut self.threads,
                        Some(&live_ids),
                    );

                    // Restore selected thread by exact `id` first so contacts sharing the
                    // same Zalo display name never jump to each other's chat window (#63).
                    if let Some(prev) = prev_selected {
                        if let Some(idx) = self.threads.iter().position(|t| t.id == prev.id) {
                            self.selected_thread_index = idx;
                        } else if let Some(idx) = self.threads.iter().position(|t| {
                            crate::protocol::messaging::threads::threads_represent_same_contact(
                                t, &prev,
                            )
                        }) {
                            self.selected_thread_index = idx;
                        }
                    }
                    if self.selected_thread_index >= self.threads.len() {
                        self.selected_thread_index = self.threads.len().saturating_sub(1);
                    }
                    self.persist_threads_async();
                }
                self.status_msg = format!("Trực tuyến · Cập nhật {}", crate::ui::clock());
            }
            GuiEvent::RefreshFinished {
                threads,
                messages,
                errors,
                manual,
            } => {
                let msg = if errors.is_empty() {
                    format!("Đã làm mới {threads} hội thoại ({messages} tin nhắn)")
                } else {
                    format!(
                        "Làm mới {threads} hội thoại · {} lỗi (vd. {})",
                        errors.len(),
                        errors[0]
                    )
                };
                self.status_msg = msg.clone();
                if manual {
                    self.show_toast(msg);
                }
            }
            GuiEvent::SyncFinished {
                added,
                errors,
                cancelled,
            } => {
                let head = if cancelled {
                    "Đã dừng đồng bộ"
                } else {
                    "Đồng bộ xong"
                };
                let mut msg = format!("{head}: lưu thêm {added} tin cũ");
                if let Some(first) = errors.first() {
                    msg.push_str(&format!(" · {} lỗi (vd. {first})", errors.len()));
                }
                self.status_msg = msg.clone();
                self.show_toast(msg);
            }
            GuiEvent::MuteSnapshotSynced(mute_map) => {
                self.settings.muted_threads = mute_map.clone();
                self.settings.save();
                let now_ts = crate::clock::unix_now();
                for t in &mut self.threads {
                    if let Some(&until) = mute_map.get(&t.id) {
                        let active = until == -1 || until > now_ts;
                        t.presentation.muted = active;
                        t.unread.muted = active;
                        t.unread.muted_until = (until > 0).then_some(until);
                        t.unread.settings_fresh = true;
                        t.unread.fresh = true;
                    } else {
                        t.presentation.muted = false;
                        t.unread.muted = false;
                        t.unread.muted_until = None;
                        t.unread.settings_fresh = true;
                        t.unread.fresh = true;
                    }
                }
                self.persist_threads_async();
            }
            GuiEvent::MuteEventsReceived(events) => {
                let mut changed = false;
                for ev in events {
                    if ev.muted {
                        let until_val = ev.muted_until.unwrap_or(-1);
                        self.settings
                            .muted_threads
                            .insert(ev.thread_id.clone(), until_val);
                    } else {
                        self.settings.muted_threads.remove(&ev.thread_id);
                    }
                    if let Some(t) = self.threads.iter_mut().find(|t| t.id == ev.thread_id) {
                        t.presentation.muted = ev.muted;
                        t.unread.muted = ev.muted;
                        t.unread.muted_until = ev.muted_until;
                        t.unread.settings_fresh = true;
                        t.unread.fresh = true;
                    }
                    changed = true;
                }
                if changed {
                    self.settings.save();
                    self.persist_threads_async();
                }
            }
            GuiEvent::PhoneSyncProgress { progress, status } => {
                self.phone_sync_progress = progress;
                self.phone_sync_status = status.clone();
                self.status_msg = status;
            }
            GuiEvent::PhoneSyncFinished {
                threads,
                added,
                active_threads,
                cancelled,
                error,
            } => {
                self.phone_sync_progress = 0.0;
                if !threads.is_empty() {
                    for nt in threads {
                        if let Some(existing) = self.threads.iter_mut().find(|t| t.id == nt.id) {
                            crate::protocol::messaging::threads::merge_thread_into(
                                existing, nt, true,
                            );
                        } else if !nt.id.is_empty() && nt.id != "0" {
                            self.threads.push(nt);
                        }
                    }
                    self.persist_threads_async();
                }
                let msg = phone_sync_completion_message(
                    added,
                    active_threads,
                    cancelled,
                    error.as_deref(),
                );
                self.phone_sync_status = msg.clone();
                self.status_msg = msg.clone();
                self.show_toast(msg);
            }
            GuiEvent::HistoryReachedStart(tid) => {
                self.history_reached_start.insert(tid);
                self.show_toast("Đã hiển thị toàn bộ lịch sử tin nhắn của cuộc trò chuyện này");
            }
            _ => unreachable!(),
        }
    }
}
