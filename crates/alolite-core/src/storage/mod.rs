//! Local message and thread store backed by SQLite (rusqlite bundled).
//!
//! Persists every message seen via polling or group history so chat history accumulates locally.

use anyhow::Result;
use rusqlite::Connection;

pub mod importer;
pub mod messages;
pub mod models;
pub mod sync_ingest;
pub mod thread_cache;
pub mod threads;

pub use importer::*;
pub use models::*;
pub use sync_ingest::{
    commit_conv_archives_to_store, commit_mobile_backup_to_store, commit_phone_sync_to_store,
    commit_sync_v2_messages_to_store, extract_conv_archives_from_entries, oldest_message_id,
    persist_resolved_thread_names, persist_thread_snapshot, ConvArchive, RawArchiveRow,
};
pub use thread_cache::{load_threads_cache, save_threads_cache};
pub use threads::{
    load_history_into_threads, persist_account_threads, persist_threads, preload_account_history,
};

impl MessageStore {
    /// Open SQLite database at `path` and initialize table schemas and indices if absent.
    pub fn open(path: &str) -> Result<Self> {
        let conn = if path == ":memory:" {
            Connection::open_in_memory()?
        } else {
            Connection::open(path)?
        };

        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS messages (
                msg_id TEXT PRIMARY KEY,
                thread_id TEXT NOT NULL,
                sender_id TEXT,
                sender_name TEXT,
                text TEXT,
                msg_type TEXT,
                ts INTEGER,
                is_outgoing INTEGER,
                raw_json TEXT
            );
            CREATE INDEX IF NOT EXISTS idx_thread_ts ON messages(thread_id, ts);
            CREATE TABLE IF NOT EXISTS threads (
                thread_id TEXT PRIMARY KEY,
                name TEXT,
                is_group INTEGER,
                last_ts INTEGER
            );",
        )?;
        // Migration 1: one-time cleanup of placeholder rows. This DELETE used to run (a full
        // table scan) on every hydrate; new rows are filtered in `persist_threads` instead.
        if conn.query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0))? < 1 {
            conn.execute_batch(
                "BEGIN;
                 DELETE FROM messages WHERE text = 'sendBubbleMessage';
                 PRAGMA user_version = 1;
                 COMMIT;",
            )?;
        }

        Ok(Self { conn })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_time_when_hydrating_seconds_and_milliseconds() {
        let mut store = MessageStore::open(":memory:").unwrap();
        store
            .upsert_thread(&StoredThread {
                thread_id: "thread-1".into(),
                name: "Example".into(),
                is_group: false,
                last_ts: 1793467800000,
            })
            .unwrap();
        for (id, ts) in [("msg-0001", 1793467800), ("msg-0002", 1793467800000)] {
            store
                .upsert_message(&StoredMessage {
                    msg_id: id.into(),
                    thread_id: "thread-1".into(),
                    sender_id: "user-1".into(),
                    sender_name: "Example".into(),
                    text: "hello".into(),
                    msg_type: "chat.text".into(),
                    ts,
                    is_outgoing: false,
                    raw_json: String::new(),
                })
                .unwrap();
        }
        let mut threads = Vec::new();
        assert_eq!(store.hydrate_threads(&mut threads, 10).unwrap(), 2);
        let expected = crate::clock::message_time(1793467800000);
        assert_eq!(threads[0].time, expected);
        for message in &threads[0].messages {
            assert_eq!(message.time, expected);
        }
        assert_eq!(
            store.messages_for("thread-1", 10).unwrap()[0].ts,
            1793467800
        );
    }

    #[test]
    fn test_insert_twice_returns_false() {
        let mut store = MessageStore::open(":memory:").unwrap();
        let msg = StoredMessage {
            msg_id: "m_1001".into(),
            thread_id: "thread_1".into(),
            sender_id: "u_1".into(),
            sender_name: "Alice".into(),
            text: "Hello".into(),
            msg_type: "chat.text".into(),
            ts: 1700000000,
            is_outgoing: false,
            raw_json: String::new(),
        };

        assert!(store.upsert_message(&msg).unwrap());
        assert!(!store.upsert_message(&msg).unwrap());

        let loaded = store.messages_for("thread_1", 10).unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].text, "Hello");
    }

    #[test]
    fn test_messages_for_returns_ordered_rows() {
        let mut store = MessageStore::open(":memory:").unwrap();
        for i in 1..=5 {
            store
                .upsert_message(&StoredMessage {
                    msg_id: format!("m_{i}"),
                    thread_id: "t_test".into(),
                    sender_id: "u_1".into(),
                    sender_name: "Alice".into(),
                    text: format!("msg {i}"),
                    msg_type: "chat.text".into(),
                    ts: 1000 + i,
                    is_outgoing: false,
                    raw_json: String::new(),
                })
                .unwrap();
        }

        let loaded = store.messages_for("t_test", 3).unwrap();
        assert_eq!(loaded.len(), 3);
        assert_eq!(loaded[0].msg_id, "m_3");
        assert_eq!(loaded[1].msg_id, "m_4");
        assert_eq!(loaded[2].msg_id, "m_5");
    }

    #[test]
    fn oldest_msg_id_skips_local_ids_and_last_ts_never_goes_back() {
        let mut store = MessageStore::open(":memory:").unwrap();

        let thread_id = "t_oldest_order";
        let msgs = [
            ("out_client_001", 1000_i64),
            ("1002", 1002),
            ("1005", 1005),
            ("t_oldest_order_1003", 1003),
            ("1001", 1001),
        ];
        for (id, ts) in msgs {
            store
                .upsert_message(&StoredMessage {
                    msg_id: id.into(),
                    thread_id: thread_id.into(),
                    sender_id: "u_1".into(),
                    sender_name: "User".into(),
                    text: "t".into(),
                    msg_type: "chat.text".into(),
                    ts,
                    is_outgoing: false,
                    raw_json: String::new(),
                })
                .unwrap();
        }

        assert_eq!(store.oldest_msg_id(thread_id).as_deref(), Some("1001"));

        store
            .upsert_thread(&StoredThread {
                thread_id: thread_id.into(),
                name: "Topic".into(),
                is_group: false,
                last_ts: 5000,
            })
            .unwrap();

        store
            .upsert_thread(&StoredThread {
                thread_id: thread_id.into(),
                name: "Topic Renamed".into(),
                is_group: false,
                last_ts: 2000,
            })
            .unwrap();

        let all = store.all_threads().unwrap();
        let loaded = all.iter().find(|t| t.thread_id == thread_id).unwrap();
        assert_eq!(loaded.name, "Topic Renamed");
        assert_eq!(loaded.last_ts, 5000);
    }

    #[test]
    fn test_persist_threads_and_upsert_thread() {
        use crate::protocol::messaging::{ChatMessage, Thread};

        let mut store = MessageStore::open(":memory:").unwrap();

        let t1 = Thread {
            id: "123".into(),
            name: "Alice".into(),
            is_group: false,
            messages: vec![ChatMessage {
                id: "m_1".into(),
                sender: "Alice".into(),
                text: "Hi".into(),
                time: "10:00".into(),
                is_outgoing: false,
                ts: 1000,
                ..Default::default()
            }],
            ..Default::default()
        };

        let t2 = Thread {
            id: "456".into(),
            name: "Nhóm bạn".into(),
            is_group: true,
            messages: vec![
                ChatMessage {
                    id: "m_2".into(),
                    sender: "Bob".into(),
                    text: "Alo".into(),
                    time: "10:01".into(),
                    is_outgoing: false,
                    ts: 2000,
                    ..Default::default()
                },
                ChatMessage {
                    id: "m_skip".into(),
                    sender: "Tôi".into(),
                    text: "sendBubbleMessage".into(),
                    time: "10:02".into(),
                    is_outgoing: true,
                    ts: 3000,
                    ..Default::default()
                },
            ],
            ..Default::default()
        };

        let new_msgs = persist_threads(&mut store, &[t1, t2]).unwrap();
        assert_eq!(new_msgs, 2);

        let threads = store.all_threads().unwrap();
        assert_eq!(threads.len(), 2);
        assert_eq!(threads[0].thread_id, "456");
        assert_eq!(threads[0].last_ts, 2000);
        assert!(threads[0].is_group);

        assert_eq!(threads[1].thread_id, "123");
        assert_eq!(threads[1].last_ts, 1000);
        assert!(!threads[1].is_group);
    }

    #[test]
    fn delete_thread_and_search_messages_work() {
        let mut store = MessageStore::open(":memory:").unwrap();
        let t = StoredThread {
            thread_id: "t_del".into(),
            name: "Hội thoại xóa".into(),
            is_group: false,
            last_ts: 1000,
        };
        store.upsert_thread(&t).unwrap();
        store
            .upsert_message(&StoredMessage {
                msg_id: "m_del_1".into(),
                thread_id: "t_del".into(),
                sender_id: "u_1".into(),
                sender_name: "Ngọc".into(),
                text: "Cần tìm kiếm keyword đặc biệt".into(),
                msg_type: "chat.text".into(),
                ts: 1000,
                is_outgoing: false,
                raw_json: String::new(),
            })
            .unwrap();

        let hits = store.search_messages("keyword đặc biệt", 10).unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].msg_id, "m_del_1");

        let name_hits = store.search_messages("ngọc", 10).unwrap();
        assert_eq!(name_hits.len(), 1);

        store.delete_thread("t_del").unwrap();
        assert!(store.messages_for("t_del", 10).unwrap().is_empty());
        assert!(!store
            .all_threads()
            .unwrap()
            .iter()
            .any(|th| th.thread_id == "t_del"));
        assert!(store
            .search_messages("keyword đặc biệt", 10)
            .unwrap()
            .is_empty());
    }

    #[test]
    fn import_history_from_path_migrates_sqlite_and_json_directory() {
        use crate::protocol::messaging::{ChatMessage, Thread};

        let temp_dir =
            std::env::temp_dir().join(format!("test_store_import_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&temp_dir);
        std::fs::create_dir_all(&temp_dir).unwrap();

        let src_json = temp_dir.join("threads_cache.json");
        let sample_threads = vec![Thread {
            id: "t_json".into(),
            name: "From JSON".into(),
            is_group: false,
            messages: vec![ChatMessage {
                id: "m_j1".into(),
                sender: "Alice".into(),
                text: "Text from JSON".into(),
                time: "12:00".into(),
                is_outgoing: false,
                ts: 1000,
                ..Default::default()
            }],
            ..Default::default()
        }];
        std::fs::write(
            &src_json,
            serde_json::to_string_pretty(&sample_threads).unwrap(),
        )
        .unwrap();

        let dst_db = temp_dir.join("imported.db");
        let rep = import_history_from_path(&temp_dir, dst_db.to_str().unwrap()).unwrap();

        assert_eq!(rep.threads_imported, 1);
        assert_eq!(rep.messages_imported, 1);

        let store = MessageStore::open(dst_db.to_str().unwrap()).unwrap();
        let loaded = store.messages_for("t_json", 10).unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].text, "Text from JSON");

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn jxl_to_jpg_url_and_decode_zalo_cipher_payload_work() {
        let jxl = "https://example.com/jxl/sample.jxl?jxlstatus=1&token=abc";
        let jpg = jxl_to_jpg_url(jxl);
        assert_eq!(jpg, "https://example.com/jpg/sample.jpg?token=abc");

        let plain = "Hello world";
        let key = "de039414865b856ea650d3305b1f387f";
        let key_chars: Vec<char> = key.chars().collect();
        let enc: String = plain
            .chars()
            .enumerate()
            .map(|(i, c)| {
                let k = key_chars[i % key_chars.len()] as u32;
                char::from_u32(c as u32 ^ k).unwrap_or(c)
            })
            .collect();

        let json_enc = serde_json::Value::String(enc).to_string();
        let dec = decode_zalo_cipher_payload(&json_enc, key);
        assert_eq!(dec, plain);
    }
}
