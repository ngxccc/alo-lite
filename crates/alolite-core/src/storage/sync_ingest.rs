//! Pure SQLite archive ingestion and history persistence for mobile and cloud synchronization.
//!
//! Bridges plain protocol archive data with local SQLite persistence ([`super::MessageStore`])
//! and cache files ([`super::thread_cache`]) without importing [`crate::protocol::session::Session`]
//! or performing any HTTP requests.

use std::collections::HashMap;

use anyhow::Result;
use rusqlite::Connection;

use super::models::{MessageStore, StoredMessage, StoredThread};
use super::thread_cache::{load_threads_cache, save_threads_cache};
use super::threads::persist_threads;
use crate::protocol::messaging::threads::{is_generic_thread_name, Thread};
use crate::protocol::sync::{
    build_synced_message_content, is_excluded_archive_msg_type, parse_bin_net_tlv,
    ParsedProtoPartition,
};

/// Extracted `ChatContent` row from a Sync V1 mobile SQLite conversation archive.
#[derive(Debug, Clone)]
pub struct RawArchiveRow {
    pub sender_id_num: i64,
    pub cli_msg_id: i64,
    pub glb_msg_id: i64,
    pub msg_type_num: i64,
    pub ts: i64,
    pub content_opt: Option<String>,
    pub bin_net_opt: Option<Vec<u8>>,
}

/// Extracted conversation archive (`{fid}.db` or `group_{gid}.db`) awaiting `/api/znoise` mapping.
#[derive(Debug, Clone)]
pub struct ConvArchive {
    pub stem_fid: Option<i64>,
    pub stem_gid: Option<i64>,
    pub rows: Vec<RawArchiveRow>,
}

/// Extract all `ChatContent` rows and plain numeric IDs from unpacked `ZDB4.0` SQLite entries.
///
/// # Errors
/// Returns an error if creating the private temporary scratch directory fails.
pub fn extract_conv_archives_from_entries(
    db_entries: Vec<(String, Vec<u8>)>,
    raw_uid: Option<i64>,
) -> Result<(Vec<ConvArchive>, Vec<i64>, Vec<i64>)> {
    let temp_dir = std::env::temp_dir().join(format!(
        "alolite_sync_{}",
        crate::param_cipher::random_hex(8)
    ));
    std::fs::create_dir_all(&temp_dir)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&temp_dir, std::fs::Permissions::from_mode(0o700));
    }
    let scratch_db = temp_dir.join("conv.sqlite");

    let mut fids = Vec::new();
    let mut gids = Vec::new();
    if let Some(uid) = raw_uid.filter(|&u| u > 0) {
        fids.push(uid);
    }
    let mut archives = Vec::with_capacity(db_entries.len());

    for (name, db_bytes) in db_entries {
        let stem = name.trim_end_matches(".db");
        let (stem_fid, stem_gid) = if let Some(rest) = stem.strip_prefix("group_") {
            let gid = rest.parse::<i64>().ok().filter(|&id| id > 0);
            if let Some(g) = gid {
                gids.push(g);
            }
            (None, gid)
        } else {
            let fid = stem.parse::<i64>().ok().filter(|&id| id > 0);
            if let Some(f) = fid {
                fids.push(f);
            }
            (fid, None)
        };
        if stem_fid.is_none() && stem_gid.is_none() {
            continue;
        }
        if std::fs::write(&scratch_db, &db_bytes).is_err() {
            continue;
        }
        let Ok(conn) = Connection::open(&scratch_db) else {
            continue;
        };
        let mut rows_out = Vec::new();
        if let Ok(mut stmt) = conn.prepare(
            "SELECT SenderId, CliMsgId, GlbMsgId, MsgStatus, MsgType, TimeStamp, MsgContent, BinNet
             FROM ChatContent
             WHERE MsgStatus > 0
             ORDER BY TimeStamp ASC",
        ) {
            if let Ok(mapped) = stmt.query_map([], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, i64>(3)?,
                    row.get::<_, i64>(4)?,
                    row.get::<_, i64>(5)?,
                    row.get::<_, Option<String>>(6)?,
                    row.get::<_, Option<Vec<u8>>>(7)?,
                ))
            }) {
                for r in mapped.flatten() {
                    let (
                        sender_id_num,
                        cli_msg_id,
                        glb_msg_id,
                        status,
                        msg_type_num,
                        ts,
                        content_opt,
                        bin_net_opt,
                    ) = r;
                    if status <= 0 || is_excluded_archive_msg_type(msg_type_num) {
                        continue;
                    }
                    if sender_id_num > 0 {
                        fids.push(sender_id_num);
                    }
                    if let Some(bn) = &bin_net_opt {
                        if let Some(parsed) = parse_bin_net_tlv(bn) {
                            if let Some(oid) =
                                parsed.quote.and_then(|q| q.owner_id).filter(|&id| id > 0)
                            {
                                fids.push(oid);
                            }
                        }
                    }
                    rows_out.push(RawArchiveRow {
                        sender_id_num,
                        cli_msg_id,
                        glb_msg_id,
                        msg_type_num,
                        ts,
                        content_opt,
                        bin_net_opt,
                    });
                }
            }
        }
        drop(conn);
        if !rows_out.is_empty() {
            archives.push(ConvArchive {
                stem_fid,
                stem_gid,
                rows: rows_out,
            });
        }
    }
    let _ = std::fs::remove_dir_all(&temp_dir);
    Ok((archives, fids, gids))
}

/// Commit extracted [`ConvArchive`] rows atomically into `store` and hydrate `cached_threads`.
///
/// # Errors
/// Returns an error if the SQLite transaction fails.
///
/// # Invariants
/// - INV-4: message rows are deduplicated idempotently by `msg_id`.
#[allow(clippy::too_many_arguments)]
pub fn commit_conv_archives_to_store(
    store: &mut MessageStore,
    cached_threads: &mut Vec<Thread>,
    conv_archives: Vec<ConvArchive>,
    fid_map: &HashMap<i64, String>,
    gid_map: &HashMap<i64, String>,
    my_uid: &str,
    my_name: &str,
    raw_uid: Option<i64>,
) -> Result<usize> {
    let resolved_my_raw_uid = raw_uid.filter(|&u| u > 0).or_else(|| {
        (!my_uid.is_empty())
            .then(|| {
                fid_map
                    .iter()
                    .find(|(_, mapped_uid)| mapped_uid.as_str() == my_uid)
                    .map(|(&fid, _)| fid)
            })
            .flatten()
    });

    let mut known_names: HashMap<String, String> = HashMap::new();
    for t in cached_threads.iter() {
        if !t.id.is_empty() && !is_generic_thread_name(&t.name) {
            known_names.insert(t.id.clone(), t.name.clone());
        }
    }

    store.conn.execute_batch("BEGIN")?;
    let tx_res = (|| -> Result<usize> {
        let mut imported = 0usize;
        for conv in conv_archives {
            let is_group = conv.stem_gid.is_some();
            let Some(raw_tid) = (if is_group {
                conv.stem_gid.and_then(|g| gid_map.get(&g)).cloned()
            } else {
                conv.stem_fid.and_then(|f| fid_map.get(&f)).cloned()
            }) else {
                continue;
            };

            let g_prefixed = format!("g{raw_tid}");
            let thread_id = if is_group && cached_threads.iter().any(|t| t.id == g_prefixed) {
                g_prefixed
            } else {
                raw_tid
            };

            let max_ts = conv.rows.iter().map(|r| r.ts).max().unwrap_or(0);
            let thread_name = known_names.get(&thread_id).cloned().unwrap_or_else(|| {
                if is_group {
                    format!("Nhóm {thread_id}")
                } else {
                    format!("Người dùng {thread_id}")
                }
            });
            store.upsert_thread(&StoredThread {
                thread_id: thread_id.clone(),
                name: thread_name,
                is_group,
                last_ts: max_ts,
            })?;

            for r in conv.rows {
                let mapped_sender = fid_map.get(&r.sender_id_num).cloned();
                let is_outgoing = r.sender_id_num == 0
                    || resolved_my_raw_uid == Some(r.sender_id_num)
                    || (!my_uid.is_empty() && mapped_sender.as_deref() == Some(my_uid));

                let sender_id = if is_outgoing {
                    my_uid.to_string()
                } else if let Some(ms) = mapped_sender {
                    ms
                } else if is_group {
                    r.sender_id_num.to_string()
                } else {
                    thread_id.clone()
                };

                let sender_name = if is_outgoing {
                    my_name.to_string()
                } else {
                    known_names.get(&sender_id).cloned().unwrap_or_default()
                };

                let msg_id = if r.glb_msg_id > 0 {
                    r.glb_msg_id.to_string()
                } else {
                    r.cli_msg_id.to_string()
                };
                if msg_id.is_empty() || msg_id == "0" {
                    continue;
                }

                let (text, msg_type, raw_json) = build_synced_message_content(
                    r.msg_type_num,
                    r.content_opt.as_deref(),
                    r.bin_net_opt.as_deref(),
                    fid_map,
                );
                if text.is_empty() && raw_json.is_empty() {
                    continue;
                }

                let sm = StoredMessage {
                    msg_id,
                    thread_id: thread_id.clone(),
                    sender_id,
                    sender_name,
                    text,
                    msg_type,
                    ts: r.ts,
                    is_outgoing,
                    raw_json,
                };
                if store.upsert_message(&sm)? {
                    imported += 1;
                }
            }
        }
        Ok(imported)
    })();

    let imported = match tx_res {
        Ok(n) => {
            store.conn.execute_batch("COMMIT")?;
            n
        }
        Err(e) => {
            let _ = store.conn.execute_batch("ROLLBACK");
            return Err(e);
        }
    };

    let _ = store.hydrate_threads(cached_threads, 50);
    Ok(imported)
}

/// Commit extracted [`ConvArchive`] rows to SQLite and update the threads cache.
///
/// # Errors
/// Returns an error if opening the store, committing archives, or hydrating threads fails.
#[allow(clippy::too_many_arguments)]
pub fn commit_mobile_backup_to_store(
    db_path: &str,
    my_uid: &str,
    my_name: &str,
    conv_archives: Vec<ConvArchive>,
    fid_map: &HashMap<i64, String>,
    gid_map: &HashMap<i64, String>,
    raw_uid: Option<i64>,
) -> Result<(usize, Vec<Thread>)> {
    let mut store = MessageStore::open(db_path)?;
    let mut cached_threads = load_threads_cache();
    let imported = commit_conv_archives_to_store(
        &mut store,
        &mut cached_threads,
        conv_archives,
        fid_map,
        gid_map,
        my_uid,
        my_name,
        raw_uid,
    )?;
    save_threads_cache(&cached_threads);
    Ok((imported, cached_threads))
}

/// Persist resolved thread names into SQLite and save updated threads cache.
///
/// # Errors
/// Returns an error if opening the store fails.
pub fn persist_resolved_thread_names(db_path: &str, threads: &[Thread]) -> Result<()> {
    let mut store = MessageStore::open(db_path)?;
    for t in threads {
        if !t.id.is_empty() && !is_generic_thread_name(&t.name) {
            let last_ts = t.messages.last().map(|m| m.ts).unwrap_or(0);
            let _ = store.upsert_thread(&StoredThread {
                thread_id: t.id.clone(),
                name: t.name.clone(),
                is_group: t.is_group,
                last_ts,
            });
        }
    }
    save_threads_cache(threads);
    Ok(())
}

/// Ingest decoded Sync V2 message partitions into SQLite and update the threads cache.
///
/// # Errors
/// Returns an error if opening SQLite or upserting fails.
pub fn commit_sync_v2_messages_to_store(
    db_path: &str,
    uin: &str,
    my_name: &str,
    partitions: Vec<ParsedProtoPartition>,
) -> Result<usize> {
    let mut store = MessageStore::open(db_path)?;
    let mut imported = 0;
    for part in partitions {
        let thread_id = part.id.clone();
        for m in part.messages {
            let is_outgoing = m.sender_id == uin;
            let sender_name = if is_outgoing {
                my_name.to_string()
            } else {
                String::new()
            };
            let msg_id = if !m.global_id.is_empty() && m.global_id != "0" {
                m.global_id
            } else {
                m.client_id.to_string()
            };

            let sm = StoredMessage {
                msg_id,
                thread_id: thread_id.clone(),
                sender_id: if is_outgoing {
                    uin.to_string()
                } else {
                    thread_id.clone()
                },
                sender_name,
                text: m.content,
                msg_type: "webchat".into(),
                ts: m.timestamp as i64,
                is_outgoing,
                raw_json: String::new(),
            };

            if store.upsert_message(&sm).unwrap_or(false) {
                imported += 1;
            }
        }
    }

    let mut cached_threads = load_threads_cache();
    let _ = store.hydrate_threads(&mut cached_threads, 50);
    save_threads_cache(&cached_threads);

    let _ = std::fs::remove_file("sync_v2_message.bin");
    let _ = std::fs::remove_file("sync_v2_session.bin");
    let _ = std::fs::remove_file("sync_v2_keys.json");

    Ok(imported)
}

/// Query the oldest message ID for a thread from the store if it exists.
pub fn oldest_message_id(db_path: &str, thread_id: &str) -> Option<String> {
    MessageStore::open(db_path).ok()?.oldest_msg_id(thread_id)
}

/// Persist a single thread snapshot into SQLite store.
///
/// # Errors
/// Returns an error if opening store or writing fails.
pub fn persist_thread_snapshot(db_path: &str, thread: &Thread) -> Result<usize> {
    let mut store = MessageStore::open(db_path)?;
    persist_threads(&mut store, std::slice::from_ref(thread))
}

/// Persist synchronized phone threads into SQLite and write `threads_cache.json`.
///
/// # Errors
/// Returns an error if SQLite writes fail.
pub fn commit_phone_sync_to_store(db_path: &str, threads: &[Thread]) -> Result<usize> {
    let mut st = MessageStore::open(db_path)?;
    let n = persist_threads(&mut st, threads)?;
    save_threads_cache(threads);
    Ok(n)
}
