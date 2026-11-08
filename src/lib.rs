//! Alolite — Minimal native messaging client for Linux in pure Rust.

#![allow(
    clippy::too_many_arguments,
    clippy::type_complexity,
    clippy::field_reassign_with_default
)]

// 1. Core Feature Modules
pub mod common;
pub mod core;
pub mod gui;
pub mod mcp;
pub mod protocol;
pub mod services;
pub mod storage;

// 2. Convenience Re-exports & Aliases
pub use common::{clock, data_dir, logger, ui};
pub use core::{
    AppSettings, CoreBootstrapSnapshot, CoreEngine, CoreEvent, CoreEventDto, CoreEventSink,
    CoreImageData, CoreStatus, DevDiagnosticsSnapshot, ImportSummaryDto, NoopEventSink,
};
pub use gui::message_navigation;
pub use protocol as zalo;
pub use protocol::{
    auth, client, crypto, endpoints, messaging, param_cipher, registry, session, social, socket,
    sync, wire, ActionAck, ApiDomain, CommandSpec, SendMessageReceipt, ZaloClient,
};
pub use storage as store;
