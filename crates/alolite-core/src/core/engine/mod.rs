//! Core application engine owning the single live [`Session`], WebSocket/poll loops,
//! SQLite persistence, and image/sticker decoding independent of any GUI framework.

use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::sync::atomic::{AtomicU32, AtomicU64};
use std::sync::{Arc, Mutex as StdMutex};

use serde::Serialize;
use tokio::sync::Mutex;

use super::events::{CoreEvent, CoreEventSink, NoopEventSink};
use crate::protocol::messaging::polling::PollState;
use crate::protocol::messaging::Thread;
use crate::protocol::session::Session;
use crate::storage::{db_path_for_account, MessageStore};

mod auth;
mod background;
mod maintenance;
mod media;
mod messaging;
mod social;
mod threads;

#[cfg(test)]
mod tests;

/// Summary returned by [`CoreEngine::status`].
#[derive(Debug, Clone, Serialize)]
pub struct CoreStatus {
    pub app: &'static str,
    pub version: &'static str,
    pub logged_in: bool,
    pub uin: String,
    pub display_name: String,
    pub threads_in_db: usize,
}

/// UI-independent runtime owning the active Zalo session, polling/WebSocket sync, and SQLite access.
pub struct CoreEngine {
    pub(super) session: Arc<Mutex<Session>>,
    pub(super) poll_state: Arc<Mutex<PollState>>,
    pub(super) known_threads: Arc<StdMutex<Vec<(String, String, bool)>>>,
    pub(super) explicit_db_path: Option<String>,
    pub(super) sink: Arc<dyn CoreEventSink>,
    pub(super) is_polling: Arc<AtomicBool>,
    pub(super) poll_failures: Arc<AtomicU32>,
    /// Epoch seconds before which background polling stays paused after repeated failures.
    pub(super) poll_paused_until: Arc<AtomicU64>,
    /// Set once the server accepted the current credentials; background polling stays idle
    /// until then so a restored-but-dead `zpw_sek` never produces long-poll traffic.
    pub(super) session_verified: Arc<AtomicBool>,
    pub(super) current_qr_task: Mutex<Option<tokio::task::JoinHandle<()>>>,
}

impl CoreEngine {
    /// Create a UI-agnostic engine resolving per-account SQLite paths dynamically (`accounts/<uin>_history.db`).
    pub fn new(session: Session, sink: Arc<dyn CoreEventSink>) -> Arc<Self> {
        Arc::new(Self {
            session: Arc::new(Mutex::new(session)),
            poll_state: Arc::new(Mutex::new(PollState::default())),
            known_threads: Arc::new(StdMutex::new(Vec::new())),
            explicit_db_path: None,
            sink,
            is_polling: Arc::new(AtomicBool::new(false)),
            poll_failures: Arc::new(AtomicU32::new(0)),
            poll_paused_until: Arc::new(AtomicU64::new(0)),
            session_verified: Arc::new(AtomicBool::new(false)),
            current_qr_task: Mutex::new(None),
        })
    }

    /// Create an engine pinned to an explicit SQLite file path (used by `--mcp` and tests).
    pub fn with_db_path(
        session: Session,
        db_path: impl Into<String>,
        sink: Arc<dyn CoreEventSink>,
    ) -> Arc<Self> {
        Arc::new(Self {
            session: Arc::new(Mutex::new(session)),
            poll_state: Arc::new(Mutex::new(PollState::default())),
            known_threads: Arc::new(StdMutex::new(Vec::new())),
            explicit_db_path: Some(db_path.into()),
            sink,
            is_polling: Arc::new(AtomicBool::new(false)),
            poll_failures: Arc::new(AtomicU32::new(0)),
            poll_paused_until: Arc::new(AtomicU64::new(0)),
            session_verified: Arc::new(AtomicBool::new(false)),
            current_qr_task: Mutex::new(None),
        })
    }

    /// Create an engine for headless CLI/MCP usage with a no-op UI sink.
    pub fn headless(session: Session, db_path: impl Into<String>) -> Arc<Self> {
        Self::with_db_path(session, db_path, Arc::new(NoopEventSink))
    }

    /// Access the shared [`Session`] mutex.
    pub fn session_arc(&self) -> &Arc<Mutex<Session>> {
        &self.session
    }

    /// Emit an event directly to the attached UI sink.
    pub fn emit(&self, event: CoreEvent) {
        self.sink.emit(event);
    }

    /// Whether the server accepted the current credentials during this run.
    pub fn is_session_verified(&self) -> bool {
        self.session_verified.load(Ordering::Relaxed)
    }

    pub(super) fn set_session_verified(&self, verified: bool) {
        self.session_verified.store(verified, Ordering::Relaxed);
    }

    /// Resolve the active SQLite path (explicit override or `accounts/<uin>_history.db`).
    pub async fn active_db_path(&self) -> String {
        if let Some(p) = &self.explicit_db_path {
            return p.clone();
        }
        let s = self.session.lock().await;
        let uin = s.login.as_ref().map(|l| l.uin.as_str()).unwrap_or_default();
        db_path_for_account(uin)
    }

    /// Synchronous DB path lookup for places using an already-captured `uin`.
    pub fn resolve_db_path_for_uin(&self, uin: &str) -> String {
        if let Some(p) = &self.explicit_db_path {
            p.clone()
        } else {
            db_path_for_account(uin)
        }
    }

    /// Publish the current known `(id, name, is_group)` threads to background polling.
    pub fn set_known_threads(&self, threads: Vec<(String, String, bool)>) {
        if let Ok(mut g) = self.known_threads.lock() {
            *g = threads;
        }
    }

    pub(super) fn known_threads_snapshot(&self) -> Vec<(String, String, bool)> {
        self.known_threads
            .lock()
            .map(|g| g.clone())
            .unwrap_or_default()
    }

    /// Return current session status and stored thread count.
    pub async fn status(&self) -> CoreStatus {
        let (logged_in, uin, display_name) = {
            let s = self.session.lock().await;
            let logged = s.has_live_credentials() && !s.is_expired_marked();
            let uin = s.login.as_ref().map(|l| l.uin.clone()).unwrap_or_default();
            let dname = s
                .login
                .as_ref()
                .map(|l| l.display_name.clone())
                .unwrap_or_default();
            (logged, uin, dname)
        };
        let db_path = self.resolve_db_path_for_uin(&uin);
        let thread_count = tokio::task::spawn_blocking(move || {
            MessageStore::open(&db_path)
                .and_then(|st| st.all_threads())
                .map(|v| v.len())
                .unwrap_or(0)
        })
        .await
        .unwrap_or(0);

        CoreStatus {
            app: "alolite",
            version: env!("CARGO_PKG_VERSION"),
            logged_in,
            uin,
            display_name,
            threads_in_db: thread_count,
        }
    }
}

pub(super) fn merge_thread_headers(
    list: &mut Vec<Thread>,
    incoming: Vec<Thread>,
    force_group: bool,
) {
    for item in incoming {
        if let Some(ex) = list.iter_mut().find(|t| t.id == item.id) {
            if !item.name.is_empty() {
                ex.name = item.name;
            }
            if crate::protocol::messaging::threads::should_replace_avatar(
                ex.avatar.as_deref(),
                item.avatar.as_deref(),
            ) {
                ex.avatar = item.avatar;
            }
            ex.is_group = if force_group {
                true
            } else {
                ex.is_group || item.is_group
            };
        } else {
            list.push(item);
        }
    }
}
