//! Sticker store catalog, category packs, Vietnamese keyword search, and sticker sending.

use std::cmp::Reverse;
use std::collections::{HashMap, HashSet};
use std::io::Read;
use std::path::Path;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::crypto::{encode_aes, md5_hex};
use crate::protocol::session::{urlencode, Session, SessionError};
use crate::protocol::wire::{
    chat_send_host, domain_or, error_text, next_client_message_id, obj, require_login,
    strip_group_prefix, strip_vn_diacritics, SendMessageReceipt, WireData,
};

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct StickerPackInfo {
    pub id: u64,
    pub name: String,
    pub total_image: u32,
    pub icon_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct StickerItem {
    pub sticker_id: u64,
    pub cate_id: u64,
    pub sticker_type: u32,
    pub label: String,
}

/// Send a sticker (`cmd 11154` 1-1 / `cmd 11165` Group).
///
/// # Errors
/// Returns an error if unauthenticated, encryption fails, or the server rejects the sticker.
pub async fn send_sticker(
    s: &Session,
    to_id: &str,
    sticker_id: u64,
    cate_id: u64,
    sticker_type: u32,
    is_group: bool,
) -> Result<SendMessageReceipt> {
    send_sticker_with_client_id(
        s,
        to_id,
        sticker_id,
        cate_id,
        sticker_type,
        is_group,
        next_client_message_id(),
    )
    .await
}

/// Send a sticker with a caller-supplied monotonic `clientId`.
///
/// # Errors
/// Returns an error if unauthenticated, encryption fails, or the server rejects the sticker.
pub async fn send_sticker_with_client_id(
    s: &Session,
    to_id: &str,
    sticker_id: u64,
    cate_id: u64,
    sticker_type: u32,
    is_group: bool,
    now: u64,
) -> Result<SendMessageReceipt> {
    require_login(s)?;
    let (url, body) = if is_group {
        let clean_grid = strip_group_prefix(to_id);
        (
            format!(
                "{}/api/group/sticker?{}",
                s.domains.group,
                s.common_params()
            ),
            obj(&[
                ("grid", json!(clean_grid)),
                ("stickerId", json!(sticker_id)),
                ("cateId", json!(cate_id)),
                ("type", json!(sticker_type)),
                ("clientId", json!(now)),
                ("imei", json!(s.imei)),
                ("zsource", json!(101)),
            ]),
        )
    } else {
        (
            format!(
                "{}/api/message/sticker?{}",
                chat_send_host(s),
                s.common_params()
            ),
            obj(&[
                ("toid", json!(to_id)),
                ("stickerId", json!(sticker_id)),
                ("cateId", json!(cate_id)),
                ("type", json!(sticker_type)),
                ("clientId", json!(now)),
                ("imei", json!(s.imei)),
                ("zsource", json!(101)),
            ]),
        )
    };

    let params = s.encoded_params(&body)?;
    let resp = s.post_form(&url, format!("params={params}")).await?;
    let resp = s.maybe_decrypt(resp, s.session_key_bytes()).await;
    let env = crate::protocol::wire::ErrorEnvelopeWireData::parse(&resp);
    if let Some(code) = env.code_opt() {
        if code != 0 {
            return Err(SessionError::new(code, error_text(env.message())).into());
        }
    }
    Ok(SendMessageReceipt::from_wire(&resp))
}

fn default_sticker_host(s: &Session) -> &str {
    let fallback = crate::endpoints::service("sticker").unwrap_or_default();
    domain_or(&s.domains.sticker, fallback)
}

/// `data` array of a sticker endpoint, falling back to a bare array root.
fn data_array(dec: &Value) -> Vec<Value> {
    dec.get("data")
        .and_then(Value::as_array)
        .or_else(|| dec.as_array())
        .cloned()
        .unwrap_or_default()
}

/// `data` object of `getStickerCatsV2` (cmd `11283` catalog bootstrap).
///
/// # Wire Response Example
/// ```json
/// { "exCateIds": [12, 34], "dataUrl": "http://sticker.zdn.vn/all_cate.json" }
/// ```
///
/// # Core Transformation
/// `exCateIds` hides excluded/hidden packs; `dataUrl` points at the gzipped CDN catalog and is
/// upgraded to HTTPS before the fetch.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct StickerCatalogWireData {
    #[serde(rename = "exCateIds")]
    pub ex_cate_ids: Option<Value>,
    #[serde(rename = "dataUrl")]
    pub data_url: Option<Value>,
}

impl StickerCatalogWireData {
    /// Category ids excluded from the store listing.
    pub fn excluded_ids(&self) -> HashSet<u64> {
        self.ex_cate_ids
            .as_ref()
            .and_then(Value::as_array)
            .map(|arr| arr.iter().filter_map(Value::as_u64).collect())
            .unwrap_or_default()
    }

    /// CDN URL of the full catalog, defaulting to the bundled endpoint (HTTPS-forced).
    pub fn data_url(&self) -> String {
        self.data_url
            .as_ref()
            .and_then(Value::as_str)
            .unwrap_or(crate::endpoints::STICKER_CDN_ALL_CATE)
            .replace("http://", "https://")
    }
}

/// One pack of the CDN `getAllCate` catalog array.
///
/// # Wire Response Example
/// ```json
/// { "id": 1234, "name": "Củ hành", "totalImage": 24, "iconUrl": "http://s.zdn.vn/1.png" }
/// ```
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct StickerPackWireData {
    #[serde(rename = "id")]
    pub id: Option<Value>,
    #[serde(rename = "name")]
    pub name: Option<Value>,
    #[serde(rename = "totalImage")]
    pub total_image: Option<Value>,
    #[serde(rename = "iconUrl")]
    pub icon_url: Option<Value>,
    #[serde(rename = "thumbImg")]
    pub thumb_img: Option<Value>,
}

impl StickerPackWireData {
    /// Pack id; a missing or zero id excludes the pack.
    pub fn id(&self) -> Option<u64> {
        self.id.as_ref().and_then(Value::as_u64)
    }

    /// Trimmed pack name.
    pub fn name(&self) -> String {
        self.name
            .as_ref()
            .and_then(Value::as_str)
            .unwrap_or("")
            .trim()
            .to_string()
    }

    /// Sticker count in the pack (default `16`).
    pub fn total_image(&self) -> u32 {
        self.total_image
            .as_ref()
            .and_then(Value::as_u64)
            .unwrap_or(16) as u32
    }

    /// Pack icon URL (`iconUrl` > `thumbImg`), HTTPS-forced.
    pub fn icon_url(&self) -> String {
        self.icon_url
            .as_ref()
            .or(self.thumb_img.as_ref())
            .and_then(Value::as_str)
            .unwrap_or("")
            .replace("http://", "https://")
    }
}

/// One sticker entry from `getStickerCategoryDetail` (cmd `11284`), `suggest/stickers`, or `search`.
///
/// # Wire Response Example
/// ```json
/// { "sticker_id": 12345, "cate_id": 1234, "type": 7 }
/// ```
///
/// # Core Transformation
/// `id`/`stickerId` and `cateId`/`cate_id` are both accepted because the catalog and the
/// suggest/search endpoints spell them differently.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct StickerItemWireData {
    #[serde(rename = "id")]
    pub id: Option<Value>,
    #[serde(rename = "stickerId")]
    pub sticker_id_camel: Option<Value>,
    #[serde(rename = "sticker_id")]
    pub sticker_id_snake: Option<Value>,
    #[serde(rename = "cateId")]
    pub cate_id_camel: Option<Value>,
    #[serde(rename = "cate_id")]
    pub cate_id_snake: Option<Value>,
    #[serde(rename = "type")]
    pub sticker_type: Option<Value>,
}

impl StickerItemWireData {
    /// Catalog sticker id (`id` > `stickerId`).
    pub fn id(&self) -> Option<u64> {
        self.id
            .as_ref()
            .or(self.sticker_id_camel.as_ref())
            .and_then(Value::as_u64)
    }

    /// Suggest/search sticker id (`sticker_id` > `id`).
    pub fn sticker_id(&self) -> Option<u64> {
        self.sticker_id_snake
            .as_ref()
            .or(self.id.as_ref())
            .and_then(Value::as_u64)
    }

    /// Category id (`cateId`), falling back to the requested category.
    pub fn cate_id(&self, default: u64) -> u64 {
        self.cate_id_camel
            .as_ref()
            .and_then(Value::as_u64)
            .unwrap_or(default)
    }

    /// Category id for suggest/search (`cate_id` > `cateId`), defaulting to `0`.
    pub fn cate_id_snake(&self) -> u64 {
        self.cate_id_snake
            .as_ref()
            .or(self.cate_id_camel.as_ref())
            .and_then(Value::as_u64)
            .unwrap_or(0)
    }

    /// Sticker type (`type`), default `7` (static PNG).
    pub fn sticker_type(&self) -> u32 {
        self.sticker_type
            .as_ref()
            .and_then(Value::as_u64)
            .unwrap_or(7) as u32
    }
}

/// `suggest/keywords` payload: the searchable Vietnamese keyword list.
///
/// # Wire Response Example
/// ```json
/// { "keywords": ["củ hành", "zkwd_hidden", "mèo"] }
/// ```
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct StickerKeywordsWireData {
    #[serde(rename = "keywords")]
    pub keywords: Option<Vec<Value>>,
}

impl StickerKeywordsWireData {
    /// Raw keyword strings in wire order.
    pub fn keyword_strings(&self) -> Vec<&str> {
        self.keywords
            .as_deref()
            .unwrap_or_default()
            .iter()
            .filter_map(Value::as_str)
            .collect()
    }
}

/// `suggest/stickers` payload (`data.sugg_sticker`).
///
/// # Wire Response Example
/// ```json
/// { "sugg_sticker": [ { "sticker_id": 12345, "cate_id": 1234, "type": 7 } ] }
/// ```
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct StickerSuggestWireData {
    #[serde(rename = "sugg_sticker")]
    pub sugg_sticker: Option<Vec<Value>>,
}

impl StickerSuggestWireData {
    /// Raw suggest entries.
    pub fn entries(&self) -> &[Value] {
        self.sugg_sticker.as_deref().unwrap_or_default()
    }
}

/// Fetch the complete Zalo Sticker Store catalog (`getStickerCatsV2` -> `getAllCate`).
///
/// # Errors
/// Returns an error if unauthenticated, session key is missing, or network/JSON decoding fails.
pub async fn fetch_sticker_store_packs(s: &Session) -> Result<Vec<StickerPackInfo>> {
    let cache_dir = Path::new("stickers_cache");
    let _ = std::fs::create_dir_all(cache_dir);
    let cache_path = cache_dir.join("store_packs.json");
    if let Ok(raw_cache) = std::fs::read_to_string(&cache_path) {
        if let Ok(cached) = serde_json::from_str::<Vec<StickerPackInfo>>(&raw_cache) {
            if cached.len() >= 20 {
                return Ok(cached);
            }
        }
    }

    require_login(s)?;
    let sticker_host = default_sticker_host(s);
    let key = s.session_key_bytes();
    if key.is_empty() {
        anyhow::bail!("thiếu session key để tải kho nhãn dán");
    }
    let encoded_param = urlencode(&encode_aes(key, "")?);
    let list_url = format!(
        "{sticker_host}/api/message/sticker/category/list/v2?{}&params={encoded_param}",
        s.common_params()
    );
    let resp = s.get(&list_url).await?;
    let dec = s.maybe_decrypt(resp, key).await;
    let catalog = StickerCatalogWireData::from_value(dec.get("data").unwrap_or(&dec));

    let ex_ids: HashSet<u64> = catalog.excluded_ids();
    let data_url = catalog.data_url();

    let cdn_resp = s.http.get(&data_url).send().await?;
    let raw_bytes = cdn_resp.bytes().await?;
    let json_text = if raw_bytes.len() >= 2 && raw_bytes[0] == 0x1f && raw_bytes[1] == 0x8b {
        let mut gz = flate2::read::GzDecoder::new(&raw_bytes[..]);
        let mut out = String::new();
        gz.read_to_string(&mut out)?;
        out
    } else {
        String::from_utf8_lossy(&raw_bytes).into_owned()
    };

    let arr: Vec<Value> = serde_json::from_str(&json_text)?;
    let mut packs = Vec::with_capacity(arr.len());
    for item in arr {
        let pack = StickerPackWireData::from_value(&item);
        let Some(id) = pack.id() else {
            continue;
        };
        if id == 0 || ex_ids.contains(&id) {
            continue;
        }
        let name = pack.name();
        if name.is_empty() || name.to_lowercase().contains("test") {
            continue;
        }
        packs.push(StickerPackInfo {
            id,
            name,
            total_image: pack.total_image(),
            icon_url: pack.icon_url(),
        });
    }

    packs.sort_by_key(|p| Reverse(p.id));
    if let Ok(txt) = serde_json::to_string(&packs) {
        let _ = std::fs::write(&cache_path, txt);
    }
    Ok(packs)
}

/// Fetch all stickers belonging to a specific category (`getStickerCategoryDetail`, cmd `11284`).
///
/// # Errors
/// Returns an error if unauthenticated or the HTTP request fails.
pub async fn fetch_sticker_pack_items(
    s: &Session,
    cate_id: u64,
    pack_name: &str,
) -> Result<Vec<StickerItem>> {
    if cate_id == 0 {
        return Ok((1..=30)
            .map(|id| StickerItem {
                sticker_id: id,
                cate_id: 0,
                sticker_type: 7,
                label: format!("Củ hành #{id}"),
            })
            .collect());
    }

    let cache_dir = Path::new("stickers_cache");
    let _ = std::fs::create_dir_all(cache_dir);
    let cache_path = cache_dir.join(format!("pack_{cate_id}.json"));
    if let Ok(raw_cache) = std::fs::read_to_string(&cache_path) {
        if let Ok(cached) = serde_json::from_str::<Vec<StickerItem>>(&raw_cache) {
            if !cached.is_empty() {
                return Ok(cached);
            }
        }
    }

    require_login(s)?;
    let sticker_host = default_sticker_host(s);
    let params = s.encoded_params(&json!({ "cid": cate_id.to_string() }))?;
    let url = format!(
        "{sticker_host}/api/message/sticker/category/sticker_detail?{}&params={params}",
        s.common_params()
    );
    let resp = s.get(&url).await?;
    let dec = s.maybe_decrypt(resp, s.session_key_bytes()).await;
    let arr = data_array(&dec);

    let mut items = Vec::with_capacity(arr.len());
    for (idx, it) in arr.iter().enumerate() {
        let wire = StickerItemWireData::from_value(it);
        let Some(sid) = wire.id() else {
            continue;
        };
        items.push(StickerItem {
            sticker_id: sid,
            cate_id: wire.cate_id(cate_id),
            sticker_type: wire.sticker_type(),
            label: format!("{pack_name} #{}", idx + 1),
        });
    }

    if !items.is_empty() {
        if let Ok(txt) = serde_json::to_string(&items) {
            let _ = std::fs::write(&cache_path, txt);
        }
    }
    Ok(items)
}

/// Normalize a sticker keyword (NBSP -> space, collapsed whitespace, lowercase).
pub fn normalize_sticker_keyword(s: &str) -> String {
    s.replace('\u{a0}', " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

/// Strip Vietnamese diacritics to lowercase ASCII (`simpleStripVietnamese`).
pub fn strip_vn_for_sticker_kw(s: &str) -> String {
    strip_vn_diacritics(s)
}

/// Map common unaccented Vietnamese keywords, English terms, or slang to accented Vietnamese.
pub fn canonical_accented_vn_for_sticker(stripped: &str) -> Option<&'static str> {
    match stripped {
        "nho" | "nho em" | "nho anh" | "nho qua" | "miss" | "miss u" | "miss you" => Some("nhớ"),
        "khoc" | "huhu" | "huhuhu" | "hic" | "cry" => Some("khóc"),
        "buon" | "buon qua" | "sad" | "tui than" => Some("buồn"),
        "vui" | "vui ve" | "vui qua" | "happy" => Some("vui"),
        "cuoi" | "mac cuoi" | "buon cuoi" => Some("cười"),
        "yeu" | "iu" | "yeu em" | "yeu anh" | "iu em" | "iu anh" | "love" | "love u"
        | "love you" | "tha tim" | "tim" => Some("yêu"),
        "thuong" | "thuong em" | "thuong anh" | "thuong qua" => Some("thương"),
        "gian" | "gian doi" | "doi hon" | "tuc" | "buc" | "buc minh" | "cay" | "angry" => {
            Some("giận")
        }
        "met" | "met qua" | "met moi" | "duoi" | "duoi qua" | "tired" => Some("mệt"),
        "doi" | "doi bung" | "doi qua" | "hungry" => Some("đói"),
        "an" | "an com" | "di an" => Some("ăn cơm"),
        "ca phe" | "cf" | "cafe" | "coffee" => Some("cà phê"),
        "tra sua" | "trasua" => Some("trà sữa"),
        "nhau" | "di nhau" | "bia" | "dzoo" | "dzo" => Some("nhậu"),
        "ngu" | "di ngu" | "buon ngu" | "ngu ngon" | "ngu ngon nhe" | "ngu ngon nha"
        | "good night" | "goodnight" | "gn" | "g9" => Some("ngủ"),
        "chao" | "xin chao" | "chao ban" | "chao anh" | "chao chi" | "chao em" | "he lo"
        | "helo" => Some("xin chào"),
        "cam on" | "cam on ban" | "cam on anh" | "cam on chi" | "cam on em" | "cam on nhieu"
        | "thank" | "thanks" | "thank you" | "tks" | "tkss" | "thank kiu" => Some("cảm ơn"),
        "xin loi" | "xin loi nha" | "xin loi nhe" | "sorry" | "sr" | "so ry" => Some("xin lỗi"),
        "tam biet" | "tam biet nhe" | "bye" | "byebye" | "good bye" | "goodbye" | "bibi" | "pp" => {
            Some("tạm biệt")
        }
        "chuc mung" | "happy birthday" | "hpbd" | "sinh nhat" | "cmsn" => Some("chúc mừng"),
        "tet" | "chuc mung nam moi" | "nam moi" => Some("tết"),
        "co len" | "co gang" | "fighting" => Some("cố lên"),
        "tuyet" | "tuyet voi" | "xuat sac" | "dinh" | "dinh qua" | "chuan" | "duoc" => {
            Some("tuyệt vời")
        }
        "ngai" | "ngai qua" | "ngai nhi" | "xau ho" => Some("ngại"),
        "so" | "so qua" | "scared" => Some("sợ"),
        "lanh" | "lanh qua" | "lanh khong" | "ret" | "ret qua" => Some("lạnh"),
        "nong" | "nong qua" | "nong buc" => Some("nóng"),
        "tien" | "luong" => Some("tiền"),
        "het tien" | "ngheo" | "ngheo qua" | "vi rong" => Some("hết tiền"),
        "quy" | "lay" | "xin" | "van xin" => Some("lạy"),
        "ban" | "ban qua" | "busy" => Some("bận"),
        "chan" | "chan qua" | "chan doi" | "boring" => Some("chán"),
        "cho" | "cho chut" | "cho xiu" | "doi chut" | "doi xiu" | "wait" => Some("chờ chút"),
        "di choi" | "choi" | "dao pho" => Some("đi chơi"),
        "di lam" | "lam viec" | "deadline" | "code" => Some("làm"),
        "hong" | "hong chuyen" | "ngac nhien" => Some("ngạc nhiên"),
        _ => None,
    }
}

/// Build up to 4 query variants (accented + unaccented + canonical synonym) for sticker search.
pub fn canonical_vn_sticker_queries(keyword: &str) -> Vec<String> {
    let norm = normalize_sticker_keyword(keyword);
    if norm.is_empty() {
        return Vec::new();
    }
    let stripped = strip_vn_for_sticker_kw(&norm);
    let mut queries = vec![norm.clone()];
    if !stripped.is_empty() && stripped != norm && !queries.contains(&stripped) {
        queries.push(stripped.clone());
    }
    if let Some(accented) = canonical_accented_vn_for_sticker(&stripped) {
        let acc_s = accented.to_string();
        if !queries.contains(&acc_s) {
            queries.push(acc_s.clone());
        }
        let acc_stripped = strip_vn_for_sticker_kw(&acc_s);
        if !acc_stripped.is_empty() && !queries.contains(&acc_stripped) {
            queries.push(acc_stripped);
        }
    }
    queries.truncate(4);
    queries
}

/// Fetch official Zalo contextual sticker trigger keywords (`getStickerListKeyWords`, cmd `11740`).
///
/// # Errors
/// Returns an error if unauthenticated or both primary and fallback sticker hosts fail.
pub async fn fetch_sticker_suggest_keywords(s: &Session) -> Result<Vec<String>> {
    let cache_dir = Path::new("stickers_cache");
    let _ = std::fs::create_dir_all(cache_dir);
    let cache_path = cache_dir.join("suggest_keywords_v2.json");
    if let Ok(raw_cache) = std::fs::read_to_string(&cache_path) {
        if let Ok(cached) = serde_json::from_str::<Vec<String>>(&raw_cache) {
            if cached.len() >= 100 {
                return Ok(cached);
            }
        }
    }
    require_login(s)?;
    let sticker_host = default_sticker_host(s);
    let params = s.encoded_params(&json!({ "imei": s.imei }))?;
    let url = format!(
        "{sticker_host}/api/message/sticker/suggest/keywords?{}&params={params}",
        s.common_params()
    );
    let resp = match s.get(&url).await {
        Ok(r) => r,
        Err(_) => {
            let fallback_url = format!(
                "{}/api/message/sticker/suggest/keywords?{}&params={params}",
                crate::endpoints::STICKER_LEGACY_HOST,
                s.common_params()
            );
            s.get(&fallback_url).await?
        }
    };
    let dec = s.maybe_decrypt(resp, s.session_key_bytes()).await;
    let mut seen = HashSet::new();
    let mut kws: Vec<String> = Vec::new();
    let keywords = StickerKeywordsWireData::from_value(dec.get("data").unwrap_or(&dec));
    for raw_str in keywords.keyword_strings() {
        let norm = normalize_sticker_keyword(raw_str);
        if norm.is_empty() || norm.starts_with("zkwd_") {
            continue;
        }
        if seen.insert(norm.clone()) {
            kws.push(norm.clone());
        }
        let stripped = strip_vn_for_sticker_kw(&norm);
        if !stripped.is_empty() && seen.insert(stripped.clone()) {
            kws.push(stripped);
        }
    }
    if !kws.is_empty() {
        if let Ok(txt) = serde_json::to_string(&kws) {
            let _ = std::fs::write(&cache_path, txt);
        }
    }
    Ok(kws)
}

async fn query_sticker_endpoints_once(
    s: &Session,
    sticker_host: &str,
    queries: &[String],
) -> Vec<(u64, u64, u32, u8)> {
    let mut futs = Vec::with_capacity(queries.len());
    for (q_idx, q) in queries.iter().enumerate() {
        let sugg_body = obj(&[
            ("keyword", json!(q)),
            ("gif", json!(1)),
            ("guggy", json!(0)),
            ("imei", json!(s.imei)),
        ]);
        if let Ok(params) = s.encoded_params(&sugg_body) {
            let url = format!(
                "{sticker_host}/api/message/sticker/suggest/stickers?{}&params={params}",
                s.common_params()
            );
            let is_primary = q_idx == 0;
            futs.push(async move {
                let Ok(resp) = s.get(&url).await else {
                    return Vec::new();
                };
                let dec = s.maybe_decrypt(resp, s.session_key_bytes()).await;
                let sugg = StickerSuggestWireData::from_value(dec.get("data").unwrap_or(&dec));
                sugg.entries()
                    .iter()
                    .filter_map(|it| {
                        let wire = StickerItemWireData::from_value(it);
                        let sid = wire.sticker_id()?;
                        Some((
                            wire.cate_id_snake(),
                            sid,
                            wire.sticker_type(),
                            if is_primary { 3u8 } else { 2u8 },
                        ))
                    })
                    .collect::<Vec<_>>()
            });
        }
    }

    let mut search_futs = Vec::with_capacity(queries.len());
    for (q_idx, q) in queries.iter().enumerate() {
        let search_body = obj(&[
            ("keyword", json!(q)),
            ("limit", json!(30)),
            ("srcType", json!(0)),
            ("imei", json!(s.imei)),
        ]);
        if let Ok(params) = s.encoded_params(&search_body) {
            let url = format!(
                "{sticker_host}/api/message/sticker/search?{}&params={params}",
                s.common_params()
            );
            let is_primary = q_idx == 0;
            search_futs.push(async move {
                let Ok(resp) = s.get(&url).await else {
                    return Vec::new();
                };
                let dec = s.maybe_decrypt(resp, s.session_key_bytes()).await;
                let arr = data_array(&dec);
                arr.into_iter()
                    .filter_map(|it| {
                        let wire = StickerItemWireData::from_value(&it);
                        let sid = wire.sticker_id()?;
                        Some((
                            wire.cate_id_snake(),
                            sid,
                            wire.sticker_type(),
                            if is_primary { 3u8 } else { 2u8 },
                        ))
                    })
                    .collect::<Vec<_>>()
            });
        }
    }

    let (sugg_batches, search_batches) = tokio::join!(
        futures_util::future::join_all(futs),
        futures_util::future::join_all(search_futs)
    );

    let mut score_map: HashMap<u64, (u64, u32, u8)> = HashMap::new();
    for batch in sugg_batches.into_iter().chain(search_batches) {
        for (cid, sid, stype, weight) in batch {
            score_map
                .entry(sid)
                .and_modify(|e| {
                    e.2 = e.2.saturating_add(weight);
                    if e.0 == 0 && cid > 0 {
                        e.0 = cid;
                    }
                })
                .or_insert((cid, stype, weight));
        }
    }

    score_map
        .into_iter()
        .map(|(sid, (cid, stype, score))| (cid, sid, stype, score))
        .collect()
}

/// Search Zalo stickers by keyword across both suggest (`cmd 11741`) and search (`cmd 11747`).
///
/// # Errors
/// Returns an error if unauthenticated.
pub async fn search_stickers_by_keyword(s: &Session, keyword: &str) -> Result<Vec<StickerItem>> {
    let kw = normalize_sticker_keyword(keyword);
    if kw.is_empty() {
        return Ok(Vec::new());
    }
    let cache_dir = Path::new("stickers_cache");
    let _ = std::fs::create_dir_all(cache_dir);
    let cache_path = cache_dir.join(format!("suggest_v2_{}.json", md5_hex(&kw)));
    if let Ok(raw_cache) = std::fs::read_to_string(&cache_path) {
        if let Ok(cached) = serde_json::from_str::<Vec<StickerItem>>(&raw_cache) {
            if !cached.is_empty() {
                return Ok(cached);
            }
        }
    }
    require_login(s)?;
    let sticker_host = default_sticker_host(s);

    let queries = canonical_vn_sticker_queries(&kw);
    let mut raw_items = query_sticker_endpoints_once(s, sticker_host, &queries).await;

    if raw_items.is_empty() {
        let words: Vec<&str> = kw.split_whitespace().collect();
        if (2..=5).contains(&words.len()) {
            let mut fallback_phrases = Vec::new();
            if words.len() >= 3 {
                fallback_phrases.push(format!(
                    "{} {}",
                    words[words.len() - 2],
                    words[words.len() - 1]
                ));
            }
            let last_w = words[words.len() - 1];
            if last_w.chars().count() >= 2 {
                fallback_phrases.push(last_w.to_string());
            }
            for fb in fallback_phrases {
                let fb_queries = canonical_vn_sticker_queries(&fb);
                raw_items = query_sticker_endpoints_once(s, sticker_host, &fb_queries).await;
                if !raw_items.is_empty() {
                    break;
                }
            }
        }
    }

    const LEGACY_NON_CHAR_PACKS: &[u64] = &[
        0, 50, 10788, 11190, 11984, 11998, 12002, 12003, 12024, 12658,
    ];
    let has_modern_char = raw_items
        .iter()
        .any(|(cid, _, _, _)| *cid >= 10000 && !LEGACY_NON_CHAR_PACKS.contains(cid));
    if has_modern_char {
        raw_items.retain(|(cid, _, _, _)| *cid >= 10000 && !LEGACY_NON_CHAR_PACKS.contains(cid));
    }
    raw_items.sort_by(|a, b| b.3.cmp(&a.3).then_with(|| b.0.cmp(&a.0)));

    let mut pack_counts: HashMap<u64, usize> = HashMap::new();
    let mut out = Vec::with_capacity(24);
    let mut deferred = Vec::new();
    for &(cid, sid, stype, _) in &raw_items {
        let cnt = pack_counts.entry(cid).or_insert(0);
        if *cnt >= 2 {
            deferred.push((cid, sid, stype));
            continue;
        }
        *cnt += 1;
        let idx = out.len() + 1;
        out.push(StickerItem {
            sticker_id: sid,
            cate_id: cid,
            sticker_type: stype,
            label: format!("{kw} #{idx}"),
        });
        if out.len() >= 24 {
            break;
        }
    }
    for (cid, sid, stype) in deferred {
        if out.len() >= 24 {
            break;
        }
        let idx = out.len() + 1;
        out.push(StickerItem {
            sticker_id: sid,
            cate_id: cid,
            sticker_type: stype,
            label: format!("{kw} #{idx}"),
        });
    }

    if !out.is_empty() {
        if let Ok(txt) = serde_json::to_string(&out) {
            let _ = std::fs::write(&cache_path, txt);
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_vn_and_canonical_queries_cover_diacritics_and_slang() {
        assert_eq!(strip_vn_for_sticker_kw("Xin chào"), "xin chao");
        assert_eq!(strip_vn_for_sticker_kw("Cảm ơn nhiều"), "cam on nhieu");
        assert_eq!(strip_vn_for_sticker_kw("Nguyễn Văn An"), "nguyen van an");
        assert_eq!(
            normalize_sticker_keyword("  iu\u{a0}that\u{a0}  "),
            "iu that"
        );

        let q1 = canonical_vn_sticker_queries("Mệt quá");
        assert!(q1.contains(&"mệt quá".to_string()));
        assert!(q1.contains(&"met qua".to_string()));

        let q2 = canonical_vn_sticker_queries("khoc");
        assert!(q2.contains(&"khoc".to_string()));
        assert!(q2.contains(&"khóc".to_string()));
    }
}
