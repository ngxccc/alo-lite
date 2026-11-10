//! Mobile synchronization HTTP/WS commands, `/api/znoise` ID translation, and control event parser.

use std::collections::HashMap;

use crate::protocol::wire::{
    opt_text, require_login, val_i64, ActionAck, ErrorEnvelopeWireData, WireData,
};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use super::latest_mobile_temp_key;
use super::v2_proto::{generate_sync_v2_ed25519_public_key, generate_sync_v2_keys};
use crate::protocol::registry;
use crate::protocol::session::Session;

/// Request mobile message sync (`pullMobileMsg`, cmd `12000`, #68, #73).
///
/// # Errors
/// Returns an error if the session is unauthenticated or the outbound request fails.
pub async fn request_phone_sync(s: &Session, public_key: &str) -> Result<ActionAck> {
    let temp_key = latest_mobile_temp_key();
    request_phone_sync_with_temp_key(s, public_key, &temp_key).await
}

/// Request mobile message sync (`pullMobileMsg`, cmd `12000`) with an explicit `temp_key`.
///
/// # Errors
/// Returns an error if the session is unauthenticated or both GET and POST candidates fail.
pub async fn request_phone_sync_with_temp_key(
    s: &Session,
    public_key: &str,
    temp_key: &str,
) -> Result<ActionAck> {
    require_login(s)?;
    let body = json!({
        "pc_name": s.computer_name,
        "public_key": public_key,
        "from_seq_id": 0,
        "is_retry": 0,
        "min_seq_id": 0,
        "temp_key": temp_key,
        "imei": s.imei
    });
    if let Ok(dec) = s
        .api_get(registry::sync::PULL_MOBILE_MSG, Some(&body))
        .await
    {
        if ErrorEnvelopeWireData::parse(&dec).code_opt() == Some(0) {
            return Ok(ActionAck::from_wire(&dec));
        }
    }
    let resp = s
        .api_post(registry::sync::PULL_MOBILE_MSG, Some(&body))
        .await?;
    Ok(ActionAck::from_wire(&resp))
}

/// Request transfer-sync v2 from phone over WebSocket (`cmd 592` wake-up + `cmd 590` `SYNC_MESSAGE.REQUEST`).
///
/// # Errors
/// Returns an error if the session is unauthenticated.
pub async fn request_phone_sync_v2(s: &Session, _public_key: &str) -> Result<ActionAck> {
    require_login(s)?;
    let sync_id = crate::param_cipher::random_hex(32);
    let temp_key = latest_mobile_temp_key();
    let (ik_pub, ek_pub) = match generate_sync_v2_keys() {
        Ok(k) => (k.ik_pub, k.ek_pub),
        Err(_) => (
            generate_sync_v2_ed25519_public_key(0xA1),
            generate_sync_v2_ed25519_public_key(0xB2),
        ),
    };
    let _ = crate::socket::send_mobile_wakeup(&sync_id);
    let sync_data =
        build_sync_v2_request_payload(&s.computer_name, &sync_id, &ek_pub, &ik_pub, &temp_key, 0);
    let _sent_ws = crate::socket::send_sync_message_ws_request(&sync_data);
    Ok(ActionAck::ok())
}

/// Request Phase 2 of Sync v2: ask phone to transmit message chunks for `sync_id` at `ussidx`.
pub fn request_sync_v2_message_chunks(s: &Session, sync_id: &str, ussidx: u64) -> bool {
    let ik_pub = generate_sync_v2_ed25519_public_key(0xA1);
    let ek_pub = generate_sync_v2_ed25519_public_key(0xB2);
    let temp_key = latest_mobile_temp_key();
    let sync_data = build_sync_v2_request_payload(
        &s.computer_name,
        sync_id,
        &ek_pub,
        &ik_pub,
        &temp_key,
        ussidx,
    );
    crate::socket::send_sync_message_ws_request(&sync_data)
}

fn build_sync_v2_request_payload(
    device_name: &str,
    sync_id: &str,
    ek_pub: &str,
    ik_pub: &str,
    temp_key: &str,
    ussidx: u64,
) -> Value {
    json!({
        "syncId": sync_id,
        "syncType": 0,
        "ek": ek_pub,
        "ik": ik_pub,
        "toDevice": 0,
        "tempKey": temp_key,
        "deviceName": device_name,
        "req": {
            "type": "message",
            "priority": 1,
            "queries": [
                {
                    "partition": "",
                    "from": 0,
                    "to": 9_007_199_254_740_991_i64,
                    "limit": 2000
                }
            ],
            "batchSize": 2000
        },
        "ver": 1,
        "ussidx": ussidx
    })
}

/// Cancel an active mobile message sync request (`cancelTransferMessage`, cmd `12700`, #68).
///
/// # Errors
/// Returns an error if the session is unauthenticated or the request fails.
pub async fn cancel_phone_sync(s: &Session, public_key: &str) -> Result<ActionAck> {
    require_login(s)?;
    let body = json!({
        "pc_name": s.computer_name,
        "public_key": public_key,
        "imei": s.imei
    });
    let resp = s
        .api_get(registry::sync::CANCEL_PULL_MOBILE_MSG, Some(&body))
        .await?;
    Ok(ActionAck::from_wire(&resp))
}

/// Notify Zalo server and mobile device that sync snapshot download completed (`cleanSync`, cmd `12003`).
///
/// # Errors
/// Returns an error if the session is unauthenticated or the request fails.
pub async fn clean_sync(s: &Session, public_key: &str) -> Result<ActionAck> {
    require_login(s)?;
    let body = json!({
        "public_key": public_key,
        "imei": s.imei
    });
    let resp = s
        .api_get(registry::sync::DELETE_SNAPSHOT_MOBILE_MSG, Some(&body))
        .await?;
    Ok(ActionAck::from_wire(&resp))
}

/// Translate plain numeric friend/sender IDs (`fids`) and group IDs (`gids`) from Sync V1 SQLite
/// snapshots into session-scoped noised IDs via `/api/znoise` (`commandId 12390`).
///
/// # Errors
/// Returns an error if the session is unauthenticated.
///
/// # Invariants
/// - INV-1: outbound `params` are encoded with `ParamCipher v2` via [`Session::api_post`].
pub async fn resolve_znoise_ids(
    s: &Session,
    fids: &[i64],
    gids: &[i64],
) -> Result<(HashMap<i64, String>, HashMap<i64, String>)> {
    require_login(s)?;
    let mut fid_map = HashMap::new();
    let mut gid_map = HashMap::new();

    let mut uniq_fids: Vec<i64> = fids.iter().copied().filter(|&id| id > 0).collect();
    uniq_fids.sort_unstable();
    uniq_fids.dedup();

    let mut uniq_gids: Vec<i64> = gids.iter().copied().filter(|&id| id > 0).collect();
    uniq_gids.sort_unstable();
    uniq_gids.dedup();

    if uniq_fids.is_empty() && uniq_gids.is_empty() {
        return Ok((fid_map, gid_map));
    }

    const BATCH_LIMIT: usize = 5_000;
    let fid_chunks: Vec<&[i64]> = if uniq_fids.is_empty() {
        vec![&[]]
    } else {
        uniq_fids.chunks(BATCH_LIMIT).collect()
    };
    let gid_chunks: Vec<&[i64]> = if uniq_gids.is_empty() {
        vec![&[]]
    } else {
        uniq_gids.chunks(BATCH_LIMIT).collect()
    };

    let steps = fid_chunks.len().max(gid_chunks.len());
    for idx in 0..steps {
        let f_batch = fid_chunks.get(idx).copied().unwrap_or(&[]);
        let g_batch = gid_chunks.get(idx).copied().unwrap_or(&[]);
        if f_batch.is_empty() && g_batch.is_empty() {
            continue;
        }
        let body = json!({
            "fids": f_batch,
            "gids": g_batch,
        });
        let mut dec = s
            .api_post(registry::sync::ZNOISE, Some(&body))
            .await
            .unwrap_or(Value::Null);
        if ErrorEnvelopeWireData::parse(&dec).code_opt() != Some(0) {
            if let Ok(fallback_dec) = s
                .api_post(registry::sync::ZNOISE_PROFILE_FALLBACK, Some(&body))
                .await
            {
                dec = fallback_dec;
            }
        }

        let data_node = crate::protocol::wire::unwrap_data(&dec);
        let resolved = ZnoiseProfileWireData::from_value(data_node);
        for (&fid, mapped) in f_batch.iter().zip(resolved.fids().iter()) {
            if let Some(uid) = mapped_uid(mapped) {
                fid_map.insert(fid, uid.to_string());
            }
        }
        for (&gid, mapped) in g_batch.iter().zip(resolved.gids().iter()) {
            if let Some(tid) = mapped_uid(mapped) {
                gid_map.insert(gid, tid.to_string());
            }
        }
    }

    Ok((fid_map, gid_map))
}

/// Mobile sync control event received via WebSocket (`cmd 601`/`602`/`603`) or `lp_control`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MobileSyncControlEvent {
    /// `act == "transfer_after_login"` — carries `temp_key` issued right after QR scan.
    TransferAfterLogin { temp_key: String },
    /// `act == "transfer_status"` (`status == 3`) — phone displayed confirmation prompt.
    WaitingConfirmOnPhone { sync_id: String },
    /// `act == "user_confirm"` (`user_action == 1 | 3`) or `transfer_status` (`status == 4 | 5`).
    UserConfirmed,
    /// `act == "user_confirm"` (`user_action == 0`) or `transfer_status` (`status == 6 | 7`).
    UserRejected,
    /// `act == "user_confirm"` (`user_action == 2`) or `transfer_status` (`status == 8`).
    MobileBusyRestoring,
    /// `act == "syncmsg_info"` — phone finished uploading Sync V1 `.db.crypt` backup package.
    BackupReady {
        url: String,
        file_name: String,
        encrypted_key: String,
        checksum: String,
        /// Plain numeric `SenderId` of the authenticated account (`data.uid` in `syncmsg_info`).
        raw_uid: Option<i64>,
    },
    /// `act == "upload_batch"` (`act_type == "transfer_sync2"`) — Sync v2 chunk.
    SyncV2Batch {
        url: String,
        sync_id: String,
        batch_type: u8,
        is_last: bool,
    },
}

/// Trimmed non-empty uid from one `/api/znoise` mapping entry.
fn mapped_uid(v: &Value) -> Option<&str> {
    v.as_str().map(str::trim).filter(|u| !u.is_empty())
}

/// `/api/znoise` (cmd `12006`) profile-resolution payload.
///
/// # Wire Response Example
/// ```json
/// { "fids": [ "1234567890" ], "gids": [ "g9876543210" ] }
/// ```
///
/// # Core Transformation
/// Entries are positional: they map one-to-one onto the requested `fids` / `gids` batches, so
/// they are zipped rather than keyed.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ZnoiseProfileWireData {
    #[serde(rename = "fids")]
    pub fids: Option<Vec<Value>>,
    #[serde(rename = "gids")]
    pub gids: Option<Vec<Value>>,
}

impl ZnoiseProfileWireData {
    /// Resolved friend uids, positionally aligned with the request.
    pub fn fids(&self) -> &[Value] {
        self.fids.as_deref().unwrap_or_default()
    }

    /// Resolved group ids, positionally aligned with the request.
    pub fn gids(&self) -> &[Value] {
        self.gids.as_deref().unwrap_or_default()
    }
}

/// Control item head of a mobile-sync event.
///
/// # Wire Response Example
/// ```json
/// { "act_type": "transfer_msg", "act": "user_confirm", "data": "{\"user_action\":1}" }
/// ```
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct MobileSyncControlWireData {
    #[serde(rename = "act_type")]
    pub act_type: Option<Value>,
    #[serde(rename = "act")]
    pub act: Option<Value>,
    #[serde(rename = "data")]
    pub data: Option<Value>,
}

impl MobileSyncControlWireData {
    /// `act_type` as sent.
    pub fn act_type(&self) -> &str {
        self.act_type
            .as_ref()
            .and_then(Value::as_str)
            .unwrap_or_default()
    }

    /// `act` as sent.
    pub fn act(&self) -> &str {
        self.act
            .as_ref()
            .and_then(Value::as_str)
            .unwrap_or_default()
    }

    /// `data` decoded from its JSON-string form when needed.
    pub fn parsed_data(&self) -> MobileSyncDataWireData {
        MobileSyncDataWireData::from_value(&crate::protocol::wire::parse_control_data(
            self.data.as_ref(),
        ))
    }
}

/// `data` object of a mobile-sync control item.
///
/// # Wire Response Example
/// ```json
/// { "syncId": "sync_v2_batch", "msgUrl": "https://s3.example/batch", "batchType": 1, "isLast": 1 }
/// ```
///
/// # Core Transformation
/// Every field is optional across the `transfer_msg` / `syncmsgmb` / `transfer_sync2` variants,
/// so accessors carry the per-branch defaults instead of the caller.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct MobileSyncDataWireData {
    #[serde(rename = "temp_key")]
    pub temp_key: Option<Value>,
    #[serde(rename = "user_action")]
    pub user_action: Option<Value>,
    #[serde(rename = "status")]
    pub status: Option<Value>,
    #[serde(rename = "syncId")]
    pub sync_id: Option<Value>,
    #[serde(rename = "msgUrl")]
    pub msg_url: Option<Value>,
    #[serde(rename = "url")]
    pub url: Option<Value>,
    #[serde(rename = "batchType")]
    pub batch_type: Option<Value>,
    #[serde(rename = "isLast")]
    pub is_last: Option<Value>,
    #[serde(rename = "file_name")]
    pub file_name: Option<Value>,
    #[serde(rename = "encrypted_key")]
    pub encrypted_key: Option<Value>,
    #[serde(rename = "checksum_code")]
    pub checksum_code: Option<Value>,
    #[serde(rename = "checksum")]
    pub checksum: Option<Value>,
    #[serde(rename = "uid")]
    pub uid: Option<Value>,
}

impl MobileSyncDataWireData {
    /// Trimmed non-empty `temp_key`.
    pub fn temp_key(&self) -> Option<&str> {
        self.temp_key
            .as_ref()
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|s| !s.is_empty())
    }

    /// `user_action` as an integer (`-1` when absent).
    pub fn user_action(&self) -> i64 {
        val_i64(self.user_action.as_ref()).unwrap_or(-1)
    }

    /// Transfer `status` as an integer (`-1` when absent).
    pub fn status(&self) -> i64 {
        val_i64(self.status.as_ref()).unwrap_or(-1)
    }

    /// `syncId`, falling back to `default`.
    pub fn sync_id(&self, default: &str) -> String {
        self.sync_id
            .as_ref()
            .and_then(Value::as_str)
            .unwrap_or(default)
            .to_string()
    }

    /// Batch URL (`msgUrl` > `url`).
    pub fn batch_url(&self) -> String {
        opt_text(self.msg_url.as_ref().or(self.url.as_ref())).to_string()
    }

    /// Plain `url`.
    pub fn url(&self) -> String {
        opt_text(self.url.as_ref()).to_string()
    }

    /// `batchType` as a byte.
    pub fn batch_type(&self) -> u8 {
        self.batch_type
            .as_ref()
            .and_then(Value::as_i64)
            .unwrap_or(0) as u8
    }

    /// Whether `isLast` is `1`.
    pub fn is_last(&self) -> bool {
        self.is_last.as_ref().and_then(Value::as_i64).unwrap_or(0) == 1
    }

    /// Backup file name.
    pub fn file_name(&self) -> String {
        opt_text(self.file_name.as_ref()).to_string()
    }

    /// Backup file encryption key.
    pub fn encrypted_key(&self) -> String {
        opt_text(self.encrypted_key.as_ref()).to_string()
    }

    /// Checksum (`checksum_code` > `checksum`).
    pub fn checksum(&self) -> String {
        opt_text(self.checksum_code.as_ref().or(self.checksum.as_ref())).to_string()
    }

    /// Positive account `uid`.
    pub fn raw_uid(&self) -> Option<i64> {
        val_i64(self.uid.as_ref()).filter(|&n| n > 0)
    }
}

/// Extract `transfer_msg` / `syncmsgmb` / `transfer_sync2` events from a control payload.
pub fn extract_mobile_sync_controls(data_node: &Value) -> Vec<MobileSyncControlEvent> {
    let mut out = Vec::new();
    for c in crate::protocol::wire::iter_control_items(data_node) {
        let control = MobileSyncControlWireData::from_value(c);
        let act_type = control.act_type();
        if !matches!(act_type, "transfer_msg" | "syncmsgmb" | "transfer_sync2") {
            continue;
        }
        let act = control.act();
        let parsed_data = control.parsed_data();
        match act {
            "transfer_after_login" => {
                if let Some(tk) = parsed_data.temp_key() {
                    out.push(MobileSyncControlEvent::TransferAfterLogin {
                        temp_key: tk.to_string(),
                    });
                }
            }
            "user_confirm" => match parsed_data.user_action() {
                1 | 3 => out.push(MobileSyncControlEvent::UserConfirmed),
                2 => out.push(MobileSyncControlEvent::MobileBusyRestoring),
                0 => out.push(MobileSyncControlEvent::UserRejected),
                _ => {}
            },
            "transfer_status" => {
                let st = parsed_data.status();
                let sid = parsed_data.sync_id("");
                match st {
                    3 => out.push(MobileSyncControlEvent::WaitingConfirmOnPhone { sync_id: sid }),
                    4 | 5 => out.push(MobileSyncControlEvent::UserConfirmed),
                    6 | 7 => out.push(MobileSyncControlEvent::UserRejected),
                    8 => out.push(MobileSyncControlEvent::MobileBusyRestoring),
                    _ => {}
                }
            }
            "upload_batch" => {
                let url = parsed_data.batch_url();
                if !url.is_empty() {
                    out.push(MobileSyncControlEvent::SyncV2Batch {
                        url,
                        sync_id: parsed_data.sync_id("sync_v2_batch"),
                        batch_type: parsed_data.batch_type(),
                        is_last: parsed_data.is_last(),
                    });
                } else {
                    out.push(MobileSyncControlEvent::MobileBusyRestoring);
                }
            }
            "syncmsg_info" => {
                let url = parsed_data.url();
                if !url.is_empty() {
                    out.push(MobileSyncControlEvent::BackupReady {
                        url,
                        file_name: parsed_data.file_name(),
                        encrypted_key: parsed_data.encrypted_key(),
                        checksum: parsed_data.checksum(),
                        raw_uid: parsed_data.raw_uid(),
                    });
                }
            }
            _ => {}
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_mobile_sync_controls_parses_v1_v2_and_raw_uid() {
        let payload = serde_json::json!({
            "controls": [
                {
                    "content": {
                        "act_type": "transfer_msg",
                        "act": "transfer_after_login",
                        "data": { "temp_key": "tk_login_999" }
                    }
                },
                {
                    "content": {
                        "act_type": "transfer_sync2",
                        "act": "transfer_status",
                        "data": "{\"syncId\":\"abc123\",\"fromDevice\":0,\"status\":3}"
                    }
                },
                {
                    "content": {
                        "act_type": "syncmsgmb",
                        "act": "user_confirm",
                        "data": "{\"user_action\":2,\"pc_name\":\"ADMIN-PC\"}"
                    }
                },
                {
                    "content": {
                        "act_type": "syncmsgmb",
                        "act": "syncmsg_info",
                        "data": {
                            "uid": "88776655",
                            "url": "https://trans-bin.zaloapp.com/snap.db.crypt",
                            "file_name": "snap.db.crypt",
                            "encrypted_key": "YmFzZTY0",
                            "checksum_code": "abc"
                        }
                    }
                },
                {
                    "content": {
                        "act_type": "transfer_sync2",
                        "act": "upload_batch",
                        "data": "{\"syncId\":\"abc123\",\"err\":0,\"msgUrl\":\"https://f.zalo.me/batch1\"}"
                    }
                }
            ]
        });
        let evs = extract_mobile_sync_controls(&payload);
        assert_eq!(
            evs,
            vec![
                MobileSyncControlEvent::TransferAfterLogin {
                    temp_key: "tk_login_999".into()
                },
                MobileSyncControlEvent::WaitingConfirmOnPhone {
                    sync_id: "abc123".into()
                },
                MobileSyncControlEvent::MobileBusyRestoring,
                MobileSyncControlEvent::BackupReady {
                    url: "https://trans-bin.zaloapp.com/snap.db.crypt".into(),
                    file_name: "snap.db.crypt".into(),
                    encrypted_key: "YmFzZTY0".into(),
                    checksum: "abc".into(),
                    raw_uid: Some(88_776_655),
                },
                MobileSyncControlEvent::SyncV2Batch {
                    url: "https://f.zalo.me/batch1".into(),
                    sync_id: "abc123".into(),
                    batch_type: 0,
                    is_last: false,
                },
            ]
        );
    }
}
