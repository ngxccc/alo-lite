//! Friend roster, phone lookup, friendship status, and online presence REST operations.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use super::groups::{get_groups, resolve_thread_names};
use crate::clock::{message_time, unix_now};
use crate::protocol::messaging::threads::{merge_thread_into, Thread};
use crate::protocol::session::Session;
use crate::protocol::wire::{
    domain_or, is_group_flag, obj, require_login, unwrap_data, val_i64, ActionAck, WireData,
};

/// Friendship status between current account and target user.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, specta::Type)]
pub enum FriendshipStatus {
    #[default]
    Unknown,
    Friend,
    OutgoingPending,
    IncomingPending,
    Stranger,
}

/// Result of a user profile lookup by phone number (`/api/friend/profile/get`, cmd `11003`).
///
/// # Wire Response Example
/// ```json
/// {
///   "error_code": 0,
///   "error_message": "Successful",
///   "data": {
///     "userId": "123456789012345",
///     "displayName": "Ngọc Trần",
///     "zaloName": "ngoctran",
///     "phoneNumber": "+84901234567",
///     "avatar": "https://s120-ava-talk.zadn.vn/avatar.jpg",
///     "status": "Hello AloLite",
///     "isFriend": 1
///   }
/// }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
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

/// Ordered alias lookup over optional wire fields, mirroring [`crate::protocol::wire::pick_str`] semantics.
fn pick_alias(fields: &[&Option<Value>]) -> Option<String> {
    fields
        .iter()
        .find_map(|field| crate::protocol::wire::field_str(field.as_ref()))
}

/// One contact entry of `/api/social/friend/getfriends` (cmd `11137`).
///
/// # Wire Response Example
/// ```json
/// {
///   "userId": "1234567890",
///   "displayName": "Ngọc Trần",
///   "zaloName": "ngoctran",
///   "avatar": "https://avatar.example/240",
///   "lastActionTime": 1731215400000,
///   "isGroup": 0
/// }
/// ```
///
/// # Core Transformation
/// Produces a [`Thread`] for the friend roster; group entries are detected separately by
/// [`is_group_flag`] because the flag has several wire spellings.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct FriendWireData {
    #[serde(rename = "userId")]
    pub user_id: Option<Value>,
    #[serde(rename = "uid")]
    pub uid: Option<Value>,
    #[serde(rename = "id")]
    pub id: Option<Value>,
    #[serde(rename = "friendId")]
    pub friend_id: Option<Value>,
    #[serde(rename = "globalId")]
    pub global_id: Option<Value>,
    #[serde(rename = "displayName")]
    pub display_name_camel: Option<Value>,
    #[serde(rename = "zaloName")]
    pub zalo_name: Option<Value>,
    #[serde(rename = "display_name")]
    pub display_name_snake: Option<Value>,
    #[serde(rename = "name")]
    pub name: Option<Value>,
    #[serde(rename = "profile_displayName")]
    pub profile_display_name: Option<Value>,
    #[serde(rename = "profile")]
    pub profile: Option<Value>,
    #[serde(rename = "avatar")]
    pub avatar_url: Option<Value>,
    #[serde(rename = "avt")]
    pub avt: Option<Value>,
    #[serde(rename = "profile_avatar")]
    pub profile_avatar: Option<Value>,
    #[serde(rename = "lastActionTime")]
    pub last_action_time: Option<Value>,
}

impl FriendWireData {
    /// Contact id (`userId` > `uid` > `id` > `friendId` > `globalId`).
    pub fn id(&self) -> Option<String> {
        pick_alias(&[
            &self.user_id,
            &self.uid,
            &self.id,
            &self.friend_id,
            &self.global_id,
        ])
    }

    /// Display name, falling back to `profile_displayName` / `profile.displayName`.
    pub fn display_name(&self) -> Option<String> {
        pick_alias(&[
            &self.display_name_camel,
            &self.zalo_name,
            &self.display_name_snake,
            &self.name,
        ])
        .or_else(|| {
            self.profile_display_name
                .as_ref()
                .or_else(|| self.profile.as_ref().and_then(|p| p.get("displayName")))
                .and_then(Value::as_str)
                .map(str::to_string)
        })
    }

    /// Avatar URL (`avatar` > `avt` > `profile_avatar`).
    pub fn avatar(&self) -> Option<String> {
        pick_alias(&[&self.avatar_url, &self.avt, &self.profile_avatar])
    }
}

/// Contact profile returned by `/api/friend/profile/get` (cmd `12004`) for phone lookup.
///
/// # Wire Response Example
/// ```json
/// {
///   "uid": "1234567890",
///   "display_name": "Ngọc Trần",
///   "zalo_name": "ngoctran",
///   "avatar": "https://avatar.example/240",
///   "status": "Hello"
/// }
/// ```
///
/// # Core Transformation
/// Fills [`PhoneLookupResult`]; the lookup aborts when `uid` is absent, because the caller
/// cannot build a thread without it.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct PhoneProfileWireData {
    #[serde(rename = "uid")]
    pub uid: Option<Value>,
    #[serde(rename = "userId")]
    pub user_id: Option<Value>,
    #[serde(rename = "id")]
    pub id: Option<Value>,
    #[serde(rename = "globalId")]
    pub global_id: Option<Value>,
    #[serde(rename = "display_name")]
    pub display_name_snake: Option<Value>,
    #[serde(rename = "displayName")]
    pub display_name_camel: Option<Value>,
    #[serde(rename = "zalo_name")]
    pub zalo_name_snake: Option<Value>,
    #[serde(rename = "zaloName")]
    pub zalo_name_camel: Option<Value>,
    #[serde(rename = "avatar")]
    pub avatar_url: Option<Value>,
    #[serde(rename = "avt")]
    pub avt: Option<Value>,
    #[serde(rename = "status")]
    pub status: Option<Value>,
}

impl PhoneProfileWireData {
    /// Contact id (`uid` > `userId` > `id` > `globalId`).
    pub fn uid(&self) -> Option<String> {
        pick_alias(&[&self.uid, &self.user_id, &self.id, &self.global_id])
    }

    /// Display name (`display_name` > `displayName` > `zalo_name` > `zaloName`).
    pub fn display_name(&self) -> Option<String> {
        pick_alias(&[
            &self.display_name_snake,
            &self.display_name_camel,
            &self.zalo_name_snake,
            &self.zalo_name_camel,
        ])
    }

    /// Zalo name (`zalo_name` > `zaloName` > `display_name` > `displayName`).
    pub fn zalo_name(&self) -> Option<String> {
        pick_alias(&[
            &self.zalo_name_snake,
            &self.zalo_name_camel,
            &self.display_name_snake,
            &self.display_name_camel,
        ])
    }

    /// Avatar URL (`avatar` > `avt`).
    pub fn avatar(&self) -> String {
        pick_alias(&[&self.avatar_url, &self.avt]).unwrap_or_default()
    }

    /// Free-form presence status string.
    pub fn status(&self) -> String {
        pick_alias(&[&self.status]).unwrap_or_default()
    }
}

pub(crate) fn parse_thread(v: &Value) -> Option<Thread> {
    let wire = FriendWireData::from_value(v);
    let id = wire.id()?;
    let name = wire.display_name().unwrap_or_else(|| id.clone());
    let avatar = wire.avatar();
    let is_group = is_group_flag(v);
    let time = val_i64(wire.last_action_time.as_ref())
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
        .get_endpoint("profile", &path_and_query)
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

/// Raw wire payload returned by Zalo for `cmd 11430` (`/api/friend/reqstatus`).
///
/// # Wire Response Schema
/// ```json
/// {
///   "error_code": 0,
///   "error_message": "Successful",
///   "data": {
///     "isFriend": 0,
///     "isRequesting": 1,
///     "isRequested": 0
///   }
/// }
/// ```
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct FriendReqStatusWireData {
    #[serde(alias = "is_friend")]
    pub is_friend: Option<Value>,
    #[serde(alias = "is_requesting")]
    pub is_requesting: Option<Value>,
    #[serde(alias = "is_requested")]
    pub is_requested: Option<Value>,
}

impl FriendReqStatusWireData {
    pub fn to_status(&self) -> FriendshipStatus {
        let flag = |v: Option<&Value>| {
            v.and_then(|val| {
                val.as_i64()
                    .or_else(|| val.as_bool().map(|b| if b { 1 } else { 0 }))
            })
            .unwrap_or(0)
        };
        if flag(self.is_friend.as_ref()) == 1 {
            FriendshipStatus::Friend
        } else if flag(self.is_requesting.as_ref()) == 1 {
            FriendshipStatus::IncomingPending
        } else if flag(self.is_requested.as_ref()) == 1 {
            FriendshipStatus::OutgoingPending
        } else {
            FriendshipStatus::Stranger
        }
    }
}

/// Parse `/api/friend/reqstatus` (`cmd 11430`) JSON into [`FriendshipStatus`].
pub fn parse_friendship_status_response(resp: &Value) -> FriendshipStatus {
    let d = unwrap_data(resp);
    let wire: FriendReqStatusWireData = serde_json::from_value(d.clone()).unwrap_or_default();
    wire.to_status()
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

/// Upstream user privacy settings returned in profile responses.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct UserProfilePrivacySettings {
    pub show_online_status: bool,
}

/// Raw wire payload returned by Zalo for `cmd 12090` (`/api/social/profile/lastOnline`).
///
/// # Raw JSON Envelope
/// ```json
/// {
///   "error_code": 0,
///   "error_message": "Successful",
///   "data": {
///     "lastOnline": 1731215400000,
///     "settings": {
///       "show_online_status": true
///     }
///   }
/// }
/// ```
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct LastOnlineWireData {
    #[serde(rename = "lastOnline")]
    pub last_online: Option<Value>,
    pub settings: Option<UserProfilePrivacySettings>,
}

impl LastOnlineWireData {
    /// Return the raw millisecond epoch timestamp.
    pub fn last_online_ms(&self) -> i64 {
        val_i64(self.last_online.as_ref()).unwrap_or(0)
    }

    /// Return whether the user allows their online status to be shown.
    pub fn is_visible(&self) -> bool {
        self.settings
            .as_ref()
            .map(|s| s.show_online_status)
            .unwrap_or(true)
    }
}

/// Query user presence status and format a human-readable relative activity label.
///
/// # Zalo HTTP Wire Request
/// - **Method / Path**: `GET /api/social/profile/lastOnline` (cmd `12090`)
/// - **Host Domain**: `profile` (`https://profile.chat.zalo.me`)
/// - **Transport**: Encrypted with `ParamCipher v2` + session cookie `zpw_sek`
/// - **Params Payload**:
///   ```json
///   { "uid": "<target_user_id>", "conv_type": 1, "imei": "<device_imei>" }
///   ```
///
/// # Raw Upstream Response (`error_code: 0`)
/// Wrapped in standard [`crate::protocol::wire::ZaloWireEnvelope`]:
/// ```json
/// {
///   "error_code": 0,
///   "error_message": "Successful",
///   "data": {
///     "lastOnline": 1731215400000,
///     "settings": {
///       "show_online_status": true
///     }
///   }
/// }
/// ```
///
/// # Core Normalization & Transformation Logic
/// 1. Deserializes raw `data` into [`LastOnlineWireData`].
/// 2. Evaluates `settings.show_online_status`. If the user hid their presence, returns `(false, "")`.
/// 3. Normalizes epoch milliseconds to seconds and compares against current unix clock:
///    - `diff <= 300s` (5 minutes) -> `(true, "Vừa mới truy cập")`
///    - `diff < 3600s` (1 hour)    -> `(false, "Truy cập x phút trước")`
///    - `diff < 86400s` (24 hours) -> `(false, "Truy cập x giờ trước")`
///    - `diff < 7 days`            -> `(false, "Truy cập x ngày trước")`
///
/// # Errors
/// Returns an error if the HTTP request fails or parameter encryption fails.
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
    let wire_data: LastOnlineWireData = serde_json::from_value(d.clone()).unwrap_or_default();
    if !wire_data.is_visible() {
        return Ok((false, String::new()));
    }
    let last_onl_raw = wire_data.last_online_ms();
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

/// Helper: encode params, POST to friend endpoint, and decrypt response.
async fn post_friend_api(s: &Session, endpoint_path: &str, body: &Value) -> Result<Value> {
    let enc = s.encoded_params(body)?;
    let host = default_friend_host(s);
    let url = format!("{host}{endpoint_path}?{}", s.common_params());
    let resp = s.post_form(&url, format!("params={enc}")).await?;
    Ok(s.maybe_decrypt(resp, s.session_key_bytes()).await)
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
    let dec = post_friend_api(s, "/api/friend/sendreq", &body).await?;
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
    let dec = post_friend_api(s, "/api/friend/accept", &body).await?;
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
    let dec = post_friend_api(s, "/api/friend/undo", &body).await?;
    Ok(ActionAck::from_wire(&dec))
}

/// Reject a pending incoming friend request (`/api/friend/reject`, cmd `11428`).
///
/// # Errors
/// Returns an error if unauthenticated or the HTTP request fails.
pub async fn reject_friend_request(s: &Session, fid: &str) -> Result<ActionAck> {
    require_login(s)?;
    let body = obj(&[("fid", json!(fid)), ("imei", json!(s.imei))]);
    let dec = post_friend_api(s, "/api/friend/reject", &body).await?;
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
    let env = crate::protocol::wire::ErrorEnvelopeWireData::parse(&dec);
    let code = env.code();
    if code != 0 {
        if code == 216 {
            anyhow::bail!(
                "Số điện thoại chưa đăng ký Zalo hoặc người dùng đã chặn tìm kiếm qua SĐT (mã 216)"
            );
        }
        let err_msg = match env.message() {
            "" => "Không tìm thấy tài khoản",
            msg => msg,
        };
        anyhow::bail!("{err_msg} (mã {code})");
    }

    let wire = PhoneProfileWireData::from_value(dec.get("data").unwrap_or(&dec));
    let Some(uid) = wire.uid() else {
        anyhow::bail!("Không tìm thấy tài khoản Zalo ứng với số điện thoại này");
    };
    let display_name = wire
        .display_name()
        .unwrap_or_else(|| raw_phone.trim().to_string());
    let zalo_name = wire.zalo_name().unwrap_or_else(|| display_name.clone());
    let avatar = wire.avatar();
    let status = wire.status();

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
