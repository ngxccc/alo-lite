//! Alolite — Minimal native messaging client for Linux in pure Rust.

#![allow(
    clippy::too_many_arguments,
    clippy::type_complexity,
    clippy::field_reassign_with_default
)]

// 1. Core Feature Modules
pub mod common;
pub mod gui;
pub mod protocol;
pub mod storage;

// 2. Convenience Re-exports & Aliases
pub use common::{clock, data_dir, logger, ui};
pub use gui::message_navigation;
pub use protocol as zalo;
pub use protocol::{
    auth, client, crypto, endpoints, messaging, param_cipher, registry, session, social, socket,
    sync, wire, ApiDomain, CommandSpec, ZaloClient,
};
pub use storage as store;

// 3. Quarantined Legacy Modules (VoIP, Feed, Scheduler, Auto-Updater)
pub mod legacy;
pub use legacy::*;
