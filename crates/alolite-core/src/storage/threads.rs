use crate::protocol::wire::{opt_string, opt_u64, WireData};
use anyhow::Result;
use rusqlite::params;
use serde::{Deserialize, Serialize};

use super::models::{MessageStore, StoredMessage, StoredThread};
use crate::protocol::messaging::threads::{
    deduplicate_threads, is_generic_thread_name, ChatMessage, Thread,
};

/// `messages.raw_json` as written by the store when a message is persisted.
///
/// # Wire Response Example
/// ```json
/// {
///   "stickerId": 12345,
///   "cateId": 1234,
///   "type": 7,
///   "reply_to_id": "1731215399000_9",
///   "reply_to_text": "Tin trước đó",
///   "is_pinned": true,
///   "link_url": "https://example.com"
/// }
/// ```
///
/// # Core Transformation
/// Rehydrates the sticker, reply, reaction, pin/star, and link columns of a cached
/// [`ChatMessage`]; the newest `raw_json` wins because the store rewrites it on merge.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct CachedMessageWireData {
    #[serde(rename = "stickerId")]
    pub sticker_id_camel: Option<serde_json::Value>,
    #[serde(rename = "id")]
    pub id: Option<serde_json::Value>,
    #[serde(rename = "cateId")]
    pub cate_id: Option<serde_json::Value>,
    #[serde(rename = "catId")]
    pub cat_id: Option<serde_json::Value>,
    #[serde(rename = "type")]
    pub sticker_type: Option<serde_json::Value>,
    #[serde(rename = "reply_to_id")]
    pub reply_to_id: Option<serde_json::Value>,
    #[serde(rename = "reply_to_sender")]
    pub reply_to_sender: Option<serde_json::Value>,
    #[serde(rename = "reply_to_text")]
    pub reply_to_text: Option<serde_json::Value>,
    #[serde(rename = "reactions")]
    pub reactions: Option<Vec<serde_json::Value>>,
    #[serde(rename = "is_pinned")]
    pub is_pinned: Option<serde_json::Value>,
    #[serde(rename = "is_starred")]
    pub is_starred: Option<serde_json::Value>,
    #[serde(rename = "link_url")]
    pub link_url: Option<serde_json::Value>,
    #[serde(rename = "link_title")]
    pub link_title: Option<serde_json::Value>,
    #[serde(rename = "link_desc")]
    pub link_desc: Option<serde_json::Value>,
    #[serde(rename = "link_thumb")]
    pub link_thumb: Option<serde_json::Value>,
    #[serde(rename = "link_src")]
    pub link_src: Option<serde_json::Value>,
}

impl CachedMessageWireData {
    /// Sticker id (`stickerId` > `id`).
    pub fn sticker_id(&self) -> Option<u64> {
        opt_u64(self.sticker_id_camel.as_ref()).or_else(|| opt_u64(self.id.as_ref()))
    }

    /// Sticker category id (`cateId` > `catId`).
    pub fn sticker_cate_id(&self) -> Option<u64> {
        opt_u64(self.cate_id.as_ref()).or_else(|| opt_u64(self.cat_id.as_ref()))
    }

    /// Sticker type as a `u32`.
    pub fn sticker_type(&self) -> Option<u32> {
        opt_u64(self.sticker_type.as_ref()).map(|n| n as u32)
    }

    /// Quoted message id.
    pub fn reply_to_id(&self) -> Option<String> {
        opt_string(self.reply_to_id.as_ref())
    }

    /// Quoted message sender.
    pub fn reply_to_sender(&self) -> Option<String> {
        opt_string(self.reply_to_sender.as_ref())
    }

    /// Quoted message text.
    pub fn reply_to_text(&self) -> Option<String> {
        opt_string(self.reply_to_text.as_ref())
    }

    /// Legacy reaction emoji list stored alongside `raw_json`.
    pub fn reactions(&self) -> Vec<String> {
        self.reactions
            .as_deref()
            .unwrap_or_default()
            .iter()
            .filter_map(|v| v.as_str().map(str::to_string))
            .collect()
    }

    /// Whether the thread/message was pinned.
    pub fn is_pinned(&self) -> bool {
        self.is_pinned
            .as_ref()
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false)
    }

    /// Whether the message was starred.
    pub fn is_starred(&self) -> bool {
        self.is_starred
            .as_ref()
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false)
    }

    /// Link card URL.
    pub fn link_url(&self) -> Option<String> {
        opt_string(self.link_url.as_ref())
    }

    /// Link card title.
    pub fn link_title(&self) -> Option<String> {
        opt_string(self.link_title.as_ref())
    }

    /// Link card description.
    pub fn link_desc(&self) -> Option<String> {
        opt_string(self.link_desc.as_ref())
    }

    /// Link card thumbnail.
    pub fn link_thumb(&self) -> Option<String> {
        opt_string(self.link_thumb.as_ref())
    }

    /// Link card source label.
    pub fn link_src(&self) -> Option<String> {
        opt_string(self.link_src.as_ref())
    }
}

impl MessageStore {
    /// Delete an entire conversation and all its messages from local SQLite store (#24).
    pub fn delete_thread(&mut self, thread_id: &str) -> Result<()> {
        if thread_id.is_empty() {
            return Ok(());
        }
        self.conn.execute(
            "DELETE FROM messages WHERE thread_id = ?1",
            params![thread_id],
        )?;
        self.conn.execute(
            "DELETE FROM threads WHERE thread_id = ?1",
            params![thread_id],
        )?;
        Ok(())
    }

    /// Insert or update thread metadata.
    pub fn upsert_thread(&mut self, t: &StoredThread) -> Result<()> {
        self.conn.execute(
            "INSERT INTO threads (thread_id, name, is_group, last_ts)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(thread_id) DO UPDATE SET
                 name = excluded.name,
                 is_group = excluded.is_group,
                 last_ts = MAX(COALESCE(last_ts, 0), excluded.last_ts)",
            params![
                t.thread_id,
                t.name,
                if t.is_group { 1 } else { 0 },
                t.last_ts,
            ],
        )?;
        Ok(())
    }

    /// Retrieve all persisted thread metadata ordered by `last_ts` descending.
    pub fn all_threads(&self) -> Result<Vec<StoredThread>> {
        let mut stmt = self.conn.prepare(
            "SELECT thread_id, name, is_group, last_ts
             FROM threads
             ORDER BY last_ts DESC",
        )?;
        let rows = stmt.query_map([], |row| {
            let is_group_num: i64 = row.get(2).unwrap_or(0);
            Ok(StoredThread {
                thread_id: row.get(0)?,
                name: row.get(1).unwrap_or_default(),
                is_group: is_group_num != 0,
                last_ts: row.get(3).unwrap_or(0),
            })
        })?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    /// Hydrate `threads` with persisted threads and message history from SQLite.
    /// Returns the total number of messages loaded into `threads`.
    pub fn hydrate_threads(
        &self,
        threads: &mut Vec<Thread>,
        limit_per_thread: usize,
    ) -> Result<usize> {
        deduplicate_threads(threads, None);

        let stored_threads = self.all_threads().unwrap_or_default();
        for st in stored_threads {
            if st.thread_id.is_empty() || st.thread_id == "0" {
                continue;
            }
            if let Some(existing) = threads.iter_mut().find(|t| t.id == st.thread_id) {
                if is_generic_thread_name(&existing.name) && !st.name.is_empty() {
                    existing.name = st.name;
                }
                existing.is_group = existing.is_group || st.is_group;
            } else if !st.is_group
                && st.name.contains("Cloud của tôi")
                && threads
                    .iter()
                    .any(|t| !t.is_group && t.name.contains("Cloud của tôi"))
            {
                // Migrate stale session-scoped `send2me_id` ("Cloud của tôi") in SQLite to the active session's `id`.
                // Normal contacts/groups must NEVER be merged by display name alone (#63).
                if let Some(live_id) = threads
                    .iter()
                    .find(|t| !t.is_group && t.name.contains("Cloud của tôi"))
                    .map(|t| t.id.clone())
                {
                    let _ = self.conn.execute(
                        "UPDATE messages SET thread_id = ?1 WHERE thread_id = ?2",
                        params![live_id, st.thread_id],
                    );
                    let _ = self.conn.execute(
                        "DELETE FROM threads WHERE thread_id = ?1",
                        params![st.thread_id],
                    );
                }
            } else {
                threads.push(Thread {
                    presentation: Default::default(),
                    unread: Default::default(),
                    id: st.thread_id.clone(),
                    name: if st.name.is_empty() {
                        format!("Người dùng {}", st.thread_id)
                    } else {
                        st.name
                    },
                    preview: String::new(),
                    time: String::new(),
                    avatar: None,
                    is_group: st.is_group,
                    messages: Vec::new(),
                });
            }
        }

        // Primary-key lookups for the messages actually in memory. This used to load every
        // (msg_id, thread_id) row in the database into a HashMap on each call. One transaction
        // around them: in autocommit every SELECT takes and drops the file lock, ~50 µs on
        // Windows, which made 62k lookups cost 3 s.
        let tx = self.conn.unchecked_transaction()?;
        let mut owner_of = self
            .conn
            .prepare("SELECT thread_id FROM messages WHERE msg_id = ?1")?;

        let mut loaded_count = 0;
        for t in threads.iter_mut() {
            let before = t.messages.len();
            let tid = t.id.clone();
            t.messages.retain(|m| {
                owner_of
                    .query_row(params![m.id], |r| r.get::<_, String>(0))
                    .map_or(true, |owner| owner == tid)
            });
            if t.messages.len() != before && t.messages.is_empty() {
                t.preview.clear();
                t.time.clear();
                let _ = self.conn.execute(
                    "UPDATE threads SET last_ts = 0 WHERE thread_id = ?1",
                    params![t.id],
                );
            }
            let saved_msgs = self.messages_for(&t.id, limit_per_thread)?;
            for sm in saved_msgs {
                let time = if sm.ts > 0 {
                    crate::clock::message_time(sm.ts)
                } else {
                    String::new()
                };
                let mut sticker_id = None;
                let mut sticker_cate_id = None;
                let mut sticker_type = None;
                let mut reply_to_id = None;
                let mut reply_to_sender = None;
                let mut reply_to_text = None;
                let mut reactions = Vec::new();
                let mut is_pinned = false;
                let mut is_starred = false;
                let mut link_url = None;
                let mut link_title = None;
                let mut link_desc = None;
                let mut link_thumb = None;
                let mut link_src = None;
                if !sm.raw_json.is_empty() {
                    if let Ok(val) = serde_json::from_str::<serde_json::Value>(&sm.raw_json) {
                        let cached = CachedMessageWireData::from_value(&val);
                        sticker_id = cached.sticker_id();
                        sticker_cate_id = cached.sticker_cate_id();
                        sticker_type = cached.sticker_type();
                        reply_to_id = cached.reply_to_id();
                        reply_to_sender = cached.reply_to_sender();
                        reply_to_text = cached.reply_to_text();
                        let r_summaries =
                            crate::protocol::messaging::reactions::parse_reactions(&val);
                        if !r_summaries.is_empty() {
                            reactions =
                                crate::protocol::messaging::reactions::legacy_emojis(&r_summaries);
                        } else {
                            reactions = cached.reactions();
                        }
                        is_pinned = cached.is_pinned();
                        is_starred = cached.is_starred();
                        link_url = cached.link_url();
                        link_title = cached.link_title();
                        link_desc = cached.link_desc();
                        link_thumb = cached.link_thumb();
                        link_src = cached.link_src();
                    }
                }
                let sender_id_opt = if !sm.sender_id.is_empty() {
                    Some(sm.sender_id.clone())
                } else if sm.sender_name.chars().all(|c| c.is_ascii_digit())
                    && !sm.sender_name.is_empty()
                {
                    Some(sm.sender_name.clone())
                } else {
                    None
                };
                let cm = ChatMessage {
                    id: sm.msg_id,
                    sender: if !sm.sender_name.is_empty() {
                        sm.sender_name
                    } else {
                        sm.sender_id
                    },
                    sender_id: sender_id_opt,
                    text: sm.text,
                    time,
                    is_outgoing: sm.is_outgoing,
                    ts: sm.ts,
                    sticker_id,
                    sticker_cate_id,
                    sticker_type,
                    reply_to_id,
                    reply_to_sender,
                    reply_to_text,
                    reactions,
                    is_pinned,
                    is_starred,
                    link_url,
                    link_title,
                    link_desc,
                    link_thumb,
                    link_src,

                    ..Default::default()
                };
                if let Some(existing) = t.messages.iter_mut().find(|existing| {
                    (!existing.id.is_empty() && existing.id == cm.id)
                        || (existing.text == cm.text
                            && existing.is_outgoing == cm.is_outgoing
                            && (existing.ts == cm.ts
                                || (existing.ts != 0
                                    && cm.ts != 0
                                    && (existing.ts - cm.ts).abs() < 5000)))
                }) {
                    if existing.sender_id.is_none() && cm.sender_id.is_some() {
                        existing.sender_id = cm.sender_id;
                    }
                    if existing.sender.chars().all(|c| c.is_ascii_digit())
                        && !cm.sender.is_empty()
                        && !cm.sender.chars().all(|c| c.is_ascii_digit())
                    {
                        existing.sender = cm.sender;
                    }
                    if existing.sticker_id.is_none() && cm.sticker_id.is_some() {
                        existing.sticker_id = cm.sticker_id;
                        existing.sticker_cate_id = cm.sticker_cate_id;
                        existing.sticker_type = cm.sticker_type;
                    }
                    if existing.reply_to_text.is_none() && cm.reply_to_text.is_some() {
                        existing.reply_to_id = cm.reply_to_id;
                        existing.reply_to_sender = cm.reply_to_sender;
                        existing.reply_to_text = cm.reply_to_text;
                    }
                    if existing.link_url.is_none() && cm.link_url.is_some() {
                        existing.link_url = cm.link_url;
                        existing.link_title = cm.link_title;
                        existing.link_desc = cm.link_desc;
                        existing.link_thumb = cm.link_thumb;
                        existing.link_src = cm.link_src;
                        if !cm.text.is_empty() {
                            existing.text = cm.text;
                        }
                    }
                    if existing.reactions.is_empty() && !cm.reactions.is_empty() {
                        existing.reactions = cm.reactions;
                    }
                    existing.is_pinned = existing.is_pinned || cm.is_pinned;
                    existing.is_starred = existing.is_starred || cm.is_starred;
                } else {
                    t.messages.push(cm);
                    loaded_count += 1;
                }
            }
            if !t.messages.is_empty() {
                t.messages.sort_by_key(|m| m.ts);
                if let Some(newest) = t.messages.last() {
                    if !newest.text.is_empty() {
                        t.preview = newest.text.replace('\n', " ");
                        t.time = newest.time.clone();
                    }
                }
            }
        }
        drop(owner_of);
        tx.commit()?;
        deduplicate_threads(threads, None);
        Ok(loaded_count)
    }
}

/// Persist threads and their messages to SQLite store, returning the count of newly inserted messages.
pub fn persist_threads(store: &mut MessageStore, threads: &[Thread]) -> Result<usize> {
    // One transaction for the whole snapshot: in autocommit every row was its own fsync.
    store.conn.execute_batch("BEGIN")?;
    let res = persist_threads_inner(store, threads);
    store
        .conn
        .execute_batch(if res.is_ok() { "COMMIT" } else { "ROLLBACK" })?;
    res
}

fn persist_threads_inner(store: &mut MessageStore, threads: &[Thread]) -> Result<usize> {
    let mut new_count = 0;
    for t in threads {
        let last_ts = t
            .messages
            .iter()
            .filter(|m| m.text != "sendBubbleMessage")
            .map(|m| m.ts)
            .max()
            .unwrap_or(0);
        let st = StoredThread {
            thread_id: t.id.clone(),
            name: t.name.clone(),
            is_group: t.is_group,
            last_ts,
        };
        store.upsert_thread(&st)?;

        for m in t.messages.iter().filter(|m| m.text != "sendBubbleMessage") {
            let mut sm = StoredMessage::from(m);
            sm.thread_id = t.id.clone();
            if sm.msg_id.is_empty() {
                sm.msg_id = format!("{}_{}", t.id, sm.ts);
            }
            if store.upsert_message(&sm)? {
                new_count += 1;
            }
        }
    }
    Ok(new_count)
}

/// Convenience helper: open `zalo_history.db` (or `path`) and hydrate `threads` with saved history.
pub fn load_history_into_threads(path: &str, threads: &mut Vec<Thread>, limit_per_thread: usize) {
    if let Ok(store) = MessageStore::open(path) {
        let _ = store.hydrate_threads(threads, limit_per_thread);
    }
}

/// Hydrate `threads` from the SQLite history of account `uin` (`PRELOAD_MSGS` messages per thread).
///
/// Owns the account-to-database-path mapping so `protocol/` never needs to know about the store.
pub fn preload_account_history(uin: &str, threads: &mut Vec<Thread>) {
    let db_path = super::models::db_path_for_account(uin);
    load_history_into_threads(&db_path, threads, MessageStore::PRELOAD_MSGS);
}

/// Persist `threads` into the SQLite store of account `uin`, then refresh `threads_cache.json`.
///
/// Persistence is best-effort: an unopenable store or a failed transaction is skipped silently and
/// the cache file is still written, matching the former in-protocol behaviour of `poll_new_messages`.
pub fn persist_account_threads(uin: &str, threads: &[Thread]) {
    let db_path = super::models::db_path_for_account(uin);
    if let Ok(mut store) = MessageStore::open(&db_path) {
        let _ = persist_threads(&mut store, threads);
    }
    super::thread_cache::save_threads_cache(threads);
}
