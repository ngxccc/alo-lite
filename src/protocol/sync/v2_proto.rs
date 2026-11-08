//! Sync V2 Ed25519/Curve25519 keys and `proto.Sync2` Protobuf chunk wire parser (`re-notes/12` §3.3).

use anyhow::Result;
use serde::{Deserialize, Serialize};

/// Ephemeral Sync V2 keypair metadata (`ik` + `ek`).
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SyncV2Keys {
    pub ik_pub: String,
    pub ik_priv: String,
    pub ek_pub: String,
    pub ek_priv: String,
}

/// Generate a 33-byte (`0x05` DJB curve prefix + 32-byte key) Base64 public key
/// matching `zprotoEd25519GenerateKeyPair` in `libzproto_wasm_bg.wasm` for Sync V2 (`cmd 590`).
pub fn generate_sync_v2_ed25519_public_key(domain_tag: u8) -> String {
    use base64::Engine;
    use sha2::{Digest, Sha256};

    let seed = crate::param_cipher::random_hex(32);
    let mut hasher = Sha256::new();
    hasher.update([domain_tag]);
    hasher.update(seed.as_bytes());
    let digest = hasher.finalize();
    let mut raw = [0u8; 33];
    raw[0] = 0x05;
    raw[1..].copy_from_slice(&digest);
    raw[1] &= 0xF8;
    raw[32] = (raw[32] & 0x7F) | 0x40;
    base64::engine::general_purpose::STANDARD.encode(raw)
}

/// Generate a fresh [`SyncV2Keys`] set for Sync V2 negotiation.
///
/// # Errors
/// Infallible in practice; returns `Result` for symmetry with caller key setup.
pub fn generate_sync_v2_keys() -> Result<SyncV2Keys> {
    let ik_pub = generate_sync_v2_ed25519_public_key(0xA1);
    let ek_pub = generate_sync_v2_ed25519_public_key(0xB2);
    let ik_priv = crate::param_cipher::random_hex(32);
    let ek_priv = crate::param_cipher::random_hex(32);
    Ok(SyncV2Keys {
        ik_pub,
        ik_priv,
        ek_pub,
        ek_priv,
    })
}

/// Decoded `proto.Sync2.Message` record.
#[derive(Debug, Default)]
pub struct ParsedProtoMsg {
    pub sender_id: String,
    pub client_id: u64,
    pub msg_status: i32,
    pub msg_type: i32,
    pub timestamp: u64,
    pub global_id: String,
    pub content: String,
}

/// Decoded `proto.Sync2.Partition` record inside a `SyncChunk`.
#[derive(Debug, Default)]
pub struct ParsedProtoPartition {
    pub id: String,
    pub messages: Vec<ParsedProtoMsg>,
}

fn parse_vli(buf: &[u8], mut offset: usize) -> Option<(u64, usize)> {
    let mut result: u64 = 0;
    let mut shift = 0;
    while offset < buf.len() {
        let b = buf[offset];
        offset += 1;
        result |= ((b & 0x7F) as u64) << shift;
        if (b & 0x80) == 0 {
            return Some((result, offset));
        }
        shift += 7;
        if shift >= 64 {
            return None;
        }
    }
    None
}

fn parse_message(buf: &[u8]) -> Option<ParsedProtoMsg> {
    let mut msg = ParsedProtoMsg::default();
    let mut offset = 0;
    while offset < buf.len() {
        let (tag, next) = parse_vli(buf, offset)?;
        offset = next;
        let field_num = tag >> 3;
        let wire_type = tag & 0x07;
        match wire_type {
            0 => {
                let (val, next) = parse_vli(buf, offset)?;
                offset = next;
                match field_num {
                    2 => msg.client_id = val,
                    3 => msg.msg_status = val as i32,
                    4 => msg.msg_type = val as i32,
                    5 => msg.timestamp = val,
                    _ => {}
                }
            }
            2 => {
                let (len, next) = parse_vli(buf, offset)?;
                offset = next;
                let len = len as usize;
                if offset + len > buf.len() {
                    return None;
                }
                let bytes = &buf[offset..offset + len];
                offset += len;
                match field_num {
                    1 => msg.sender_id = String::from_utf8_lossy(bytes).into_owned(),
                    6 => msg.global_id = String::from_utf8_lossy(bytes).into_owned(),
                    7 => msg.content = String::from_utf8_lossy(bytes).into_owned(),
                    _ => {}
                }
            }
            1 => offset += 8,
            5 => offset += 4,
            _ => return None,
        }
    }
    Some(msg)
}

fn parse_partition(buf: &[u8]) -> Option<ParsedProtoPartition> {
    let mut part = ParsedProtoPartition::default();
    let mut offset = 0;
    while offset < buf.len() {
        let (tag, next) = parse_vli(buf, offset)?;
        offset = next;
        let field_num = tag >> 3;
        let wire_type = tag & 0x07;
        match (field_num, wire_type) {
            (1, 2) => {
                let (len, next) = parse_vli(buf, offset)?;
                offset = next;
                let len = len as usize;
                if offset + len > buf.len() {
                    return None;
                }
                part.id = String::from_utf8_lossy(&buf[offset..offset + len]).into_owned();
                offset += len;
            }
            (2, 2) => {
                let (len, next) = parse_vli(buf, offset)?;
                offset = next;
                let len = len as usize;
                if offset + len > buf.len() {
                    return None;
                }
                if let Some(m) = parse_message(&buf[offset..offset + len]) {
                    part.messages.push(m);
                }
                offset += len;
            }
            (_, 0) => {
                let (_, next) = parse_vli(buf, offset)?;
                offset = next;
            }
            (_, 2) => {
                let (len, next) = parse_vli(buf, offset)?;
                offset = next + len as usize;
            }
            _ => return None,
        }
    }
    Some(part)
}

/// Decode an uncompressed `proto.Sync2.SyncChunk` protobuf buffer into partitions.
pub fn parse_sync_chunk(buf: &[u8]) -> Option<Vec<ParsedProtoPartition>> {
    let mut parts = Vec::new();
    let mut offset = 0;
    while offset < buf.len() {
        let (tag, next) = parse_vli(buf, offset)?;
        offset = next;
        let field_num = tag >> 3;
        let wire_type = tag & 0x07;
        match (field_num, wire_type) {
            (2, 2) => {
                let (len, next) = parse_vli(buf, offset)?;
                offset = next;
                let len = len as usize;
                if offset + len > buf.len() {
                    return None;
                }
                if let Some(p) = parse_partition(&buf[offset..offset + len]) {
                    parts.push(p);
                }
                offset += len;
            }
            (_, 0) => {
                let (_, next) = parse_vli(buf, offset)?;
                offset = next;
            }
            (_, 2) => {
                let (len, next) = parse_vli(buf, offset)?;
                offset = next + len as usize;
            }
            _ => return None,
        }
    }
    Some(parts)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_sync_v2_ed25519_public_key_produces_33_byte_djb_curve_key() {
        use base64::Engine;
        let ik = generate_sync_v2_ed25519_public_key(0xA1);
        let ek = generate_sync_v2_ed25519_public_key(0xB2);
        assert_ne!(ik, ek);
        assert!(ik.starts_with('B'));
        let raw = base64::engine::general_purpose::STANDARD
            .decode(&ik)
            .unwrap();
        assert_eq!(raw.len(), 33);
        assert_eq!(raw[0], 0x05);
    }
}
