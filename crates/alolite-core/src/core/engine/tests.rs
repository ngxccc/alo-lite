use super::*;
use crate::protocol::session::Session;
use crate::storage::{MessageStore, StoredMessage};

#[tokio::test]
async fn core_engine_runs_ui_free_and_queries_sqlite_store() {
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let db_file = std::env::temp_dir().join(format!("alolite-core-test-{nonce}.db"));
    let db_path = db_file.to_string_lossy().into_owned();

    {
        let mut store = MessageStore::open(&db_path).unwrap();
        store
            .upsert_thread(&crate::storage::StoredThread {
                thread_id: "t_core".into(),
                name: "Bob".into(),
                is_group: false,
                last_ts: 123_456,
            })
            .unwrap();
        store
            .upsert_message(&StoredMessage {
                msg_id: "m_core_1".into(),
                thread_id: "t_core".into(),
                sender_id: "u1".into(),
                sender_name: "Bob".into(),
                text: "CoreEngine runs without egui".into(),
                msg_type: "chat.text".into(),
                ts: 123_456,
                is_outgoing: false,
                raw_json: String::new(),
            })
            .unwrap();
    }

    let session = Session::new(false, false, false).unwrap();
    let engine = CoreEngine::headless(session, &db_path);

    let st = engine.status().await;
    assert!(!st.logged_in);
    assert_eq!(st.threads_in_db, 1);

    let hits = engine.search_messages("CoreEngine", 10).await.unwrap();
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].msg_id, "m_core_1");

    let msgs = engine.get_messages("t_core", 10).await.unwrap();
    assert_eq!(msgs.len(), 1);

    let _ = std::fs::remove_file(&db_file);
}
