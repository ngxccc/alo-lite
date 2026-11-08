use anyhow::Result;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};

pub mod crypto;
pub mod payload;

pub use crypto::*;
pub use payload::*;

use super::models::{MessageStore, StoredMessage, StoredThread};
use super::threads::persist_threads;
use crate::protocol::messaging::Thread;

/// Summary returned after migrating/importing history from an external file or directory (#9).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportReport {
    pub threads_imported: usize,
    pub messages_imported: usize,
    pub sources_read: Vec<String>,
}

/// Import / migrate message history from an existing SQLite database (`.db`/`.sqlite`),
/// a JSON backup (`threads_cache.json` or export JSON), or a directory containing them (#9).
pub fn import_history_from_path(src: &std::path::Path, dst_db_path: &str) -> Result<ImportReport> {
    if !src.exists() {
        anyhow::bail!("Đường dẫn nguồn không tồn tại: {}", src.display());
    }
    let mut dst = MessageStore::open(dst_db_path)?;
    let mut report = ImportReport::default();

    let mut files_to_import = Vec::new();
    if src.is_dir() {
        for candidate in [
            "zalo_history.db",
            "ctdozlite_history.db",
            "threads_cache.json",
            "export.json",
        ] {
            let p = src.join(candidate);
            if p.is_file() {
                files_to_import.push(p);
            }
        }
        if let Ok(entries) = std::fs::read_dir(src) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.is_file() && !files_to_import.contains(&p) {
                    if let Some(ext) = p.extension().and_then(|e| e.to_str()) {
                        if matches!(
                            ext.to_ascii_lowercase().as_str(),
                            "db" | "sqlite" | "sqlite3" | "json"
                        ) {
                            files_to_import.push(p);
                        }
                    }
                }
            }
        }
    } else {
        files_to_import.push(src.to_path_buf());
    }

    if files_to_import.is_empty() {
        anyhow::bail!(
            "Không tìm thấy tệp dữ liệu lịch sử (.db, .sqlite, .json) trong {}",
            src.display()
        );
    }

    for file_path in files_to_import {
        if let Ok(canon_src) = std::fs::canonicalize(&file_path) {
            if let Ok(canon_dst) = std::fs::canonicalize(dst_db_path) {
                if canon_src == canon_dst {
                    continue;
                }
            }
        }
        let bytes = std::fs::read(&file_path)?;
        if bytes.starts_with(b"SQLite format 3") {
            let src_store = MessageStore::open(file_path.to_string_lossy().as_ref())?;
            let src_threads = src_store.all_threads().unwrap_or_default();
            dst.conn.execute_batch("BEGIN")?;
            let step = (|| -> Result<(usize, usize)> {
                let mut t_cnt = 0;
                let mut m_cnt = 0;
                for st in &src_threads {
                    dst.upsert_thread(st)?;
                    t_cnt += 1;
                }
                let mut stmt = src_store.conn.prepare(
                    "SELECT msg_id, thread_id, sender_id, sender_name, text, msg_type, ts, is_outgoing, raw_json
                     FROM messages ORDER BY ts ASC",
                )?;
                let rows = stmt.query_map([], |row| {
                    let is_out: i64 = row.get(7).unwrap_or(0);
                    Ok(StoredMessage {
                        msg_id: row.get(0)?,
                        thread_id: row.get(1)?,
                        sender_id: row.get(2).unwrap_or_default(),
                        sender_name: row.get(3).unwrap_or_default(),
                        text: row.get(4).unwrap_or_default(),
                        msg_type: row.get(5).unwrap_or_default(),
                        ts: row.get(6).unwrap_or(0),
                        is_outgoing: is_out != 0,
                        raw_json: row.get(8).unwrap_or_default(),
                    })
                })?;
                for r in rows {
                    let sm = r?;
                    if dst.upsert_message(&sm)? {
                        m_cnt += 1;
                    }
                }
                Ok((t_cnt, m_cnt))
            })();
            match step {
                Ok((tc, mc)) => {
                    dst.conn.execute_batch("COMMIT")?;
                    report.threads_imported += tc;
                    report.messages_imported += mc;
                    report.sources_read.push(
                        file_path
                            .file_name()
                            .unwrap_or_default()
                            .to_string_lossy()
                            .into(),
                    );
                }
                Err(e) => {
                    let _ = dst.conn.execute_batch("ROLLBACK");
                    return Err(e);
                }
            }
        } else if let Ok(text) = std::str::from_utf8(&bytes) {
            if let Ok(threads) = serde_json::from_str::<Vec<Thread>>(text) {
                let added = persist_threads(&mut dst, &threads)?;
                report.threads_imported += threads.len();
                report.messages_imported += added;
                report.sources_read.push(
                    file_path
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .into(),
                );
            } else if let Ok(val) = serde_json::from_str::<serde_json::Value>(text) {
                if let Some(arr) = val.get("threads").and_then(|v| v.as_array()) {
                    let threads: Vec<Thread> = arr
                        .iter()
                        .filter_map(|item| serde_json::from_value(item.clone()).ok())
                        .collect();
                    if !threads.is_empty() {
                        let added = persist_threads(&mut dst, &threads)?;
                        report.threads_imported += threads.len();
                        report.messages_imported += added;
                        report.sources_read.push(
                            file_path
                                .file_name()
                                .unwrap_or_default()
                                .to_string_lossy()
                                .into(),
                        );
                    }
                }
            }
        } else if bytes.len() >= 4096 {
            if let Some(decrypted_bytes) = try_decrypt_zalo_pc_database(&bytes) {
                let tmp_dir = std::env::temp_dir();
                let tmp_name = format!(
                    "ctdozlite_import_{}_{}.db",
                    std::process::id(),
                    report.sources_read.len()
                );
                let tmp_path = tmp_dir.join(tmp_name);
                if std::fs::write(&tmp_path, &decrypted_bytes).is_ok() {
                    if let Ok(src_conn) = Connection::open(&tmp_path) {
                        let _ = import_official_zalo_pc_db(
                            &src_conn,
                            &mut dst,
                            &file_path,
                            &mut report,
                        );
                    }
                    let _ = std::fs::remove_file(&tmp_path);
                }
            }
        }
    }

    if src.is_dir() {
        let _ = scan_and_import_zalo_data_dir(src, &mut dst, &mut report);
    }

    Ok(report)
}

/// Scan a directory for Zalo PC's `Core/Message` database folder and `Index.db`, decrypting and importing them.
fn scan_and_import_zalo_data_dir(
    root: &std::path::Path,
    dst: &mut MessageStore,
    report: &mut ImportReport,
) -> Result<()> {
    // 1. Recover the 32-char XOR key from Local Storage LevelDB if present
    let mut cipher_key = recover_zalo_localstorage_key(root);
    if cipher_key.is_empty() {
        cipher_key = "de039414865b856ea650d3305b1f387f".to_string();
    }

    // 2. Discover user folders under Database/_production
    let mut message_dirs = Vec::new();
    let mut index_files = Vec::new();

    let search_roots = vec![root.to_path_buf()];

    for s_root in search_roots {
        let prod = s_root.join("Database").join("_production");
        if let Ok(entries) = std::fs::read_dir(&prod) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.is_dir() {
                    let msg_dir = p.join("Core").join("Message");
                    if msg_dir.is_dir() {
                        message_dirs.push(msg_dir);
                    }
                    let idx_file = p.join("Core").join("Index.db");
                    if idx_file.is_file() {
                        index_files.push(idx_file);
                    }
                }
            }
        }
    }

    // 3. Build contact/group name dictionary from Index.db
    let mut id_to_name: std::collections::HashMap<String, (String, bool)> =
        std::collections::HashMap::new();
    for idx_path in index_files {
        if let Ok(bytes) = std::fs::read(&idx_path) {
            if let Some(dec) = decrypt_sqlcipher_bytes(&bytes, cipher_key.as_bytes()) {
                let tmp_p = std::env::temp_dir().join(format!("idx_{}.db", std::process::id()));
                if std::fs::write(&tmp_p, &dec).is_ok() {
                    if let Ok(conn) = Connection::open(&tmp_p) {
                        let _ = query_names_from_index_db(&conn, &mut id_to_name);
                    }
                    let _ = std::fs::remove_file(&tmp_p);
                }
            }
        }
    }

    // 4. Import all message databases in Core/Message
    for msg_dir in message_dirs {
        if let Ok(entries) = std::fs::read_dir(msg_dir) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.extension().and_then(|e| e.to_str()) == Some("db") {
                    let stem = p
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or_default()
                        .to_string();
                    if let Ok(bytes) = std::fs::read(&p) {
                        if let Some(dec) = decrypt_sqlcipher_bytes(&bytes, cipher_key.as_bytes()) {
                            let tmp_p = std::env::temp_dir().join(format!(
                                "msg_{}_{}.db",
                                std::process::id(),
                                stem
                            ));
                            if std::fs::write(&tmp_p, &dec).is_ok() {
                                if let Ok(conn) = Connection::open(&tmp_p) {
                                    let thread_id = stem.clone();
                                    let (t_name, is_group) =
                                        id_to_name.get(&thread_id).cloned().unwrap_or_else(|| {
                                            (
                                                format!("Người dùng {thread_id}"),
                                                thread_id.starts_with('g'),
                                            )
                                        });
                                    let _ = import_message_table(
                                        &conn,
                                        dst,
                                        &thread_id,
                                        &t_name,
                                        is_group,
                                        &cipher_key,
                                        report,
                                    );
                                }
                                let _ = std::fs::remove_file(&tmp_p);
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

/// Attempt to query contact & group display names from a decrypted `Index.db`.
fn query_names_from_index_db(
    conn: &Connection,
    out: &mut std::collections::HashMap<String, (String, bool)>,
) -> Result<()> {
    if let Ok(mut stmt) = conn.prepare("SELECT userId, displayName, zaloName FROM friends_info") {
        let rows = stmt.query_map([], |r| {
            let uid: String = r.get(0)?;
            let dname: String = r.get(1).unwrap_or_default();
            let zname: String = r.get(2).unwrap_or_default();
            let name = if !zname.is_empty() { zname } else { dname };
            Ok((uid, name))
        })?;
        for r in rows.flatten() {
            if !r.0.is_empty() && !r.1.is_empty() {
                out.insert(r.0, (r.1, false));
            }
        }
    }
    if let Ok(mut stmt) = conn.prepare("SELECT userId, displayName, zaloName FROM friend") {
        let rows = stmt.query_map([], |r| {
            let uid: String = r.get(0)?;
            let dname: String = r.get(1).unwrap_or_default();
            let zname: String = r.get(2).unwrap_or_default();
            let name = if !zname.is_empty() { zname } else { dname };
            Ok((uid, name))
        })?;
        for r in rows.flatten() {
            if !r.0.is_empty() && !r.1.is_empty() {
                out.insert(r.0, (r.1, false));
            }
        }
    }
    if let Ok(mut stmt) = conn.prepare("SELECT userId, displayName FROM [group]") {
        let rows = stmt.query_map([], |r| {
            let uid: String = r.get(0)?;
            let dname: String = r.get(1).unwrap_or_default();
            Ok((uid, dname))
        })?;
        for r in rows.flatten() {
            if !r.0.is_empty() && !r.1.is_empty() {
                let mut gid = r.0.clone();
                out.insert(gid.clone(), (r.1.clone(), true));
                if !gid.starts_with('g') {
                    gid.insert(0, 'g');
                    out.insert(gid, (r.1, true));
                }
            }
        }
    }
    Ok(())
}

/// Import a decrypted official Zalo PC `message` table into `dst`.
fn import_message_table(
    src_conn: &Connection,
    dst: &mut MessageStore,
    thread_id: &str,
    thread_name: &str,
    is_group: bool,
    cipher_key: &str,
    report: &mut ImportReport,
) -> Result<()> {
    let mut stmt = match src_conn.prepare(
        "SELECT msgId, cliMsgId, fromUid, toUid, dName, msgType, sendDttm, message, quote
         FROM message ORDER BY CAST(sendDttm AS INTEGER) ASC",
    ) {
        Ok(s) => s,
        Err(_) => return Ok(()),
    };

    let mut msgs = Vec::new();
    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, String>(0).unwrap_or_default(),
            row.get::<_, String>(1).unwrap_or_default(),
            row.get::<_, String>(2).unwrap_or_default(),
            row.get::<_, String>(3).unwrap_or_default(),
            row.get::<_, String>(4).unwrap_or_default(),
            row.get::<_, i64>(5).unwrap_or(1),
            row.get::<_, String>(6).unwrap_or_default(),
            row.get::<_, String>(7).unwrap_or_default(),
            row.get::<_, Option<String>>(8).unwrap_or_default(),
        ))
    })?;

    for r in rows.flatten() {
        let (msg_id, cli_msg_id, from_uid, _to_uid, dname, mtype, send_dttm, raw_msg, quote_str) =
            r;
        let ts: i64 = send_dttm.parse().unwrap_or(0);
        let is_outgoing = from_uid == "0" || from_uid.is_empty();
        let (text, mut meta) = decode_zalo_cipher_payload_with_meta(&raw_msg, cipher_key, mtype);
        if let Some(q) = quote_str {
            if !q.is_empty() {
                meta.insert("quote".into(), serde_json::Value::String(q));
            }
        }
        let msg_type_str = match mtype {
            1 => "chat.text",
            2 => "chat.photo",
            4 => "chat.sticker",
            6 => "chat.link",
            18 => "chat.video",
            19 => "chat.file",
            _ => "chat.text",
        };
        msgs.push(StoredMessage {
            msg_id: if !msg_id.is_empty() {
                msg_id
            } else {
                cli_msg_id
            },
            thread_id: thread_id.to_string(),
            sender_id: from_uid,
            sender_name: if !dname.is_empty() {
                dname
            } else {
                if is_outgoing {
                    "Tôi".into()
                } else {
                    thread_name.into()
                }
            },
            text,
            msg_type: msg_type_str.to_string(),
            ts,
            is_outgoing,
            raw_json: if meta.is_empty() {
                String::new()
            } else {
                serde_json::Value::Object(meta).to_string()
            },
        });
    }

    if msgs.is_empty() {
        return Ok(());
    }

    let last_ts = msgs.iter().map(|m| m.ts).max().unwrap_or(0);
    dst.upsert_thread(&StoredThread {
        thread_id: thread_id.to_string(),
        name: thread_name.to_string(),
        is_group,
        last_ts,
    })?;

    dst.conn.execute_batch("BEGIN")?;
    let mut added = 0;
    for m in &msgs {
        if dst.upsert_message(m)? {
            added += 1;
        }
    }
    dst.conn.execute_batch("COMMIT")?;

    report.threads_imported += 1;
    report.messages_imported += added;
    report
        .sources_read
        .push(format!("ZaloPC: {thread_name} ({thread_id})"));
    Ok(())
}

fn import_official_zalo_pc_db(
    src_conn: &Connection,
    dst: &mut MessageStore,
    file_path: &std::path::Path,
    report: &mut ImportReport,
) -> Result<()> {
    let stem = file_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("import_pc");
    let cipher_key = "de039414865b856ea650d3305b1f387f";
    import_message_table(
        src_conn,
        dst,
        stem,
        stem,
        stem.starts_with('g'),
        cipher_key,
        report,
    )
}
