use crate::protocol::wire::{opt_text, val_u64, WireData};

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

/// Decrypted archived-message payload of one imported Zalo `.db` row.
///
/// # Wire Response Example
/// ```json
/// {
///   "title": "Zalo",
///   "description": "Ứng dụng nhắn tin",
///   "href": "https://zalo.me",
///   "thumb": "https://thumb.example/1"
/// }
/// ```
///
/// # Core Transformation
/// Produces the message text plus the `link_*` / sticker metadata map; each branch screens on the
/// archived `mtype` (`2` photo, `4` sticker) as well as the object shape. The same shape appears
/// both as a bare object and as an element of an array payload, so accessors are shared.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct ImportedContentWireData {
    #[serde(rename = "title")]
    pub title: Option<serde_json::Value>,
    #[serde(rename = "description")]
    pub description: Option<serde_json::Value>,
    #[serde(rename = "desc")]
    pub desc: Option<serde_json::Value>,
    #[serde(rename = "href")]
    pub href: Option<serde_json::Value>,
    #[serde(rename = "thumb")]
    pub thumb: Option<serde_json::Value>,
    #[serde(rename = "action")]
    pub action: Option<serde_json::Value>,
    #[serde(rename = "catId")]
    pub cat_id: Option<serde_json::Value>,
    #[serde(rename = "id")]
    pub id: Option<serde_json::Value>,
    #[serde(rename = "type")]
    pub sticker_type: Option<serde_json::Value>,
    #[serde(rename = "oriUrl")]
    pub ori_url: Option<serde_json::Value>,
    #[serde(rename = "normalUrl")]
    pub normal_url: Option<serde_json::Value>,
    #[serde(rename = "thumbUrl")]
    pub thumb_url: Option<serde_json::Value>,
    #[serde(rename = "params")]
    pub params: Option<serde_json::Value>,
    #[serde(rename = "childnumber")]
    pub child_number: Option<serde_json::Value>,
}

impl ImportedContentWireData {
    /// Playback title (`title`), empty when absent.
    pub fn title(&self) -> &str {
        opt_text(self.title.as_ref())
    }

    /// Playback description (`description`), empty when absent.
    pub fn description(&self) -> &str {
        opt_text(self.description.as_ref())
    }

    /// Link URL (`href`), empty when absent.
    pub fn href(&self) -> &str {
        opt_text(self.href.as_ref())
    }

    /// Thumbnail URL (`thumb`), empty when absent.
    pub fn thumb(&self) -> &str {
        opt_text(self.thumb.as_ref())
    }

    /// Card action token (`action`), empty when absent.
    pub fn action(&self) -> &str {
        opt_text(self.action.as_ref())
    }

    /// Description falling back to `desc`.
    pub fn description_or_desc(&self) -> &str {
        match self.description() {
            "" => opt_text(self.desc.as_ref()),
            d => d,
        }
    }

    /// Title for the link/file bubble, rejecting Zalo's `sendBubbleMessage` placeholder.
    pub fn bubble_title(&self) -> &str {
        let t = self.title();
        if t.is_empty() || t == "sendBubbleMessage" {
            ""
        } else {
            t
        }
    }

    /// Description for the link/file bubble, rejecting the `Cuộc gọi` placeholder.
    pub fn bubble_desc(&self) -> &str {
        let d = self.description_or_desc();
        if d.is_empty() || d == "Cuộc gọi" {
            ""
        } else {
            d
        }
    }

    /// Sticker id (`id`).
    pub fn sticker_id(&self) -> Option<u64> {
        val_u64(self.id.as_ref())
    }

    /// Sticker category id (`catId`), defaulting to `0`.
    pub fn sticker_cate_id(&self) -> u64 {
        val_u64(self.cat_id.as_ref()).unwrap_or(0)
    }

    /// Sticker type, defaulting to `7` (static PNG).
    pub fn sticker_type(&self) -> u64 {
        val_u64(self.sticker_type.as_ref()).unwrap_or(7)
    }

    /// Whether the object carries a `catId` key at all.
    pub fn has_cat_id(&self) -> bool {
        self.cat_id.is_some()
    }

    /// Photo URL (`oriUrl` > `normalUrl`), empty when absent.
    pub fn photo_url(&self) -> &str {
        match opt_text(self.ori_url.as_ref()) {
            "" => opt_text(self.normal_url.as_ref()),
            u => u,
        }
    }

    /// Photo thumbnail URL (`thumbUrl` > `oriUrl`), empty when absent.
    pub fn photo_thumb_url(&self) -> &str {
        match opt_text(self.thumb_url.as_ref()) {
            "" => opt_text(self.ori_url.as_ref()),
            u => u,
        }
    }

    /// Whether the object carries `oriUrl` or `thumbUrl`.
    pub fn has_photo_keys(&self) -> bool {
        self.ori_url.is_some() || self.thumb_url.is_some()
    }

    /// Whether the object carries both `params` and `childnumber` (system notice bubble).
    pub fn is_system_notice(&self) -> bool {
        self.params.is_some() && self.child_number.is_some()
    }
}

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
                let item_wire = ImportedContentWireData::from_value(item);
                let title = item_wire.title();
                let desc = item_wire.description();
                let href = item_wire.href();
                let thumb = item_wire.thumb();
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
        let wire = ImportedContentWireData::from_value(&val);
        if mtype == 4 || wire.has_cat_id() {
            if let Some(sid) = wire.sticker_id() {
                meta.insert("stickerId".into(), serde_json::json!(sid));
                meta.insert("cateId".into(), serde_json::json!(wire.sticker_cate_id()));
                meta.insert("type".into(), serde_json::json!(wire.sticker_type()));
                return ("[Nhãn dán]".to_string(), meta);
            }
        }
        // Photo (msgType == 2)
        if mtype == 2 || wire.has_photo_keys() {
            let ori = jxl_to_jpg_url(wire.photo_url());
            let thumb = jxl_to_jpg_url(wire.photo_thumb_url());
            let title = match wire.title() {
                "" => "[Hình ảnh]",
                t => t,
            };
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
        match wire.action() {
            "recommened.calltime" => return ("📞 Cuộc gọi thoại/video".to_string(), meta),
            "recommened.misscall" => return ("📞 Cuộc gọi nhỡ".to_string(), meta),
            _ => {}
        }
        let href = wire.href();
        let title = wire.bubble_title();
        let desc = wire.bubble_desc();
        let thumb = wire.thumb();
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
        if wire.is_system_notice() {
            return ("📢 Thông báo hệ thống Zalo".to_string(), meta);
        }
    }
    (xored.to_string(), meta)
}
