//! Complete native desktop GUI powered by `eframe` and `egui`.
//!
//! Designed with the UI/UX Pro Max Minimalism & Swiss Style system:
//! - Zero-idle-CPU reactive architecture (event-driven repaint, zero per-frame Thread cloning)
//! - Custom vector icons rendered via `egui::Painter` (zero texture/SVG bloat)
//! - High-contrast Light & Dark semantic color tokens (`#0068FF` / `#2563EB` primary, `#059669` online accent)
//! - Filterable conversation sidebar (`Tất cả`, `Cá nhân`, `Nhóm`), avatar badges, and grouped chat bubbles
//! - Built-in Zalo Sticker rendering & picker (`[Nhãn dán]`)

use std::collections::{HashMap, HashSet};
use std::sync::mpsc::channel;
use std::time::Duration;

use eframe::egui::{
    self, Align, Color32, CursorIcon, FontId, Id, Key, Layout, Margin, Pos2, Rect, RichText,
    Rounding, Sense, Stroke, TextureHandle, TextureOptions, Vec2,
};

use crate::logger::{self, FindingLevel};
use crate::protocol::messaging::{ChatMessage, Thread};
use crate::session::Session;

pub mod app_state;
pub mod components;
pub mod contacts;
pub mod event_handler;
pub mod events;
pub mod image_viewer;
pub mod login;
pub mod message_navigation;
mod messages;
mod modals;
pub mod settings;
mod stickers;
mod theme;
pub mod title_bar;
pub mod views;
mod worker;

pub use app_state::*;
pub use events::*;
pub use settings::*;

use messages::*;
pub use stickers::builtin_pack_stickers;
pub use stickers::CustomStickerPack;
pub use stickers::ThreadFilter;
use stickers::*;
use theme::*;
pub use theme::{emoji_to_tex_key, load_app_icon, lucide, REACTION_LIST};
pub use worker::WorkerCmd;
use worker::*;

fn merge_sent_message(thread: &mut Thread, message: ChatMessage) {
    crate::protocol::messaging::threads::merge_sent_message(thread, message);
}

fn thread_texts(threads: &[Thread]) -> impl Iterator<Item = &str> {
    threads.iter().flat_map(|t| {
        std::iter::once(t.name.as_str()).chain(t.messages.iter().map(|m| m.text.as_str()))
    })
}

impl ZaloGuiApp {
    pub fn new(
        cc: &eframe::CreationContext<'_>,
        mut session: Session,
        dev_mode: bool,
        initial_threads: Vec<Thread>,
        api_port: Option<u16>,
    ) -> Self {
        setup_custom_fonts(&cc.egui_ctx);
        let mut loaded_settings = AppSettings::load();
        if session.proxy_url.is_none() && !loaded_settings.proxy_url.trim().is_empty() {
            let _ = session.set_proxy(Some(&loaded_settings.proxy_url));
        } else if let Some(ref p) = session.proxy_url {
            if loaded_settings.proxy_url.trim().is_empty() {
                loaded_settings.proxy_url = p.clone();
            }
        }
        crate::session::set_privacy_blocker(
            loaded_settings.block_typing,
            loaded_settings.block_delivered,
            loaded_settings.block_seen,
        );
        let dark_mode = loaded_settings.theme_mode != "light";
        ThemePalette::load_with_custom_override(dark_mode).apply_to_ctx(&cc.egui_ctx);

        let (tx, rx) = channel();

        // A session the server already refused is not "logged in": jump straight to the
        // login prompt instead of re-testing it (#21).
        let is_logged_in = session.login.is_some() && !session.is_expired_marked();
        let (user_name, user_uin, user_avatar) = session
            .login
            .as_ref()
            .map(|l| {
                (
                    l.display_name.clone(),
                    l.uin.clone(),
                    if l.avatar.is_empty() {
                        None
                    } else {
                        Some(l.avatar.clone())
                    },
                )
            })
            .unwrap_or_default();

        // main() already read threads_cache.json and hydrated it from SQLite; reading, merging,
        // hydrating and rewriting the cache again here doubled startup I/O.
        let mut threads = initial_threads;
        if let Some(li) = session.login.as_ref() {
            if !li.send2me_id.is_empty() {
                for t in threads.iter_mut() {
                    if !t.is_group && t.name.contains("Cloud của tôi") {
                        t.id = li.send2me_id.clone();
                    }
                }
            }
        }
        let now_ts = crate::clock::unix_now();
        for t in threads.iter_mut() {
            if let Some(&until) = loaded_settings.muted_threads.get(&t.id) {
                if until == -1 || until > now_ts {
                    t.presentation.muted = true;
                    t.unread.muted = true;
                    t.unread.muted_until = (until > 0).then_some(until);
                }
            } else if t.unread.is_muted() {
                t.presentation.muted = true;
            }
        }
        crate::protocol::messaging::threads::deduplicate_threads(&mut threads, None);

        let selected_thread_index = threads
            .iter()
            .enumerate()
            .max_by_key(|(_, t)| t.messages.last().map(|m| m.ts).unwrap_or(0))
            .map(|(idx, _)| idx)
            .unwrap_or(0);

        let app = Self {
            dark_mode,
            theme_dirty: false,
            is_logged_in,
            user_name,
            user_uin,
            user_avatar,
            login_tab: LoginTab::QrCode,
            phone_input: String::new(),
            password_input: String::new(),
            login_in_progress: false,
            login_status_msg: String::new(),
            qr_texture: None,
            qr_status: "Đang tạo mã QR đăng nhập…".into(),
            qr_scanned_name: None,
            qr_scanned_avatar: None,
            threads,
            selected_thread_index,
            thread_filter: ThreadFilter::All,
            search_query: String::new(),
            input_message: String::new(),
            replying_to: None,
            forward_msg: None,
            forward_filter: String::new(),
            filter_starred_only: false,
            image_viewer: None,
            status_msg: "Trực tuyến · Sẵn sàng".into(),
            info_drawer_open: loaded_settings.info_drawer_open,
            profile_menu_open: false,
            wipe_confirm_open: false,
            toast_notice: None,
            phone_modal_open: false,
            phone_modal_input: String::new(),
            phone_sync_modal_open: false,
            migrate_path_input: String::new(),
            phone_lookup_loading: false,
            phone_lookup_result: None,
            phone_lookup_error: None,
            in_chat_search_open: false,
            in_chat_search_query: String::new(),
            in_chat_search_focus_pending: false,
            sticker_picker_open: false,
            sticker_tab: StickerTab::Popular,
            sticker_textures: HashMap::new(),
            loading_stickers: HashSet::new(),
            emoji_textures: load_default_emoji_textures(&cc.egui_ctx),
            app_logo_texture: load_app_logo_texture(&cc.egui_ctx),
            sticker_store_packs: Vec::new(),
            sticker_store_loading: false,
            sticker_store_filter: String::new(),
            sticker_pack_items: FEATURED_PACK_TABS
                .iter()
                .filter_map(|&(cid, _)| builtin_pack_stickers(cid).map(|items| (cid, items)))
                .collect(),
            loading_sticker_packs: HashSet::new(),
            pinned_sticker_packs: FEATURED_PACK_TABS
                .iter()
                .map(|&(cid, title)| (cid, title.to_string()))
                .collect(),
            sticker_search_query: String::new(),
            sticker_search_results: Vec::new(),
            sticker_search_loading: false,
            last_sticker_search_dispatched: String::new(),
            last_sticker_search_edit_at: std::time::Instant::now(),
            sticker_suggest_keywords: HashSet::new(),
            composer_sticker_cache: HashMap::new(),
            composer_sticker_loading: HashSet::new(),
            dismissed_sticker_suggest_for: String::new(),
            last_composer_text: String::new(),
            last_composer_edit_at: std::time::Instant::now(),
            composer_sticker_selected: None,
            composer_focus_pending: false,
            pending_clipboard_image: None,
            unread_counts: HashMap::new(),
            link_metadata_cache: HashMap::new(),
            loading_link_previews: HashSet::new(),
            dismissed_link_preview_for: String::new(),
            friend_status_map: HashMap::new(),
            checking_friend_status: HashSet::new(),
            user_online_map: HashMap::new(),
            allow_quit: false,
            settings_modal_open: false,
            settings_tab: SettingsTab::General,
            notification_authorization: Default::default(),
            settings: loaded_settings,
            settings_blur_tex: None,
            settings_blur_req_at: None,
            update_checking: false,
            update_downloading: false,
            update_progress: None,
            update_status: String::new(),
            update_info: None,
            update_modal_open: false,
            update_banner_dismissed: false,
            last_notified_update_version: None,
            dev_mode,
            dev_panel_open: false,
            dev_tab: DevTab::Logs,
            selected_log: None,
            scanner_findings: Vec::new(),
            contacts_view_open: false,
            contacts_sub_tab: ContactsSubTab::Friends,
            contacts_search_query: String::new(),
            phone_sync_progress: 0.0,
            phone_sync_status: String::new(),
            last_typing_sent_at: None,
            last_synced_icon_unread: None,
            event_rx: rx,
            event_tx: tx.clone(),
            chat_render_window: (String::new(), CHAT_RENDER_STEP),
            quote_jump: None,
            ime_active: false,
            ime_just_committed: false,
            history_reached_start: HashSet::new(),
            force_scroll_to_bottom: false,
        };
        let mut app = app;
        let _ = app.reload_custom_sticker_packs();

        // Clean up any leftover `.old` executable from a previous in-place self-update
        if let Ok(exe_path) = std::env::current_exe() {
            let old_path = exe_path.with_extension("exe.old");
            let _ = std::fs::remove_file(&old_path);
            let old_unix = exe_path.with_extension("old");
            let _ = std::fs::remove_file(&old_unix);
        }

        spawn_session_worker(
            session,
            GuiTx {
                tx,
                ctx: cc.egui_ctx.clone(),
            },
            cc.egui_ctx.clone(),
            api_port,
        );
        app.publish_known_threads();
        ensure_cjk_font(&cc.egui_ctx, thread_texts(&app.threads));

        if let Some(wtx) = WORKER_TX.get() {
            if is_logged_in {
                let _ = wtx.send(WorkerCmd::RefreshThreads);
            } else {
                let _ = wtx.send(WorkerCmd::RequestQr);
            }
            let _ = wtx.send(WorkerCmd::CheckForUpdates { silent: true });
        }
        app
    }

    fn palette(&self) -> ThemePalette {
        ThemePalette::load_with_custom_override(self.dark_mode)
    }

    pub(super) fn reload_custom_sticker_packs(&mut self) -> usize {
        let packs = load_custom_sticker_packs();
        let count = packs.len();
        for p in packs {
            if !self
                .pinned_sticker_packs
                .iter()
                .any(|(cid, _)| *cid == p.cate_id)
            {
                self.pinned_sticker_packs.push((p.cate_id, p.name.clone()));
            }
            if let Some(items) = builtin_pack_stickers(p.cate_id) {
                self.sticker_pack_items.insert(p.cate_id, items);
            }
        }
        count
    }

    /// Hand the worker's poll timer the thread ids/names it maps incoming messages onto.
    fn publish_known_threads(&self) {
        let known = self
            .threads
            .iter()
            .map(|t| (t.id.clone(), t.name.clone(), t.is_group))
            .collect();
        worker::publish_known_threads_to_engine(known);
    }

    fn persist_threads_async(&self) {
        self.publish_known_threads();
        let snapshot = self.threads.clone();
        if let Some(wtx) = WORKER_TX.get() {
            let _ = wtx.send(WorkerCmd::PersistThreads(snapshot));
        }
    }
}

impl eframe::App for ZaloGuiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if ctx.input(|i| i.modifiers.ctrl && i.key_pressed(Key::Q))
            || ctx.input(|i| i.viewport().close_requested())
        {
            self.allow_quit = true;
            std::process::exit(0);
        }
        if ctx.input(|i| {
            (i.modifiers.ctrl || i.modifiers.command) && i.modifiers.shift && i.key_pressed(Key::P)
        }) {
            self.settings_modal_open = true;
            self.settings_tab = SettingsTab::Privacy;
        }
        if self.theme_dirty {
            self.palette().apply_to_ctx(ctx);
            self.theme_dirty = false;
        }

        self.check_events(ctx);

        self.ime_just_committed = false;
        ctx.input_mut(|i| {
            let mut ime_commit_in_frame = false;
            let mut ime_space_commit = false;

            #[cfg(target_os = "macos")]
            let space_is_down = {
                #[link(name = "CoreGraphics", kind = "framework")]
                extern "C" {
                    fn CGEventSourceKeyState(state_id: i32, key: u16) -> bool;
                }
                // state_id = 0 (kCGEventSourceStateCombinedSessionState), key = 49 (kVK_Space)
                unsafe { CGEventSourceKeyState(0, 49) }
            };
            #[cfg(not(target_os = "macos"))]
            let space_is_down = false;

            let space_in_events = i.events.iter().any(|e| match e {
                egui::Event::Key {
                    key: Key::Space,
                    pressed: true,
                    ..
                } => true,
                egui::Event::Text(s) if s == " " => true,
                _ => false,
            }) || i.key_down(Key::Space);

            // Check whether Enter, non-space punctuation/navigation, or a mouse click
            // occurred in the same frame so we only auto-append a trailing space when
            // Spacebar triggered the macOS Vietnamese Telex IME commit (#71).
            let enter_pressed = i.events.iter().any(|e| {
                matches!(
                    e,
                    egui::Event::Key {
                        key: Key::Enter,
                        pressed: true,
                        ..
                    }
                )
            });
            let non_space_trigger = i.events.iter().any(|e| match e {
                egui::Event::Text(s) => s != " ",
                egui::Event::Key {
                    key, pressed: true, ..
                } => !matches!(key, Key::Space | Key::Backspace),
                _ => false,
            });
            let mouse_clicked =
                i.pointer.any_pressed() || i.pointer.any_released() || i.pointer.any_click();
            let space_pressed = space_is_down || space_in_events;
            let mut was_composing = self.ime_active;

            for event in &mut i.events {
                match event {
                    egui::Event::Ime(egui::ImeEvent::Enabled) => {
                        self.ime_active = true;
                        was_composing = true;
                    }
                    egui::Event::Ime(egui::ImeEvent::Preedit(text)) => {
                        self.ime_active = !text.is_empty();
                        if !text.is_empty() {
                            was_composing = true;
                        }
                    }
                    egui::Event::Ime(egui::ImeEvent::Commit(ref mut text)) => {
                        let inferred_macos_space_commit = cfg!(target_os = "macos")
                            && was_composing
                            && !enter_pressed
                            && !non_space_trigger
                            && !mouse_clicked;
                        if (space_pressed || inferred_macos_space_commit)
                            && !text.is_empty()
                            && !text.ends_with(' ')
                            && !text.chars().any(crate::gui::theme::is_cjk)
                        {
                            text.push(' ');
                            ime_space_commit = true;
                        }
                        self.ime_active = false;
                        self.ime_just_committed = true;
                        ime_commit_in_frame = true;
                    }
                    egui::Event::Ime(egui::ImeEvent::Disabled) => {
                        self.ime_active = false;
                    }
                    _ => {}
                }
            }

            if ime_space_commit {
                // Prevent duplicate space insertion if winit also forwarded a Space event in the same frame.
                i.events.retain(|e| {
                    !matches!(e, egui::Event::Text(s) if s == " ")
                        && !matches!(
                            e,
                            egui::Event::Key {
                                key: Key::Space,
                                pressed: true,
                                ..
                            }
                        )
                });
            }

            if self.ime_active || ime_commit_in_frame {
                // Suppress Enter key events during IME composition/commit so pressing Enter
                // to accept an IME candidate does not blur TextEdit or send prematurely (#71).
                i.events.retain(|e| {
                    !matches!(
                        e,
                        egui::Event::Key {
                            key: Key::Enter,
                            ..
                        }
                    )
                });
            }
        });

        // No timer repaint: the worker owns the poll timer and every GuiTx send wakes the UI.

        let pal = self.palette();

        Self::handle_frameless_resize(ctx);
        self.render_custom_title_bar(ctx, &pal);
        // Dev Inspector Panel
        if self.dev_mode && self.dev_panel_open {
            egui::TopBottomPanel::bottom("dev_inspector")
                .resizable(true)
                .default_height(250.0)
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.strong("Dev Inspector & Protocol Scanner");
                        ui.separator();
                        ui.selectable_value(
                            &mut self.dev_tab,
                            DevTab::Logs,
                            "Lưu lượng mạng (Logs)",
                        );
                        ui.selectable_value(
                            &mut self.dev_tab,
                            DevTab::Scanner,
                            "Quét lỗi hệ thống (Scanner)",
                        );
                        ui.selectable_value(
                            &mut self.dev_tab,
                            DevTab::CryptoState,
                            "Trạng thái Crypto & Keys",
                        );

                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            if ui.button("Làm mới quét").clicked() {
                                self.scanner_findings = logger::scan_logs();
                            }
                            if ui.button("Xóa log").clicked() {
                                logger::clear_entries();
                                self.selected_log = None;
                                self.scanner_findings.clear();
                            }
                        });
                    });
                    ui.separator();

                    match self.dev_tab {
                        DevTab::Logs => self.render_dev_logs(ui),
                        DevTab::Scanner => self.render_dev_scanner(ui),
                        DevTab::CryptoState => self.render_dev_crypto(ui),
                    }
                });
        }

        // Main Workspace
        if !self.is_logged_in {
            egui::CentralPanel::default()
                .frame(egui::Frame::none().fill(pal.bg_chat_canvas))
                .show(ctx, |ui| {
                    self.render_login_screen(ui, &pal);
                });
        } else {
            self.render_chat_workspace(ctx, &pal);
        }

        // Auto-notification modal for new GitHub Releases (works on both Login & Chat screens)
        self.render_update_notification_modal(ctx, &pal);

        if !ctx.input(|i| i.viewport().maximized.unwrap_or(false)) {
            let screen = ctx.screen_rect();
            ctx.layer_painter(egui::LayerId::new(
                egui::Order::Foreground,
                Id::new("window_border_overlay"),
            ))
            .rect_stroke(
                screen.shrink(0.5),
                Rounding::ZERO,
                Stroke::new(1.0_f32, pal.border),
            );
        }
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        std::process::exit(0);
    }
}

// ---------------------------------------------------------------------------
// Screen Renderers
// ---------------------------------------------------------------------------

impl ZaloGuiApp {
    fn show_toast(&mut self, msg: impl Into<String>) {
        let text = msg.into();
        self.status_msg = text.clone();
        self.toast_notice = Some((text, std::time::Instant::now()));
    }
}

/// Translates raw server errors into friendly, actionable Vietnamese messages.
pub fn format_user_friendly_login_error(err: &str) -> String {
    if err.contains("18022") || err.contains("Invalid Signature") {
        "Chữ ký xác thực mật khẩu không khớp (mã 18022). Zalo hiện đã thắt chặt đăng nhập bằng mật khẩu trên thiết bị mới — vui lòng chuyển sang tab 'Quét mã QR' để đăng nhập nhanh và an toàn nhất.".into()
    } else if err.contains("18002") || err.contains("18003") || err.contains("18004") {
        "Số điện thoại hoặc mật khẩu không chính xác. Vui lòng kiểm tra lại.".into()
    } else if err.contains("18012") {
        "Chưa nhận được xác nhận từ điện thoại. Vui lòng mở Zalo trên điện thoại và bấm 'Đăng nhập'.".into()
    } else if err.contains("18060") {
        "Phiên bản xác thực đã cũ. Vui lòng sử dụng tính năng 'Quét mã QR'.".into()
    } else if err.contains("18063") || err.contains("18064") || err.contains("18065") {
        "Tài khoản yêu cầu xác minh bảo mật trên điện thoại. Vui lòng đăng nhập bằng 'Quét mã QR'."
            .into()
    } else if err.contains("hết thời gian") || err.contains("timeout") || err.contains("408") {
        "Hết thời gian chờ phản hồi từ máy chủ Zalo. Vui lòng thử lại.".into()
    } else {
        let clean = err
            .trim_start_matches("Lỗi đăng nhập: ")
            .trim_start_matches("login error ")
            .trim_start_matches("pc-status: ");
        format!("Đăng nhập không thành công: {clean}")
    }
}

#[cfg(test)]
mod tests {
    use super::{format_user_friendly_login_error, ChatMessage, Thread};

    #[test]
    fn formats_signature_error_into_friendly_qr_guidance() {
        let raw = "login error 18022: Invalid Signature";
        let friendly = format_user_friendly_login_error(raw);
        assert!(friendly.contains("18022"));
        assert!(friendly.contains("Quét mã QR"));
    }

    use super::{is_newer_semver, phone_sync_completion_message};

    #[test]
    fn sent_message_reconciles_when_server_push_arrives_first() {
        let mut thread = Thread::default();
        let server = ChatMessage {
            id: "server-a".into(),
            client_message_id: "client-a".into(),
            text: "Server push".into(),
            is_outgoing: true,
            delivery: crate::protocol::messaging::DeliveryState::Delivered,
            ..Default::default()
        };
        super::merge_sent_message(&mut thread, server.clone());
        for local_id in ["out_client-a", "server-a"] {
            super::merge_sent_message(
                &mut thread,
                ChatMessage {
                    id: local_id.into(),
                    client_message_id: "client-a".into(),
                    text: "Local echo".into(),
                    is_outgoing: true,
                    ..Default::default()
                },
            );
            assert_eq!(thread.messages.len(), 1);
            assert_eq!(thread.messages[0].id, "server-a");
            assert_eq!(thread.messages[0].delivery, server.delivery);
        }
    }

    #[test]
    fn delayed_local_ack_preserves_server_chronology() {
        let mut thread = Thread::default();
        for (id, client, ts) in [
            ("server-a", "client-a", 1000),
            ("server-b", "client-b", 2000),
        ] {
            thread.messages.push(ChatMessage {
                id: id.into(),
                client_message_id: client.into(),
                ts,
                time: ts.to_string(),
                is_outgoing: true,
                ..Default::default()
            });
        }
        for ack_id in ["out_client-a", "server-a"] {
            super::merge_sent_message(
                &mut thread,
                ChatMessage {
                    id: ack_id.into(),
                    client_message_id: "client-a".into(),
                    ts: 3000,
                    time: "3000".into(),
                    is_outgoing: true,
                    ..Default::default()
                },
            );
            let chronology: Vec<_> = thread
                .messages
                .iter()
                .map(|message| (message.id.as_str(), message.ts, message.time.as_str()))
                .collect();
            assert_eq!(
                chronology,
                [("server-a", 1000, "1000"), ("server-b", 2000, "2000")]
            );
        }
    }

    #[test]
    fn semver_comparison_handles_v_prefix_and_prerelease() {
        assert!(is_newer_semver("v0.0.2", "0.0.1"));
        assert!(is_newer_semver("0.1.0", "0.0.9"));
        assert!(is_newer_semver("V1.0.0-beta.1", "0.9.9"));
        assert!(!is_newer_semver("v0.0.1", "0.0.1"));
        assert!(!is_newer_semver("0.0.1", "0.0.2"));
    }

    #[test]
    fn phone_sync_completion_does_not_claim_phone_messages_were_imported() {
        let none = phone_sync_completion_message(0, 0, false, None);
        assert!(none.contains("Lịch sử điện thoại chưa được tải vào ứng dụng"));
        assert!(none.contains("0 tin mới từ Cloud/Nhóm"));

        let cloud = phone_sync_completion_message(12, 3, false, None);
        assert!(cloud.contains("12 tin mới từ Cloud/Nhóm (3 hội thoại)"));
        assert!(cloud.contains("Lịch sử điện thoại chưa được tải vào ứng dụng"));

        let failed = phone_sync_completion_message(0, 0, false, Some("network"));
        assert!(failed.contains("Đồng bộ thất bại: network"));
    }
}
