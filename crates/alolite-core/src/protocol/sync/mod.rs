//! Pure mobile and cloud synchronization protocol, binary container crypto, and wire decoders.
//!
//! Submodules keep `src/protocol/` free of `rusqlite` and `MessageStore` dependencies:
//! - [`controls`]: Sync V1/V2 HTTP/WS requests, `/api/znoise` ID translation, and `MobileSyncControlEvent`.
//! - [`zdb4`]: Ephemeral RSA-2048 sync keypair and `ZDB4.0` AES-256-CBC + XZ/LZMA container unpacker.
//! - [`bin_net`]: `ChatContent.BinNet` binary TLV decoder for attachments, quotes, and mentions.
//! - [`v2_proto`]: Sync V2 Ed25519/Curve25519 keys and `proto.Sync2` Protobuf chunk parser.
//! - [`history`]: Pure thread cloud history fetch loop and pagination seam.
//! - [`phone`]: Pure phone message sync, long-polling drain, and conversation preview fetching.
//! - [`container`]: Mobile backup container (`.db.crypt`) and Sync V2 HTTP download and decryption.
//!
//! Pure SQLite archive persistence lives in `crate::storage::sync_ingest`, composed by `CoreEngine`.

use std::sync::atomic::AtomicBool;
use std::sync::Mutex;

use crate::protocol::messaging::Thread;

pub mod bin_net;
pub mod container;
pub mod controls;
pub mod history;
pub mod phone;
pub mod v2_proto;
pub mod zdb4;
pub use bin_net::{
    build_synced_message_content, is_excluded_archive_msg_type, parse_bin_net_tlv, BinNetAttach,
    BinNetData, BinNetMention, BinNetQuote,
};
pub use container::{
    fetch_mobile_backup_container, fetch_sync_v2_messages, fetch_sync_v2_meta, DownloadedBackup,
};
pub use controls::{
    cancel_phone_sync, clean_sync, extract_mobile_sync_controls, request_phone_sync,
    request_phone_sync_v2, request_phone_sync_with_temp_key, request_sync_v2_message_chunks,
    resolve_znoise_ids, MobileSyncControlEvent,
};
pub use history::{fetch_thread_history, MAX_PAGES, PAGE, PAUSE};
pub use phone::{fetch_phone_messages, format_phone_sync_final_message, PhoneFetchOutcome};
pub use v2_proto::{
    generate_sync_v2_ed25519_public_key, generate_sync_v2_keys, parse_sync_chunk, ParsedProtoMsg,
    ParsedProtoPartition, SyncV2Keys,
};
pub use zdb4::{
    clear_sync_keys, decrypt_and_unpack_zdb4_container, generate_sync_public_key, get_sync_keys,
    get_sync_public_key,
};

/// Set by the GUI's "Dừng" button; checked between Cloud/Group history pages.
pub static CANCEL: AtomicBool = AtomicBool::new(false);
/// True while a Cloud/Group sync runs, so a second click doesn't start a parallel one.
pub static RUNNING: AtomicBool = AtomicBool::new(false);

/// Set by the GUI's "Dừng đồng bộ điện thoại" button (#68).
pub static PHONE_SYNC_CANCEL: AtomicBool = AtomicBool::new(false);
/// True while a phone message sync (`sync_phone_messages`) is active (#68).
pub static PHONE_SYNC_RUNNING: AtomicBool = AtomicBool::new(false);

static LATEST_MOBILE_TEMP_KEY: Mutex<String> = Mutex::new(String::new());
static LAST_MOBILE_SYNC_STATUS: Mutex<Option<String>> = Mutex::new(None);

/// Record a fresh `temp_key` received from a `transfer_after_login` control event.
pub fn set_latest_mobile_temp_key(temp_key: &str) {
    if let Ok(mut g) = LATEST_MOBILE_TEMP_KEY.lock() {
        *g = temp_key.trim().to_string();
    }
}

/// Return the most recent `temp_key` issued by the user's phone on login (if any).
pub fn latest_mobile_temp_key() -> String {
    LATEST_MOBILE_TEMP_KEY
        .lock()
        .map(|g| g.clone())
        .unwrap_or_default()
}

/// Record the latest status returned by the user's phone (`WaitingConfirmOnPhone`, `MobileBusyRestoring`, etc.).
pub fn set_last_mobile_sync_status(status: Option<String>) {
    if let Ok(mut g) = LAST_MOBILE_SYNC_STATUS.lock() {
        *g = status;
    }
}

/// Read the latest status returned by the user's phone during `sync_phone_messages`.
pub fn last_mobile_sync_status() -> Option<String> {
    LAST_MOBILE_SYNC_STATUS.lock().ok().and_then(|g| g.clone())
}

/// Return the oldest server message ID in `t` when it advances past `current`, skipping `out_*` echoes.
pub fn next_cursor(t: &Thread, current: &str) -> Option<String> {
    let oldest = t
        .messages
        .iter()
        .map(|m| m.id.as_str())
        .find(|id| !id.is_empty() && id.bytes().all(|b| b.is_ascii_digit()))?;
    (oldest != current).then(|| oldest.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::messaging::ChatMessage;

    fn thread_with(ids: &[&str]) -> Thread {
        Thread {
            id: "g1".into(),
            name: "Nhóm".into(),
            is_group: true,
            messages: ids
                .iter()
                .map(|id| ChatMessage {
                    id: (*id).into(),
                    ..Default::default()
                })
                .collect(),
            ..Default::default()
        }
    }

    #[test]
    fn cursor_moves_to_the_oldest_server_message() {
        assert_eq!(
            next_cursor(&thread_with(&["5", "9"]), "9").as_deref(),
            Some("5")
        );
        assert_eq!(
            next_cursor(&thread_with(&["out_1", "7", "9"]), "9").as_deref(),
            Some("7")
        );
    }

    #[test]
    fn cursor_stops_when_page_brought_nothing_older() {
        assert_eq!(next_cursor(&thread_with(&["5", "9"]), "5"), None);
        assert_eq!(next_cursor(&thread_with(&[]), "5"), None);
        assert_eq!(next_cursor(&thread_with(&["", "out_2"]), "5"), None);
    }
}
