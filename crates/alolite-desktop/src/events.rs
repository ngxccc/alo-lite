use alolite_core::{CoreEvent, CoreEventDto, CoreEventSink};
use tauri::{AppHandle, Emitter};

/// Bridges [`alolite_core::core::CoreEngine`] domain events to the Tauri WebView via `"core-event"`.
pub struct TauriEventSink {
    pub app: AppHandle,
}

impl CoreEventSink for TauriEventSink {
    fn emit(&self, event: CoreEvent) {
        let dto = CoreEventDto::from(event);
        let _ = self.app.emit("core-event", dto);
    }
}
