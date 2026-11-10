//! Group listing, `getmg-v2` profiles, mini member profiles, history fetch, and display-name resolution.

use std::collections::{BTreeMap, HashSet};

use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};

use crate::protocol::messaging::threads::{
    avatar_http_url_count, is_generic_thread_name, should_replace_avatar, SidebarMember, Thread,
};
use crate::protocol::session::Session;
use crate::protocol::wire::{
    obj, require_login, strip_group_prefix, strip_vn_diacritics, unwrap_data,
};

/// Coerce a raw JSON scalar to `String`, mirroring `wire::pick_str` for a single value:
/// non-empty string first, then integer forms. Empty strings and non-scalars yield `None`.
fn value_str(v: &Value) -> Option<String> {
    if let Some(s) = v.as_str() {
        if !s.is_empty() {
            return Some(s.to_string());
        }
    }
    if let Some(n) = v.as_i64() {
        return Some(n.to_string());
    }
    if let Some(n) = v.as_u64() {
        return Some(n.to_string());
    }
    None
}

/// One per-group metadata object inside `gridInfoMap` of `GET /api/group/getmg-v2` (cmd `12028`).
///
/// # Wire Response Example
/// ```json
/// {
///   "name": "Team AloLite",
///   "fullAvt": "https://s120-ava-group.zadn.vn/group.jpg",
///   "totalMember": 12,
///   "creatorId": "1234567890",
///   "adminIds": ["1234567890"],
///   "memVerList": ["1234567890_0", "9876543210_0"]
/// }
/// ```
///
/// # Core Transformation
/// Supplies `name`, `fullAvt`, and `totalMember` to [`Thread`] sidebar metadata, and feeds the
/// member-id ordering used by `extract_all_group_member_ids` (creator, then admins, then the
/// remaining `memVerList` entries).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct GroupInfoWireData {
    #[serde(alias = "groupName", alias = "displayName")]
    pub name: Option<Value>,
    #[serde(rename = "fullAvt", alias = "avt", alias = "avatar")]
    pub full_avt: Option<Value>,
    #[serde(rename = "totalMember")]
    pub total_member: Option<Value>,
    #[serde(rename = "memVerList")]
    pub mem_ver_list: Vec<Value>,
    #[serde(rename = "creatorId")]
    pub creator_id: Option<Value>,
    #[serde(rename = "adminIds")]
    pub admin_ids: Vec<Value>,
}

impl GroupInfoWireData {
    /// Group display name (`name` / `groupName` / `displayName`).
    pub fn display_name(&self) -> Option<String> {
        self.name
            .as_ref()
            .and_then(value_str)
            .filter(|s| !s.is_empty())
    }

    /// Group avatar URL, ignoring upstream `/default` placeholders.
    pub fn avatar(&self) -> Option<String> {
        self.full_avt
            .as_ref()
            .and_then(value_str)
            .filter(|u| !u.ends_with("/default"))
    }

    /// Server-reported total member count (`0` when absent).
    pub fn total_member(&self) -> usize {
        self.total_member
            .as_ref()
            .and_then(Value::as_u64)
            .map(|n| n as usize)
            .unwrap_or(0)
    }
}

/// Minimal user profile returned by `POST /api/social/group/members` (cmd `12030`).
///
/// # Wire Response Example
/// ```json
/// { "zaloName": "Ngọc Trần", "avatar": "https://s120-ava-talk.zadn.vn/a.jpg" }
/// ```
///
/// # Core Transformation
/// Resolves placeholder thread names and avatars in [`resolve_thread_names`]. Field priority for
/// the name mirrors the shipped client: `zaloName` > `displayName` > `name` > `display_name`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct MiniProfileWireData {
    #[serde(rename = "zaloName")]
    pub zalo_name: Option<Value>,
    #[serde(rename = "displayName")]
    pub display_name: Option<Value>,
    #[serde(rename = "name")]
    pub name: Option<Value>,
    #[serde(rename = "display_name")]
    pub display_name_snake: Option<Value>,
    #[serde(rename = "avatar")]
    pub avatar_url: Option<Value>,
    #[serde(rename = "avt")]
    pub avt: Option<Value>,
    #[serde(rename = "profile_avatar")]
    pub profile_avatar: Option<Value>,
}

impl MiniProfileWireData {
    /// Preferred display name across the shipped-client key priority.
    pub fn display_name(&self) -> Option<String> {
        [
            &self.zalo_name,
            &self.display_name,
            &self.name,
            &self.display_name_snake,
        ]
        .into_iter()
        .find_map(|v| v.as_ref().and_then(value_str))
        .filter(|s| !s.is_empty())
    }

    /// Avatar URL, ignoring upstream `/default` placeholders.
    pub fn avatar(&self) -> Option<String> {
        [&self.avatar_url, &self.avt, &self.profile_avatar]
            .into_iter()
            .find_map(|v| v.as_ref().and_then(value_str))
            .filter(|u| !u.is_empty() && !u.ends_with("/default"))
    }
}

/// `GET /api/group/history` (cmd `11240`) — group message history.
///
/// # Errors
/// Returns an error if unauthenticated or both primary and raw probes fail.
pub async fn get_group_history(s: &Session, group_id: &str, count: u32) -> Result<Value> {
    require_login(s)?;
    let grid = strip_group_prefix(group_id);
    let body = obj(&[
        ("grid", json!(grid)),
        ("count", json!(count)),
        ("imei", json!(s.imei)),
    ]);
    let params = match s.encoded_params(&body) {
        Ok(p) => format!("params={p}"),
        Err(e) => {
            crate::logger::log_error("groups::get_group_history::encoded_params", &e.to_string());
            Session::plain_params(&body)
        }
    };
    let url = format!(
        "{}/api/group/history?{}&{params}",
        s.domains.group,
        s.common_params()
    );
    match s.get(&url).await {
        Ok(resp) => Ok(s.maybe_decrypt(resp, s.session_key_bytes()).await),
        Err(_) => get_group_history_raw(s, grid, count).await,
    }
}

/// Byte-exact replica of the shipped bundle's `getHistoryMessage(e, t)` (`grid` + `count` only).
///
/// # Errors
/// Returns an error if unauthenticated or the HTTP request fails.
pub async fn get_group_history_raw(s: &Session, grid: &str, count: u32) -> Result<Value> {
    require_login(s)?;
    let grid = strip_group_prefix(grid);
    let body = obj(&[("grid", json!(grid)), ("count", json!(count))]);
    let params = match s.encoded_params(&body) {
        Ok(p) => format!("params={p}"),
        Err(e) => {
            crate::logger::log_error(
                "groups::get_group_history_raw::encoded_params",
                &e.to_string(),
            );
            Session::plain_params(&body)
        }
    };
    let url = format!(
        "{}/api/group/history?{}&{params}",
        s.domains.group,
        s.common_params()
    );
    let resp = s.get(&url).await?;
    Ok(s.maybe_decrypt(resp, s.session_key_bytes()).await)
}

/// Envelope of `POST /api/group/getmg-v2` (cmd `12028`).
///
/// # Wire Response Example
/// ```json
/// {
///   "error_code": 0,
///   "data": {
///     "gridInfoMap": { "1234567890": { "name": "Team AloLite", "totalMember": 12 } },
///     "removedsGroup": ["9876543210"]
///   }
/// }
/// ```
///
/// # Core Transformation
/// `gridInfoMap` entries become [`GroupInfoWireData`] via [`Self::typed_info_map`];
/// `removedsGroup` becomes a set of group ids removed from the user's list.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct GroupProfilesWireData {
    #[serde(rename = "gridInfoMap")]
    pub grid_info_map: Map<String, Value>,
    #[serde(rename = "removedsGroup")]
    pub removeds_group: Vec<Value>,
}

impl GroupProfilesWireData {
    /// Typed view of `gridInfoMap`; non-object entries degrade to `Default` instead of dropping.
    pub fn typed_info_map(&self) -> BTreeMap<String, GroupInfoWireData> {
        self.grid_info_map
            .iter()
            .map(|(k, v)| {
                (
                    k.clone(),
                    serde_json::from_value::<GroupInfoWireData>(v.clone()).unwrap_or_default(),
                )
            })
            .collect()
    }

    /// Removed group ids (`removedsGroup`), keeping string entries only.
    pub fn removed_ids(&self) -> HashSet<String> {
        self.removeds_group
            .iter()
            .filter_map(Value::as_str)
            .map(str::to_string)
            .collect()
    }
}

/// Fetch group metadata (`name`, `avt`, `totalMember`, `removedsGroup`) from `POST /api/group/getmg-v2`.
///
/// # Errors
/// Returns an error if encryption or the HTTP call fails.
pub async fn fetch_group_profiles(
    s: &Session,
    group_ids: &[String],
) -> Result<(BTreeMap<String, GroupInfoWireData>, HashSet<String>)> {
    if s.login.is_none() || group_ids.is_empty() {
        return Ok((BTreeMap::new(), HashSet::new()));
    }
    let mut gv_zero = Map::new();
    for gid in group_ids {
        let clean = strip_group_prefix(gid).trim();
        if !clean.is_empty() {
            gv_zero.insert(clean.to_string(), json!(0));
        }
    }
    let gv_str = serde_json::to_string(&gv_zero)?;
    let body = obj(&[("gridVerMap", json!(gv_str)), ("imei", json!(s.imei))]);
    let params = s.encoded_params(&body)?;
    let mg_url = format!(
        "{}/api/group/getmg-v2?{}",
        s.domains.group,
        s.common_params()
    );
    let mg_resp = s.post_form(&mg_url, format!("params={params}")).await?;
    let mut mg_dec = s.maybe_decrypt(mg_resp, s.session_key_bytes()).await;
    if let Some(d_str) = mg_dec.get("data").and_then(Value::as_str) {
        if let Ok(parsed) = serde_json::from_str::<Value>(d_str.trim()) {
            mg_dec["data"] = parsed;
        }
    }
    let data_node = unwrap_data(&mg_dec);
    let wire: GroupProfilesWireData = serde_json::from_value(data_node.clone()).unwrap_or_default();
    Ok((wire.typed_info_map(), wire.removed_ids()))
}

fn extract_all_group_member_ids(info: &GroupInfoWireData) -> (Vec<String>, usize) {
    let mut raw_ids: Vec<String> = Vec::new();
    for item in &info.mem_ver_list {
        if let Some(s) = item.as_str() {
            let uid = s.split('_').next().unwrap_or("").trim();
            if !uid.is_empty() && !raw_ids.iter().any(|x| x == uid) {
                raw_ids.push(uid.to_string());
            }
        }
    }
    let mut sorted: Vec<String> = Vec::new();
    if let Some(creator) = info.creator_id.as_ref().and_then(value_str) {
        if raw_ids.iter().any(|x| x == &creator) {
            sorted.push(creator);
        }
    }
    for a in &info.admin_ids {
        let aid = value_str(a).unwrap_or_default();
        if !aid.is_empty() && raw_ids.iter().any(|x| x == &aid) && !sorted.iter().any(|x| x == &aid)
        {
            sorted.push(aid);
        }
    }
    for uid in &raw_ids {
        if !sorted.iter().any(|x| x == uid) {
            sorted.push(uid.clone());
        }
    }
    let total = info.total_member().max(sorted.len());
    sorted.truncate(100);
    (sorted, total)
}

fn extract_group_member_ids(info: &GroupInfoWireData) -> (Vec<String>, usize) {
    let (mut sorted, total) = extract_all_group_member_ids(info);
    sorted.truncate(4);
    (sorted, total)
}

fn format_multi_group_avatar(
    group_name: &str,
    mem_ids: &[String],
    total_members: usize,
    profiles: &BTreeMap<String, MiniProfileWireData>,
) -> Option<String> {
    let fallback_names: Vec<&str> = group_name
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect();
    let mut slots: Vec<String> = Vec::new();
    for (idx, uid) in mem_ids.iter().take(4).enumerate() {
        let mut m_name = fallback_names
            .get(idx)
            .copied()
            .unwrap_or("Thành viên")
            .to_string();
        let mut m_ava = String::new();
        if let Some(p) = profiles.get(uid) {
            if let Some(n) = p.display_name() {
                if !n.is_empty() {
                    m_name = n;
                }
            }
            if let Some(a) = p.avatar() {
                if !a.is_empty() && !a.ends_with("/default") {
                    m_ava = a;
                }
            }
        }
        let clean_name = m_name.replace([';', '|'], " ");
        let clean_ava = m_ava.replace([';', '|'], "");
        slots.push(format!("{clean_name}|{clean_ava}"));
    }
    if slots.is_empty() {
        return None;
    }
    if fallback_names.len() >= 2 {
        slots.sort_by_key(|slot| {
            let s_name = slot.split('|').next().unwrap_or("").trim();
            let s_fold = strip_vn_diacritics(s_name);
            fallback_names
                .iter()
                .position(|cand| {
                    let c_fold = strip_vn_diacritics(cand);
                    !c_fold.is_empty()
                        && (c_fold == s_fold
                            || s_fold.contains(&c_fold)
                            || c_fold.contains(&s_fold))
                })
                .unwrap_or(99)
        });
    }
    let tot = total_members.max(slots.len());
    Some(format!("multi:{tot};{}", slots.join(";")))
}

/// `GET /api/group/getlg/v4` (cmd `12027`) — list all groups user belongs to.
///
/// # Errors
/// Returns an error if the HTTP request fails.
pub async fn get_groups(s: &Session) -> Result<Vec<Thread>> {
    if s.login.is_none() {
        return Ok(Vec::new());
    }
    let url = format!(
        "{}/api/group/getlg/v4?{}",
        s.domains.group,
        s.common_params()
    );
    let resp = s.get(&url).await?;
    let mut resp = s.maybe_decrypt(resp, s.session_key_bytes()).await;
    if let Some(d_str) = resp.get("data").and_then(Value::as_str) {
        if let Ok(parsed) = serde_json::from_str::<Value>(d_str.trim()) {
            resp["data"] = parsed;
        }
    }
    let data_node = unwrap_data(&resp);

    let mut groups = Vec::new();
    if let Some(map) = data_node.get("gridVerMap").and_then(Value::as_object) {
        for grid in map.keys() {
            groups.push(Thread {
                id: grid.clone(),
                name: format!("Nhóm {grid}"),
                is_group: true,
                ..Default::default()
            });
        }
        if !groups.is_empty() {
            let gids: Vec<String> = groups.iter().map(|g| g.id.clone()).collect();
            if let Ok((info_map, rem_ids)) = fetch_group_profiles(s, &gids).await {
                let mut need_mem_uids: Vec<String> = Vec::new();
                for g in &mut groups {
                    if let Some(info) = info_map.get(&g.id) {
                        if let Some(name) = info.display_name() {
                            g.name = name;
                        }
                        if let Some(avt) = info.avatar() {
                            g.avatar = Some(avt);
                        } else {
                            let (mids, _) = extract_group_member_ids(info);
                            for uid in mids {
                                if !need_mem_uids.iter().any(|x| x == &uid) {
                                    need_mem_uids.push(uid);
                                }
                            }
                        }
                        if g.preview.is_empty() {
                            let n_mem = info.total_member();
                            if n_mem > 0 {
                                g.preview = format!("Nhóm · {n_mem} thành viên");
                            }
                        }
                    }
                }
                if !need_mem_uids.is_empty() {
                    let mut all_profiles: BTreeMap<String, MiniProfileWireData> = BTreeMap::new();
                    for chunk in need_mem_uids.chunks(50) {
                        if let Ok(mp) = get_mini_profiles(s, chunk).await {
                            for (k, v) in mp {
                                all_profiles.insert(k, v);
                            }
                        }
                    }
                    for g in &mut groups {
                        if g.avatar.is_none() {
                            if let Some(info) = info_map.get(&g.id) {
                                let (mids, tot) = extract_group_member_ids(info);
                                g.avatar =
                                    format_multi_group_avatar(&g.name, &mids, tot, &all_profiles);
                            }
                        }
                    }
                }
                if !rem_ids.is_empty() {
                    groups.retain(|g| !rem_ids.contains(&g.id));
                }
            }
        }
    }
    Ok(groups)
}

/// `POST /api/social/group/members` (cmd `12030`) — fetch member display names and avatars.
///
/// # Wire Response Example
/// ```json
/// {
///   "error_code": 0,
///   "data": {
///     "profiles": {
///       "1234567890": { "zaloName": "Ngọc Trần", "avatar": "https://s120-ava-talk.zadn.vn/a.jpg" }
///     }
///   }
/// }
/// ```
/// `data` may nest `{"data": {"data": {...}}}` or hold the profile map directly; both shapes are
/// normalised with [`unwrap_data`].
///
/// # Core Transformation
/// Returns a map of user id → [`MiniProfileWireData`] used to resolve placeholder thread names
/// and avatars. Non-object entries degrade to `Default` instead of dropping the whole batch.
///
/// # Errors
/// Returns an error if encryption or the HTTP request fails.
pub async fn get_mini_profiles(
    s: &Session,
    user_ids: &[String],
) -> Result<BTreeMap<String, MiniProfileWireData>> {
    if s.login.is_none() || user_ids.is_empty() {
        return Ok(BTreeMap::new());
    }
    let versioned_ids: Vec<String> = user_ids
        .iter()
        .map(|uid| {
            let clean = uid.trim();
            if clean.contains('_') {
                clean.to_string()
            } else {
                format!("{clean}_0")
            }
        })
        .collect();
    let body = obj(&[("friend_pversion_map", json!(versioned_ids))]);
    let params = s.encoded_params(&body)?;
    let url = format!(
        "{}/api/social/group/members?{}",
        s.domains.profile,
        s.common_params()
    );
    let resp = s.post_form(&url, format!("params={params}")).await?;
    let mut resp = s.maybe_decrypt(resp, s.session_key_bytes()).await;
    if let Some(d_str) = resp.get("data").and_then(Value::as_str) {
        if let Ok(parsed) = serde_json::from_str::<Value>(d_str.trim()) {
            resp["data"] = parsed;
        }
    }
    if resp.get("data").is_none() {
        return Ok(BTreeMap::new());
    }
    let inner = unwrap_data(&resp);
    let node = inner.get("profiles").unwrap_or(inner);
    let raw = node.as_object().cloned().unwrap_or_default();
    Ok(raw
        .iter()
        .map(|(k, v)| {
            (
                k.clone(),
                serde_json::from_value::<MiniProfileWireData>(v.clone()).unwrap_or_default(),
            )
        })
        .collect())
}

/// Resolve real display names and avatars for 1-1 and group threads with placeholder names.
pub async fn resolve_thread_names(s: &Session, threads: &mut [Thread]) {
    let unknown_ids: Vec<String> = threads
        .iter()
        .filter(|t| {
            !t.is_group
                && !t.name.contains("Cloud của tôi")
                && (t.name.starts_with("Người dùng ")
                    || t.name.is_empty()
                    || t.avatar.as_deref().unwrap_or("").is_empty())
        })
        .map(|t| t.id.clone())
        .collect();

    if !unknown_ids.is_empty() {
        for chunk in unknown_ids.chunks(50) {
            if let Ok(profiles) = get_mini_profiles(s, chunk).await {
                for t in threads.iter_mut() {
                    if let Some(p) = profiles.get(&t.id) {
                        if t.name.starts_with("Người dùng ") || t.name.is_empty() {
                            if let Some(name) = p.display_name() {
                                if !name.is_empty() {
                                    t.name = name;
                                }
                            }
                        }
                        if let Some(ava) = p.avatar() {
                            if !ava.is_empty() {
                                t.avatar = Some(ava);
                            }
                        }
                    }
                }
            }
        }
    }

    let is_numeric_uid = |val: &str| -> bool {
        let trimmed = val.trim();
        trimmed.len() >= 6 && trimmed.chars().all(|c| c.is_ascii_digit())
    };

    let unknown_group_ids: Vec<String> = threads
        .iter()
        .filter(|t| {
            t.is_group
                && (is_generic_thread_name(&t.name)
                    || t.preview.is_empty()
                    || t.avatar.as_deref().unwrap_or("").is_empty()
                    || t.presentation.members.is_empty())
        })
        .map(|t| t.id.clone())
        .collect();

    let mut all_profiles: BTreeMap<String, MiniProfileWireData> = BTreeMap::new();
    if let Some(login) = s.login.as_ref() {
        if !login.uin.is_empty() && !login.display_name.is_empty() {
            all_profiles.insert(
                login.uin.clone(),
                MiniProfileWireData {
                    display_name: Some(Value::String(login.display_name.clone())),
                    avatar_url: Some(Value::String(login.avatar.clone())),
                    ..Default::default()
                },
            );
        }
    }

    let mut need_mem_uids: Vec<String> = Vec::new();
    for t in threads.iter_mut() {
        for m in &mut t.messages {
            if is_numeric_uid(&m.sender) {
                if m.sender_id.is_none() {
                    m.sender_id = Some(m.sender.trim().to_string());
                }
                let uid = m.sender.trim().to_string();
                if !need_mem_uids.iter().any(|x| x == &uid) {
                    need_mem_uids.push(uid);
                }
            } else if let Some(sid) = m.sender_id.as_ref() {
                if is_numeric_uid(sid) && !need_mem_uids.iter().any(|x| x == sid) {
                    need_mem_uids.push(sid.clone());
                }
            }
            if let Some(rs) = m.reply_to_sender.as_ref() {
                if is_numeric_uid(rs) && !need_mem_uids.iter().any(|x| x == rs) {
                    need_mem_uids.push(rs.clone());
                }
            }
        }
    }

    let mut group_info_map: BTreeMap<String, GroupInfoWireData> = BTreeMap::new();
    if !unknown_group_ids.is_empty() {
        for chunk in unknown_group_ids.chunks(50) {
            if let Ok((info_map, _)) = fetch_group_profiles(s, chunk).await {
                for (k, v) in info_map {
                    let (all_mids, _) = extract_all_group_member_ids(&v);
                    for uid in all_mids {
                        if !need_mem_uids.iter().any(|x| x == &uid) {
                            need_mem_uids.push(uid);
                        }
                    }
                    group_info_map.insert(k, v);
                }
            }
        }
    }

    if !need_mem_uids.is_empty() {
        for mchunk in need_mem_uids.chunks(50) {
            if let Ok(mp) = get_mini_profiles(s, mchunk).await {
                for (k, v) in mp {
                    all_profiles.insert(k, v);
                }
            }
        }
    }

    for t in threads.iter_mut() {
        if t.is_group {
            let clean_id = strip_group_prefix(&t.id).to_string();
            if let Some(info) = group_info_map
                .get(&t.id)
                .or_else(|| group_info_map.get(&clean_id))
            {
                if let Some(name) = info.display_name() {
                    if !name.is_empty() {
                        t.name = name;
                    }
                }
                if let Some(avt) = info.avatar() {
                    t.avatar = Some(avt);
                } else {
                    let cur = t.avatar.as_deref().unwrap_or("");
                    if cur.is_empty() || cur.starts_with("multi:") {
                        let (mids, tot) = extract_group_member_ids(info);
                        if let Some(multi_s) =
                            format_multi_group_avatar(&t.name, &mids, tot, &all_profiles)
                        {
                            if should_replace_avatar(t.avatar.as_deref(), Some(&multi_s)) {
                                t.avatar = Some(multi_s);
                            }
                        }
                    }
                }
                let (all_mids, tot) = extract_all_group_member_ids(info);
                if tot > 0 {
                    t.presentation.member_count = Some(tot as u64);
                    if t.preview.is_empty() {
                        t.preview = format!("Nhóm · {tot} thành viên");
                    }
                }
                for uid in all_mids {
                    if let Some(p) = all_profiles.get(&uid) {
                        let name = p.display_name().unwrap_or_else(|| uid.clone());
                        let avatar = p.avatar();
                        if let Some(existing_mem) =
                            t.presentation.members.iter_mut().find(|m| m.id == uid)
                        {
                            if !name.is_empty() {
                                existing_mem.name = name;
                            }
                            if avatar.is_some() {
                                existing_mem.avatar = avatar;
                            }
                        } else {
                            t.presentation.members.push(SidebarMember {
                                id: uid,
                                name,
                                avatar,
                            });
                        }
                    }
                }
            }
        }

        for m in &mut t.messages {
            let lookup_uid = if is_numeric_uid(&m.sender) {
                Some(m.sender.trim().to_string())
            } else {
                m.sender_id.clone().filter(|sid| is_numeric_uid(sid))
            };
            if let Some(uid) = lookup_uid {
                if m.sender_id.is_none() {
                    m.sender_id = Some(uid.clone());
                }
                if let Some(p) = all_profiles.get(&uid) {
                    if let Some(name) = p.display_name() {
                        if !name.is_empty() {
                            m.sender = name.clone();
                        }
                        let avatar = p.avatar();
                        if t.is_group && !t.presentation.members.iter().any(|mem| mem.id == uid) {
                            t.presentation.members.push(SidebarMember {
                                id: uid,
                                name,
                                avatar,
                            });
                        }
                    }
                } else if let Some(mem) = t.presentation.members.iter().find(|mem| mem.id == uid) {
                    if !mem.name.is_empty() && !is_numeric_uid(&mem.name) {
                        m.sender = mem.name.clone();
                    }
                }
            }
            if let Some(rs) = m.reply_to_sender.clone() {
                if is_numeric_uid(&rs) {
                    if let Some(p) = all_profiles.get(rs.trim()) {
                        if let Some(name) = p.display_name() {
                            if !name.is_empty() {
                                m.reply_to_sender = Some(name);
                            }
                        }
                    }
                }
            }
        }
    }
    let known_contacts: Vec<(String, String)> = threads
        .iter()
        .filter(|t| {
            !t.is_group
                && !t.name.contains("Cloud của tôi")
                && !t.avatar.as_deref().unwrap_or("").is_empty()
                && !t.avatar.as_deref().unwrap_or("").ends_with("/default")
        })
        .map(|t| (t.name.clone(), t.avatar.clone().unwrap_or_default()))
        .collect();

    for t in threads.iter_mut() {
        if !t.is_group {
            continue;
        }
        let cur_ava = t.avatar.as_deref().unwrap_or("");
        if avatar_http_url_count(Some(cur_ava)) > 0 {
            continue;
        }
        let parts: Vec<&str> = t
            .name
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .collect();
        if parts.len() >= 2 {
            let mut slots: Vec<String> = Vec::new();
            for p in parts.iter().take(4) {
                let p_fold = strip_vn_diacritics(p);
                let matched_url = known_contacts
                    .iter()
                    .find(|(cname, _)| {
                        let c_fold = strip_vn_diacritics(cname);
                        !p_fold.is_empty() && c_fold == p_fold
                    })
                    .map(|(_, url)| url.as_str())
                    .unwrap_or("");
                slots.push(format!("{}|{}", p.replace([';', '|'], " "), matched_url));
            }
            let new_multi = format!("multi:{};{}", parts.len(), slots.join(";"));
            if should_replace_avatar(t.avatar.as_deref(), Some(&new_multi)) {
                t.avatar = Some(new_multi);
            }
        }
    }
}
