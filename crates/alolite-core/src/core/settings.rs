//! UI-agnostic persistent application settings (`app_settings.json`).

use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type)]
#[serde(default)]
pub struct AppSettings {
    // 1. Cài đặt chung
    pub show_all_contacts: bool,
    pub language: String,
    pub autostart_on_boot: bool,
    pub remember_login: bool,
    pub download_dir: String,
    // 2. Quyền riêng tư
    pub show_birthday: String,
    pub show_online_status: bool,
    pub show_seen_status: bool,
    pub allow_message_from: String,
    pub block_typing: bool,
    pub block_delivered: bool,
    pub block_seen: bool,
    pub allow_call_from: String,
    pub allow_search_by_phone: bool,
    pub friend_src_qr: bool,
    pub friend_src_group: bool,
    pub friend_src_card: bool,
    // 3. Giao diện
    pub theme_mode: String, // "light" | "dark" | "system"
    pub use_avatar_as_wallpaper: bool,
    // 4. Thông báo
    pub enable_notifications: bool,
    pub play_notification_sound: bool,
    // 5. Tin nhắn
    pub quick_messages_enabled: bool,
    pub hidden_chat_pin_enabled: bool,
    pub split_priority_inbox: bool,
    pub excel_paste_mode: String, // "image" | "text" | "ask"
    pub double_click_to_reply: bool,
    pub show_typing_indicator: bool,
    pub send_message_key: String, // "enter" | "cmd_enter"
    // 6. Tiện ích
    pub suggest_stickers_while_typing: bool,
    pub suggest_mentions: bool,
    pub enable_gif_search: bool,
    pub enable_sticker_keyword_search: bool,
    // 7. Thiết bị phụ trợ (giữ tương thích schema cũ)
    pub selected_mic_device: String,
    pub selected_camera_device: String,
    pub mic_gain_percent: u32,
    // 8. Quản lý hội thoại (Context menu: Ghim, Phân loại, Tắt thông báo, Ẩn)
    pub pinned_threads: Vec<String>,
    pub muted_threads: HashMap<String, i64>,
    pub hidden_threads: HashSet<String>,
    pub thread_labels: HashMap<String, String>,
    // 9. Cập nhật tự động qua GitHub Releases API
    pub auto_check_updates: bool,
    pub skipped_update_version: String,
    // 10. Đa tài khoản
    pub multi_profile_name: String,
    pub info_drawer_open: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        let default_dl = std::env::var("USERPROFILE")
            .ok()
            .map(|p| format!(r"{p}\Downloads\Zalo Received Files"))
            .or_else(|| {
                std::env::var("HOME")
                    .ok()
                    .map(|h| format!("{h}/Downloads/Zalo Received Files"))
            })
            .unwrap_or_else(|| "Downloads/Zalo Received Files".into());
        Self {
            show_all_contacts: false,
            language: "Tiếng Việt".into(),
            autostart_on_boot: true,
            remember_login: true,
            download_dir: default_dl,
            show_birthday: "Không hiện".into(),
            show_online_status: true,
            show_seen_status: true,
            block_typing: false,
            block_delivered: false,
            block_seen: false,
            allow_message_from: "Tất cả mọi người".into(),
            allow_call_from: "Bạn bè và người lạ từng liên hệ".into(),
            allow_search_by_phone: true,
            friend_src_qr: true,
            friend_src_group: true,
            friend_src_card: true,
            theme_mode: "dark".into(),
            use_avatar_as_wallpaper: false,
            enable_notifications: true,
            play_notification_sound: false,
            quick_messages_enabled: true,
            hidden_chat_pin_enabled: true,
            split_priority_inbox: false,
            excel_paste_mode: "ask".into(),
            double_click_to_reply: true,
            show_typing_indicator: true,
            send_message_key: "enter".into(),
            suggest_stickers_while_typing: true,
            suggest_mentions: true,
            enable_gif_search: true,
            enable_sticker_keyword_search: true,
            selected_mic_device: String::new(),
            selected_camera_device: String::new(),
            mic_gain_percent: 100,
            pinned_threads: Vec::new(),
            muted_threads: HashMap::new(),
            hidden_threads: HashSet::new(),
            thread_labels: HashMap::new(),
            auto_check_updates: true,
            skipped_update_version: String::new(),
            multi_profile_name: "acc2".into(),
            info_drawer_open: false,
        }
    }
}

impl AppSettings {
    pub fn load() -> Self {
        std::fs::read_to_string("app_settings.json")
            .ok()
            .and_then(|s| serde_json::from_str::<Self>(&s).ok())
            .unwrap_or_default()
    }

    pub fn save(&self) {
        if let Ok(json) = serde_json::to_string_pretty(self) {
            let _ = std::fs::write("app_settings.json", json);
        }
    }

    /// Synchronize active privacy-blocker flags inside the protocol session layer.
    pub fn apply_privacy_blockers(&self) {
        crate::session::set_privacy_blocker(
            self.block_typing,
            self.block_delivered,
            self.block_seen,
        );
    }
}
