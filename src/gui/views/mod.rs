use crate::gui::theme::*;
use crate::gui::*;
use eframe::egui;

pub mod chat_area;
pub mod composer;
pub mod info_drawer;
pub mod nav_rail;
pub mod quote_target;
pub mod threads_sidebar;

#[allow(unused_imports)]
pub(crate) use quote_target::render_quote_target;

impl ZaloGuiApp {
    /// Start loading the full-resolution texture the image viewer shows. Message bubbles and
    /// the media grid keep a 480 px copy; the  key keeps this one (and its disk cache) apart.
    pub(crate) fn request_hd_texture(&mut self, full_url: &str) -> Option<String> {
        let lower_u = full_url.to_lowercase();
        let is_direct_img = lower_u.contains("zdn.vn")
            || lower_u.ends_with(".png")
            || lower_u.ends_with(".jpg")
            || lower_u.ends_with(".jpeg")
            || lower_u.ends_with(".webp")
            || lower_u.ends_with(".gif")
            || lower_u.ends_with(".bmp")
            || (!lower_u.starts_with("http://") && !lower_u.starts_with("https://"));
        if full_url.is_empty() || !is_direct_img {
            return None;
        }
        let hk = format!("hd_{}", crate::crypto::md5_hex(full_url));
        if !self.sticker_textures.contains_key(&hk) && self.loading_stickers.insert(hk.clone()) {
            if let Some(tx) = WORKER_TX.get() {
                let _ = tx.send(WorkerCmd::FetchStickerImage {
                    key: hk.clone(),
                    url: full_url.to_string(),
                });
            }
        }
        Some(hk)
    }

    pub(in crate::gui) fn render_chat_workspace(
        &mut self,
        ctx: &egui::Context,
        pal: &ThemePalette,
    ) {
        let screen_rect = ctx.screen_rect();
        let screen_w = screen_rect.width();
        let is_compact_w = false;
        let is_narrow_w = false;
        let left_rail_w = 62.0;

        // 1. Left Navigation Rail
        self.render_nav_rail(ctx, pal, left_rail_w, is_narrow_w);

        if self.contacts_view_open {
            self.render_contacts_page(ctx, pal);
            self.render_all_modals(ctx, pal);
            return;
        }

        // 2. Conversation List Sidebar
        let sidebar_exact_w = 300.0_f32;
        self.render_threads_sidebar(ctx, pal, sidebar_exact_w);

        // Extract metadata for Info Drawer and Chat Area
        let thread_meta = self.threads.get(self.selected_thread_index).map(|t| {
            (
                t.id.clone(),
                t.name.clone(),
                t.avatar.clone(),
                t.is_group,
                t.messages.len(),
            )
        });

        // 3. Collapsible Right Conversation Info Drawer
        let show_info_drawer = self.info_drawer_open && !is_compact_w;
        let drawer_w = (screen_w * 0.26).clamp(240.0, 310.0);
        if show_info_drawer {
            self.render_info_drawer(ctx, pal, drawer_w, &thread_meta);
        }

        // 4. Central Chat Area
        self.render_chat_area(ctx, pal, &thread_meta);

        // 5. Modals & Overlays
        self.render_all_modals(ctx, pal);
    }

    fn render_all_modals(&mut self, ctx: &egui::Context, pal: &ThemePalette) {
        self.render_profile_popover(ctx, pal);
        self.render_wipe_confirm(ctx, pal);
        self.render_toast_overlay(ctx, pal);
        self.render_forward_dialog(ctx, pal);
        self.render_image_viewer_modal(ctx, pal);
        self.render_phone_lookup_modal(ctx, pal);
        self.render_phone_sync_modal(ctx, pal);
        self.render_settings_modal(ctx, pal);
    }
}
