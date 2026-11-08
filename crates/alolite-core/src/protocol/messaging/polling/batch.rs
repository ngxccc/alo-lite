//! Batch structures returned by polling and history endpoints.

use serde_json::Value;

use super::merge::{merge_group_history, merge_previews};
use crate::protocol::messaging::threads::Thread;
use crate::protocol::wire::pick_str;

/// Typed batch returned by `get_chat_from_polling`.
#[derive(Debug, Clone)]
pub struct PollBatch {
    pub more: bool,
    pub has_reset: bool,
    pub sync_controls: Vec<crate::protocol::sync::MobileSyncControlEvent>,
    pub mute_events: Vec<crate::protocol::messaging::unread::MuteControlEvent>,
    raw: Value,
}

impl PollBatch {
    pub fn from_wire(raw: Value, now_unix: i64) -> Self {
        let data_node = raw.get("data").unwrap_or(&raw);
        let more = data_node.get("more").and_then(Value::as_i64).unwrap_or(0) != 0;
        let has_reset =
            pick_str(data_node, &["resetLastActionId"]).is_some_and(|r| r != "0" && !r.is_empty());
        let sync_controls = crate::protocol::sync::extract_mobile_sync_controls(data_node);
        let mute_events =
            crate::protocol::messaging::unread::extract_mute_controls(data_node, now_unix);
        Self {
            more,
            has_reset,
            sync_controls,
            mute_events,
            raw,
        }
    }

    /// Merge preview/chat messages from this poll batch into `threads`.
    pub fn merge_into(&self, threads: &mut Vec<Thread>) {
        merge_previews(threads, &self.raw);
    }

    /// Apply read-point watermarks from this poll batch onto `threads`.
    pub fn merge_read_points_into(&self, threads: &mut [Thread]) {
        if let Some(data_node) = self.raw.get("data") {
            crate::protocol::messaging::unread::merge_read_points(threads, data_node);
        }
    }

    /// Raw JSON envelope for diagnostic CLI tools.
    pub fn raw_value(&self) -> &Value {
        &self.raw
    }
}

/// Typed preview batch returned by `get_last_msgs`.
#[derive(Debug, Clone)]
pub struct LastMsgsBatch {
    raw: Value,
}

impl LastMsgsBatch {
    pub fn new(raw: Value) -> Self {
        Self { raw }
    }

    /// Merge message previews from this batch into `threads`.
    pub fn merge_into(&self, threads: &mut Vec<Thread>) {
        merge_previews(threads, &self.raw);
    }

    /// Raw JSON envelope for diagnostic CLI tools.
    pub fn raw_value(&self) -> &Value {
        &self.raw
    }
}

/// Typed cloud/group history page returned by `get_cloud_messages`.
#[derive(Debug, Clone)]
pub struct CloudMessagesPage {
    pub has_more: bool,
    pub last_msg_id: Option<String>,
    raw: Value,
}

impl CloudMessagesPage {
    pub fn from_wire(raw: Value) -> Self {
        let data_node = raw
            .pointer("/data/data")
            .or_else(|| raw.get("data"))
            .unwrap_or(&raw);
        let has_more = data_node
            .get("hasMore")
            .and_then(Value::as_i64)
            .unwrap_or(0)
            != 0;
        let last_msg_id = data_node
            .get("lastMsgId")
            .and_then(|v| {
                v.as_str()
                    .map(str::to_string)
                    .or_else(|| v.as_u64().map(|n| n.to_string()))
            })
            .filter(|s| !s.is_empty() && s != "0" && s.chars().all(|c| c.is_ascii_digit()));
        Self {
            has_more,
            last_msg_id,
            raw,
        }
    }

    /// Merge messages from this page into a single group/cloud thread.
    pub fn merge_into_thread(&self, thread: &mut Thread) {
        merge_group_history(thread, &self.raw);
    }

    /// Merge messages from this page as previews into `threads`.
    pub fn merge_into_threads(&self, threads: &mut Vec<Thread>) {
        merge_previews(threads, &self.raw);
    }

    /// Raw JSON envelope for diagnostic CLI tools.
    pub fn raw_value(&self) -> &Value {
        &self.raw
    }
}
