//! Friend roster, phone lookup, friendship status, and online presence REST operations.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use super::groups::{get_groups, resolve_thread_names};
use crate::clock::{message_time, unix_now};
use crate::protocol::messaging::threads::{merge_thread_into, Thread};
use crate::protocol::session::Session;
use crate::protocol::wire::{
    domain_or, is_group_flag, obj, pick_str, require_login, unwrap_data, val_i64, ActionAck,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum FriendshipStatus {
    #[default]
    Unknown,
    Friend,
    OutgoingPending,
    IncomingPending,
    Stranger,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhoneLookupResult {
    pub user_id: String,
    pub display_name: String,
    pub zalo_name: String,
    pub phone: String,
    pub avatar: String,
    pub status: String,
    #[serde(default)]
    pub friendship: FriendshipStatus,
}

fn default_friend_host(s: &Session) -> &str {
    let fallback = crate::endpoints::service("friend").unwrap_or_default();
    domain_or(&s.domains.friend, fallback)
}

fn default_profile_host(s: &Session) -> &str {
    let fallback = crate::endpoints::service("profile").unwrap_or_default();
    domain_or(&s.domains.profile, fallback)
}

pub(crate) fn parse_thread(v: &Value) -> Option<Thread> {
    let id = pick_str(v, &["userId", "uid", "id", "friendId", "globalId"])?;
    let name = pick_str(v, &["displayName", "zaloName", "display_name", "name"])
        .or_else(|| {
            v.get("profile_displayName")
                .or_else(|| v.get("profile").and_then(|p| p.get("displayName")))
                .and_then(Value::as_str)
                .map(str::to_string)
        })
        .unwrap_or_else(|| id.clone());
    let avatar = pick_str(v, &["avatar", "avt", "profile_avatar"]);
    let is_group = is_group_flag(v);
    let time = val_i64(v.get("lastActionTime"))
        .map(message_time)
        .unwrap_or_default();

    Some(Thread {
        id,
        name,
        time,
        avatar,
        is_group,
        ..Default::default()
    })
}

/// `/api/social/friend/getfriends` — fetch friends and merge groups, without any local history.
///
/// # Errors
/// Returns an error if unauthenticated or the friend endpoint fails across all candidates.
pub async fn get_friends(s: &Session) -> Result<Vec<Thread>> {
    get_friends_with(s, |_| {}).await
}

/// Like [`get_friends`], but runs `preload` on the parsed friend list before groups are merged.
///
/// The hook is the seam that lets the caller hydrate local history (SQLite lives outside
/// `protocol/`) at the exact point the former in-protocol preload ran.
///
/// # Errors
/// Returns an error if unauthenticated or the friend endpoint fails across all candidates.
pub async fn get_friends_with<F>(s: &Session, preload: F) -> Result<Vec<Thread>>
where
    F: FnOnce(&mut Vec<Thread>),
{
    require_login(s)?;
    let body = obj(&[
        ("incInvalid", json!(1)),
        ("page", json!(1)),
        ("count", json!(20000)),
        ("avatar_size", json!(160)),
        ("actiontime", json!(0)),
        ("imei", json!(s.imei)),
    ]);
    let params = match s.encoded_params(&body) {
        Ok(p) => format!("params={p}"),
        Err(e) => {
            crate::logger::log_error("friends::get_friends::encoded_params", &e.to_string());
            Session::plain_params(&body)
        }
    };
    let path_and_query = format!(
        "/api/social/friend/getfriends?{}{}",
        s.common_params(),
        if params.is_empty() {
            String::new()
        } else {
            format!("&{params}")
        }
    );
    let raw = s
        .get_endpoint("friend", &path_and_query)
        .await
        .map_err(|e| e.context("không tải được danh sách bạn bè"))?;
    let resp = s.maybe_decrypt(raw, s.session_key_bytes()).await;
    let data_root = resp.get("data").unwrap_or(&resp);
    let mut threads: Vec<Thread> = match data_root {
        Value::Array(items) => items.iter().filter_map(parse_thread).collect(),
        Value::Object(o) if o.contains_key("friends") => o["friends"]
            .as_array()
            .map(|a| a.iter().filter_map(parse_thread).collect())
            .unwrap_or_default(),
        Value::Object(o) if o.get("data").and_then(Value::as_array).is_some() => o["data"]
            .as_array()
            .map(|a| a.iter().filter_map(parse_thread).collect())
            .unwrap_or_default(),
        _ => Vec::new(),
    };

    if let Some(li) = &s.login {
        if !li.send2me_id.is_empty() && !threads.iter().any(|t| t.id == li.send2me_id) {
            threads.insert(
                0,
                Thread {
                    id: li.send2me_id.clone(),
                    name: "Cloud của tôi (Lưu trữ)".into(),
                    ..Default::default()
                },
            );
        }
    }

    preload(&mut threads);

    if let Ok(mut groups) = get_groups(s).await {
        for g in groups.drain(..) {
            if let Some(existing) = threads.iter_mut().find(|t| t.id == g.id) {
                merge_thread_into(existing, g, true);
            } else {
                threads.push(g);
            }
        }
    }

    resolve_thread_names(s, &mut threads).await;

    Ok(threads)
}

/// Parse `/api/friend/reqstatus` (`cmd 11430`) JSON into [`FriendshipStatus`].
pub fn parse_friendship_status_response(resp: &Value) -> FriendshipStatus {
    let d = unwrap_data(resp);
    let read_flag = |k1: &str, k2: &str| {
        d.get(k1)
            .or_else(|| d.get(k2))
            .and_then(|v| {
                v.as_i64()
                    .or_else(|| v.as_bool().map(|b| if b { 1 } else { 0 }))
            })
            .unwrap_or(0)
    };
    if read_flag("is_friend", "isFriend") == 1 {
        FriendshipStatus::Friend
    } else if read_flag("is_requesting", "isRequesting") == 1 {
        FriendshipStatus::IncomingPending
    } else if read_flag("is_requested", "isRequested") == 1 {
        FriendshipStatus::OutgoingPending
    } else {
        FriendshipStatus::Stranger
    }
}

/// Query `/api/friend/reqstatus` (`cmd 11430`) and return typed [`FriendshipStatus`].
///
/// # Errors
/// Returns an error if unauthenticated or the HTTP call fails.
pub async fn check_friendship_status(s: &Session, fid: &str) -> Result<FriendshipStatus> {
    let resp = get_friend_request_status(s, fid).await?;
    Ok(parse_friendship_status_response(&resp))
}

/// Check raw friend request status with `fid` (`/api/friend/reqstatus`, cmd `11430`).
///
/// # Errors
/// Returns an error if unauthenticated, encryption fails, or the HTTP call fails.
pub async fn get_friend_request_status(s: &Session, fid: &str) -> Result<Value> {
    require_login(s)?;
    let body = obj(&[("fid", json!(fid)), ("imei", json!(s.imei))]);
    let enc = s.encoded_params(&body)?;
    let host = default_friend_host(s);
    let url = format!(
        "{host}/api/friend/reqstatus?{}&params={enc}",
        s.common_params()
    );
    let resp = s.get(&url).await?;
    Ok(s.maybe_decrypt(resp, s.session_key_bytes()).await)
}

/// Query `/api/social/profile/lastOnline` (`cmd 12090`) and return `(is_online, label)`.
///
/// # Errors
/// Returns an error if the HTTP request fails.
pub async fn get_user_online_status(s: &Session, uid: &str) -> Result<(bool, String)> {
    if s.login.is_none() || uid.trim().is_empty() {
        return Ok((false, String::new()));
    }
    let body = obj(&[
        ("uid", json!(uid.trim())),
        ("conv_type", json!(1)),
        ("imei", json!(s.imei)),
    ]);
    let enc = s.encoded_params(&body)?;
    let host = default_profile_host(s);
    let url = format!(
        "{host}/api/social/profile/lastOnline?{}&params={enc}",
        s.common_params()
    );
    let resp = s.get(&url).await?;
    let mut dec = s.maybe_decrypt(resp, s.session_key_bytes()).await;
    if let Some(d_str) = dec.get("data").and_then(Value::as_str) {
        if let Ok(parsed) = serde_json::from_str::<Value>(d_str.trim()) {
            dec["data"] = parsed;
        }
    }
    let d = unwrap_data(&dec);
    let show_online = d
        .pointer("/settings/show_online_status")
        .and_then(|v| v.as_bool().or_else(|| v.as_i64().map(|n| n != 0)))
        .unwrap_or(true);
    if !show_online {
        return Ok((false, String::new()));
    }
    let last_onl_raw = val_i64(d.get("lastOnline")).unwrap_or(0);
    if last_onl_raw <= 0 {
        return Ok((false, String::new()));
    }
    let last_sec = if last_onl_raw > 10_000_000_000 {
        last_onl_raw / 1000
    } else {
        last_onl_raw
    };
    let now_sec = unix_now();
    let diff_sec = (now_sec - last_sec).max(0);
    if diff_sec <= 300 {
        return Ok((true, "Vừa mới truy cập".into()));
    }
    let mins = diff_sec / 60;
    if mins < 60 {
        return Ok((false, format!("Truy cập {mins} phút trước")));
    }
    let hours = diff_sec / 3600;
    if hours < 24 {
        return Ok((false, format!("Truy cập {hours} giờ trước")));
    }
    let days = (diff_sec / 86400).min(7);
    Ok((false, format!("Truy cập {days} ngày trước")))
}

/// Send a friend request (`/api/friend/sendreq`, cmd `11424`).
///
/// # Errors
/// Returns an error if unauthenticated or the server returns a non-zero `error_code`.
pub async fn send_friend_request(s: &Session, to_id: &str, msg: &str) -> Result<ActionAck> {
    require_login(s)?;
    let body = obj(&[
        ("toid", json!(to_id)),
        ("msg", json!(msg)),
        ("reqsrc", json!(30)),
        ("imei", json!(s.imei)),
        ("language", json!("vi")),
    ]);
    let enc = s.encoded_params(&body)?;
    let host = default_friend_host(s);
    let url = format!("{host}/api/friend/sendreq?{}", s.common_params());
    let resp = s.post_form(&url, format!("params={enc}")).await?;
    let dec = s.maybe_decrypt(resp, s.session_key_bytes()).await;
    crate::protocol::wire::ensure_ok_code(&dec, "sendreq error")?;
    Ok(ActionAck::from_wire(&dec))
}

/// Accept a pending incoming friend request (`/api/friend/accept`, cmd `11427`).
///
/// # Errors
/// Returns an error if unauthenticated or the server returns a non-zero `error_code`.
pub async fn accept_friend_request(s: &Session, fid: &str) -> Result<ActionAck> {
    require_login(s)?;
    let body = obj(&[("fid", json!(fid)), ("language", json!("vi"))]);
    let enc = s.encoded_params(&body)?;
    let host = default_friend_host(s);
    let url = format!("{host}/api/friend/accept?{}", s.common_params());
    let resp = s.post_form(&url, format!("params={enc}")).await?;
    let dec = s.maybe_decrypt(resp, s.session_key_bytes()).await;
    crate::protocol::wire::ensure_ok_code(&dec, "accept error")?;
    Ok(ActionAck::from_wire(&dec))
}

/// Cancel / undo a pending outgoing friend request (`/api/friend/undo`, cmd `11441`).
///
/// # Errors
/// Returns an error if unauthenticated or the HTTP request fails.
pub async fn undo_friend_request(s: &Session, fid: &str) -> Result<ActionAck> {
    require_login(s)?;
    let body = obj(&[("fid", json!(fid)), ("imei", json!(s.imei))]);
    let enc = s.encoded_params(&body)?;
    let host = default_friend_host(s);
    let url = format!("{host}/api/friend/undo?{}", s.common_params());
    let resp = s.post_form(&url, format!("params={enc}")).await?;
    let dec = s.maybe_decrypt(resp, s.session_key_bytes()).await;
    Ok(ActionAck::from_wire(&dec))
}

/// Reject a pending incoming friend request (`/api/friend/reject`, cmd `11428`).
///
/// # Errors
/// Returns an error if unauthenticated or the HTTP request fails.
pub async fn reject_friend_request(s: &Session, fid: &str) -> Result<ActionAck> {
    require_login(s)?;
    let body = obj(&[("fid", json!(fid)), ("imei", json!(s.imei))]);
    let enc = s.encoded_params(&body)?;
    let host = default_friend_host(s);
    let url = format!("{host}/api/friend/reject?{}", s.common_params());
    let resp = s.post_form(&url, format!("params={enc}")).await?;
    let dec = s.maybe_decrypt(resp, s.session_key_bytes()).await;
    Ok(ActionAck::from_wire(&dec))
}

/// Normalize a Vietnamese or international phone string for `/api/friend/profile/get`.
pub fn normalize_phone_for_lookup(raw: &str) -> String {
    let digits: String = raw.chars().filter(|c| c.is_ascii_digit()).collect();
    if let Some(rest) = digits.strip_prefix('0') {
        format!("84{rest}")
    } else {
        digits
    }
}

/// Look up a Zalo user profile by phone number (`getUserByPhone`, cmd `11420`).
///
/// # Errors
/// Returns an error if unauthenticated, the phone number is too short, or no account matches.
pub async fn find_user_by_phone(s: &Session, raw_phone: &str) -> Result<PhoneLookupResult> {
    require_login(s)?;
    let normalized = normalize_phone_for_lookup(raw_phone);
    if normalized.len() < 8 {
        anyhow::bail!("Số điện thoại không hợp lệ (cần ít nhất 9 chữ số)");
    }
    let body = obj(&[
        ("phone", json!(normalized)),
        ("avatar_size", json!(240)),
        ("language", json!("vi")),
        ("imei", json!(s.imei)),
        ("reqSrc", json!(40)),
    ]);
    let enc = s.encoded_params(&body)?;
    let host = default_friend_host(s);
    let url = format!(
        "{host}/api/friend/profile/get?{}&params={enc}",
        s.common_params()
    );
    let resp = s.get(&url).await?;
    let dec = s.maybe_decrypt(resp, s.session_key_bytes()).await;
    if let Some(code) = dec.get("error_code").and_then(Value::as_i64) {
        if code != 0 {
            if code == 216 {
                anyhow::bail!(
                    "Số điện thoại chưa đăng ký Zalo hoặc người dùng đã chặn tìm kiếm qua SĐT (mã 216)"
                );
            }
            let err_msg = dec
                .get("error_message")
                .and_then(Value::as_str)
                .unwrap_or("Không tìm thấy tài khoản");
            anyhow::bail!("{err_msg} (mã {code})");
        }
    }

    let data = dec.get("data").unwrap_or(&dec);
    let Some(uid) = pick_str(data, &["uid", "userId", "id", "globalId"]) else {
        anyhow::bail!("Không tìm thấy tài khoản Zalo ứng với số điện thoại này");
    };
    let display_name = pick_str(
        data,
        &["display_name", "displayName", "zalo_name", "zaloName"],
    )
    .unwrap_or_else(|| raw_phone.trim().to_string());
    let zalo_name = pick_str(
        data,
        &["zalo_name", "zaloName", "display_name", "displayName"],
    )
    .unwrap_or_else(|| display_name.clone());
    let avatar = pick_str(data, &["avatar", "avt"]).unwrap_or_default();
    let status = pick_str(data, &["status"]).unwrap_or_default();

    let friendship = check_friendship_status(s, &uid)
        .await
        .unwrap_or(FriendshipStatus::Unknown);

    Ok(PhoneLookupResult {
        user_id: uid,
        display_name,
        zalo_name,
        phone: raw_phone.trim().to_string(),
        avatar,
        status,
        friendship,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_friendship_status_response_maps_all_states() {
        for (flags, expected) in [
            ((1, 0, 0), FriendshipStatus::Friend),
            ((0, 0, 1), FriendshipStatus::IncomingPending),
            ((0, 1, 0), FriendshipStatus::OutgoingPending),
            ((0, 0, 0), FriendshipStatus::Stranger),
        ] {
            let r = json!({
                "error_code": 0,
                "data": { "is_friend": flags.0, "is_requested": flags.1, "is_requesting": flags.2 }
            });
            assert_eq!(parse_friendship_status_response(&r), expected);
        }
    }

    #[test]
    fn parse_thread_handles_boolean_is_group_and_string_time() {
        let val = json!({
            "userId": "grp_test",
            "displayName": "Nhom Chem Gio",
            "isGroup": true,
            "lastActionTime": "1729505000000"
        });
        let thread = parse_thread(&val).unwrap();
        assert!(thread.is_group);
        assert_eq!(thread.name, "Nhom Chem Gio");
        assert!(!thread.time.is_empty());
    }
}
