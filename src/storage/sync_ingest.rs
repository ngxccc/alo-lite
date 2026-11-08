//! Store-backed history synchronization and mobile backup SQLite archive ingestion.
//!
//! Bridges pure protocol containers ([`crate::protocol::sync`]) with local SQLite persistence
//! ([`super::MessageStore`]) without leaking `rusqlite` into `src/protocol/`.

use std::collections::HashMap;
use std::sync::atomic::Ordering;
use std::time::Duration;

use anyhow::Result;
use rusqlite::Connection;

use super::models::{db_path_for_account, MessageStore, StoredMessage, StoredThread};
use super::thread_cache::{load_threads_cache, save_threads_cache};
use super::threads::persist_threads;
use crate::protocol::messaging::polling::{
    get_chat_from_polling, get_cloud_messages, get_last_msgs, PollState,
};
use crate::protocol::messaging::threads::{deduplicate_threads, is_generic_thread_name, Thread};
use crate::protocol::session::{Session, ZALO_UA};
use crate::protocol::social::groups::resolve_thread_names;
use crate::protocol::sync::{
    build_synced_message_content, cancel_phone_sync, clean_sync, clear_sync_keys,
    decrypt_and_unpack_zdb4_container, generate_sync_public_key, get_sync_keys,
    get_sync_public_key, is_excluded_archive_msg_type, last_mobile_sync_status, next_cursor,
    parse_bin_net_tlv, parse_sync_chunk, request_phone_sync, resolve_znoise_ids,
    set_last_mobile_sync_status, CANCEL, PHONE_SYNC_CANCEL,
};
use crate::protocol::wire::require_login;

const PAGE: u32 = 50;
const MAX_PAGES: usize = 20;
const PAUSE: Duration = Duration::from_millis(400);

/// Fetch older pages for `t` until the server has nothing older, `MAX_PAGES` is hit, or `CANCEL` is set.
///
/// # Errors
/// Returns an error if cloud history fetching or SQLite persistence fails.
///
/// # Invariants
/// - INV-3: SQLite queries and upserts execute inside `spawn_blocking`.
/// - INV-4: `persist_threads` deduplicates rows idempotently by `msg_id`.
pub async fn sync_thread(s: &Session, t: &mut Thread) -> Result<usize> {
    let id = t.id.clone();
    let uin = s.login.as_ref().map(|l| l.uin.clone()).unwrap_or_default();
    let db_path = db_path_for_account(&uin);
    let db_path_init = db_path.clone();
    let mut cursor = tokio::task::spawn_blocking(move || {
        MessageStore::open(&db_path_init).ok()?.oldest_msg_id(&id)
    })
    .await?
    .unwrap_or_default();
    let mut added = 0;
    for _ in 0..MAX_PAGES {
        if CANCEL.load(Ordering::Relaxed) {
            break;
        }
        let page = get_cloud_messages(s, &t.id, &cursor, true, PAGE).await?;
        let before = t.messages.len();
        page.merge_into_thread(t);
        if t.messages.len() == before {
            break;
        }
        let snapshot = t.clone();
        let db_path_page = db_path.clone();
        added += tokio::task::spawn_blocking(move || {
            persist_threads(
                &mut MessageStore::open(&db_path_page)?,
                std::slice::from_ref(&snapshot),
            )
        })
        .await??;
        match next_cursor(t, &cursor) {
            Some(c) => cursor = c,
            None => break,
        }
        tokio::time::sleep(PAUSE).await;
    }
    Ok(added)
}

/// Synchronize 1-1 and group messages from the user's phone and Zalo cloud (`#68`).
///
/// # Errors
/// Returns an error if unauthenticated or SQLite persistence fails.
pub async fn sync_phone_messages<F>(
    s: &Session,
    poll_state: &tokio::sync::Mutex<PollState>,
    threads: &mut Vec<Thread>,
    on_progress: F,
) -> Result<(usize, usize)>
where
    F: FnMut(f32, String),
{
    sync_phone_messages_opts(s, poll_state, threads, true, on_progress).await
}

/// Synchronize messages with explicit control over whether to send a wake-up/pull prompt (`send_phone_prompt`).
///
/// # Errors
/// Returns an error if unauthenticated or SQLite persistence fails.
pub async fn sync_phone_messages_opts<F>(
    s: &Session,
    poll_state: &tokio::sync::Mutex<PollState>,
    threads: &mut Vec<Thread>,
    send_phone_prompt: bool,
    mut on_progress: F,
) -> Result<(usize, usize)>
where
    F: FnMut(f32, String),
{
    require_login(s)?;
    let pub_key = generate_sync_public_key();
    let before_total: usize = threads.iter().map(|t| t.messages.len()).sum();

    if send_phone_prompt {
        set_last_mobile_sync_status(None);
        let _ = clean_sync(s, &pub_key).await;
        on_progress(
            0.10,
            "Đang gửi yêu cầu đồng bộ tới Zalo trên điện thoại (hãy mở Zalo trên điện thoại và bấm 'Đồng bộ ngay')…".to_string(),
        );
        let _ = request_phone_sync(s, &pub_key).await;
        let deadline = std::time::Instant::now() + Duration::from_secs(90);
        let mut confirmed = false;
        while std::time::Instant::now() < deadline {
            if PHONE_SYNC_CANCEL.load(Ordering::Relaxed) {
                let _ = cancel_phone_sync(s, &pub_key).await;
                return Ok((0, 0));
            }
            if let Some(ref st) = last_mobile_sync_status() {
                if st.contains("đã xác nhận")
                    || st.contains("đang nén")
                    || st.contains("giải mã")
                    || st.contains("Đã nhận gói đồng bộ")
                    || st.contains("thành công")
                {
                    confirmed = true;
                    on_progress(0.40, st.clone());
                    break;
                } else if st.contains("từ chối") {
                    on_progress(1.0, st.clone());
                    return Ok((0, 0));
                } else {
                    on_progress(0.20, st.clone());
                }
            }
            tokio::time::sleep(Duration::from_millis(500)).await;
        }

        if confirmed {
            let finish_deadline = std::time::Instant::now() + Duration::from_secs(180);
            while std::time::Instant::now() < finish_deadline {
                if PHONE_SYNC_CANCEL.load(Ordering::Relaxed) {
                    let _ = cancel_phone_sync(s, &pub_key).await;
                    return Ok((0, 0));
                }
                if let Some(ref st) = last_mobile_sync_status() {
                    if st.contains("thành công")
                        || st.contains("Không thể")
                        || st.contains("thất bại")
                    {
                        on_progress(1.0, st.clone());
                        break;
                    } else {
                        on_progress(0.70, st.clone());
                    }
                }
                tokio::time::sleep(Duration::from_millis(500)).await;
            }
        }
    }

    on_progress(
        0.22,
        "Đang nhận hàng đợi tin nhắn 1-1 & nhóm từ điện thoại…".to_string(),
    );
    for batch_idx in 0..5 {
        if PHONE_SYNC_CANCEL.load(Ordering::Relaxed) {
            let _ = cancel_phone_sync(s, &pub_key).await;
            return Ok((0, 0));
        }
        let lp_res = {
            let mut ps = poll_state.lock().await;
            get_chat_from_polling(s, &mut ps).await
        };
        let Ok(batch) = lp_res else {
            break;
        };
        batch.merge_into(threads);
        batch.merge_read_points_into(threads);
        on_progress(
            0.22 + (batch_idx as f32 + 1.0) * 0.04,
            format!("Đã nhận lô hàng đợi #{} từ điện thoại…", batch_idx + 1),
        );
        if !batch.more && !batch.has_reset {
            break;
        }
    }

    on_progress(
        0.45,
        "Đang đồng bộ tin nhắn các cuộc trò chuyện 1-1 & nhóm…".to_string(),
    );
    if let Ok(preview_batch) = get_last_msgs(s, &[]).await {
        preview_batch.merge_into(threads);
    }
    let known_ids: Vec<String> = threads
        .iter()
        .filter(|t| !t.id.is_empty())
        .map(|t| {
            if t.is_group && !t.id.starts_with('g') && !t.id.starts_with("group_") {
                format!("g{}", t.id)
            } else {
                t.id.clone()
            }
        })
        .collect();
    let chunks: Vec<&[String]> = known_ids.chunks(25).collect();
    let total_chunks = chunks.len().max(1);
    for (idx, chunk) in chunks.into_iter().enumerate() {
        if PHONE_SYNC_CANCEL.load(Ordering::Relaxed) {
            let _ = cancel_phone_sync(s, &pub_key).await;
            break;
        }
        if let Ok(preview_batch) = get_last_msgs(s, chunk).await {
            preview_batch.merge_into(threads);
        }
        let frac = 0.45 + 0.25 * ((idx + 1) as f32 / total_chunks as f32);
        on_progress(
            frac,
            format!(
                "Đang đồng bộ hội thoại 1-1 & nhóm ({}/{total_chunks} nhóm lô)…",
                idx + 1
            ),
        );
        tokio::time::sleep(Duration::from_millis(120)).await;
    }

    let send2me_id = s
        .login
        .as_ref()
        .map(|l| l.send2me_id.clone())
        .unwrap_or_default();
    let cm_targets: Vec<usize> = threads
        .iter()
        .enumerate()
        .filter(|(_, t)| {
            !t.id.is_empty()
                && (t.is_group
                    || t.name.contains("Cloud của tôi")
                    || (!send2me_id.is_empty() && t.id == send2me_id))
        })
        .map(|(i, _)| i)
        .collect();
    let total_cm = cm_targets.len().max(1);
    for (pos, tidx) in cm_targets.into_iter().enumerate() {
        if PHONE_SYNC_CANCEL.load(Ordering::Relaxed) {
            let _ = cancel_phone_sync(s, &pub_key).await;
            break;
        }
        let (tid, tname) = {
            let t = &threads[tidx];
            (t.id.clone(), t.name.clone())
        };
        let frac = 0.70 + 0.25 * ((pos + 1) as f32 / total_cm as f32);
        on_progress(
            frac,
            format!("Đang đồng bộ lịch sử: {tname} ({}/{total_cm})…", pos + 1),
        );
        if let Ok(cm_page) = get_cloud_messages(s, &tid, "0", false, PAGE).await {
            cm_page.merge_into_thread(&mut threads[tidx]);
        }
        if let Some(oldest) = next_cursor(&threads[tidx], "") {
            if let Ok(old_page) = get_cloud_messages(s, &tid, &oldest, true, PAGE).await {
                old_page.merge_into_thread(&mut threads[tidx]);
            }
        }
        tokio::time::sleep(Duration::from_millis(150)).await;
    }

    on_progress(
        0.96,
        "Đang lưu tin nhắn đã đồng bộ vào cơ sở dữ liệu lịch sử…".to_string(),
    );
    resolve_thread_names(s, threads).await;
    deduplicate_threads(threads, None);

    let snapshot = threads.clone();
    let uin_str = s.login.as_ref().map(|l| l.uin.clone()).unwrap_or_default();
    let db_path = db_path_for_account(&uin_str);
    let db_added = tokio::task::spawn_blocking(move || -> Result<usize> {
        let mut st = MessageStore::open(&db_path)?;
        let n = persist_threads(&mut st, &snapshot)?;
        save_threads_cache(&snapshot);
        Ok(n)
    })
    .await??;
    let after_total: usize = threads.iter().map(|t| t.messages.len()).sum();
    let added = db_added.max(after_total.saturating_sub(before_total));
    let active_threads = threads.iter().filter(|t| !t.messages.is_empty()).count();
    let final_msg = if let Some(phone_note) = last_mobile_sync_status() {
        if phone_note.contains("thành công") {
            phone_note
        } else if added > 0 {
            format!(
                "{phone_note} · Đã nạp +{added} tin nhắn Cloud/Nhóm ({active_threads} hội thoại)"
            )
        } else {
            "Điện thoại chưa gửi tin nhắn 1-1. Bạn hãy mở Zalo trên điện thoại bấm 'Đồng bộ ngay', hoặc dùng tính năng 'Nhập file sao lưu' bên dưới.".to_string()
        }
    } else if added > 0 {
        format!("Đã nạp +{added} tin nhắn Cloud/Nhóm ({active_threads} hội thoại)")
    } else {
        "Chưa có tin nhắn mới nào từ điện thoại. Hãy mở Zalo trên điện thoại bấm 'Đồng bộ ngay' hoặc nhập từ file sao lưu (.db / .json).".to_string()
    };
    on_progress(1.0, final_msg);
    Ok((added, active_threads))
}

/// Extracted `ChatContent` row from a Sync V1 mobile SQLite conversation archive.
#[derive(Debug)]
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
#[derive(Debug)]
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

/// Download, decrypt, decompress and ingest a Sync V1 mobile backup container (`.db.crypt`).
///
/// # Errors
/// Returns an error if the RSA sync key is missing, download/decryption fails, or SQLite fails.
pub async fn import_mobile_backup_container(
    s: &Session,
    url: &str,
    encrypted_key: &str,
) -> Result<usize> {
    import_mobile_backup_container_with_uid(s, url, encrypted_key, None).await
}

/// Download, decrypt, decompress and ingest a Sync V1 mobile backup container (`.db.crypt`)
/// with an optional plain numeric `raw_uid` (`data.uid` from `syncmsg_info`).
///
/// # Errors
/// Returns an error if the RSA sync key is missing, download/decryption fails, or SQLite fails.
///
/// # Invariants
/// - INV-3: AES/LZMA unpacking and SQLite reads/writes execute on `spawn_blocking`.
/// - INV-4: message rows are deduplicated idempotently by `msg_id`.
pub async fn import_mobile_backup_container_with_uid(
    s: &Session,
    url: &str,
    encrypted_key: &str,
    raw_uid: Option<i64>,
) -> Result<usize> {
    use base64::Engine;
    use rsa::Pkcs1v15Encrypt;

    if url.is_empty() {
        anyhow::bail!("empty backup url");
    }
    if encrypted_key.is_empty() {
        anyhow::bail!("encrypted_key is empty");
    }

    let (priv_key, pub_key_b64) = get_sync_keys()
        .ok_or_else(|| anyhow::anyhow!("Không tìm thấy khoá RSA phiên đồng bộ hiện tại"))?;

    let my_uid = s.login.as_ref().map(|l| l.uin.clone()).unwrap_or_default();
    let my_name = s
        .login
        .as_ref()
        .map(|l| l.display_name.clone())
        .unwrap_or_else(|| "Tôi".into());
    let sek = s.zpw_sek.clone().unwrap_or_default();
    let cookie_hdr = format!("zpw_sek={sek}; zpw_uin={my_uid}");

    let resp = s
        .http
        .get(url)
        .header("Cookie", cookie_hdr)
        .header("User-Agent", ZALO_UA)
        .send()
        .await?;

    if !resp.status().is_success() {
        anyhow::bail!("failed to download backup: HTTP {}", resp.status());
    }
    let crypt_data = resp.bytes().await?.to_vec();
    let enc_key_owned = encrypted_key.to_string();

    let (conv_archives, all_fids, all_gids) = tokio::task::spawn_blocking(move || -> Result<_> {
        let enc_bytes = base64::engine::general_purpose::STANDARD.decode(enc_key_owned.trim())?;
        let raw_key = priv_key.decrypt(Pkcs1v15Encrypt, &enc_bytes)?;
        let db_entries = decrypt_and_unpack_zdb4_container(&crypt_data, &raw_key)?;
        extract_conv_archives_from_entries(db_entries, raw_uid)
    })
    .await??;

    let (fid_map, gid_map) = resolve_znoise_ids(s, &all_fids, &all_gids)
        .await
        .unwrap_or_default();

    let db_path = db_path_for_account(&my_uid);
    let my_uid_clone = my_uid.clone();
    let my_name_clone = my_name.clone();
    let (total_imported, mut hydrated_threads) =
        tokio::task::spawn_blocking(move || -> Result<(usize, Vec<Thread>)> {
            let mut store = MessageStore::open(&db_path)?;
            let mut cached_threads = load_threads_cache();
            let imported = commit_conv_archives_to_store(
                &mut store,
                &mut cached_threads,
                conv_archives,
                &fid_map,
                &gid_map,
                &my_uid_clone,
                &my_name_clone,
                raw_uid,
            )?;
            save_threads_cache(&cached_threads);
            Ok((imported, cached_threads))
        })
        .await??;

    resolve_thread_names(s, &mut hydrated_threads).await;
    let db_path_names = db_path_for_account(&my_uid);
    let resolved_snapshot = hydrated_threads;
    let _ = tokio::task::spawn_blocking(move || -> Result<()> {
        let mut store = MessageStore::open(&db_path_names)?;
        for t in &resolved_snapshot {
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
        save_threads_cache(&resolved_snapshot);
        Ok(())
    })
    .await;

    let _ = clean_sync(s, &pub_key_b64).await;
    clear_sync_keys();

    Ok(total_imported)
}

/// Download and acknowledge a Sync V2 metadata batch.
///
/// # Errors
/// Returns an error if the HTTP download fails.
pub async fn import_sync_v2_meta(s: &Session, url: &str) -> Result<()> {
    let uin = s.login.as_ref().map(|l| l.uin.clone()).unwrap_or_default();
    let sek = s.zpw_sek.clone().unwrap_or_default();
    let cookie_hdr = format!("zpw_sek={sek}; zpw_uin={uin}");

    let resp = s
        .http
        .get(url)
        .header("Cookie", cookie_hdr)
        .header("User-Agent", ZALO_UA)
        .send()
        .await?;

    if !resp.status().is_success() {
        anyhow::bail!("failed to download v2 meta: HTTP {}", resp.status());
    }
    let _bytes = resp.bytes().await?;
    Ok(())
}

/// Download, decompress, and ingest an unencrypted Zstd Sync V2 message batch into SQLite.
///
/// # Errors
/// Returns an error if the chunk is encrypted (`zprotoSync2`), malformed, or SQLite fails.
pub async fn import_sync_v2_messages(s: &Session, url: &str) -> Result<usize> {
    let uin = s.login.as_ref().map(|l| l.uin.clone()).unwrap_or_default();
    let sek = s.zpw_sek.clone().unwrap_or_default();
    let cookie_hdr = format!("zpw_sek={sek}; zpw_uin={uin}");

    let resp = s
        .http
        .get(url)
        .header("Cookie", cookie_hdr)
        .header("User-Agent", ZALO_UA)
        .send()
        .await?;

    if !resp.status().is_success() {
        anyhow::bail!("failed to download v2 messages: HTTP {}", resp.status());
    }

    let bytes = resp.bytes().await?;
    let decompressed = zstd::decode_all(std::io::Cursor::new(&bytes)).map_err(|_| {
        anyhow::anyhow!(
            "Gói Sync v2 đang mã hoá zprotoSync2 (hãy dùng luồng đồng bộ tiêu chuẩn pull_mobile_msg)"
        )
    })?;
    let partitions = parse_sync_chunk(&decompressed)
        .ok_or_else(|| anyhow::anyhow!("Không giải mã được cấu trúc SyncChunk của gói Sync v2"))?;
    let db_path = db_path_for_account(&uin);
    let mut store = MessageStore::open(&db_path)?;
    let my_name = s
        .login
        .as_ref()
        .map(|l| l.display_name.clone())
        .unwrap_or_else(|| "Tôi".into());

    let mut imported = 0;
    for part in partitions {
        let thread_id = part.id.clone();
        for m in part.messages {
            let is_outgoing = m.sender_id == uin;
            let sender_name = if is_outgoing {
                my_name.clone()
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
                    uin.clone()
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

    if let Some(pub_k) = get_sync_public_key() {
        let _ = clean_sync(s, &pub_k).await;
    }

    let _ = std::fs::remove_file("sync_v2_message.bin");
    let _ = std::fs::remove_file("sync_v2_session.bin");
    let _ = std::fs::remove_file("sync_v2_keys.json");

    Ok(imported)
}
