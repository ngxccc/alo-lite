//! Pure protocol cloud thread history fetching.
//!
//! Submodule fetches cloud message pages for individual threads via
//! [`crate::protocol::messaging::polling::get_cloud_messages`] and yields each page to a
//! caller-supplied persistence seam without leaking `rusqlite` or `MessageStore`.

use std::future::Future;
use std::sync::atomic::Ordering;
use std::time::Duration;

use anyhow::Result;

use super::{next_cursor, CANCEL};
use crate::protocol::messaging::polling::get_cloud_messages;
use crate::protocol::messaging::threads::Thread;
use crate::protocol::session::Session;

pub const PAGE: u32 = 50;
pub const MAX_PAGES: usize = 20;
pub const PAUSE: Duration = Duration::from_millis(400);

/// Fetch older pages for `t` until the server has nothing older, `MAX_PAGES` is hit, or `CANCEL` is set.
///
/// Calls `on_page` after each page is merged into `t` to allow caller composition (e.g. SQLite persistence)
/// without buffering all pages or leaking database dependencies into `protocol/sync`.
///
/// # Errors
/// Returns an error if cloud history fetching or the `on_page` callback fails.
pub async fn fetch_thread_history<F, Fut>(
    s: &Session,
    t: &mut Thread,
    initial_cursor: &str,
    mut on_page: F,
) -> Result<usize>
where
    F: FnMut(&Thread) -> Fut,
    Fut: Future<Output = Result<usize>>,
{
    let mut cursor = initial_cursor.to_string();
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
        added += on_page(t).await?;
        match next_cursor(t, &cursor) {
            Some(c) => cursor = c,
            None => break,
        }
        tokio::time::sleep(PAUSE).await;
    }
    Ok(added)
}
