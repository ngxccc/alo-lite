# 10 — Clarification on 1-1 Message Flow, Group Cloud Messages, and Phone Sync

Date: 2026-11-07  
Status: Ground-Truth Verified  
Evidence tags: **[O]** Observed in runtime / bundle, **[V]** Verified via live capture.

---

## 1. Ground Truth Findings

### 1.1 `/api/cm/getrecentv2` is Group/Cloud Only (`error_code: 10002`)
- Host: `tt-group-cm.api.zaloapp.com` (Group & Cloud Message Server).
- Commands: `12506` (`getrecentv2`) and `12507` (`getoldv2`).
- Payload requires a valid `groupId` (e.g. numeric group ID or `send2me_id` for My Cloud).
- **When called with a 1-1 friend user ID**:
  The server responds with:
  ```json
  { "data": "", "error_code": 10002, "error_message": "Successful." }
  ```
  **`error_code: 10002` indicates "Group not found" / invalid group ID.**  
  It is **not** a biometric authentication or encryption error.

### 1.2 QR Login Does NOT Require Biometric Verification by Default
- When scanning the QR code to log in (whether on Zalo PC via `/api/login/reqqr` or Zalo Web via `id.zalo.me`), Zalo Mobile displays:
  > *"Đăng nhập trên máy tính / Web - Bấm ĐĂNG NHẬP"*
- The user taps **"ĐĂNG NHẬP"** to authorize the session without biometric (FaceID / Fingerprint) check.
- **Origin of the "Biometric Login" Hypothesis**: Previous documentation conflated standard QR login (`qr_form: 0`) with the **Trusted Companion Device Linking Protocol** (`qr_form: 1` / QR2, documented in `re-notes/11_trusted_device_protocol.md`). When `qr_format_trust_device` is enabled by server config, companion linking cryptographically registers the PC into the primary's `DeviceList`, prompting mobile companion authorization. On live production servers, this feature flag is disabled by default, so standard login succeeds without companion cryptographic linking.

### 1.2.1 Root Cause: a Single Server-Side Login Flag Gates the Biometric Prompt [O, V]
Re-verified 2026-11-07 against the official bundle (`zalo-for-linux/app/pc-dist`, identical bytes in the macOS `.app/asar`).

**The reference client's QR builder** (`lazy/login-startup.*.js`):

```js
// key_sync singleton
this.serverConfig = { self_g: false, qr_format_trust_device: false, logo_qr: true }
// login bootstrap
mergeServerConfig({
  self_g:   Nr.authentication.trusted_device.self_g === 1,
  qr_format_trust_device: Nr.authentication.trusted_device.qr_format_trust_device === 1,
  logo_qr:  Nr.authentication.trusted_device.logo_qr === 1,
})
// QR request — the flag *is* the qr_form argument
getQRCode(imei, logged_uids, language, self_g ? 1 : 0, qr_format_trust_device ? 1 : 0)
// payload builder
genFirstQR(tokenId, base64Qr, self_g) {
  if (!self_g)                       return `data:image/png;base64,${base64Qr}`
  if (!qr_format_trust_device)       return T(qrCnt, logo_qr)          // QR1, plain
  /* else */                         return T(await linkingState.generateQr(tokenId, ts)) // QR2, Z1@
}
```

So `qr_form`, the payload format, the `encLinkingMetaData/…` params on the confirm URL, and therefore the
mobile biometric authorization are **all derived from one flag** — `qr_format_trust_device`.

**Live value** (probe over every reachable Zalo auth host, anonymous *and* authenticated, client
`type` 30/31/32/33/24/1 — identical everywhere):

| Auth host | `setttings.authen.trusted_device` |
|:---|:---|
| `https://wpa.zaloapp.com` | `{"logo_qr":0,"qr_format_trust_device":0,"self_g":0}` |
| `https://wpa.zalo.me` | `{"logo_qr":0,"qr_format_trust_device":0,"self_g":0}` |
| `https://wpa.chat.zalo.me` | `{"logo_qr":0,"qr_format_trust_device":0,"self_g":0}` |

Consequences:
1. `setttings.authen.trusted_device` is only present in the **unauthenticated** `getServerInfo` response
   (the login-screen one). Authenticated responses omit it.
2. With the flag at `0`, the official client generates **QR1** — the phone shows only *"Đăng nhập …"*
   with **no** FaceID/fingerprint. The observed "login works but asks for no biometrics" is therefore
   the reference behavior, not an Alolite defect.
3. `reqqr` accepts `qr_form=1` and returns the same response shape, but the server never sets up the
   companion linking session, so forcing a `Z1@` QR against a flag-`0` account fails on the phone.
4. The client `type` is *not* the gate: `apiType` is `30` for every bundle ("31" only for the unused
   `embed` bundle name), and the server answered the same flag for types 30–33 regardless.

Alolite now mirrors the reference logic: [`Session::qr_format_trust_device`](../src/protocol/session.rs)
is parsed from `getServerInfo` in `apply_server_info`, `qr::request_qr` feeds it into the QR2/QR1 choice,
and `ZALO_QR_TRUSTED=1|0` overrides it for protocol experiments. The QR2 path activates by itself if
Zalo enables the flag.

### 1.2.2 The Biometric Prompt Is the Message-Key Transfer Confirmation [O, V 2026-11-07]
The phone's FaceID/fingerprint prompt is **not** produced by the QR itself. It is the mobile
confirmation step of the **E2EE message-key transfer** (`transfer_after_login`):

| Step | Evidence |
|:---|:---|
| Desktop asks for the phone's message keys | `/api/message/pull_mobile_msg` (cmd `12000`), `/api/transfer-sync-v2/request-sync` (cmd `12888`, body `{reqId, data}` + `public_key`), `/api/message/cancel_pull_mobile_msg` (cmd `12700`); WS wake-ups `cmd 592` / `590` |
| Phone shows the confirm dialog (biometric if the account requires it) | WS `cmd 601` control `transfer_status {syncId, fromDevice:0, status:3}` |
| User approves on the phone | WS `cmd 601` control `user_confirm {user_action:1, pc_name, public_key}` — the reference client matches it with `isUserConfirm` (`e.hostName === data.pc_name && e.publicKey === data.public_key`) |
| Phone releases the key material | control `transfer_after_login {temp_key}` with `data.imei === getZaloClientID()` (`isTransferAfterLoginControlEvent`) |
| Phone uploads an encrypted snapshot | `*.db.crypt` package + decryption key over WS, downloaded from `trans-bin.zaloapp.com`; key exchange via `POST https://zwid.api.zalo.me/api/znoise` |

Reproduced end-to-end from Alolite itself (`cargo run --bin test_phone_sync`, 2026-11-07): the phone
showed the sync confirmation, the accept arrived as `user_confirm {user_action:1, pc_name:"MacBook Pro"}`
(our `computer_name`), and the encrypted backup package was downloaded and decrypted.

Consequences:
- The `trust`/QR2 companion-linking module is **unrelated** to this prompt (see §1.2.1: `qr_form = 0`).
- Alolite already implements every leg (`chat::request_phone_sync*`, `request_sync_v2_message_chunks`,
  `MobileSyncControlEvent::{TransferAfterLogin, UserConfirm, …}`, `history_sync::set_latest_mobile_temp_key`).
  What it lacks is the reference client's post-login trigger: a "Tin nhắn chưa được đồng bộ" banner whose
  "Đồng bộ" CTA (`ui_ux_transfer_revamp.contents.suggest_sync_banner`) starts the flow. Today Alolite only
  starts it from the manual sync dialog.

### 1.2.3 Official Login Request Shape (decrypted) [O, V 2026-11-07]
`params` of the reference `reqqr` decrypted with Alolite's own `ParamCipher` (AES-256-CBC/IV 0, key derived
from `zcid` + `zcid_ext`):

```json
{"language":"vi","client_time":1794045259,"imei":"6a428d9f-…-13b7374efd47710d59d67dbb993f9180",
 "computer_name":"arch-t14s","logged_uids":"46784157848677799","self_g":0,"qr_form":0}
```

- `zcid` decodes to `23,<imei>,<firstLaunchTime>`; the Linux wrapper identifies as `type=23`,
  `client_version=695` (Alolite now sends the same).
- `logged_uids` is a **plain uid string** (not a JSON array; empty when the device never logged in).
- `self_g=0` — the server renders the QR and returns `base64_qr` (a 512×512 PNG); there is no
  `qr_cnt`, so the raw `token_id` must never be substituted for the code.
- The full `reqqr` query string is
  `zcid, zcid_ext, enc_ver=v2, params, type, client_version, signkey`, with
  `signkey = md5("zsecure" + "reqqr" + values sorted by key)` over
  `{client_version, enc_ver, params, type, zcid, zcid_ext}` — recomputed from the capture and
  byte-identical (`16cc9dfdcf98892c31f46f96eeb511fc`). Omitting it yields
  **`error_code 18022: Invalid Signature`** (observed live when Alolite first tried the
  four-field variant).
- Hosts: `getServerInfo`/`reqqr` on `wpa.chat.zalo.me`, QR polling on `qr.id.zalo.me`.
- User-Agent: `ZaloPC-linux-23v695` on every API call (the only exception seen is
  `Zalo-linux-23v695` on `/api/login/chkUpdate`).
- Tooling: the opt-in capture hook `zalo-for-linux/plugins/netlog.js` (`ZALO_NETLOG=<file>`,
  dormant by default) records all `webRequest` traffic plus WebSocket frames through the
  in-process debugger.

Verified live 2026-11-07 with Alolite's own client after the change: `getServerInfo` and the
signed `reqqr` both answer `error_code 0`, and the returned PNG renders in the GUI and terminal.

### 1.3 How 1-1 Direct Messages Are Actually Handled
Zalo does not provide a REST cloud history endpoint for 1-1 direct messages. Instead:
1. **Latest Preview Messages**:
   - Fetched via `getLastMessagesForPreview` (`/api/preloadconvers/get-last-msgs`, cmd `10053`).
   - Returns the most recent messages for conversation previews.
2. **Real-Time Incoming & Outgoing Messages**:
   - Transmitted via WebSocket frames (`cmd 501` / `cmd 521`) and long-polling (`lp_chat`).
   - Stored directly into the local SQLite message database (`zalo_history.db`).
3. **Historical Message Archive from Mobile**:
   - Transferred via two parallel **Phone Message Sync** protocols: **Sync V1** (`pull_mobile_msg` cmd `12000`, RSA-2048 + `ZDB4.0` AES-256-CBC/LZMA SQLite chunks) and **Sync V2** (`cmd 592` / `cmd 590` / cmd `12888`, Trusted Device `ik` + `libzproto_wasm_bg.wasm` + Zstd + `proto.Sync2` Protobuf chunks).
   - Full wire specification for Sync V1, Sync V2, and multi-device Signal E2EE (`cmd 514..582`, HTTP `16000..16204`): see [note 12](12_message_sync_and_e2ee_protocol.md).
