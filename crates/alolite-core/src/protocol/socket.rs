//! Control websocket (`zpw_ws`) — the real-time transport.
//!
//! \[O\] Decoded from Zalo PC 26.10.11 (`pc-dist_compact-app-pc.*.js`):
//!
//! * URL — `Q[i] + "?" + commonParams`, then the client appends `&t=<ms>`
//!   (`getSocketUrl`, `_connectSocket`). `Q` is the login-config `zpw_ws` list.
//! * Frame — 4-byte header + UTF-8 JSON: `u8 version`, `u16 LE cmd`, `u8 subCmd`
//!   (`_send`, `_getHeader`, `_processEventBlob`).
//! * The server speaks first: `cmd 1` (`AUTHEN`) with `{key, encrypt, data}`.
//!   `key` is base64 and becomes the AES-GCM key (`_cipherKey`).
//! * `encrypt`: 0 plain, 1 base64+zlib, 2 AES-GCM+zlib, 3 AES-GCM
//!   (`_decrypt`, `_decompress`). AES-GCM blob = `iv(16) | aad(16) | ciphertext||tag`.
//! * After auth the payload lists the queues to pull (`data.ctrl_qCmds`); each is pulled with
//!   `{cmd, subCmd, data:{first, reqId:"req_N", lastId, preIds}}` (`doGetOffline`).
//! * Ping — `{version:1, cmd:2, subCmd:1, data:{eventId}}`; `_onPong` matches `eventId`.
//!
//! HTTP long-polling only ever carries durable actions; live VoIP invites are socket-only,
//! which is why a polling-only callee sees `act:"answer" status:5` and never `act:"request"`.

use aes::{Aes128, Aes256};
use aes_gcm::aead::{Aead, KeyInit, Payload};
use aes_gcm::AesGcm;
use anyhow::{anyhow, bail, Result};
use base64::{engine::general_purpose::STANDARD as B64, Engine};
use futures_util::{SinkExt, StreamExt};
use generic_array::typenum::U16;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::time::Duration;
use tokio::sync::mpsc;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::http::header::{COOKIE, ORIGIN, USER_AGENT};
use tokio_tungstenite::tungstenite::http::HeaderValue;
use tokio_tungstenite::tungstenite::Message;

use crate::clock::now_ms;
use crate::protocol::registry::ws;
use crate::protocol::wire::WireData;
use crate::session::{truncate, Session};
static WS_OUTBOUND_TX: std::sync::Mutex<Option<mpsc::UnboundedSender<Vec<u8>>>> =
    std::sync::Mutex::new(None);

/// Send a raw encoded frame over the active `zpw_ws` WebSocket if connected.
pub fn send_ws_frame(cmd: u16, sub_cmd: u8, version: u8, data: Option<&Value>) -> bool {
    let frame = encode_frame(cmd, sub_cmd, version, data);
    if let Ok(guard) = WS_OUTBOUND_TX.lock() {
        if let Some(ref tx) = *guard {
            return tx.send(frame).is_ok();
        }
    }
    false
}

/// Emit `SYNC_MESSAGE.REQUEST_MOBILE_WAKE_UP` (`cmd 592`, `subCmd 0`) over the live WebSocket
/// to wake up the user's phone Zalo client for cross-device message sync.
pub fn send_mobile_wakeup(sync_id: &str) -> bool {
    let req_id = format!("req_592_{}", now_ms());
    let body = json!({
        "reqId": req_id,
        "syncId": sync_id,
        "toDevice": 0
    });
    send_ws_frame(ws::SYNC_REQUEST_MOBILE_WAKE_UP, 0, 1, Some(&body))
}

/// Emit `SYNC_MESSAGE.REQUEST` (`cmd 590`, `subCmd 0`) over the live WebSocket.
pub fn send_sync_message_ws_request(payload: &Value) -> bool {
    let req_id = format!("req_590_{}", now_ms());
    let body = json!({
        "reqId": req_id,
        "data": payload
    });
    send_ws_frame(ws::SYNC_MESSAGE_REQUEST, 0, 1, Some(&body))
}
const UA: &str = crate::protocol::session::ZALO_UA;

const PING_SECS: u64 = 15;

// -- wire level ----------------------------------------------------------------

fn trace_on() -> bool {
    std::env::var("ZCALL_WS_TRACE").ok().as_deref() == Some("1")
}

fn trace(msg: impl std::fmt::Display) {
    if trace_on() {
        eprintln!("[ws] {msg}");
    }
}

/// Encode one frame: `u8 version | u16 LE cmd | u8 subCmd | JSON body`.
pub fn encode_frame(cmd: u16, sub_cmd: u8, version: u8, data: Option<&Value>) -> Vec<u8> {
    let body = data.map(|d| d.to_string()).unwrap_or_default();
    let mut out = Vec::with_capacity(4 + body.len());
    out.push(version);
    out.extend_from_slice(&cmd.to_le_bytes());
    out.push(sub_cmd);
    out.extend_from_slice(body.as_bytes());
    out
}

/// Decode one frame into `(version, cmd, subCmd, JSON)`.
pub fn decode_frame(buf: &[u8]) -> Option<(u8, u16, u8, Value)> {
    if buf.len() < 4 {
        return None;
    }
    let version = buf[0];
    let cmd = u16::from_le_bytes([buf[1], buf[2]]);
    let sub_cmd = buf[3];
    let body = &buf[4..];
    if body.is_empty() {
        return Some((version, cmd, sub_cmd, Value::Null));
    }
    let value = serde_json::from_slice(body).ok()?;
    Some((version, cmd, sub_cmd, value))
}

/// `decodeURIComponent` for the AES branch, tolerating raw `+`/`/`/`=` base64.
fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            let hi = (bytes[i + 1] as char).to_digit(16);
            let lo = (bytes[i + 2] as char).to_digit(16);
            if let (Some(h), Some(l)) = (hi, lo) {
                out.push((h * 16 + l) as u8);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Inflate a decrypted payload.
///
/// \[O\] The server sends **gzip** (`1f 8b 08 …`) for `encrypt` 2/3; the JS side calls a generic
/// `inflate` helper that sniffs the container. Older captures decoded zlib, so try gzip first
/// (by magic), then zlib, then raw deflate.
fn inflate(buf: &[u8]) -> Result<Vec<u8>> {
    use std::io::Read;
    if trace_on() {
        let head: String = buf.iter().take(12).map(|b| format!("{b:02x}")).collect();
        trace(format!("inflate input {} bytes head={head}", buf.len()));
    }
    let attempts: [(&str, fn(&[u8]) -> Box<dyn Read + '_>); 2] = [
        ("gzip", |b| Box::new(flate2::read::GzDecoder::new(b))),
        ("zlib", |b| Box::new(flate2::read::ZlibDecoder::new(b))),
    ];
    let order: [usize; 2] = if buf.starts_with(&[0x1f, 0x8b]) {
        [0, 1]
    } else {
        [1, 0]
    };
    let mut last_err: Option<anyhow::Error> = None;
    for idx in order {
        let (name, make) = attempts[idx];
        let mut out = Vec::new();
        match make(buf).read_to_end(&mut out) {
            Ok(_) if !out.is_empty() => {
                trace(format!(
                    "inflate ok via {name}: {} -> {} bytes",
                    buf.len(),
                    out.len()
                ));
                return Ok(out);
            }
            Ok(_) => last_err = Some(anyhow!("{name}: empty output")),
            Err(e) => last_err = Some(anyhow!("{name}: {e}")),
        }
    }
    let mut out = Vec::new();
    flate2::read::DeflateDecoder::new(buf).read_to_end(&mut out)?;
    if out.is_empty() {
        return Err(last_err.unwrap_or_else(|| anyhow!("empty inflate output")));
    }
    Ok(out)
}

/// AES-GCM with a 16-byte nonce: `iv(16) | aad(16) | ciphertext||tag(16)`.
fn gcm_decrypt(key: &[u8], blob: &[u8]) -> Result<Vec<u8>> {
    if blob.len() < 48 {
        bail!("aes-gcm blob too short ({})", blob.len());
    }
    let nonce = aes_gcm::Nonce::<U16>::from_slice(&blob[..16]);
    let payload = Payload {
        msg: &blob[32..],
        aad: &blob[16..32],
    };
    let plain = match key.len() {
        32 => AesGcm::<Aes256, U16>::new_from_slice(key)
            .map_err(|e| anyhow!("aes key: {e}"))?
            .decrypt(nonce, payload)
            .map_err(|e| anyhow!("aes-gcm: {e}"))?,
        16 => AesGcm::<Aes128, U16>::new_from_slice(key)
            .map_err(|e| anyhow!("aes key: {e}"))?
            .decrypt(nonce, payload)
            .map_err(|e| anyhow!("aes-gcm: {e}"))?,
        n => bail!("unsupported AES key length {n}"),
    };
    Ok(plain)
}

/// Apply the envelope's `encrypt` scheme to its `data` field.
pub fn decrypt_payload(cipher_key: Option<&[u8]>, encrypt: u8, data: &str) -> Result<Vec<u8>> {
    match encrypt {
        0 => Ok(data.as_bytes().to_vec()),
        1 => inflate(&B64.decode(data)?),
        2 => {
            let key = cipher_key.ok_or_else(|| anyhow!("no cipher key"))?;
            let raw = B64.decode(percent_decode(data))?;
            inflate(&gcm_decrypt(key, &raw)?)
        }
        3 => {
            let key = cipher_key.ok_or_else(|| anyhow!("no cipher key"))?;
            let raw = B64.decode(percent_decode(data))?;
            gcm_decrypt(key, &raw)
        }
        other => bail!("unsupported encrypt type {other}"),
    }
}

// -- client --------------------------------------------------------------------

/// One decrypted socket event.
#[derive(Debug, Clone)]
pub struct WsEvent {
    pub cmd: u16,
    pub sub_cmd: u8,
    pub version: u8,
    /// Decrypted body when possible, otherwise the raw envelope.
    pub payload: Value,
}

impl WsEvent {
    /// `true` for anything that may carry a VoIP invite (`601` push, `604` pull page).
    pub fn is_voip_ish(&self) -> bool {
        matches!(
            self.cmd,
            ws::CTRL_PUSH_CTRL | ws::CTRL_ACTION | ws::CTRL_PULL_CTRL | ws::CTRL_PULL_VOIP
        )
    }

    /// `true` for real-time or offline chat message frames (`501`, `521`, `510`, `511`, `515`, `570`, …).
    pub fn is_chat_ish(&self) -> bool {
        matches!(
            self.cmd,
            ws::PUSH_MSG_1_1
                | ws::PUSH_MSG_GROUP
                | ws::PULL_MSG_1_1
                | ws::PULL_MSG_GROUP
                | ws::PULL_MSG_LEGACY_513
                | ws::E2EE_RECEIVE_GROUP_OFFLINE
                | ws::E2EE_VERIFY_MSG_515
                | ws::E2EE_SESSION_OFFLINE_WEB
                | ws::E2EE_OFFLINE_ACK_ONE_ONE
                | ws::PUSH_MISS_MSG
                | ws::GET_MSG
        )
    }
}

fn build_url(sess: &Session, endpoint_idx: usize) -> String {
    let endpoints = &sess.domains.ws;
    let base = std::env::var("ZCALL_WS")
        .ok()
        .filter(|s| !s.is_empty())
        .or_else(|| {
            if endpoints.is_empty() {
                None
            } else {
                Some(endpoints[endpoint_idx % endpoints.len()].clone())
            }
        })
        .unwrap_or_else(|| crate::endpoints::WS_DEFAULT.to_string());
    // The server's `zlpws` front end answers `501 Not Implemented` to a request line with an
    // empty path (`GET ?zpw_ver=...`), so always send an explicit `/`.
    let (host, query) = match base.split_once('?') {
        Some((h, q)) => (h, Some(q)),
        None => (base.as_str(), None),
    };
    // Authority-only URLs (`wss://host`) need an explicit `/` before the query string.
    let after_scheme = host.split_once("://").map(|(_, rest)| rest).unwrap_or(host);
    let has_path = after_scheme.contains('/');
    let mut url = format!("{host}{}", if has_path { "" } else { "/" });
    let extra = format!("{}&t={}", sess.common_params(), now_ms());
    url.push('?');
    if let Some(q) = query {
        url.push_str(q);
        url.push('&');
    }
    url.push_str(&extra);
    url
}

/// Queues the server told us to pull.
///
/// The auth payload lists the exact shard-local queue names (`603_1`, `610_1`, …); pulling a
/// queue the worker does not own (e.g. `604_0`) closes the socket with `3002 NOT_FOUND_WORKER`,
/// so only server-listed queues are used unless `ZCALL_WS_FORCE_VOIP=1` asks for `604_0`.
/// One pull-queue descriptor listed in a WebSocket `AUTHEN` payload.
///
/// # Wire Response Example
/// ```json
/// { "cmd": 603, "subCmd": 1 }
/// ```
///
/// # Core Transformation
/// Yields the `(cmd, subCmd)` queue key the server assigned to this socket shard.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct QueueEntryWireData {
    #[serde(rename = "cmd")]
    pub cmd: Option<Value>,
    #[serde(rename = "subCmd")]
    pub sub_cmd: Option<Value>,
}

impl QueueEntryWireData {
    /// `(cmd, subCmd)` pair; `None` when `cmd` is absent or not a number.
    pub fn pair(&self) -> Option<(u16, u8)> {
        let cmd = self.cmd.as_ref().and_then(Value::as_u64)?;
        let sub = self.sub_cmd.as_ref().and_then(Value::as_u64).unwrap_or(0);
        Some((cmd as u16, sub as u8))
    }
}

/// Decoded WebSocket frame: header fields plus the `data` payload.
///
/// # Wire Response Example
/// ```json
/// { "cmd": 1, "subCmd": 0, "encrypt": 2, "key": "<base64>", "data": "<base64 zlib>" }
/// ```
///
/// # Core Transformation
/// `encrypt` selects the payload codec (`0` plain, `1` base64+zlib, `2` AES-GCM+zlib,
/// `3` AES-GCM); `key` is the base64 AES key sent once by the `AUTHEN` frame.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct SocketFrameWireData {
    #[serde(rename = "encrypt")]
    pub encrypt: Option<Value>,
    #[serde(rename = "error_code")]
    pub error_code: Option<Value>,
    #[serde(rename = "key")]
    pub key: Option<Value>,
    #[serde(rename = "data")]
    pub data: Option<Value>,
}

impl SocketFrameWireData {
    /// Payload codec selector (`0` plain, `1` base64+zlib, `2` AES-GCM+zlib, `3` AES-GCM).
    pub fn encrypt(&self) -> u8 {
        self.encrypt.as_ref().and_then(Value::as_u64).unwrap_or(0) as u8
    }

    /// Frame `error_code`, when present.
    pub fn error_code(&self) -> Option<i64> {
        self.error_code.as_ref().and_then(Value::as_i64)
    }

    /// Base64 cipher key from the `AUTHEN` frame, when non-empty.
    pub fn cipher_key(&self) -> Option<&str> {
        self.key
            .as_ref()
            .and_then(Value::as_str)
            .filter(|k| !k.is_empty())
    }

    /// Raw `data` payload as sent (a JSON string in this transport).
    pub fn raw_data(&self) -> &str {
        self.data
            .as_ref()
            .and_then(Value::as_str)
            .unwrap_or_default()
    }

    /// `data` as a structured value, when it is not a string.
    pub fn data_value(&self) -> Option<&Value> {
        self.data.as_ref().filter(|v| !v.is_string())
    }
}

fn queues_from_auth(auth: &Value) -> Vec<(u16, u8)> {
    let mut out: Vec<(u16, u8)> = Vec::new();
    for key in ["ctrl_qCmds", "qCmds"] {
        if let Some(arr) = auth
            .pointer(&format!("/data/{key}"))
            .and_then(|v| v.as_array())
        {
            for q in arr {
                if let Some(pair) = QueueEntryWireData::from_value(q).pair() {
                    if !out.contains(&pair) {
                        out.push(pair);
                    }
                }
            }
        }
    }
    if std::env::var("ZCALL_WS_FORCE_VOIP").ok().as_deref() == Some("1")
        && !out.iter().any(|(c, _)| *c == ws::CTRL_PULL_VOIP)
    {
        out.push((ws::CTRL_PULL_VOIP, 0));
    }
    out
}

fn pull_frame(cmd: u16, sub_cmd: u8, req_id: &str) -> Vec<u8> {
    let data = json!({
        "first": true,
        "reqId": req_id,
        "lastId": 1,
        "preIds": [],
    });
    encode_frame(cmd, sub_cmd, 1, Some(&data))
}

fn pull_next_frame(cmd: u16, sub_cmd: u8, req_id: &str, last_id: Value, pre_ids: Value) -> Vec<u8> {
    let data = json!({
        "first": false,
        "reqId": req_id,
        "lastId": last_id,
        "preIds": pre_ids,
    });
    encode_frame(cmd, sub_cmd, 1, Some(&data))
}

/// Connect, authenticate, pull the control/voip queues, then stream events.
///
/// Reconnects forever with capped backoff. Dropping the returned receiver stops the task.
pub fn spawn(sess: Session) -> mpsc::UnboundedReceiver<WsEvent> {
    let (tx, rx) = mpsc::unbounded_channel();
    tokio::spawn(async move {
        let mut backoff = 2u64;
        let mut endpoint_idx = 0usize;
        loop {
            match run_once(&sess, &tx, endpoint_idx).await {
                Ok(reason) => trace(format!("closed: {reason}")),
                Err(e) => trace(format!("error: {e:#}")),
            }
            if tx.is_closed() {
                break;
            }
            // `3002 NOT_FOUND_WORKER` means this shard does not host the session; rotate.
            endpoint_idx = endpoint_idx.wrapping_add(1);
            tokio::time::sleep(Duration::from_secs(backoff)).await;
            backoff = (backoff * 2).min(30);
        }
    });
    rx
}

async fn run_once(
    sess: &Session,
    tx: &mpsc::UnboundedSender<WsEvent>,
    endpoint_idx: usize,
) -> Result<String> {
    let url = build_url(sess, endpoint_idx);
    trace(format!("connect {url}"));

    let mut req = url.clone().into_client_request()?;
    {
        let h = req.headers_mut();
        if let Some(sek) = sess.zpw_sek.as_deref().filter(|s| !s.is_empty()) {
            h.insert(
                COOKIE,
                HeaderValue::from_str(&format!("zpw_sek={sek}; _zlang=vi"))?,
            );
            h.insert("zpw_sek", HeaderValue::from_str(sek)?);
        }
        h.insert(
            ORIGIN,
            HeaderValue::from_static(crate::endpoints::CHAT_ORIGIN),
        );
        h.insert(USER_AGENT, HeaderValue::from_static(UA));
    }

    let (mut ws, resp) = match connect_async(req).await {
        Ok(pair) => pair,
        Err(tokio_tungstenite::tungstenite::Error::Http(resp)) => {
            let mut detail = format!("handshake status {}", resp.status());
            for (k, v) in resp.headers() {
                detail.push_str(&format!("\n  {k}: {}", v.to_str().unwrap_or("<bin>")));
            }
            bail!("{detail}");
        }
        Err(e) => return Err(e.into()),
    };
    trace(format!("handshake {}", resp.status()));

    let mut cipher_key: Option<Vec<u8>> = None;
    let mut authed = false;
    let mut req_id = 0u64;
    let mut ping = tokio::time::interval(Duration::from_secs(PING_SECS));
    ping.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    ping.tick().await; // consume the immediate first tick

    let (out_tx, mut out_rx) = mpsc::unbounded_channel::<Vec<u8>>();
    if let Ok(mut g) = WS_OUTBOUND_TX.lock() {
        *g = Some(out_tx);
    }
    loop {
        tokio::select! {
            _ = ping.tick() => {
                if authed {
                    let body = json!({ "eventId": now_ms() });
                    let f = encode_frame(ws::PING, ws::SUB_KEEP_CONNECTION, 1, Some(&body));
                    ws.send(Message::Binary(f)).await?;
                    trace("-> ping");
                }
            }
            Some(frame) = out_rx.recv() => {
                if authed {
                    ws.send(Message::Binary(frame)).await?;
                    trace("-> outbound ws frame");
                }
            }
            incoming = ws.next() => {
                let Some(incoming) = incoming else {
                    return Ok("eof".into());
                };
                let incoming = incoming?;
                let buf = match incoming {
                    Message::Binary(b) => b,
                    Message::Text(t) => {
                        trace(format!("<- text {}", truncate(t.as_str(), 200)));
                        continue;
                    }
                    Message::Ping(p) => {
                        ws.send(Message::Pong(p)).await?;
                        continue;
                    }
                    Message::Close(c) => return Ok(format!("close {c:?}")),
                    _ => continue,
                };

                let Some((version, cmd, sub_cmd, envelope)) = decode_frame(&buf) else {
                    trace(format!("<- undecodable {} bytes", buf.len()));
                    continue;
                };

                let frame = SocketFrameWireData::from_value(&envelope);
                let encrypt = frame.encrypt();
                let err = frame.error_code();
                trace(format!(
                    "<- v{version} cmd={cmd} sub={sub_cmd} enc={encrypt} err={err:?} {}",
                    truncate(&envelope.to_string(), 300)
                ));

                if cmd == ws::AUTHEN {
                    if let Some(key) = frame.cipher_key() {
                        cipher_key = Some(B64.decode(key)?);
                        trace(format!(
                            "cipher key {} bytes",
                            cipher_key.as_ref().map(|k| k.len()).unwrap_or(0)
                        ));
                    }
                    let raw = frame.raw_data();
                    let plain = decrypt_payload(cipher_key.as_deref(), encrypt, raw)?;
                    let text = String::from_utf8_lossy(&plain).into_owned();
                    trace(format!("auth body {}", truncate(&text, 1200)));
                    let auth: Value = serde_json::from_str(&text)?;
                    if SocketFrameWireData::from_value(&auth).error_code() != Some(0) {
                        return Err(anyhow!("auth rejected: {text}"));
                    }
                    authed = true;
                    let _ = tx.send(WsEvent { cmd, sub_cmd, version, payload: auth.clone() });

                    for (qcmd, qsub) in queues_from_auth(&auth) {
                        let id = format!("req_{req_id}");
                        req_id += 1;
                        ws.send(Message::Binary(pull_frame(qcmd, qsub, &id))).await?;
                        trace(format!("-> pull {qcmd}_{qsub} {id}"));
                    }
                    continue;
                }

                let payload = if encrypt == 0 {
                    envelope.clone()
                } else {
                    match frame.data.as_ref().and_then(Value::as_str) {
                        Some(d) => decrypt_payload(cipher_key.as_deref(), encrypt, d)
                            .ok()
                            .and_then(|b| serde_json::from_slice::<Value>(&b).ok())
                            .unwrap_or_else(|| envelope.clone()),
                        None => envelope.clone(),
                    }
                };
                let payload_frame = SocketFrameWireData::from_value(&payload);
                let payload_data = payload_frame.data_value();
                if cmd == ws::HAS_MSG {
                    if let (Some(qid), Some(ids)) = (
                        payload_data.and_then(|d| d.get("queueId")).cloned(),
                        payload_data.and_then(|d| d.get("ids")).cloned(),
                    ) {
                        let id = format!("req_{req_id}");
                        req_id += 1;
                        let get_req = json!({
                            "queueId": qid,
                            "ids": ids,
                            "lastId": "1",
                            "reqId": id,
                        });
                        let _ = ws
                            .send(Message::Binary(
                                encode_frame(ws::GET_MSG, 0, 1, Some(&get_req)),
                            ))
                            .await;
                        trace(format!("-> get_msg 570_0 {id}"));
                    }
                }
                if payload_data.and_then(|d| d.get("more")).and_then(Value::as_i64) == Some(1) {
                    let qkey = format!("{cmd}_{sub_cmd}");
                    let queue_status = payload_data.and_then(|d| d.get("queueStatus"));
                    let queue_entry = queue_status.and_then(|q| q.get(&qkey));
                    let last_id = payload_data
                        .and_then(|d| d.get("lastActionId"))
                        .or_else(|| queue_entry.and_then(|e| e.get("lastId")))
                        .cloned()
                        .unwrap_or(json!(1));
                    let pre_ids = queue_entry
                        .and_then(|e| e.get("ids"))
                        .cloned()
                        .unwrap_or_else(|| json!([]));
                    let id = format!("req_{req_id}");
                    req_id += 1;
                    let _ = ws
                        .send(Message::Binary(
                            pull_next_frame(cmd, sub_cmd, &id, last_id, pre_ids),
                        ))
                        .await;
                    trace(format!("-> pull next {qkey} {id}"));
                }
                let _ = tx.send(WsEvent { cmd, sub_cmd, version, payload });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_roundtrip() {
        let body = json!({ "a": 1, "b": "x" });
        let f = encode_frame(604, 0, 1, Some(&body));
        assert_eq!(f[0], 1);
        assert_eq!(u16::from_le_bytes([f[1], f[2]]), 604);
        assert_eq!(f[3], 0);
        let (v, cmd, sub, val) = decode_frame(&f).expect("decode");
        assert_eq!((v, cmd, sub), (1, 604, 0));
        assert_eq!(val, body);
    }

    #[test]
    fn frame_without_body() {
        let f = encode_frame(2, 1, 1, None);
        assert_eq!(f.len(), 4);
        let (_, cmd, sub, val) = decode_frame(&f).expect("decode");
        assert_eq!((cmd, sub), (2, 1));
        assert!(val.is_null());
    }

    #[test]
    fn queues_follow_auth_lists() {
        let auth = json!({ "error_code": 0, "data": {
            "ctrl_qCmds": [
                { "cmd": 610, "subCmd": 1 }, { "cmd": 610, "subCmd": 1 }, { "cmd": 603, "subCmd": 1 }
            ],
            "qCmds": [{ "cmd": 511, "subCmd": 1 }]
        } });
        assert_eq!(queues_from_auth(&auth), vec![(610, 1), (603, 1), (511, 1)]);

        // An empty list stays empty: injecting an unowned queue kills the socket.
        let auth = json!({ "error_code": 0, "data": {} });
        assert!(queues_from_auth(&auth).is_empty());
    }

    #[test]
    fn percent_decode_passthrough() {
        assert_eq!(percent_decode("abc+/="), "abc+/=");
        assert_eq!(percent_decode("a%2Bb"), "a+b");
    }

    #[test]
    fn gcm_rejects_short_blob() {
        let err = gcm_decrypt(&[0u8; 32], &[0u8; 16]).unwrap_err().to_string();
        assert!(err.contains("too short"), "{err}");
    }
}
