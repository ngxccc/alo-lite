use anyhow::Result;
use rusqlite::params;

use super::models::{MessageStore, StoredMessage};

impl MessageStore {
    /// Insert a message. Returns `true` if a new row was inserted, `false` if already existed.
    pub fn upsert_message(&mut self, msg: &StoredMessage) -> Result<bool> {
        let rows = self.conn.execute(
            "INSERT OR IGNORE INTO messages (msg_id, thread_id, sender_id, sender_name, text, msg_type, ts, is_outgoing, raw_json)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                msg.msg_id,
                msg.thread_id,
                msg.sender_id,
                msg.sender_name,
                msg.text,
                msg.msg_type,
                msg.ts,
                if msg.is_outgoing { 1 } else { 0 },
                msg.raw_json,
            ],
        )?;
        if rows == 0 && !msg.raw_json.is_empty() {
            let _ = self.conn.execute(
                "UPDATE messages SET raw_json = ?1, msg_type = ?2, text = ?4 WHERE msg_id = ?3",
                params![msg.raw_json, msg.msg_type, msg.msg_id, msg.text],
            );
        }
        Ok(rows > 0)
    }

    /// Delete a message from local SQLite store when user selects "Xóa chỉ ở phía tôi".
    pub fn delete_message(&mut self, thread_id: &str, msg_id: &str, ts: i64) -> Result<()> {
        if !msg_id.is_empty() {
            let _ = self
                .conn
                .execute("DELETE FROM messages WHERE msg_id = ?1", params![msg_id]);
        }
        if ts > 0 && !thread_id.is_empty() {
            let _ = self.conn.execute(
                "DELETE FROM messages WHERE thread_id = ?1 AND ts = ?2",
                params![thread_id, ts],
            );
        }
        Ok(())
    }

    /// Search messages across all threads by keyword (case-insensitive substring match) (#35).
    pub fn search_messages(&self, query: &str, limit: usize) -> Result<Vec<StoredMessage>> {
        let q = query.trim();
        if q.is_empty() {
            return Ok(Vec::new());
        }
        let pattern = format!("%{}%", q.replace('%', "\\%").replace('_', "\\_"));
        let mut stmt = self.conn.prepare(
            "SELECT msg_id, thread_id, sender_id, sender_name, text, msg_type, ts, is_outgoing, raw_json
             FROM messages
             WHERE LOWER(text) LIKE LOWER(?1) ESCAPE '\\'
                OR LOWER(sender_name) LIKE LOWER(?1) ESCAPE '\\'
             ORDER BY ts DESC
             LIMIT ?2",
        )?;
        let rows = stmt.query_map(params![pattern, limit as i64], |row| {
            let is_outgoing_num: i64 = row.get(7)?;
            Ok(StoredMessage {
                msg_id: row.get(0)?,
                thread_id: row.get(1)?,
                sender_id: row.get(2).unwrap_or_default(),
                sender_name: row.get(3).unwrap_or_default(),
                text: row.get(4).unwrap_or_default(),
                msg_type: row.get(5).unwrap_or_default(),
                ts: row.get(6)?,
                is_outgoing: is_outgoing_num != 0,
                raw_json: row.get(8).unwrap_or_default(),
            })
        })?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    /// Id of the oldest stored server message in a thread, where a history sync resumes from.
    /// Local ids (`out_*`, `{thread}_{ts}`) are skipped: the server rejects them as cursors.
    pub fn oldest_msg_id(&self, thread_id: &str) -> Option<String> {
        self.conn
            .query_row(
                "SELECT msg_id FROM messages
                 WHERE thread_id = ?1 AND msg_id <> '' AND msg_id NOT GLOB '*[^0-9]*'
                 ORDER BY ts ASC LIMIT 1",
                params![thread_id],
                |row| row.get(0),
            )
            .ok()
    }

    /// Retrieve the newest `limit` messages for a thread, ordered by timestamp ascending.
    pub fn messages_for(&self, thread_id: &str, limit: usize) -> Result<Vec<StoredMessage>> {
        let mut stmt = self.conn.prepare(
            "SELECT msg_id, thread_id, sender_id, sender_name, text, msg_type, ts, is_outgoing, raw_json
             FROM (
                 SELECT msg_id, thread_id, sender_id, sender_name, text, msg_type, ts, is_outgoing, raw_json
                 FROM messages
                 WHERE thread_id = ?1
                 ORDER BY ts DESC
                 LIMIT ?2
             ) ORDER BY ts ASC",
        )?;

        let rows = stmt.query_map(params![thread_id, limit as i64], |row| {
            let is_outgoing_num: i64 = row.get(7)?;
            Ok(StoredMessage {
                msg_id: row.get(0)?,
                thread_id: row.get(1)?,
                sender_id: row.get(2).unwrap_or_default(),
                sender_name: row.get(3).unwrap_or_default(),
                text: row.get(4).unwrap_or_default(),
                msg_type: row.get(5).unwrap_or_default(),
                ts: row.get(6)?,
                is_outgoing: is_outgoing_num != 0,
                raw_json: row.get(8).unwrap_or_default(),
            })
        })?;

        let mut list = Vec::new();
        for r in rows {
            list.push(r?);
        }
        Ok(list)
    }
}
