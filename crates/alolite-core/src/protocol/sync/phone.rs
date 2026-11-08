//! Pure network fetch and thread merging for phone message synchronization (#68).
//!
//! Orchestrates phone prompts, long-polling batch queue draining, conversation preview fetching,
//! and cloud message backfill entirely in memory without touching SQLite.

use std::sync::atomic::Ordering;
use std::time::Duration;

use anyhow::Result;

use super::controls::{cancel_phone_sync, clean_sync, request_phone_sync};
use super::history::PAGE;
use super::zdb4::generate_sync_public_key;
use super::{last_mobile_sync_status, next_cursor, set_last_mobile_sync_status, PHONE_SYNC_CANCEL};
use crate::protocol::messaging::polling::{
    get_chat_from_polling, get_cloud_messages, get_last_msgs, PollState,
};
use crate::protocol::messaging::threads::{deduplicate_threads, Thread};
use crate::protocol::session::Session;
use crate::protocol::social::groups::resolve_thread_names;
use crate::protocol::wire::require_login;

/// Outcome of the pure-network phone synchronization phase.
#[derive(Debug, Clone)]
pub struct PhoneFetchOutcome {
    pub before_total: usize,
    pub cancelled: bool,
}

/// Synchronize messages from user's phone and cloud into in-memory `threads` without database access.
///
/// # Errors
/// Returns an error if unauthenticated or network requests fail.
pub async fn fetch_phone_messages<F>(
    s: &Session,
    poll_state: &tokio::sync::Mutex<PollState>,
    threads: &mut Vec<Thread>,
    send_phone_prompt: bool,
    mut on_progress: F,
) -> Result<PhoneFetchOutcome>
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
                return Ok(PhoneFetchOutcome {
                    before_total,
                    cancelled: true,
                });
            }
            if let Some(st) = &last_mobile_sync_status() {
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
                    return Ok(PhoneFetchOutcome {
                        before_total,
                        cancelled: true,
                    });
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
                    return Ok(PhoneFetchOutcome {
                        before_total,
                        cancelled: true,
                    });
                }
                if let Some(st) = &last_mobile_sync_status() {
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
            return Ok(PhoneFetchOutcome {
                before_total,
                cancelled: true,
            });
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

    Ok(PhoneFetchOutcome {
        before_total,
        cancelled: false,
    })
}

/// Format user-facing Vietnamese summary message after phone sync persistence finishes.
pub fn format_phone_sync_final_message(added: usize, active_threads: usize) -> String {
    if let Some(phone_note) = last_mobile_sync_status() {
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
    }
}
