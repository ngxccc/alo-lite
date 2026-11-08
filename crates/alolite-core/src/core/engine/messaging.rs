//! Message dispatch (text, link preview, quotes, forwards, stickers, attachments),
//! reactions, recalls, deletions, seen/typing indicators, and SQLite message queries.

use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Result;

use super::CoreEngine;
use crate::clock::unix_now;
use crate::core::events::CoreEvent;
use crate::protocol::messaging::attachments::send_attachment;
use crate::protocol::messaging::links::{
    extract_first_url, parse_link_metadata, send_link_message_with_client_id,
};
use crate::protocol::messaging::reactions::send_reaction;
use crate::protocol::messaging::send::{
    delete_message_for_me, forward_message_with_client_id, recall_message,
    send_group_message_with_client_id, send_message_with_client_id,
    send_quote_message_with_client_id, send_typing_indicator,
};
use crate::protocol::messaging::stickers::send_sticker_with_client_id;
use crate::protocol::messaging::unread::send_seen_v2;
use crate::protocol::messaging::{ChatMessage, LinkMetadata};
use crate::protocol::session::SessionStatus;
use crate::protocol::wire::{next_client_message_id, SendMessageReceipt};
use crate::protocol::ActionAck;
use crate::storage::{MessageStore, StoredMessage};

impl CoreEngine {
    /// Read stored messages for `thread_id` from SQLite (`spawn_blocking`, `INV-3`).
    pub async fn get_messages(&self, thread_id: &str, limit: usize) -> Result<Vec<StoredMessage>> {
        let db_path = self.active_db_path().await;
        let tid = thread_id.to_string();
        tokio::task::spawn_blocking(move || MessageStore::open(&db_path)?.messages_for(&tid, limit))
            .await?
    }

    /// Full-text search across stored messages in SQLite (`spawn_blocking`, `INV-3`).
    pub async fn search_messages(&self, query: &str, limit: usize) -> Result<Vec<StoredMessage>> {
        let db_path = self.active_db_path().await;
        let q = query.to_string();
        tokio::task::spawn_blocking(move || {
            MessageStore::open(&db_path)?.search_messages(&q, limit)
        })
        .await?
    }

    /// Send a plain 1-1 or group text message directly and return the typed server receipt.
    pub async fn send_text(
        &self,
        to_id: &str,
        text: &str,
        is_group: bool,
    ) -> Result<SendMessageReceipt> {
        let s = self.session.lock().await.clone();
        let client_id = next_client_message_id();
        if is_group {
            send_group_message_with_client_id(&s, to_id, text, client_id).await
        } else {
            send_message_with_client_id(&s, to_id, text, client_id).await
        }
    }

    /// Send a message from interactive UI (auto-upgrading embedded URLs to link previews) and emit events.
    pub async fn send_message_or_link(
        self: &Arc<Self>,
        to_id: String,
        text: String,
        is_group: bool,
    ) {
        let s = self.session.lock().await.clone();
        let client_id = next_client_message_id();
        if let Some(found_url) = extract_first_url(&text) {
            let meta = parse_link_metadata(&s, &found_url)
                .await
                .unwrap_or_else(|_| LinkMetadata {
                    url: found_url.clone(),
                    title: found_url.clone(),
                    desc: String::new(),
                    thumb: String::new(),
                    src: "Liên kết".into(),
                });
            self.emit(CoreEvent::LinkPreviewReady(meta.clone()));
            match send_link_message_with_client_id(&s, &to_id, &text, &meta, is_group, client_id)
                .await
            {
                Ok(receipt) => {
                    let final_id = if receipt.msg_id.is_empty() {
                        format!("out_{client_id}")
                    } else {
                        receipt.msg_id
                    };
                    let msg = ChatMessage {
                        id: final_id,
                        client_message_id: client_id.to_string(),
                        sender: "Tôi".into(),
                        text,
                        time: crate::ui::clock(),
                        is_outgoing: true,
                        ts: unix_now() * 1000,
                        link_url: Some(meta.url),
                        link_title: (!meta.title.is_empty()).then_some(meta.title),
                        link_desc: (!meta.desc.is_empty()).then_some(meta.desc),
                        link_thumb: (!meta.thumb.is_empty()).then_some(meta.thumb),
                        link_src: (!meta.src.is_empty()).then_some(meta.src),
                        ..Default::default()
                    };
                    self.emit(CoreEvent::AttachmentSent {
                        thread_id: to_id,
                        msg,
                    });
                    return;
                }
                Err(e) => {
                    let err_s = e.to_string();
                    crate::logger::log_error("core::send_link", &err_s);
                    self.emit(CoreEvent::SendFailed {
                        thread_id: to_id,
                        text,
                        error: err_s,
                    });
                    return;
                }
            }
        }

        let res = if is_group {
            send_group_message_with_client_id(&s, &to_id, &text, client_id).await
        } else {
            send_message_with_client_id(&s, &to_id, &text, client_id).await
        };
        match res {
            Ok(receipt) => {
                self.emit(CoreEvent::MessageSent {
                    client_message_id: client_id.to_string(),
                    thread_id: to_id,
                    text,
                    msg_id: receipt.msg_id,
                });
            }
            Err(e) => {
                let err_s = e.to_string();
                crate::logger::log_error("core::send_message", &err_s);
                self.emit(CoreEvent::SendFailed {
                    thread_id: to_id,
                    text,
                    error: err_s,
                });
                if SessionStatus::from_error(&e) == SessionStatus::Expired {
                    self.handle_expired_session("Không thể gửi vì phiên zpw_sek đã hết hạn")
                        .await;
                }
            }
        }
    }

    pub async fn send_quote(
        &self,
        to_id: String,
        text: String,
        quoted: ChatMessage,
        is_group: bool,
    ) {
        let s = self.session.lock().await.clone();
        let client_id = next_client_message_id();
        match send_quote_message_with_client_id(&s, &to_id, &text, &quoted, is_group, client_id)
            .await
        {
            Ok(receipt) => {
                self.emit(CoreEvent::QuoteSent {
                    client_message_id: client_id.to_string(),
                    thread_id: to_id,
                    text,
                    msg_id: receipt.msg_id,
                    quoted,
                });
            }
            Err(e) => {
                let err_s = e.to_string();
                crate::logger::log_error("core::send_quote", &err_s);
                self.emit(CoreEvent::SendFailed {
                    thread_id: to_id,
                    text,
                    error: err_s,
                });
            }
        }
    }

    pub async fn forward_message(
        &self,
        target_id: String,
        target_is_group: bool,
        msg: ChatMessage,
    ) {
        let s = self.session.lock().await.clone();
        let client_id = next_client_message_id();
        match forward_message_with_client_id(&s, &target_id, target_is_group, &msg, client_id).await
        {
            Ok(receipt) if receipt.is_ok() => {
                if let Some(sid) = msg.sticker_id.filter(|v| *v > 0) {
                    self.emit(CoreEvent::StickerSent {
                        client_message_id: client_id.to_string(),
                        thread_id: target_id,
                        msg_id: receipt.msg_id,
                        sticker_id: sid,
                        cate_id: msg.sticker_cate_id.unwrap_or(0),
                        sticker_type: msg.sticker_type.unwrap_or(7),
                    });
                } else {
                    self.emit(CoreEvent::MessageSent {
                        client_message_id: client_id.to_string(),
                        thread_id: target_id,
                        text: msg.text.clone(),
                        msg_id: receipt.msg_id,
                    });
                }
                self.emit(CoreEvent::StatusUpdate("Đã chuyển tiếp tin nhắn".into()));
            }
            Ok(receipt) => {
                let err_s = format!("server error {}", receipt.err_code().unwrap_or(0));
                crate::logger::log_error("core::forward", &err_s);
                self.emit(CoreEvent::StatusUpdate(format!(
                    "Chuyển tiếp thất bại: {err_s}"
                )));
            }
            Err(e) => {
                crate::logger::log_error("core::forward", &e.to_string());
                self.emit(CoreEvent::StatusUpdate(format!(
                    "Chuyển tiếp thất bại: {e}"
                )));
            }
        }
    }

    pub async fn react(&self, to_id: String, is_group: bool, msg: ChatMessage, reaction: String) {
        let s = self.session.lock().await.clone();
        match send_reaction(&s, &to_id, is_group, &msg, &reaction).await {
            Ok(ack) if ack.is_ok() => {
                let db_path = self.active_db_path().await;
                let msg_id = msg.id.clone();
                let react_val = reaction.clone();
                let tid = to_id.clone();
                let _ = tokio::task::spawn_blocking(move || {
                    if let Ok(mut store) = MessageStore::open(&db_path) {
                        if let Ok(msgs) = store.messages_for(&tid, 200) {
                            for mut m in msgs {
                                if m.msg_id == msg_id {
                                    let mut raw: serde_json::Value =
                                        serde_json::from_str(&m.raw_json)
                                            .unwrap_or_else(|_| serde_json::json!({}));
                                    raw["reactions"] = serde_json::json!([react_val]);
                                    m.raw_json = raw.to_string();
                                    let _ = store.upsert_message(&m);
                                    break;
                                }
                            }
                        }
                    }
                })
                .await;

                self.emit(CoreEvent::Reacted {
                    thread_id: to_id,
                    msg_id: msg.id,
                    ts: msg.ts,
                    reaction,
                });
            }
            Ok(ack) => {
                let err_s = format!("server error {}", ack.err_code().unwrap_or(0));
                crate::logger::log_error("core::react", &err_s);
                self.emit(CoreEvent::StatusUpdate(format!(
                    "Thả cảm xúc thất bại: {err_s}"
                )));
            }
            Err(e) => {
                crate::logger::log_error("core::react", &e.to_string());
                self.emit(CoreEvent::StatusUpdate(format!(
                    "Thả cảm xúc thất bại: {e}"
                )));
            }
        }
    }

    pub async fn delete_for_me(&self, to_id: String, is_group: bool, msg: ChatMessage) {
        let s = self.session.lock().await.clone();
        let local_only =
            msg.id.is_empty() || msg.id.starts_with("out_") || msg.id.starts_with("local_");
        let res = if local_only {
            Ok(ActionAck::ok())
        } else {
            delete_message_for_me(&s, &to_id, is_group, &msg).await
        };
        match res {
            Ok(ack) if ack.is_ok() => {
                let db_path = self.active_db_path().await;
                let (tid, mid, ts) = (to_id.clone(), msg.id.clone(), msg.ts);
                let _ = tokio::task::spawn_blocking(move || {
                    if let Ok(mut store) = MessageStore::open(&db_path) {
                        let _ = store.delete_message(&tid, &mid, ts);
                    }
                })
                .await;
                self.emit(CoreEvent::MessageDeleted {
                    thread_id: to_id,
                    msg_id: msg.id,
                    ts: msg.ts,
                });
            }
            Ok(ack) => {
                let err_s = format!("server error {}", ack.err_code().unwrap_or(0));
                crate::logger::log_error("core::delete", &err_s);
                self.emit(CoreEvent::StatusUpdate(format!(
                    "Xóa tin nhắn thất bại: {err_s}"
                )));
            }
            Err(e) => {
                crate::logger::log_error("core::delete", &e.to_string());
                self.emit(CoreEvent::StatusUpdate(format!(
                    "Xóa tin nhắn thất bại: {e}"
                )));
            }
        }
    }

    pub async fn recall_message(&self, to_id: String, is_group: bool, msg: ChatMessage) {
        let s = self.session.lock().await.clone();
        let local_only =
            msg.id.is_empty() || msg.id.starts_with("out_") || msg.id.starts_with("local_");
        let res = if local_only {
            Ok(ActionAck::ok())
        } else {
            recall_message(&s, &to_id, is_group, &msg).await
        };
        match res {
            Ok(ack) if ack.is_ok() => {
                self.emit(CoreEvent::MessageRecalled {
                    thread_id: to_id,
                    msg_id: msg.id,
                    ts: msg.ts,
                });
            }
            Ok(ack) => {
                let err_s = format!("server error {}", ack.err_code().unwrap_or(0));
                crate::logger::log_error("core::recall", &err_s);
                self.emit(CoreEvent::StatusUpdate(format!(
                    "Đã thu hồi cục bộ (Máy chủ: {err_s})"
                )));
            }
            Err(e) => {
                crate::logger::log_error("core::recall", &e.to_string());
                self.emit(CoreEvent::StatusUpdate(format!(
                    "Không thể thu hồi trên máy chủ: {e}"
                )));
            }
        }
    }

    pub async fn send_sticker(
        self: &Arc<Self>,
        to_id: String,
        sticker_id: u64,
        cate_id: u64,
        sticker_type: u32,
        is_group: bool,
    ) {
        let s = self.session.lock().await.clone();
        let client_id = next_client_message_id();
        match send_sticker_with_client_id(
            &s,
            &to_id,
            sticker_id,
            cate_id,
            sticker_type,
            is_group,
            client_id,
        )
        .await
        {
            Ok(receipt) => {
                self.emit(CoreEvent::StickerSent {
                    client_message_id: client_id.to_string(),
                    thread_id: to_id,
                    msg_id: receipt.msg_id,
                    sticker_id,
                    cate_id,
                    sticker_type,
                });
            }
            Err(e) => {
                let err_s = e.to_string();
                crate::logger::log_error("core::send_sticker", &err_s);
                self.emit(CoreEvent::SendFailed {
                    thread_id: to_id,
                    text: String::new(),
                    error: err_s,
                });
                if SessionStatus::from_error(&e) == SessionStatus::Expired {
                    self.handle_expired_session(
                        "Không thể gửi nhãn dán vì phiên zpw_sek đã hết hạn",
                    )
                    .await;
                }
            }
        }
    }

    pub async fn send_attachment_file(
        &self,
        to_id: String,
        is_group: bool,
        path: PathBuf,
        caption: String,
    ) {
        let fname = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("tập tin")
            .to_string();
        self.emit(CoreEvent::StatusUpdate(format!(
            "⏳ Đang tải lên và gửi '{fname}'…"
        )));
        let s = self.session.lock().await.clone();
        match send_attachment(&s, &to_id, is_group, &path, &caption).await {
            Ok(cm) => {
                self.emit(CoreEvent::AttachmentSent {
                    thread_id: to_id,
                    msg: cm,
                });
            }
            Err(e) => {
                let err_s = e.to_string();
                crate::logger::log_error("core::send_attachment", &err_s);
                self.emit(CoreEvent::SendFailed {
                    thread_id: to_id,
                    text: caption,
                    error: format!("Lỗi gửi '{fname}': {err_s}"),
                });
            }
        }
    }

    pub async fn send_typing(&self, thread_id: &str, is_group: bool) {
        let s = self.session.lock().await.clone();
        let _ = send_typing_indicator(&s, thread_id, is_group).await;
    }

    pub async fn send_seen(&self, thread_id: &str, is_group: bool, last_msg_id: &str) {
        let s = self.session.lock().await.clone();
        let _ = send_seen_v2(&s, thread_id, is_group, last_msg_id).await;
    }

    /// Write pasted/dropped WebView file bytes to a temp path and upload as an attachment.
    pub async fn send_attachment_bytes(
        &self,
        to_id: String,
        is_group: bool,
        file_name: String,
        bytes: Vec<u8>,
        caption: String,
    ) {
        let sanitized: String = file_name
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() || c == '.' || c == '-' || c == '_' {
                    c
                } else {
                    '_'
                }
            })
            .collect();
        let safe_name = if sanitized.is_empty() {
            format!("upload_{}.png", unix_now())
        } else {
            sanitized
        };
        let temp_path =
            std::env::temp_dir().join(format!("alolite_up_{}_{}", unix_now(), safe_name));
        let write_path = temp_path.clone();
        let write_res = tokio::task::spawn_blocking(move || std::fs::write(&write_path, &bytes))
            .await
            .map_err(|e| std::io::Error::other(e.to_string()))
            .and_then(|r| r);
        if let Err(e) = write_res {
            self.emit(CoreEvent::SendFailed {
                thread_id: to_id,
                text: caption,
                error: format!("Không thể lưu tệp tạm: {e}"),
            });
            return;
        }
        self.send_attachment_file(to_id, is_group, temp_path.clone(), caption)
            .await;
        let _ = tokio::task::spawn_blocking(move || std::fs::remove_file(&temp_path)).await;
    }
}
