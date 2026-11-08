//! Fast-first-paint JSON snapshot cache (`threads_cache.json` and `accounts/{uin}_threads.json`).
//!
//! Keeps local disk I/O inside `src/storage/` so `src/protocol/` remains pure protocol & wire logic.

use crate::protocol::messaging::threads::{deduplicate_threads, Thread};
use crate::storage::models::MessageStore;

fn active_uin_from_session() -> Option<String> {
    if let Ok(text) = std::fs::read_to_string("session.json") {
        if let Ok(val) = serde_json::from_str::<serde_json::Value>(&text) {
            if let Some(uin) = val
                .pointer("/login/uin")
                .and_then(serde_json::Value::as_str)
            {
                if !uin.is_empty() {
                    return Some(uin.to_string());
                }
            }
        }
    }
    None
}

/// Load locally cached conversations and message history scoped to the active account.
pub fn load_threads_cache() -> Vec<Thread> {
    if let Some(uin) = active_uin_from_session() {
        let acc_path = format!("accounts/{uin}_threads.json");
        if let Ok(text) = std::fs::read_to_string(&acc_path) {
            if let Ok(mut threads) = serde_json::from_str::<Vec<Thread>>(&text) {
                deduplicate_threads(&mut threads, None);
                return threads;
            }
        }
    }
    if std::path::Path::new("session.json").exists() {
        if let Ok(text) = std::fs::read_to_string("threads_cache.json") {
            if let Ok(mut threads) = serde_json::from_str::<Vec<Thread>>(&text) {
                deduplicate_threads(&mut threads, None);
                return threads;
            }
        }
    }
    Vec::new()
}

/// Persist a slim snapshot (`PRELOAD_MSGS` newest messages per thread) for instant UI startup.
pub fn save_threads_cache(threads: &[Thread]) {
    let keep = MessageStore::PRELOAD_MSGS;
    let slim: Vec<Thread> = threads
        .iter()
        .map(|t| Thread {
            presentation: t.presentation.clone(),
            unread: t.unread.clone(),
            id: t.id.clone(),
            name: t.name.clone(),
            preview: t.preview.clone(),
            time: t.time.clone(),
            avatar: t.avatar.clone(),
            is_group: t.is_group,
            messages: t.messages[t.messages.len().saturating_sub(keep)..].to_vec(),
        })
        .collect();
    if let Ok(text) = serde_json::to_string(&slim) {
        let _ = std::fs::write("threads_cache.json", &text);
        if let Some(uin) = active_uin_from_session() {
            let _ = std::fs::create_dir_all("accounts");
            let _ = std::fs::write(format!("accounts/{uin}_threads.json"), text);
        }
    }
}
