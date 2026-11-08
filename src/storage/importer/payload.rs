/// Decrypt message payload string using the 32-character XOR key.
pub fn decode_zalo_cipher_payload(raw: &str, key: &str) -> String {
    if raw.is_empty() || key.is_empty() {
        return raw.to_string();
    }
    let s = if let Ok(val) = serde_json::from_str::<serde_json::Value>(raw) {
        if let Some(st) = val.as_str() {
            st.to_string()
        } else {
            raw.to_string()
        }
    } else {
        raw.to_string()
    };

    let key_chars: Vec<char> = key.chars().collect();
    let k_len = key_chars.len();
    let xored: String = s
        .chars()
        .enumerate()
        .map(|(i, c)| {
            let k = key_chars[i % k_len] as u32;
            let code = c as u32 ^ k;
            char::from_u32(code).unwrap_or(c)
        })
        .collect();

    let (text, _meta) = extract_zalo_message_text_and_meta(&xored, 1);
    text
}

pub(super) fn decode_zalo_cipher_payload_with_meta(
    raw: &str,
    key: &str,
    mtype: i64,
) -> (String, serde_json::Map<String, serde_json::Value>) {
    if raw.is_empty() || key.is_empty() {
        return (raw.to_string(), serde_json::Map::new());
    }
    let s = if let Ok(val) = serde_json::from_str::<serde_json::Value>(raw) {
        if let Some(st) = val.as_str() {
            st.to_string()
        } else {
            raw.to_string()
        }
    } else {
        raw.to_string()
    };
    let key_chars: Vec<char> = key.chars().collect();
    let k_len = key_chars.len();
    let xored: String = s
        .chars()
        .enumerate()
        .map(|(i, c)| {
            let k = key_chars[i % k_len] as u32;
            let code = c as u32 ^ k;
            char::from_u32(code).unwrap_or(c)
        })
        .collect();
    extract_zalo_message_text_and_meta(&xored, mtype)
}

/// Convert Zalo `.jxl` URLs (`/jxl/` ... `.jxl?jxlstatus=1`) to standard JPEG URLs (`/jpg/` ... `.jpg`)
/// matching `QWlH.toJPGUrl` in `compact-app-pc.js:8396800`.
pub use crate::protocol::messaging::attachments::jxl_to_jpg_url;

pub(super) fn extract_zalo_message_text_and_meta(
    xored: &str,
    mtype: i64,
) -> (String, serde_json::Map<String, serde_json::Value>) {
    let mut meta = serde_json::Map::new();
    if let Ok(val) = serde_json::from_str::<serde_json::Value>(xored) {
        if let Some(inner) = val.as_str() {
            return (inner.to_string(), meta);
        }
        if let Some(arr) = val.as_array() {
            for item in arr {
                let title = item
                    .get("title")
                    .and_then(|v| v.as_str())
                    .filter(|s| !s.is_empty())
                    .unwrap_or_default();
                let desc = item
                    .get("description")
                    .and_then(|v| v.as_str())
                    .filter(|s| !s.is_empty())
                    .unwrap_or_default();
                let href = item
                    .get("href")
                    .and_then(|v| v.as_str())
                    .filter(|s| !s.is_empty())
                    .unwrap_or_default();
                let thumb = item
                    .get("thumb")
                    .and_then(|v| v.as_str())
                    .filter(|s| !s.is_empty())
                    .unwrap_or_default();
                if !title.is_empty() || !desc.is_empty() {
                    if !href.is_empty() {
                        meta.insert("link_url".into(), serde_json::json!(href));
                    }
                    if !title.is_empty() {
                        meta.insert("link_title".into(), serde_json::json!(title));
                    }
                    if !desc.is_empty() {
                        meta.insert("link_desc".into(), serde_json::json!(desc));
                    }
                    if !thumb.is_empty() {
                        meta.insert("link_thumb".into(), serde_json::json!(thumb));
                    }
                    let txt = if !title.is_empty() { title } else { desc };
                    return (txt.to_string(), meta);
                }
            }
        }
        if mtype == 4 || val.get("catId").is_some() {
            if let Some(sid) = val
                .get("id")
                .and_then(|v| v.as_u64().or_else(|| v.as_str()?.parse().ok()))
            {
                let cid = val
                    .get("catId")
                    .and_then(|v| v.as_u64().or_else(|| v.as_str()?.parse().ok()))
                    .unwrap_or(0);
                let st = val
                    .get("type")
                    .and_then(|v| v.as_u64().or_else(|| v.as_str()?.parse().ok()))
                    .unwrap_or(7);
                meta.insert("stickerId".into(), serde_json::json!(sid));
                meta.insert("cateId".into(), serde_json::json!(cid));
                meta.insert("type".into(), serde_json::json!(st));
                return ("[Nhãn dán]".to_string(), meta);
            }
        }
        // Photo (msgType == 2)
        if mtype == 2 || val.get("oriUrl").is_some() || val.get("thumbUrl").is_some() {
            let ori = jxl_to_jpg_url(
                val.get("oriUrl")
                    .or_else(|| val.get("normalUrl"))
                    .and_then(|v| v.as_str())
                    .unwrap_or_default(),
            );
            let thumb = jxl_to_jpg_url(
                val.get("thumbUrl")
                    .or_else(|| val.get("oriUrl"))
                    .and_then(|v| v.as_str())
                    .unwrap_or_default(),
            );
            let title = val
                .get("title")
                .and_then(|v| v.as_str())
                .filter(|s| !s.is_empty())
                .unwrap_or("[Hình ảnh]");
            if !ori.is_empty() {
                meta.insert("link_url".into(), serde_json::json!(ori));
            }
            if !thumb.is_empty() {
                meta.insert("link_thumb".into(), serde_json::json!(thumb));
            }
            meta.insert("link_title".into(), serde_json::json!(title));
            meta.insert("link_src".into(), serde_json::json!("Hình ảnh Zalo"));
            return (title.to_string(), meta);
        }
        // Call bubble or link/file (msgType == 6, 18, 19)
        if let Some(action) = val.get("action").and_then(|v| v.as_str()) {
            if action == "recommened.calltime" {
                return ("📞 Cuộc gọi thoại/video".to_string(), meta);
            } else if action == "recommened.misscall" {
                return ("📞 Cuộc gọi nhỡ".to_string(), meta);
            }
        }
        let href = val.get("href").and_then(|v| v.as_str()).unwrap_or_default();
        let title = val
            .get("title")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty() && *s != "sendBubbleMessage")
            .unwrap_or_default();
        let desc = val
            .get("description")
            .or_else(|| val.get("desc"))
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty() && *s != "Cuộc gọi")
            .unwrap_or_default();
        let thumb = val
            .get("thumb")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        if !href.is_empty() {
            meta.insert("link_url".into(), serde_json::json!(href));
        }
        if !title.is_empty() {
            meta.insert("link_title".into(), serde_json::json!(title));
        }
        if !desc.is_empty() {
            meta.insert("link_desc".into(), serde_json::json!(desc));
        }
        if !thumb.is_empty() {
            meta.insert("link_thumb".into(), serde_json::json!(thumb));
        }
        if !title.is_empty() {
            return (title.to_string(), meta);
        }
        if !desc.is_empty() {
            return (desc.to_string(), meta);
        }
        if !href.is_empty() {
            return (href.to_string(), meta);
        }
        if val.get("params").is_some() && val.get("childnumber").is_some() {
            return ("📢 Thông báo hệ thống Zalo".to_string(), meta);
        }
    }
    (xored.to_string(), meta)
}
