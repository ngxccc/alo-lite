//! AloLite Core — UI-independent Zalo protocol, Tokio `CoreEngine`, SQLite store, and MCP server.

#![allow(
    clippy::too_many_arguments,
    clippy::type_complexity,
    clippy::field_reassign_with_default
)]

pub mod common;
pub mod core;
pub mod mcp;
pub mod protocol;
pub mod services;
pub mod storage;

pub use common::{clock, data_dir, logger, ui};
pub use core::{
    AppSettings, CoreBootstrapSnapshot, CoreEngine, CoreEvent, CoreEventDto, CoreEventSink,
    CoreImageData, CoreStatus, DevDiagnosticsSnapshot, ImportSummaryDto, NoopEventSink,
};
pub use protocol as zalo;
pub use protocol::{
    auth, client, crypto, endpoints, messaging, param_cipher, registry, session, social, socket,
    sync, wire, ActionAck, ApiDomain, CommandSpec, SendMessageReceipt, ZaloClient,
};
pub use storage as store;
