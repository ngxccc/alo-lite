use super::*;

impl ZaloGuiApp {
    pub(super) fn handle_services_event(&mut self, ctx: &egui::Context, event: GuiEvent) {
        match event {
            GuiEvent::NotificationAuthorization { request_id, result } => {
                self.notification_authorization.finish(request_id, result);
            }
            GuiEvent::FriendStatusChecked {
                user_id,
                status,
                online,
            } => {
                self.checking_friend_status.remove(&user_id);
                self.friend_status_map.insert(user_id.clone(), status);
                if let Some(onl) = online {
                    self.user_online_map.insert(user_id.clone(), onl);
                }
                if let Some(ref mut prof) = self.phone_lookup_result {
                    if prof.user_id == user_id {
                        prof.friendship = status;
                    }
                }
            }
            GuiEvent::PhoneLookupSuccess(res) => {
                self.phone_lookup_loading = false;
                self.phone_lookup_error = None;
                self.phone_modal_open = true;
                self.show_toast(format!("Đã tìm thấy tài khoản: {}", res.display_name));
                self.friend_status_map
                    .insert(res.user_id.clone(), res.friendship);
                self.phone_lookup_result = Some(res);
            }
            GuiEvent::PhoneLookupFailed(err) => {
                self.phone_lookup_loading = false;
                self.phone_lookup_result = None;
                self.phone_modal_open = true;
                self.phone_lookup_error = Some(err);
            }
            GuiEvent::CallStateChanged(state) => {
                if state == "free" {
                    // Keep modal open if there is a rejection reason so the user can read it
                    let has_error = self
                        .active_call
                        .as_ref()
                        .and_then(|c| c.error_reason.as_ref())
                        .is_some();
                    self.sticker_textures.remove("call_remote_video");
                    self.sticker_textures.remove("call_remote_video_1");
                    self.sticker_textures.remove("call_remote_video_2");
                    self.sticker_textures.remove("call_remote_video_3");
                    self.sticker_textures.remove("call_local_video");
                    if !has_error {
                        self.active_call = None;
                    }
                } else if let Some(ref mut call) = self.active_call {
                    if state == "incall" || state == "connected" {
                        call.is_incoming_ringing = false;
                    }
                    if call.error_reason.is_none() {
                        call.state_label = match state.as_str() {
                            "calling" | "ringing" => "Đang đổ chuông…".into(),
                            "incoming_ringing" => "Cuộc gọi đến đang đổ chuông…".into(),
                            "connecting" => "Đang kết nối luồng âm thanh/video…".into(),
                            "incall" | "connected" => "Đang trong cuộc gọi (Pure-Rust VoIP)".into(),
                            _ => format!("Đang kết nối ({state})…"),
                        };
                    }
                }
            }
            GuiEvent::IncomingCall {
                call_id: _,
                caller_id,
                caller_name,
                is_video,
                is_group,
            } => {
                let resolved_name = self
                    .threads
                    .iter()
                    .find(|t| t.id == caller_id)
                    .map(|t| t.name.clone())
                    .unwrap_or(caller_name);
                self.status_msg = format!(
                    "Cuộc gọi {} đến từ {resolved_name}…",
                    if is_video { "video" } else { "thoại" }
                );
                self.active_call = Some(ActiveCallOverlay {
                    partner_id: caller_id,
                    partner_name: resolved_name,
                    is_video,
                    is_group,
                    state_label: "Cuộc gọi đến đang đổ chuông…".into(),
                    started_at: std::time::Instant::now(),
                    mic_muted: false,
                    cam_enabled: is_video,
                    is_incoming_ringing: true,
                    error_code: None,
                    error_reason: None,
                    speaker_view: false,
                    pinned_tile_idx: 0,
                });
                ctx.send_viewport_cmd(egui::ViewportCommand::RequestUserAttention(
                    egui::UserAttentionType::Critical,
                ));
            }
            GuiEvent::CallRejected { code, reason } => {
                self.status_msg = format!("Từ chối cuộc gọi (mã {code}): {reason}");
                if let Some(ref mut call) = self.active_call {
                    call.state_label = "Không thể thực hiện cuộc gọi".into();
                    call.error_code = Some(code);
                    call.error_reason = Some(reason);
                }
            }
            GuiEvent::StatusUpdate(msg) => {
                self.status_msg = msg;
            }
            GuiEvent::UpdateCheckFinished { result, silent } => {
                self.update_checking = false;
                match result {
                    Ok(info) => {
                        if info.has_update {
                            let msg = format!(
                                "Có bản cập nhật mới v{} trên GitHub (hiện tại: v{})",
                                info.latest_version, info.current_version
                            );
                            self.update_status = msg.clone();
                            let is_skipped = silent
                                && !self.settings.skipped_update_version.is_empty()
                                && self.settings.skipped_update_version == info.latest_version;
                            let already_notified = self.last_notified_update_version.as_deref()
                                == Some(info.latest_version.as_str());
                            if !is_skipped {
                                self.show_toast(msg);
                                if !already_notified || !silent {
                                    self.last_notified_update_version =
                                        Some(info.latest_version.clone());
                                    self.update_modal_open = true;
                                    self.update_banner_dismissed = false;
                                    if self.settings.enable_notifications {
                                        crate::updater::notify_os_new_update(
                                            &info.latest_version,
                                            &info.current_version,
                                            &info.release_title,
                                        );
                                    }
                                }
                            }
                        } else {
                            self.update_status = format!(
                                "Bạn đang sử dụng phiên bản mới nhất (v{})",
                                info.current_version
                            );
                            if !silent {
                                self.show_toast(self.update_status.clone());
                            }
                        }
                        self.update_info = Some(info);
                    }
                    Err(e) => {
                        self.update_status = format!("Không kiểm tra được cập nhật: {e}");
                        if !silent {
                            self.show_toast(self.update_status.clone());
                        }
                    }
                }
            }
            GuiEvent::UpdateDownloadStatus {
                downloading,
                message,
            } => {
                self.update_downloading = downloading;
                if !downloading {
                    self.update_progress = None;
                }
                self.update_status = message.clone();
                self.show_toast(message);
            }
            GuiEvent::UpdateDownloadProgress { downloaded, total } => {
                self.update_downloading = true;
                self.update_progress = Some((downloaded, total));
            }
            GuiEvent::UpdateAppliedAndRestarting(ver) => {
                self.update_downloading = false;
                self.show_toast(format!("Đã cập nhật lên v{ver} — Đang khởi động lại…"));
                self.allow_quit = true;
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                std::process::exit(0);
            }
            GuiEvent::SocialProfileLoaded(res) => {
                self.feed_loading_profile = false;
                match res {
                    Ok(prof) => {
                        self.feed_profile = Some(prof);
                        self.feed_profile_error = None;
                    }
                    Err(e) => {
                        self.feed_profile_error = Some(e);
                    }
                }
            }
            GuiEvent::SocialAvatarsLoaded(res) => {
                self.feed_loading_avatars = false;
                match res {
                    Ok(photos) => {
                        self.feed_avatars = photos;
                        self.feed_avatars_error = None;
                    }
                    Err(e) => {
                        self.feed_avatars_error = Some(e);
                    }
                }
            }
            GuiEvent::SocialTimelineLoaded(res) => {
                self.feed_loading_timeline = false;
                match res {
                    Ok((profiles, items)) => {
                        self.feed_friend_profiles = profiles;
                        self.feed_items = items;
                    }
                    Err(e) => {
                        self.feed_profile_error = Some(e);
                    }
                }
            }
            GuiEvent::ProxyUpdated(res) => match res {
                Ok(Some(url)) => {
                    let redacted = crate::session::redact_proxy_url(&url);
                    self.settings.proxy_url = url;
                    self.settings.save();
                    let msg = format!("🌐 Đã áp dụng Proxy: {redacted}");
                    self.status_msg = msg.clone();
                    self.show_toast(msg);
                }
                Ok(None) => {
                    self.settings.proxy_url.clear();
                    self.settings.save();
                    let msg = "🌐 Đã tắt Proxy (kết nối trực tiếp)".to_string();
                    self.status_msg = msg.clone();
                    self.show_toast(msg);
                }
                Err(e) => {
                    let msg = format!("Lỗi cấu hình Proxy: {e}");
                    self.status_msg = msg.clone();
                    self.show_toast(msg);
                }
            },
            GuiEvent::FileDownloaded(res) => match res {
                Ok((fname, path)) => {
                    let msg = format!("Đã lưu tập tin \"{fname}\" tại {path}");
                    self.status_msg = msg.clone();
                    self.show_toast(msg);
                }
                Err(e) => {
                    let msg = format!("Lỗi lưu tập tin: {e}");
                    self.status_msg = msg.clone();
                    self.show_toast(msg);
                }
            },
            _ => unreachable!(),
        }
    }
}
