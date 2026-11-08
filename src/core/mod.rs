//! UI-agnostic application core engine and domain events.
//!
//! Keeps all protocol, session, WebSocket, long-poll, SQLite persistence, and image decoding
//! completely decoupled from `eframe`/`egui` so GUI frontends (egui today, Tauri later) and
//! headless modes (`--mcp`, `--api-port`) share one runtime owner.

pub mod engine;
pub mod events;

pub use engine::{CoreEngine, CoreStatus};
pub use events::{CoreEvent, CoreEventSink, CoreImageData, NoopEventSink};
