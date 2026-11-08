use std::collections::{HashMap, HashSet};
use std::sync::mpsc::{Receiver, Sender};

use eframe::egui::{TextureHandle, Vec2};

use super::events::{GuiEvent, UpdateCheckResult};
use super::settings::{AppSettings, SettingsTab};
use super::stickers::StickerTab;
use super::ThreadFilter;
use crate::logger::{LogEntry, ScanFinding};
use crate::protocol::messaging::{ChatMessage, Thread};

#[derive(Debug, Clone)]
pub(super) struct ImageViewerModal {
    pub tex_key: String,
    pub hd_tex_key: Option<String>,
    pub full_url: String,
    pub title: String,
    pub sender: String,
    pub time: String,
    pub zoom: f32,
    pub pan: Vec2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ContactsSubTab {
    Friends,
    Groups,
    FriendRequests,
    GroupInvites,
}

#[derive(Debug, PartialEq, Eq)]
pub(super) enum LoginTab {
    QrCode,
    Password,
}

#[derive(Debug, PartialEq, Eq)]
pub(super) enum DevTab {
    Logs,
    Scanner,
    CryptoState,
}

/// A loaded-message jump stays bound to its conversation and exact reference.
pub(super) struct QuoteJump {
    pub thread_id: String,
    pub reference: String,
    pub highlight_until: std::time::Instant,
    pub scroll_pending: bool,
}

impl QuoteJump {
    pub(super) fn target_index(&self, thread_id: &str, messages: &[ChatMessage]) -> Option<usize> {
        if self.thread_id != thread_id || std::time::Instant::now() >= self.highlight_until {
            return None;
        }
        crate::message_navigation::unique_reference_index(messages, &self.reference)
    }
}

/// Bubbles rendered per step in the chat view; "Tải thêm tin nhắn cũ" adds another step.
pub(super) const CHAT_RENDER_STEP: usize = 150;

pub struct ZaloGuiApp {
    // Theme state
    pub(super) dark_mode: bool,
    pub(super) theme_dirty: bool,

    // Session state
    pub(super) is_logged_in: bool,
    pub(super) user_name: String,
    pub(super) user_uin: String,
    pub(super) user_avatar: Option<String>,

    // Login screen state
    pub(super) login_tab: LoginTab,
    pub(super) phone_input: String,
    pub(super) password_input: String,
    pub(super) login_in_progress: bool,
    pub(super) login_status_msg: String,

    // QR state
    pub(super) qr_texture: Option<TextureHandle>,
    pub(super) qr_status: String,
    pub(super) qr_scanned_name: Option<String>,
    pub(super) qr_scanned_avatar: Option<String>,

    // Chat screen state
    pub(super) threads: Vec<Thread>,
    pub(super) selected_thread_index: usize,
    pub(super) thread_filter: ThreadFilter,
    pub(super) search_query: String,
    pub(super) input_message: String,
    pub(super) status_msg: String,

    // Reply / forward state
    pub(super) replying_to: Option<ChatMessage>,
    pub(super) forward_msg: Option<ChatMessage>,
    pub(super) forward_filter: String,
    pub(super) filter_starred_only: bool,
    pub(super) image_viewer: Option<ImageViewerModal>,

    pub(super) info_drawer_open: bool,
    pub(super) profile_menu_open: bool,
    /// "Xoá dữ liệu tin nhắn cục bộ" is pending confirmation (#21).
    pub(super) wipe_confirm_open: bool,
    pub(super) toast_notice: Option<(String, std::time::Instant)>,
    pub(super) phone_modal_open: bool,
    pub(super) phone_modal_input: String,
    pub(super) phone_sync_modal_open: bool,
    pub(super) migrate_path_input: String,
    pub(super) phone_lookup_loading: bool,
    pub(super) phone_lookup_result: Option<crate::protocol::social::PhoneLookupResult>,
    pub(super) phone_lookup_error: Option<String>,
    pub(super) in_chat_search_open: bool,
    pub(super) in_chat_search_query: String,
    pub(super) in_chat_search_focus_pending: bool,
    // Sticker picker & image cache state
    pub(super) sticker_picker_open: bool,
    pub(super) sticker_tab: StickerTab,
    pub(super) sticker_textures: HashMap<String, TextureHandle>,
    pub(super) loading_stickers: HashSet<String>,
    pub(super) emoji_textures: HashMap<String, TextureHandle>,
    pub(super) app_logo_texture: Option<TextureHandle>,
    pub(super) sticker_store_packs: Vec<crate::protocol::messaging::StickerPackInfo>,
    pub(super) sticker_store_loading: bool,
    pub(super) sticker_store_filter: String,
    pub(super) sticker_pack_items: HashMap<u64, Vec<crate::protocol::messaging::StickerItem>>,
    pub(super) loading_sticker_packs: HashSet<u64>,
    pub(super) pinned_sticker_packs: Vec<(u64, String)>,
    pub(super) sticker_search_query: String,
    pub(super) sticker_search_results: Vec<crate::protocol::messaging::StickerItem>,
    pub(super) sticker_search_loading: bool,
    pub(super) last_sticker_search_dispatched: String,
    pub(super) last_sticker_search_edit_at: std::time::Instant,
    pub(super) sticker_suggest_keywords: HashSet<String>,
    pub(super) composer_sticker_cache:
        HashMap<String, Vec<crate::protocol::messaging::StickerItem>>,
    pub(super) composer_sticker_loading: HashSet<String>,
    pub(super) dismissed_sticker_suggest_for: String,
    pub(super) last_composer_text: String,
    pub(super) last_composer_edit_at: std::time::Instant,
    pub(super) composer_sticker_selected: Option<usize>,
    pub(super) composer_focus_pending: bool,
    pub(super) pending_clipboard_image: Option<(std::path::PathBuf, u32, u32, TextureHandle)>,
    pub(super) unread_counts: HashMap<String, usize>,
    pub(super) link_metadata_cache: HashMap<String, crate::protocol::messaging::LinkMetadata>,
    pub(super) loading_link_previews: HashSet<String>,
    pub(super) dismissed_link_preview_for: String,
    pub(super) friend_status_map: HashMap<String, crate::protocol::social::FriendshipStatus>,
    pub(super) checking_friend_status: HashSet<String>,
    pub(super) user_online_map: HashMap<String, (bool, String)>,
    pub(super) allow_quit: bool,
    pub(super) settings_modal_open: bool,
    pub(super) settings_tab: SettingsTab,
    pub(super) notification_authorization:
        crate::services::native_notifications::AuthorizationState,
    pub(super) settings: AppSettings,
    pub(super) settings_blur_tex: Option<TextureHandle>,
    pub(super) settings_blur_req_at: Option<std::time::Instant>,
    pub(super) update_checking: bool,
    pub(super) update_downloading: bool,
    pub(super) update_progress: Option<(u64, u64)>,
    pub(super) update_status: String,
    pub(super) update_info: Option<UpdateCheckResult>,
    pub(super) update_modal_open: bool,
    pub(super) update_banner_dismissed: bool,
    pub(super) last_notified_update_version: Option<String>,
    pub(super) dev_mode: bool,
    pub(super) dev_panel_open: bool,
    pub(super) dev_tab: DevTab,
    pub(super) selected_log: Option<LogEntry>,
    pub(super) scanner_findings: Vec<ScanFinding>,
    pub(super) contacts_view_open: bool,
    pub(super) contacts_sub_tab: ContactsSubTab,
    pub(super) contacts_search_query: String,
    pub(super) phone_sync_progress: f32,
    pub(super) phone_sync_status: String,
    pub(super) last_typing_sent_at: Option<(String, std::time::Instant)>,
    pub(super) last_synced_icon_unread: Option<usize>,

    // Background channels
    pub(super) event_rx: Receiver<GuiEvent>,
    #[allow(dead_code)]
    pub(super) event_tx: Sender<GuiEvent>,
    /// (thread id, number of newest bubbles rendered) — see `CHAT_RENDER_STEP`.
    pub(super) chat_render_window: (String, usize),
    pub(super) quote_jump: Option<QuoteJump>,
    pub(super) ime_active: bool,
    pub(super) ime_just_committed: bool,
    pub(super) history_reached_start: HashSet<String>,
    pub(super) force_scroll_to_bottom: bool,
}
