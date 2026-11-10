//! `ChatContent.BinNet` binary TLV decoder for attachments (`0x06`), quotes (`0x07`), and mentions (`0x08`).
//!
//! Pure byte-slice wire decoder (`re-notes/12-message-sync-and-e2ee-protocol.md` §3.2.4).

use std::collections::HashMap;
use std::ops::RangeInclusive;

use serde_json::json;

use crate::protocol::messaging::attachments::jxl_to_jpg_url;
use crate::protocol::messaging::model::MessageKind;

/// Structured attachment decoded from a `ChatContent.BinNet` TLV blob (`tag 0x06`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BinNetAttach {
    pub attach_type: Option<String>,
    pub cat_id: Option<i64>,
    pub id: Option<i64>,
    pub ext_info: Option<String>,
    pub child_number: Option<i64>,
    pub action: Option<String>,
    pub params: Option<String>,
    pub title: Option<String>,
    pub href: Option<String>,
    pub thumb: Option<String>,
    pub description: Option<String>,
    pub size: Option<String>,
    pub ext: Option<String>,
}

/// Quoted-reply metadata decoded from a `ChatContent.BinNet` TLV blob (`tag 0x07`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BinNetQuote {
    pub owner_id: Option<i64>,
    pub cli_msg_id: Option<i64>,
    pub global_msg_id: Option<i64>,
    pub cli_msg_type: Option<i64>,
    pub ts: Option<i64>,
    pub ttl: Option<i64>,
    pub msg: Option<String>,
    pub attach: Option<String>,
    pub from_d: Option<String>,
}

/// Mention span decoded from a `ChatContent.BinNet` TLV blob (`tag 0x08`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BinNetMention {
    pub uid: Option<i64>,
    pub pos: Option<i64>,
    pub len: Option<i64>,
    pub mention_type: Option<i64>,
}

/// Complete structured metadata decoded from `ChatContent.BinNet` in Sync V1 SQLite snapshots.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BinNetData {
    pub attachs: Vec<BinNetAttach>,
    pub quote: Option<BinNetQuote>,
    pub mentions: Vec<BinNetMention>,
}

fn read_u32_endian(bytes: &[u8], le: bool) -> u32 {
    let arr: [u8; 4] = [bytes[0], bytes[1], bytes[2], bytes[3]];
    if le {
        u32::from_le_bytes(arr)
    } else {
        u32::from_be_bytes(arr)
    }
}

fn parse_tlv_entries(buf: &[u8], le: bool) -> Option<Vec<(u32, &[u8])>> {
    if buf.len() < 8 {
        return None;
    }
    let mut off = 0;
    let mut out = Vec::new();
    while off + 8 <= buf.len() {
        let tag = read_u32_endian(&buf[off..off + 4], le);
        let len = read_u32_endian(&buf[off + 4..off + 8], le) as usize;
        off += 8;
        if len > buf.len() - off {
            return None;
        }
        out.push((tag, &buf[off..off + len]));
        off += len;
    }
    if buf.len() - off > 8 || out.is_empty() {
        return None;
    }
    Some(out)
}

fn score_tlv_entries(entries: &[(u32, &[u8])], expected_range: RangeInclusive<u32>) -> i32 {
    let mut score = 0i32;
    for &(tag, _) in entries {
        if expected_range.contains(&tag) {
            score += 4;
        } else if tag <= 0x80 {
            score += 1;
        } else if tag > 0x10_000 {
            score -= 3;
        }
    }
    score
}

fn parse_tlv_auto(
    buf: &[u8],
    expected_range: RangeInclusive<u32>,
) -> Option<(bool, Vec<(u32, &[u8])>)> {
    let le = parse_tlv_entries(buf, true);
    let be = parse_tlv_entries(buf, false);
    match (le, be) {
        (Some(le_v), Some(be_v)) => {
            let le_s = score_tlv_entries(&le_v, expected_range.clone());
            let be_s = score_tlv_entries(&be_v, expected_range);
            if be_s > le_s {
                Some((false, be_v))
            } else {
                Some((true, le_v))
            }
        }
        (Some(le_v), None) => Some((true, le_v)),
        (None, Some(be_v)) => Some((false, be_v)),
        (None, None) => None,
    }
}

fn decode_tlv_int(bytes: &[u8], le: bool) -> Option<i64> {
    match bytes.len() {
        1 => Some(bytes[0] as i64),
        2 => {
            let a = [bytes[0], bytes[1]];
            Some(if le {
                i16::from_le_bytes(a) as i64
            } else {
                i16::from_be_bytes(a) as i64
            })
        }
        4 => {
            let a = [bytes[0], bytes[1], bytes[2], bytes[3]];
            Some(if le {
                i32::from_le_bytes(a) as i64
            } else {
                i32::from_be_bytes(a) as i64
            })
        }
        8 => {
            let a = [
                bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
            ];
            Some(if le {
                i64::from_le_bytes(a)
            } else {
                i64::from_be_bytes(a)
            })
        }
        _ => std::str::from_utf8(bytes)
            .ok()
            .and_then(|s| s.trim_matches('\0').trim().parse::<i64>().ok()),
    }
}

fn decode_tlv_str(bytes: &[u8]) -> Option<String> {
    if bytes.is_empty() {
        return None;
    }
    if let Ok(utf8) = std::str::from_utf8(bytes) {
        let trimmed = utf8.trim_matches('\0').trim();
        if !trimmed.is_empty()
            && !trimmed
                .chars()
                .any(|c| c.is_control() && c != '\n' && c != '\r' && c != '\t')
        {
            return Some(trimmed.to_string());
        }
    }
    if bytes.len() >= 2 && bytes.len().is_multiple_of(2) {
        // Even length is guaranteed above, so `as_chunks` leaves no remainder.
        let (pairs, _) = bytes.as_chunks::<2>();
        let units: Vec<u16> = pairs.iter().map(|c| u16::from_le_bytes(*c)).collect();
        if let Ok(utf16) = String::from_utf16(&units) {
            let trimmed = utf16.trim_matches('\0').trim();
            if !trimmed.is_empty()
                && !trimmed
                    .chars()
                    .any(|c| c.is_control() && c != '\n' && c != '\r' && c != '\t')
            {
                return Some(trimmed.to_string());
            }
        }
    }
    None
}

/// Decode a `ChatContent.BinNet` binary TLV buffer from a Sync V1 SQLite snapshot.
pub fn parse_bin_net_tlv(buf: &[u8]) -> Option<BinNetData> {
    let (_, top_entries) = parse_tlv_auto(buf, 0x05..=0x0B)?;
    let mut out = BinNetData::default();

    for (tag, val) in top_entries {
        match tag {
            0x06 => {
                let Some((att_le, att_entries)) = parse_tlv_auto(val, 0x28..=0x44) else {
                    continue;
                };
                let mut att = BinNetAttach::default();
                for (atag, aval) in att_entries {
                    match atag {
                        0x28 => att.attach_type = decode_tlv_str(aval),
                        0x29 => att.cat_id = decode_tlv_int(aval, att_le),
                        0x2A => att.id = decode_tlv_int(aval, att_le),
                        0x2B => att.ext_info = decode_tlv_str(aval),
                        0x2C => att.child_number = decode_tlv_int(aval, att_le),
                        0x2D => att.action = decode_tlv_str(aval),
                        0x2E => att.params = decode_tlv_str(aval),
                        0x2F => att.title = decode_tlv_str(aval),
                        0x30 => att.href = decode_tlv_str(aval),
                        0x31 => att.thumb = decode_tlv_str(aval),
                        0x32 => att.description = decode_tlv_str(aval),
                        0x33 => att.size = decode_tlv_str(aval),
                        0x34 => att.ext = decode_tlv_str(aval),
                        _ => {}
                    }
                }
                if att != BinNetAttach::default() {
                    out.attachs.push(att);
                }
            }
            0x07 => {
                let Some((q_le, q_entries)) = parse_tlv_auto(val, 0x50..=0x5A) else {
                    continue;
                };
                let mut q = BinNetQuote::default();
                for (qtag, qval) in q_entries {
                    match qtag {
                        0x50 => q.owner_id = decode_tlv_int(qval, q_le),
                        0x51 => q.cli_msg_id = decode_tlv_int(qval, q_le),
                        0x52 => q.global_msg_id = decode_tlv_int(qval, q_le),
                        0x53 => q.cli_msg_type = decode_tlv_int(qval, q_le),
                        0x54 => q.ts = decode_tlv_int(qval, q_le),
                        0x55 => q.ttl = decode_tlv_int(qval, q_le),
                        0x56 => q.msg = decode_tlv_str(qval),
                        0x57 => q.attach = decode_tlv_str(qval),
                        0x5A => q.from_d = decode_tlv_str(qval),
                        _ => {}
                    }
                }
                if q != BinNetQuote::default() {
                    out.quote = Some(q);
                }
            }
            0x08 => {
                let Some((m_le, m_entries)) = parse_tlv_auto(val, 0x65..=0x68) else {
                    continue;
                };
                let mut m = BinNetMention::default();
                for (mtag, mval) in m_entries {
                    match mtag {
                        0x65 => m.uid = decode_tlv_int(mval, m_le),
                        0x66 => m.pos = decode_tlv_int(mval, m_le),
                        0x67 => m.len = decode_tlv_int(mval, m_le),
                        0x68 => m.mention_type = decode_tlv_int(mval, m_le),
                        _ => {}
                    }
                }
                if m != BinNetMention::default() {
                    out.mentions.push(m);
                }
            }
            _ => {}
        }
    }

    (out != BinNetData::default()).then_some(out)
}

/// System and control archive `MsgType` codes excluded by Zalo Desktop (`Qe` filter in `shared-worker`).
pub fn is_excluded_archive_msg_type(code: i64) -> bool {
    matches!(code, 20 | 21 | 25 | 26 | 29 | 32 | 34 | 35 | 45 | 51 | 52)
}

/// Convert one `ChatContent` row (`MsgType`, `MsgContent`, `BinNet`) into `(text, wire_msg_type, raw_json)`.
pub fn build_synced_message_content(
    msg_type_num: i64,
    content_opt: Option<&str>,
    bin_net_opt: Option<&[u8]>,
    fid_map: &HashMap<i64, String>,
) -> (String, String, String) {
    let wire_type = MessageKind::archive_wire_name(msg_type_num)
        .unwrap_or("webchat")
        .to_string();
    let mut text = content_opt.unwrap_or_default().trim().to_string();
    let mut meta = serde_json::Map::new();

    let parsed_bin = bin_net_opt.and_then(parse_bin_net_tlv);
    if let Some(bn) = &parsed_bin {
        if let Some(q) = &bn.quote {
            let q_id = q
                .global_msg_id
                .filter(|&id| id > 0)
                .or_else(|| q.cli_msg_id.filter(|&id| id > 0))
                .map(|id| id.to_string());
            if let Some(qid) = q_id {
                meta.insert("reply_to_id".into(), json!(qid));
            }
            let q_sender = q
                .from_d
                .clone()
                .filter(|s| !s.is_empty())
                .or_else(|| q.owner_id.and_then(|oid| fid_map.get(&oid).cloned()));
            if let Some(qs) = q_sender {
                meta.insert("reply_to_sender".into(), json!(qs));
            }
            let q_text = q.msg.clone().filter(|s| !s.is_empty()).or_else(|| {
                q.attach
                    .as_ref()
                    .filter(|a| !a.is_empty())
                    .map(|_| "[Đính kèm]".to_string())
            });
            if let Some(qt) = q_text {
                meta.insert("reply_to_text".into(), json!(qt));
            }
        }

        if let Some(att) = bn.attachs.first() {
            if let Some(action) = att.action.as_deref() {
                if action == "recommened.calltime" && text.is_empty() {
                    text = "[Cuộc gọi thoại/video]".to_string();
                } else if action == "recommened.misscall" && text.is_empty() {
                    text = "[Cuộc gọi nhỡ]".to_string();
                }
            }

            if msg_type_num == 10 || att.cat_id.is_some() {
                if let Some(sid) = att.id.filter(|&n| n >= 0) {
                    let cid = att.cat_id.unwrap_or(0).max(0);
                    let st = att
                        .attach_type
                        .as_deref()
                        .and_then(|t| t.parse::<u64>().ok())
                        .unwrap_or(7);
                    meta.insert("stickerId".into(), json!(sid as u64));
                    meta.insert("cateId".into(), json!(cid as u64));
                    meta.insert("type".into(), json!(st));
                    if text.is_empty() {
                        text = "[Nhãn dán]".to_string();
                    }
                }
            } else if matches!(msg_type_num, 2 | 3 | 4 | 31) {
                let ori = att.href.as_deref().map(jxl_to_jpg_url).unwrap_or_default();
                let thumb = att
                    .thumb
                    .as_deref()
                    .or(att.href.as_deref())
                    .map(jxl_to_jpg_url)
                    .unwrap_or_default();
                let title = att
                    .title
                    .as_deref()
                    .filter(|s| !s.is_empty() && *s != "sendBubbleMessage")
                    .unwrap_or("[Hình ảnh]");
                if !ori.is_empty() {
                    meta.insert("link_url".into(), json!(ori));
                }
                if !thumb.is_empty() {
                    meta.insert("link_thumb".into(), json!(thumb));
                }
                meta.insert("link_title".into(), json!(title));
                meta.insert("link_src".into(), json!("Hình ảnh Zalo"));
                if text.is_empty() {
                    text = title.to_string();
                }
            } else {
                let href = att.href.as_deref().unwrap_or_default();
                let thumb = att.thumb.as_deref().map(jxl_to_jpg_url).unwrap_or_default();
                let title = att
                    .title
                    .as_deref()
                    .filter(|s| !s.is_empty() && *s != "sendBubbleMessage")
                    .unwrap_or_default();
                let desc = att
                    .description
                    .as_deref()
                    .filter(|s| !s.is_empty())
                    .unwrap_or_default();
                if !href.is_empty() {
                    meta.insert("link_url".into(), json!(href));
                }
                if !title.is_empty() {
                    meta.insert("link_title".into(), json!(title));
                }
                if !desc.is_empty() {
                    meta.insert("link_desc".into(), json!(desc));
                }
                if !thumb.is_empty() {
                    meta.insert("link_thumb".into(), json!(thumb));
                }
                if text.is_empty() {
                    if !title.is_empty() {
                        text = title.to_string();
                    } else if !desc.is_empty() {
                        text = desc.to_string();
                    } else if !href.is_empty() {
                        text = href.to_string();
                    }
                }
            }
        }
    } else if text.is_empty() {
        if let Some(bn_raw) = bin_net_opt {
            let s_latin = String::from_utf8_lossy(bn_raw);
            if let Some(pos) = s_latin.find("maps.app.goo.gl") {
                let sub = &s_latin[pos..];
                let end_url = sub.find(['"', '}', '\\', ' ', '\0']).unwrap_or(sub.len());
                let url = format!("https://{}", &sub[..end_url]);
                meta.insert("link_url".into(), json!(url));
                text = format!("Chia sẻ vị trí trực tiếp: {url}");
            }
        }
    }

    if text.is_empty() {
        text = match msg_type_num {
            2 | 3 | 4 | 31 => "[Hình ảnh]".to_string(),
            6 => "[Tin nhắn thoại]".to_string(),
            10 => "[Nhãn dán]".to_string(),
            12 | 24 | 53 => "[Liên kết]".to_string(),
            18 => "[Vị trí]".to_string(),
            19 => "[Video]".to_string(),
            22 => "[Tệp đính kèm]".to_string(),
            23 => "[GIF]".to_string(),
            _ => String::new(),
        };
    }

    let raw_json = if meta.is_empty() {
        String::new()
    } else {
        serde_json::Value::Object(meta).to_string()
    };
    (text, wire_type, raw_json)
}
