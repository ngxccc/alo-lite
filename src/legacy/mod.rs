//! Legacy & non-core modules quarantined from the minimal Linux client core.
//!
//! These modules (VoIP, Video Feed, Multi-Account, Scheduler, Auto-Updater)
//! are scheduled for deprecation/pruning in Phase 2 of the roadmap.

pub mod account_manager;
pub mod api_mcp;
pub mod audio;
pub mod call;
pub mod feed;
pub mod media;
pub mod media_resources;
pub mod native_notifications;
pub mod scheduler;
pub mod updater;
pub mod voip_log;
