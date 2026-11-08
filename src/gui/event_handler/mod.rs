use super::*;

pub mod auth;
pub mod chat;
pub mod services;
pub mod threads;

impl ZaloGuiApp {
    pub(super) fn check_events(&mut self, ctx: &egui::Context) {
        while let Ok(event) = self.event_rx.try_recv() {
            match event {
                GuiEvent::QrReady { .. }
                | GuiEvent::QrScanned { .. }
                | GuiEvent::QrExpired(..)
                | GuiEvent::LoginSuccess { .. }
                | GuiEvent::LoginFailed(..)
                | GuiEvent::LoggedOut
                | GuiEvent::AccountSwitched { .. }
                | GuiEvent::SessionExpired(..)
                | GuiEvent::SessionUnreachable(..)
                | GuiEvent::LocalDataWiped => self.handle_auth_event(ctx, event),

                GuiEvent::MessageSent { .. }
                | GuiEvent::StickerSent { .. }
                | GuiEvent::AttachmentSent { .. }
                | GuiEvent::QuoteSent { .. }
                | GuiEvent::Reacted { .. }
                | GuiEvent::MessageDeleted { .. }
                | GuiEvent::MessageRecalled { .. }
                | GuiEvent::SendFailed { .. }
                | GuiEvent::StickerImageReady { .. }
                | GuiEvent::StickerStoreLoaded(..)
                | GuiEvent::StickerPackItemsLoaded { .. }
                | GuiEvent::StickerSearchResults { .. }
                | GuiEvent::StickerSuggestKeywordsLoaded(..)
                | GuiEvent::ComposerStickerSuggestions { .. }
                | GuiEvent::LinkPreviewReady(..) => self.handle_chat_event(ctx, event),

                GuiEvent::ThreadsUpdated(..)
                | GuiEvent::RefreshFinished { .. }
                | GuiEvent::ScheduledListUpdated
                | GuiEvent::SyncFinished { .. }
                | GuiEvent::MuteSnapshotSynced(..)
                | GuiEvent::MuteEventsReceived(..)
                | GuiEvent::PhoneSyncProgress { .. }
                | GuiEvent::PhoneSyncFinished { .. }
                | GuiEvent::HistoryReachedStart(..) => self.handle_threads_event(ctx, event),

                _ => self.handle_services_event(ctx, event),
            }
        }
    }
}
