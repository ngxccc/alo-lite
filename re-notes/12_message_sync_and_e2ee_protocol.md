# 12 — Mobile-to-PC Message Sync (V1 & V2) and End-to-End Encryption (E2EE) Protocol Specification

Date: 2026-11-08  
Status: Ground-Truth Verified (Static MCP REA + Clean-Room Cross-Verification)  
Evidence tags: **[O]** Observed in official universal bundle (`app.asar`), **[V]** Cross-verified against clean-room reference implementations (`db-cross-v4`) and live Alolite captures.

> **Clean-Room Compliance Notice (`docs/standards/compliance-and-ip.md`):** This document records only wire formats, HTTP/WebSocket command IDs, JSON/Protobuf schemas, binary container layouts, and cryptographic state machines. No verbatim proprietary JavaScript or decompiled C/C++ source code is included.
>
> **Companion Reference:** For the complete `app.asar` process/worker topology, cross-platform forensic diff (`macOS DMG` vs `Windows EXE` vs `zalo-for-linux`), `$z*` preload IPC surface, all WASM exports, non-E2EE WebSocket `cmd` IDs (`1..800`), and the full ~300-endpoint HTTP `commandId` registry, see [note 13](13_app_asar_architecture_and_command_registry.md).

---

## 1. Executive Summary & Architectural Truth

Static reverse-engineering of the official `Zalo 26.11.10-universal` application archive (`app.asar`) via **MCP REA** established three foundational architectural facts:

1. **Two Parallel Mobile-to-PC Sync Protocols Coexist (`Sync V1` & `Sync V2`):**
   - **Sync V1 (`SyncMessageController`)** uses an ephemeral **RSA-2048** keypair (`PKCS#1 v1.5` padding), downloads encrypted **SQLite** backup snapshots (`ZDB4.0` container: `AES-256-CBC` 64 KiB chunks with zero IV + `LZMA` stream), and decodes rich-message columns encoded as little-endian **binary TLV** (`parseBinNet`).
   - **Sync V2 (`SyncController` + `sync2-proto`)** requires a **Trusted Companion Device Identity Key (`ik`)** established during QR2 linking ([note 11](11_trusted_device_protocol.md)). It performs session establishment and chunk decryption inside **`libzproto_wasm_bg.wasm`**, decompresses payloads with **Zstandard (`Zstd`)**, and deserializes **`proto.Sync2.*` Protobuf** chunks (`SyncChunk` → `Partition` → `Message` / `Conversation`). It does **not** use SQLite or `db-cross-v4`.
2. **Live E2EE Runs Entirely in JavaScript/WebSocket + HTTP (`0%` Native Addon):**
   - The native addon `db-cross-v4.node` contains **zero** Signal, Curve25519, or real-time E2EE logic; `node_sqlite3.node` uses SQLCipher solely for local at-rest disk encryption.
   - All live E2EE (1-1 X3DH + Double Ratchet and Group Sender Keys) runs in JavaScript (`libsignal-protocol.static.js` + `MessageProtocol.proto`) using a **multi-device subdevice model** (`MOBILE = 0`, `WEB = 1`, `PC = 2`, `SYSTEM = 3`, `UNKNOWN = 6`).
   - All E2EE key/session management and incoming E2EE message delivery run over **WebSocket (`cmd 514..582`)**, whereas outbound E2EE messages are sent over **HTTP POST (`/api/e2ee/pc/message/*` `16000..16012` and `/api/e2ee/pc/group/*` `16100..16113`)**.
3. **How Historical Sync Connects to E2EE:**
   - Zalo PC **never** imports Mobile's `deviceId = 0` Signal ratchet state.
   - **Live E2EE messages** after login are encrypted specifically for PC's own `deviceId = 2` PreKey bundle (`SUBMIT_BUNDLE`, `cmd 540`). Messages sent before PC registered a session trigger `MSG_E2EE_WAIT` / `MSG_E2EE_CHANGE_DEVICE`.
   - **Historical E2EE messages** were already decrypted by Mobile into Mobile's local message store upon receipt; when the user approves Mobile-to-PC sync (`user_confirm`), Mobile reads those already-decrypted records from its local DB and re-encrypts them at the transport layer via **Sync V1 (`ZDB4.0`)** or **Sync V2 (`zprotoSync2`)**.

---

## 2. REA Evidence Ledger & Inspected Artifacts [O]

### 2.1 Primary Artifact Digests (`Zalo 26.11.10-universal`)

| Artifact Path (inside `Zalo.app/Contents/Resources/`) | Size (Bytes) | SHA-256 Digest | Role |
|:---|---:|:---|:---|
| `app.asar` | `192,832,202` | `9217d0e419c4f2b89033cf23e89073de0052d7417e6e16270afc2d68e31896d2` | Complete Electron application bundle (175 source entries) |
| `app.asar:pc-dist/libs/trusted_device_protocol_bg.97af2bf51fb002cefc5a888e932bcab2.wasm` | `346,340` | `716f0a0637e5b64168010899f4ad380be7d0c6a67778137c88ecb2ff9d9e49da` | QR2 companion linking & device-list verification ([note 11](11_trusted_device_protocol.md)) |
| `app.asar:pc-dist/libs/libzproto_wasm_bg.5ea0fb2c9af4a45bc7882b71307b6f83.wasm` | `287,049` | `398a2016ede72d1ec403d2e838c5e3a48fa5a3fc9305532f1e188504faef78c3` | **Sync V2** (`zprotoSync2*`) + Trusted Device + Ed25519/X25519 |
| `app.asar:pc-dist/libs/libzproto_backup_wasm_bg.042497d03f0f504b6b1444e25a7db0d9.wasm` | `129,796` | `3a980db6271f7588814a5625641a1889a7a8f28c92a43c92df60513619c28612` | **PC-to-Mobile / Cloud Backup** (`zprotobackupcryptosession_*`) |
| `app.asar:pc-dist/libs/libsignal-protocol.static.js` | `628,895` | `a53709c414c6...` *(recorded in `ev_aebc5cc4...`)* | Signal X3DH, Double Ratchet, Group `SenderKey`, `getSessionTs` |
| `app.asar:pc-dist/signal/MessageProtocol.proto` | `4,462` | *(in `app.asar`)* | Protobuf (`proto2`) wire & storage definitions (`package MessageProto`) |
| `app.asar:pc-dist/shared-worker.cfeba43a9a8e4e40dda3.js` | `434,548` | `6304c3fece19...` *(recorded in `ev_11f283c2...`)* | `sync2-proto` WASM bridge, Zstd + `proto.Sync2.*` codec, `backup-proto` |
| `app.asar:pc-dist/trust-worker.cfeba43a9a8e4e40dda3.js` | `194,619` | `f647cc926a90...` *(recorded in `ev_11f283c2...`)* | Dedicated Web Worker for `trusted_device_protocol_bg.wasm` |
| `app.asar:pc-dist/lazy/main-startup.89bb530b91927fd5dd28.js` | *(in `app.asar`)* | *(recorded in `ev_aebc5cc4...`)* | `SyncMessageController` (V1), `sync-message-key-generator` (RSA-2048), `SyncController` (V2) |
| `app.asar:pc-dist/lazy/default-login-main-startup-shared-worker-znotification.4bb5e1e2c2e9bab4f7b3.js` | *(in `app.asar`)* | *(in `app.asar`)* | HTTP `commandId` registry, WebSocket `cmd` registry, Signal transport & E2EE cipher orchestration |
| `app.asar.unpacked/.../db-cross-v4/prebuilt/darwin/electron/arm64/db-cross-v4-native.node` | `1,561,032` | `7b713f45fda5db41816ecffcf2750e27681dc6e427bdab9b9b6a9bf4bf7e0432` | **Sync V1 only**: `decompressAndDecryptDb` (V0), `decompressAndDecryptDb_V2` (`ZDB4.0`), `parseBinNet` (TLV) |

### 2.2 MCP REA Evidence IDs

| Evidence ID | REA Operation | Scope & Verified Facts |
|:---|:---|:---|
| `ev_f423a1d4225bdf20a0fc53315b5a05a627bcba843a1917117177345cdc5872e5` | `analyze_javascript_application` | Full `app.asar` inventory (175 files); identified 52 unpacked Mach-O `.dylib`/`.node` files modified post-ASAR by Apple `codesign` (`integrity_policy: "record-and-continue"`). |
| `ev_11f283c23d654134dffdd3bb679f2968050d5cf19fcfb56871bcfbb63454d0b2` | `analyze_javascript_application` | Complete AST & module graph of `shared-worker` + `trust-worker` (`171,259` visited AST nodes; summary projection `ev_12884c01f81af6d779d658429b9a139c36f8b47144d2f09f89d73c82bbe2f5b6`; module graph `ev_074f49d9b2d3e11fd19ecabbbd45af98205b77aa5f822d60ea30bd3ea5c2b4fe`). |
| `ev_aebc5cc474eb7c3e2cbd173522eba6567011a2046e5b282ae992b098cea8b2fb` | `analyze_javascript_application` | Complete AST & semantic graph across `trust-worker`, `libsignal-protocol.static.js`, `utility-process-sqlite.js`, `main-startup`, and `zd-worker` (`1,629,387` visited AST nodes, `1,271` findings; summary `ev_4b7f7adcae5af06ec76e477ccd71620103523a81d5302faf60ed71a27ce42f63`; routes `ev_2e58fe2d9b0fbf1d1580a364fe2e6a8428fe4ce3c644d9f5bc8145050aa1855d`). |

### 2.3 Desktop Process, Worker, Native Addon & Device Identity Architecture [O, V]

1. **Electron Process & Web Worker Topology (`app.asar`)**:
   - **Main Process (`main-dist/main.js`)**: Window lifecycle, updater, IPC router, device identity (`iqBP`: `MAC_CLIENT_TYPE = 23`, `WIN_CLIENT_TYPE = 24`, `LINUX_CLIENT_TYPE = 25`; `zalo-for-linux` patches `LINUX -> 23`).
   - **SQLite Utility Process (`main-dist/utility-process-sqlite.js`)**: Hosts `node_sqlite3.node` linked against **SQLCipher v4** (`4096`-byte pages, `80`-byte reserve, `PBKDF2-HMAC-SHA512` `256,000` iterations) strictly for local at-rest DB encryption.
   - **Renderer (`pc-dist/`)**: React UI, HTTP/WebSocket client (`default-login`), `SyncMessageController` (Sync V1), `SyncController` (Sync V2), and `libsignal-protocol.static.js` (live E2EE).
   - **Dedicated Workers**:
     - `shared-worker`: `sync2-proto` (`libzproto_wasm_bg.wasm`), `backup-proto` (`libzproto_backup_wasm_bg.wasm`), `ZstdStream` + `proto.Sync2.*` codec, and Sync V1 SQLite `ChatContent` importer.
     - `trust-worker`: `trusted_device_protocol_bg.wasm` (QR2 companion linking & device-list verification).
     - `sync-v2-sub-worker`: Sync V2 batch orchestration & offline E2EE queue coordination.
     - `zd-worker`: Diagnostic/telemetry worker.
2. **Unpacked Native Addon Inventory (`app.asar.unpacked/native/nativelibs/`)**:
   - Contains 15 OS-specific N-API `.node`/`.dylib` modules: `db-cross-v4` (Sync V1 `ZDB4.0` + `parseBinNet` only), `sqlite3` (SQLCipher local DB), `zcall` (`zcall_mac.node` VoIP/WebRTC engine), `zimage` (`libvips`), `zjxl` (`libjxl`), `mp4thumb`, `zwalker` (local media crawler), `zocr` (ONNX OCR), `v8-profiles`, `file-utilities`, `os-sys-info`, `zlang`.
   - **Cross-Platform Packaging Note**: `ZaloSetup-universal-26.11.10.dmg` (macOS) exposes `app.asar` + `app.asar.unpacked` directly; `ZaloSetup-26.11.10.exe` (Windows) nests `app.asar` inside a 32-bit `app-32.7z` sub-archive; `zalo-for-linux` reuses the macOS `26.11.10` JS bundle and applies runtime patches (`patch-disk-space.js`, `patch-network-and-sync-fix.js` forcing Sync V2 off so Sync V1 runs).
3. **Device Identity Persistence (`z_uuid` & `logged_uids` in `reqqr`)**:
   - Official Zalo PC generates `<uuid_v4>-<md5(UserAgent)>` (`69` ASCII chars) once and persists it in `localStorage["z_uuid"]` across logouts, while `Le()` in `login-startup.js` passes all previously authenticated UIDs on that machine joined by commas (`"<uid1>,<uid2>"`) in `reqqr`'s `logged_uids` field.
   - If either `imei` changes or `logged_uids` is empty, Zalo Mobile displays the red *"Log into Zalo PC with QR code on an unknown device?"* anti-phishing prompt (still allowing login + `[✓] Sync messages to this device` upon confirmation). `alolite` mirrors this via `~/.local/share/alolite/device.json` (`DeviceIdentity`).

---

## 3. Mobile-to-PC Message Sync Protocols (`Sync V1` vs `Sync V2`)

```mermaid
sequenceDiagram
    participant PC as Desktop Client (PC)
    participant Srv as Zalo Server (HTTP + WS)
    participant Mob as Primary Device (Mobile)

    alt Sync V1 (SyncMessageController — RSA-2048 + SQLite ZDB4.0)
        PC->>PC: Generate ephemeral RSA-2048 keypair (SPKI DER Base64 / PKCS#1 PEM)
        PC->>Srv: GET /api/message/pull_mobile_msg (cmd 12000: public_key, from_seq_id, min_seq_id, temp_key, imei)
        Srv->>Mob: Push backup export prompt
        Mob-->>Srv: WS cmd 601 transfer_status / user_confirm (pc_name, public_key, user_action=1)
        Mob->>Srv: Upload .db.crypt (ZDB4.0) + RSA_PKCS1_PADDING(AES_key, public_key)
        Srv-->>PC: WS cmd 601 act_type="syncmsgmb" | "transfer_msg" (url, checksum, encryptedKey, format=0|1)
        PC->>PC: RSA privateDecrypt(encryptedKey) -> uppercase 32-byte ASCII key -> AES-256-CBC (64 KiB, IV=0) + LZMA
        PC->>PC: Open SQLite snapshot & decode binary TLV columns (parseBinNet)
        PC->>Srv: GET /api/syncmsg/confirm (cmd 12111) & /api/message/delete_snapshot_mobile_msg (cmd 12003)
    else Sync V2 (SyncController — Trusted Device IK + libzproto_wasm + Zstd Protobuf)
        PC->>PC: Load Companion IdentityKeyPair (ik) from QR2 + instantiate libzproto_wasm_bg.wasm
        PC->>Srv: WS cmd 592 (REQUEST_MOBILE_WAKE_UP) & cmd 590 (SYNC_MESSAGE.REQUEST) [or HTTP cmd 12888]
        Srv->>Mob: Forward Sync V2 request
        Mob->>Srv: Upload Zstd-compressed, zprotoSync2-encrypted Protobuf chunks
        Srv-->>PC: WS cmd 601 act_type="transfer_sync2" (sessionId, senderIdentity, encMeta, msgUrl)
        PC->>PC: zprotoSync2DecryptMetadata(ik) -> sessionRecord -> zprotoSync2DecryptMessage(chunk)
        PC->>PC: Zstd decompress -> Protobuf proto.Sync2.Message.SyncChunk / Conversation.SyncChunk
        PC->>Srv: WS cmd 591 (ACK_DELETE_SYNC_SESSION)
    end
```

### 3.1 Side-by-Side Comparison

| Dimension | Sync V1 (`SyncMessageController`) | Sync V2 (`SyncController` + `sync2-proto`) |
|:---|:---|:---|
| **Feature Gate** | Fallback / default when Sync V2 is disabled (`zalo-for-linux` forces `SyncController.isFeatureEnabled() -> false` to use V1). | Enabled when server config activates Sync V2 **and** companion holds a valid Trusted Device `IdentityKeyPair` (`ik`). |
| **Asymmetric Key** | Ephemeral **RSA-2048** (`SPKI` DER Base64 `public_key` starting `MIIBIjANBgkq...`). | Persistent 33-byte **Curve25519/Ed25519 Identity Key (`ik`)** (`0x05` prefix) + ephemeral key `ek` (`zprotoEd25519GenerateKeyPair`). |
| **Key Transport** | Mobile encrypts symmetric key with `RSA_PKCS1_PADDING` (`encryptedKey` in WS control event). | WASM `zprotoSync2CreateMetadataCipher` / `zprotoSync2DecryptMetadata` produces opaque `sessionRecord`. |
| **Chunk Cipher** | `AES-256-CBC` in independent `65,536`-byte (`64 KiB`) blocks with `IV = [0u8; 16]` reset per block (`ZDB4.0`). | `zprotoSync2EncryptMessage` / `zprotoSync2DecryptMessage` inside `libzproto_wasm_bg.wasm`. |
| **Compression** | **LZMA** (`liblzma` stream starting immediately after 6-byte `"ZDB4.0"` magic). | **Zstandard (`Zstd`)** (`@oneidentity/zstd-js` `ZstdStream`). |
| **Data Format** | Raw **SQLite 3** database file with binary **TLV** blobs for structured columns (`attach`, `quote`, `mention`). | **Protobuf (`proto3`)** `SyncChunk` messages (`Partition` → `Message` / `Conversation`). Zero SQLite involvement. |
| **Control WS Routing** | `cmd 601` `act_type = "syncmsgmb"` or `"transfer_msg"` (when `SyncController.isFeatureEnabled()` is `false`). | `cmd 601` `act_type = "transfer_sync2"` or `"transfer_msg"` (when `SyncController.isFeatureEnabled()` is `true`). |

---

### 3.2 Sync V1 Detailed Wire Specification [O, V]

#### 3.2.1 Ephemeral RSA-2048 Key Generation & Decryption (`sync-message-key-generator`)
1. **Keypair Generation**:
   - Algorithm: `RSA`, modulus length `2048` bits, public exponent `65537`.
   - `publicKey`: `SPKI` DER encoded as standard Base64 (`294` bytes → `392` Base64 characters).
   - `privateKey`: `PKCS#1` PEM (`-----BEGIN RSA PRIVATE KEY-----`).
   - Key TTL: `cross_setting.timeoutKey * 3_600_000` ms.
2. **Decrypting `encryptedKey` from Mobile**:
   - Padding: **`RSA_PKCS1_PADDING`** (PKCS#1 v1.5).
   - Ciphertext input: Base64 string `encryptedKey` from the mobile backup notification.
   - Output encoding depends on backup `format`:
     - `format == 1` (`ZDB4.0` — modern cross-platform backup): decode decrypted bytes as lowercase/uppercase **hex string**; `db-cross-v4` then converts to uppercase ASCII and takes the **first 32 ASCII bytes** (`privateKey.toUpperCase()[0..32]`) as the raw 256-bit AES key.
     - `format == 0` (legacy `V0` backup): decode decrypted bytes as Base64 string for `decompressAndDecryptDb` (`aescryptzalo` + `.tar.lzma` / `.zip`).

#### 3.2.2 Sync V1 HTTP Endpoints (`FileDomain` / `ChatDomain`)
All endpoints attach common session query parameters (`zpw_ver`, `zpw_type`, `signkey`) and `ParamCipher v2` AES-encrypted `params`:

| `commandId` | Method & Path | Domain | Plaintext JSON `params` Schema | Purpose |
|---:|:---|:---|:---|:---|
| `12000` | `GET /api/message/pull_mobile_msg` | `file` | `{"pc_name": string, "public_key": string, "from_seq_id": u64, "is_retry": 0\|1, "min_seq_id": u64, "temp_key": string, "imei": string}` *(note: if `maxSeq > 0`, client sends `from_seq_id = maxSeq + 1`, else `0`)* | Request Mobile to package & upload message DB snapshot |
| `12412` | `GET /api/message/get_crossdb` | `file` | `{"pc_name": string, "sync_session": string}` (+ HTTP header `nretry`) | Poll cross-DB package descriptor by `sync_session` |
| `12700` | `GET /api/message/cancel_pull_mobile_msg` | `file` | `{"pc_name": string, "public_key": string, "imei": string}` | Cancel in-flight Sync V1 request |
| `12003` | `GET /api/message/delete_snapshot_mobile_msg` | `file` | `{"public_key": string, "imei": string}` | Delete temporary encrypted snapshot from server after import |
| `12096` | `GET /api/message/get_backupmsginfo` | `file` | *(query common params only; no `params` body; header `nretry`)* | Query available cloud/mobile backup metadata |
| `12390` | `POST /api/znoise` | `zwid` (`https://zwid.api.zalo.me`) / `profile` fallback | `{"fids": i64[], "gids": i64[]}` (capped at `10,000` combined IDs per call) | Translate plain numeric SQLite `SenderId` / filename IDs into session-scoped noised IDs |
| `12111` | `GET /api/syncmsg/confirm` | `login` | *(registered in command table only; **zero callers** in `app.asar` — Sync V1 finalizes via `12003` `cleanSync`)* | Legacy/unused command registry entry |

**WebSocket Control Payload (`cmd 601`, `act_type = "syncmsgmb"`, `act = "syncmsg_info"`):**
Delivers `{"uid": i64|string, "url": string, "file_name": string, "encrypted_key": string, "checksum_code": string, "from_seq_id": i64, "is_full_transfer": 0|1, "public_key": string, "db_info": "{\"db_format\":1,\"backup_db\":{\"msg_thread\":N,\"msg_total\":M}}"}`, where `uid` (`rawUid`) is the authenticated user's plain numeric `SenderId` inside `ChatContent`.

#### 3.2.3 `ZDB4.0` Binary Container Layouts & Chunk Decryption (`decompressAndDecryptDb_V2`) [O, V]
1. **Input**: Encrypted `.db.crypt` file downloaded from CDN (`trans-bin.zaloapp.com`), `16`-byte AES block aligned.
2. **AES Key Derivation**:
   $$\text{K}_{\text{AES}} = \text{ASCII}(\text{ Uppercase}(\text{Hex}(\text{RSA\_Decrypt}(\text{encryptedKey})))[0..32] ) \in \{0..255\}^{32}$$
3. **Chunked `AES-256-CBC` Decryption**:
   - Split ciphertext into contiguous chunks of **`65,536` bytes (`0x10000`, 64 KiB)**; for **every** chunk, reset $\text{IV} = 0^{16}$ (`NoPadding` per chunk; trailing PKCS#7 pad bytes follow the compressed stream).
4. **Two Observed Inner Container Layouts**:
   - **Layout A (Live Mobile Multi-DB TOC + XZ, verified live 2026-11-07/08)**:
     - `P[0..7] == "ZDB4.0\0"`, `P[7..14]` = 7-byte header metadata, `P[14..18]` = `u32_be n_files`.
     - Followed by `n_files` big-endian TOC records: `[name_len: u32_be][name: utf8 (e.g. "<fid>.db" or "group_<gid>.db")][uncompressed_size: u32_be]`.
     - Followed immediately by an **XZ** stream (`\xFD7zXZ\0` … `YZ` footer) whose decompressed byte buffer slices sequentially into `(name, uncompressed_size)` SQLite 3 files.
   - **Layout B (Direct Stream)**: `P[0..6] == "ZDB4.0"` followed directly by an LZMA/XZ stream of a single SQLite file.

#### 3.2.4 Restored SQLite `ChatContent` Schema & Archive `MsgType` Mapping [O, V]
- **Query**: `SELECT SenderId, CliMsgId, GlbMsgId, MsgStatus, MsgType, TimeStamp, MsgContent, BinNet FROM ChatContent WHERE MsgStatus > 0 ORDER BY TimeStamp ASC`
- **Excluded System/Control `MsgType` Filter (`Qe` in `shared-worker`)**: `[20, 21, 25, 26, 29, 32, 34, 35, 45, 51, 52]`.
- **Archive `MsgType` → Wire Type (`Le` / `je`)**:
  `0 -> "webchat"`, `2 -> "chat.doodle"`, `3 | 4 | 31 -> "chat.photo"`, `6 -> "chat.voice"`, `10 -> "chat.sticker"`, `12 -> "chat.recommended"`, `15 -> "chat.list.action"`, `18 -> "chat.location.new"`, `19 -> "chat.video.msg"`, `22 -> "share.file"`, `23 -> "chat.gif"`, `24 -> "chat.webcontent"`, `25 -> "chat.video.live.msg"`, `33 -> "chat.delete"`, `36 -> "chat.undo"`, `53 -> "chat.webcontent.v2"`.
- **Sender & Outgoing Attribution**: `SenderId` is a plain numeric ID (not noised). A row is outgoing iff `SenderId == 0`, `SenderId == syncmsg_info.uid` (`rawUid`), or `/api/znoise(SenderId) == session.login.uin`. In `group_<gid>.db`, incoming `SenderId`s map via `/api/znoise` (`fids`) to each group member's noised UID.

#### 3.2.5 Complete `BinNet` Binary TLV Specification (`parseBinNet`) [O, V]
- **Entry Framing**: contiguous `[tag: u32][length: u32][value: u8 × length]` starting at byte `0` (little-endian on mobile ARM; big-endian auto-detected by tag-range scoring). Integers use 1/2/4/8-byte signed encoding matching container endianness; strings use UTF-8 (or UTF-16LE).

| Scope | Tag (`hex`) | Field Name | Value Type & Semantics |
|:---|:---|:---|:---|
| **Top-Level** | `0x05` | `property` | Nested TLV (`0x1E..0x22` text styling) |
| **Top-Level** | `0x06` | `attachs[]` | Repeated nested TLV (`0x28..0x44` media/sticker/file/link card) |
| **Top-Level** | `0x07` | `quote` | Nested TLV (`0x50..0x5A` quoted reply metadata) |
| **Top-Level** | `0x08` | `mentions[]` | Repeated nested TLV (`0x65..0x68` `@mention` spans) |
| **Top-Level** | `0x09` / `0x0A` / `0x0B` | `quoteStatus` / `dataSource` / `data` | `i32` / UTF-8 / UTF-8 |
| **`property` (`0x05`)** | `0x1E`, `0x1F`, `0x20`, `0x21`, `0x22` | `size`, `subType`, `type`, `ext`, `color` | `int`, `int`, `int`, `string`, `int` |
| **`attachs` (`0x06`)** | `0x28`, `0x29`, `0x2A`, `0x2B`, `0x2C` | `type`, `catId`, `id`, `extInfo`, `childNumber` | `string`, `int` (sticker cateId), `int` (stickerId), `string`, `int` |
| **`attachs` (`0x06`)** | `0x2D`, `0x2E`, `0x2F`, `0x30`, `0x31` | `action`, `params`, `title`, `href`, `thumb` | `string` (`recommened.*`), `string` (JSON), `string`, `string` (URL; `.jxl` normalizable to `.jpg`), `string` (URL) |
| **`attachs` (`0x06`)** | `0x32`, `0x33`, `0x34`, `0x35`, `0x36`, `0x37` | `description`, `size`, `ext`, `subType`, `color`, `property` | `string` |
| **`attachs` (`0x06`)** | `0x40`, `0x41`, `0x42`, `0x43`, `0x44` | `dataSource`, `data`, `remains`, `zinstantData`, `zinstantMsg` | `string` |
| **`quote` (`0x07`)** | `0x50`, `0x51`, `0x52`, `0x53`, `0x54`, `0x55` | `ownerId`, `cliMsgId`, `globalMsgId`, `cliMsgType`, `ts`, `ttl` | `i32` (plain UID), `i64`, `i64`, `i32`, `i64`, `i64` |
| **`quote` (`0x07`)** | `0x56`, `0x57`, `0x58`, `0x5A` | `msg`, `attach`, `quoteStatus`, `fromD` | `string` (quoted text), `string` (JSON), `string`, `string` (quoted sender display name) |
| **`mentions` (`0x08`)** | `0x65`, `0x66`, `0x67`, `0x68` | `uid`, `pos`, `len`, `type` | `int` (plain UID), `int`, `int`, `int` |
---

### 3.3 Sync V2 Detailed Wire Specification (`SyncController` & `sync2-proto`) [O]

#### 3.3.1 Transport Commands (WebSocket & HTTP)

| Transport | Identifier | Envelope / `params` Schema | Purpose |
|:---|:---|:---|:---|
| **WebSocket** | `cmd: 592, subCmd: 0` (`SYNC_MESSAGE.REQUEST_MOBILE_WAKE_UP`) | `{"data": <wakeup_payload>}` (timeout `3000 ms`, exponential backoff) | Wake up Mobile app before initiating Sync V2 session |
| **WebSocket** | `cmd: 590, subCmd: 0` (`SYNC_MESSAGE.REQUEST`) | `{"data": {"data": <sync_v2_request>}}` (timeout `10000 ms`) | Primary Sync V2 request carrying `syncId`, companion `ik`, and partition/time scope |
| **HTTP Fallback** | `commandId: 12888` `GET /api/transfer-sync-v2/request-sync` | `{"reqId": string, "data": JSON.stringify(<sync_v2_request>)}` (`FileDomain`) | HTTP request-sync fallback / companion trigger |
| **WebSocket Push** | `cmd: 601` (`CTRL_PUSH_CTRL`) `act_type = "transfer_sync2"` | Routed to `SyncController.handleReceiveServerResponse` | Delivers encrypted metadata (`encMeta`), `senderIdentity`, `sessionId`, and chunk download URL (`msgUrl`) |
| **WebSocket** | `cmd: 591, subCmd: 0` (`SYNC_MESSAGE.ACK_DELETE_SYNC_SESSION`) | `{"data": {"data": <ack_payload>}}` (timeout `3000 ms`) | Acknowledge chunk import & instruct server to delete Sync V2 session artifacts |


**Inner `<sync_v2_request>` JSON Schema (`cmd 590` & `commandId 12888`):**
```json
{
  "syncId": "<32-hex>",
  "syncType": 0,
  "ek": "<base64_33B_ephemeral_pub>",
  "ik": "<base64_33B_companion_identity_pub>",
  "toDevice": 0,
  "tempKey": "<temp_key_from_transfer_after_login>",
  "deviceName": "<hostname>",
  "req": {
    "type": "message",
    "priority": 1,
    "queries": [{"partition": "", "from": 0, "to": 9007199254740991, "limit": 2000}],
    "batchSize": 2000
  },
  "ver": 1,
  "ussidx": 0
}
```
#### 3.3.2 WASM Cryptographic Bridge (`libzproto_wasm_bg.wasm`)
`sync2-proto` (in `shared-worker`) initializes `libzproto_wasm_bg.5ea0fb2c9af4a45bc7882b71307b6f83.wasm` with the companion's Trusted Device `identityKey` (`{ publicKey: Uint8Array(33), privateKey: Uint8Array(32) }`):

| Exported WASM Function | Inputs | Outputs | Role |
|:---|:---|:---|:---|
| `zprotoEd25519GenerateKeyPair` | *(none)* | `{ publicKey: [u8; 33], privateKey: [u8; 32] }` | Generates ephemeral sync keypair `ek` (or new `ik`) |
| `zprotoSync2CreateMetadataCipher` | `(sessionId, ikPub, ikPriv, receiverIkPub, plaintextMeta, optSeed)` | `{ sessionId, ciphertext, receiverIdentity, sessionRecord }` | Sender (Mobile) creates encrypted session metadata & stateful `sessionRecord` |
| `zprotoSync2DecryptMetadata` | `(sessionId, ikPub, ikPriv, senderIkPub, ciphertextMeta)` | `{ sessionId, plaintext, senderIdentity, sessionRecord }` | Receiver (PC) authenticates `senderIdentity`, decrypts metadata, and derives `sessionRecord` |
| `zprotoSync2EncryptMessage` | `(sessionRecord, plaintextBytes)` | `{ sessionId, ciphertext }` | Encrypts a Zstd-compressed `SyncChunk` |
| `zprotoSync2DecryptMessage` | `(sessionRecord, ciphertextBytes)` | `{ sessionId, plaintext }` | Decrypts a downloaded chunk into a Zstd frame |
| `zprotoSync2GetSessionId` | `(ciphertextOrRecord)` | `sessionId: Uint8Array` | Extracts session identifier from header |

**Sync V2 Error Code Taxonomy (`sync-v2-sub-worker`):**
`Success = 0`, `EncryptMeta = 101`, `DecryptMeta = 102`, `EncryptMessage = 201`, `DecryptMessage = 202`, `Serialize = 301`, `Deserialize = 302`, `Compress = 401`, `Decompress = 402`, `IO = 501`, `PartitionNotFound = 502`, `NoiseId = 601`, `UploadError = 701`, `DownloadError = 702`, `Unknow = 999`, `ClientNotSupport = 1000`, `Timeout = 1100`, `ConnectTimeout = 1200`, `WakeupTimeout = 1300`, `BatchTimeout = 1400`, `Network = 1500`, `BackupRangeError = 1600`, `DatabaseClosed = 1700`, `RequestSocketClosed = 1800`.

#### 3.3.3 Clean-Room Protobuf Schema (`proto.Sync2.*`) [O]
After `zprotoSync2DecryptMessage` + `ZstdStream.decompress`, the plaintext bytes decode as `proto3` binary messages:

```protobuf
syntax = "proto3";
package Sync2;

// Message chunk stream (proto.Sync2.Message.*)
message MessageSyncChunk {
  int32              platform   = 1;
  repeated Partition partitions = 2;
}

message Partition {
  string           id       = 1; // Conversation/partition ID (noised global ID; "g..." for groups)
  repeated Message messages = 2;
}

message Message {
  string   sender_id  = 1;
  uint64   client_id  = 2;
  int32    msg_status = 3;
  int32    msg_type   = 4;
  uint64   timestamp  = 5;
  string   global_id  = 6;
  string   content    = 7;
  MetaData meta       = 8;
}

message MetaData {
  repeated Attach  attachs  = 1;
  repeated Mention mentions = 2;
  Reference        reference = 3;
  Quote            quote     = 4;
  Properties       property  = 5;
  int64            ttl       = 6;
}

message Attach {
  string type          = 1;
  string ext_info      = 2;
  string action        = 3;
  string params        = 4;
  string title         = 5;
  string href          = 6;
  string thumb         = 7;
  string description   = 8;
  int32  cat_id        = 9;
  string id            = 10;
  int32  child_number  = 11;
  int32  remains       = 12;
  bytes  zinstant_data = 13;
  string zinstant_msg  = 14;
}

message Mention {
  string uid  = 1;
  int32  pos  = 2;
  int32  len  = 3;
  int32  type = 4;
}

message Quote {
  string attach        = 1;
  string owner_id      = 2;
  uint64 ts            = 3;
  string from_d        = 4;
  uint64 cli_msg_id    = 5;
  string global_msg_id = 6;
  string msg           = 7;
  int32  cli_msg_type  = 8;
  int64  ttl           = 9;
  string partial_quote = 10;
}

message Properties {
  int32  size     = 1;
  int32  color    = 2;
  int32  type     = 3;
  int32  sub_type = 4;
  string ext      = 5;
}

message Reference {
  int32  type = 1;
  string data = 2;
}

// Conversation index stream (proto.Sync2.Conversation.*)
message ConversationSyncChunk {
  int32                 platform      = 1;
  repeated Conversation conversations = 2;
}

message Conversation {
  string conv_id          = 1;
  int32  conv_type        = 2;
  bool   responded_by_me  = 3;
  uint64 last_ts          = 4;
  string last_global_id   = 5; // uint64 stringified
  uint64 last_client_id   = 6;
}
```

---

### 3.4 Reverse Flow: PC-to-Mobile Backup (`BACKUP_MSG`) [O]
Zalo Desktop also includes a reverse backup pipeline (`BackupMsgHandler` in `shared-worker`) using **`libzproto_backup_wasm_bg.042497d03f0f504b6b1444e25a7db0d9.wasm`**:
- **WebSocket Commands**: `630` (`BACKUP_MSG.INIT`), `631` (`CREATE_SESSION`), `632` (`GET_METADATA`), `633` (`SIGNAL_RESTORE`), `634` (`GET_CONFIGS`).
- **HTTP Upload / Cleanup**: `commandId: 13538` `POST /api/backuppc2mb/upload` (`BackupPcMobileDomain`) and `commandId: 15308` `POST /api/transfer/delete` (`ZInstantMsgDomain`).
- **Crypto Session (`zprotobackupcryptosession_new`)**: initialized from `(serverNonce, clientSalt, obfuscatedStaticSecret)` where the static secret is reconstructed in JS by XORing three byte arrays; chunks are serialized with `proto.Sync2.*`, compressed with `Zstd`, and encrypted via `zprotobackupcryptosession_encryptMessage`. Partition IDs are translated from PC-noised IDs to global-noised IDs (`pcNoised2gNoisedMulti`) before export.

---

## 4. End-to-End Encryption (E2EE) Protocol Specification [O]

```mermaid
sequenceDiagram
    participant PC as Sender PC (deviceId=2)
    participant WS as Zalo WebSocket (SignalCommands)
    participant API as Zalo HTTP (CHAT_E2EE / GROUP)
    participant Peer as Recipient Subdevices (0=Mob, 1=Web, 2=PC)

    Note over PC,WS: 1. Subdevice Registration & X3DH Discovery
    PC->>WS: cmd 564 GET_KEY_ID (platformType=2)
    PC->>WS: cmd 540 SUBMIT_BUNDLE (registrationId, identityKey, spKey+sig, opKeys[])
    PC->>WS: cmd 563 GET_LIST_USER_E2EE (paginated active E2EE threads/uids)

    alt 2A. Outbound 1-1 E2EE Message (Sesame Multi-Device Fan-Out)
        PC->>WS: cmd 541 GET_BUNDLE (target uid + sender's own other subdevices [0,1,2])
        PC->>PC: Build X3DH sessions -> encrypt ordered field vector ciphers[] per (userId, deviceId)
        PC->>WS: cmd 543 INIT_SESSION (when establishing new session)
        PC->>API: POST /api/e2ee/pc/message/{sms|photo|file|...} (cmd 16000..16012: listParams[], destType=3)
        WS-->>Peer: cmd 551 RECEIVE_ONEONE (or cmd 535 PUSH_MISS)
        Peer->>WS: cmd 557 DELETE_QUEUE_ACK_ONEONE & HTTP cmd 16200/16201 delivered/seen
    else 2B. Outbound Group E2EE Message (SenderKey Broadcast)
        PC->>PC: Generate SenderKeyDistributionMessage for groupId
        PC->>WS: cmd 561 INIT_SESSION_GROUP (pairwise 1-1 SessionCipher-encrypted SenderKey per member device)
        PC->>PC: Encrypt ordered field vector ciphers[] once via GroupCipher (SenderKeyMessage)
        PC->>API: POST /api/e2ee/pc/group/{sms|photo|mention|...} (cmd 16100..16113)
        WS-->>Peer: cmd 552 RECEIVE_GROUP (or cmd 514 RECEIVE_GROUP_OFFLINE)
        Peer->>WS: cmd 558 DELETE_QUEUE_ACK_GROUP & HTTP cmd 16203/16204 delivered/seen
    end
```

### 4.1 Multi-Device Addressing & Zalo Signal Customizations

1. **Subdevice Identifiers (`DeviceIds`)**:
   - `MOBILE = 0`
   - `WEB = 1`
   - `PC = 2` (Zalo Desktop always operates as `deviceId = 2`)
   - `SYSTEM = 3`
   - `UNKNOWN = 6`
   - Broadcast target set for 1-1 fan-out: `LIST_SUBDEVICES = [0, 1, 2]` across **both** the recipient's active devices and the sender's own active companion devices (skipping self `(myUid, 2)` and devices marked `inactive` in `MetadataStorageService`).
2. **Zalo Extensions over Upstream `libsignal-protocol-javascript`**:
   - **Session Timestamp Guard (`getSessionTs` / `messageTs`)**: `decryptPreKeyWhisperMessage` and `decryptWhisperMessage` accept `messageTs` and compare/store `sessionTs` (`libsignal_control_session_state`) to reject stale prekey promotions (`onPreventPromoteSession`) when multiple devices re-initialize concurrently.
   - **Common Virtual Group Scopes (`FakeGroupId`)**: `COMMON_INFO = "g1000"`, `COMMON_PAIRWISES_INFO = "g1001"`.
   - **Message Type Compatibility Versions (`VERSION_MSG`)**:
     - `version = 0, compat_version = 0`: `TEXT_MSG`, `LINK_MSG`, `GIF_MSG`, `STICKER_MSG`, `FILE_MSG`, `PHOTO_MSG`, `FWD_VOICE_MSG`, `FWD_VIDEO_MSG`, `FWD_LOCATION_MSG`, `DOODLE_MSG`.
     - `version = 1, compat_version = 0`: `OA_MSG`, `FWD_CONTACT_MSG`.
   - **Undecryptable Placeholder States**: `MSG_E2EE_WAIT` (waiting for sender resend / offline session queue), `MSG_E2EE_CHANGE_DEVICE`, `MSG_E2EE_REPLACE_WAITING`, `MSG_E2EE_DUPLICATED`.

### 4.2 Ciphertext Wire Framing & Per-Field Ratchet Order

#### 4.2.1 1-1 Ciphertext String Encoding (`splitTypeAndMsg`)
Each element in `ciphers[]` for a 1-1 message is a standard Base64 string encoding a **1-byte type prefix** concatenated with the `MessageProto` binary protobuf payload:

$$\text{cipher\_b64} = \text{Base64}\big(\, [\text{type\_u8}] \;\parallel\; \text{ProtobufBytes} \,\big)$$

| `type_u8` | Signal Message Class | Protobuf Schema (`MessageProtocol.proto`, `proto2`) |
|:---:|:---|:---|
| `0x03` (`3`) | `PreKeySignalMessage` | `1: optional uint32 preKeyId`, `2: optional bytes baseKey`, `3: optional bytes identityKey`, `4: optional bytes message` (`SignalMessage`), `5: optional uint32 registrationId`, `6: optional uint32 signedPreKeyId` |
| `0x01` (`1`) | `SignalMessage` | `1: optional bytes ratchetKey`, `2: optional uint32 counter`, `3: optional uint32 previousCounter`, `4: optional bytes ciphertext` *(followed by 8-byte truncated Signal MAC per standard Signal wire format)* |

For **Group E2EE**, `GroupCipher` produces `MessageProto.SenderKeyMessage` (`1: optional uint32 id`, `2: optional uint32 iteration`, `3: optional bytes ciphertext`), after distributing `MessageProto.SenderKeyDistributionMessage` (`1: id`, `2: iteration`, `3: chainKey`, `4: signingKey`) pairwise via `cmd 561`.

#### 4.2.2 Ordered Per-Field Encryption Vector (`ciphers[]`)
Zalo E2EE **does not** encrypt the entire message JSON as a single blob. Instead, `extractTextContentFromServer` / `encrypt` extracts an ordered list of sensitive fields and encrypts **each non-empty field sequentially** through the same `SessionCipher` / `GroupCipher` (advancing the chain counter once per field):

| Normalized `msgType` | Ordered Elements Encrypted into `ciphers[0..N]` |
|:---|:---|
| `MSG_TEXT` | `[0]` plain text (or RTF title); `[+]` RTF `content.params` (if RTF); `[+]` if `quote` present: `quote.msg`, `quote.fromD`, `quote.gOwnerId`, `quote.attach` (each if present); `[+]` each `mentions[i].gUid`; `[+]` `reference.data` (if present). |
| `MSG_PHOTO` | `[0]` embedded Base64 file key from `content.href` (if E2EE CDN link, else `content.href`); `[1]` `content.params`; `[2]` embedded Base64 key from `content.thumb` (or `content.thumb`); `[3]` URI-decoded `content.title`; `[4]` `content.description`; `[+]` `mentions[i].gUid`; `[+]` `reference.data`. |
| `MSG_FILE` | `[0]` embedded Base64 file key from `content.href`; `[1]` `content.params`; `[2]` URI-decoded `content.title`; `[3]` embedded Base64 key from `content.thumb`; `[4]` `content.description`; `[5]` `content.type`; `[+]` `reference.data`. |
| `MSG_CONTACT` (`recommened.link`) | `[0]` URI-decoded `content.href`; `[1]` `content.params`; `[2]` `content.title`; `[3]` E2EE key or URI-decoded `content.thumb`; `[4]` `content.description`; `[5]` `content.type`; `[+]` `mentions[i].gUid`; `[+]` `reference.data`. |

> **Media Attachment Note:** Binary photo/file payloads are symmetrically encrypted client-side before CDN upload; only the per-attachment symmetric decryption key (embedded in the E2EE upload URL query/fragment) and metadata strings are passed through the Signal ratchet in `ciphers[]`.

---

### 4.3 Complete WebSocket `SignalCommands` Table (`cmd 514..582`, `subCmd: 0`)

| `cmd` | Symbolic Name | Direction | Wire `data` Payload Schema (`{ "reqId": "signal_req_N", "data": ... }`) |
|---:|:---|:---|:---|
| `514` | `MSG.RECEIVE_GROUP_OFFLINE` | Server → PC | Offline group E2EE message batch (`groupMsgs[]`, `lastActionId`, `more`) |
| `516` | `INIT.SESSION_OFFLINE` | Server → PC | Offline E2EE session control queue (`eesession[]`, `queueStatus`) |
| `517` | `INIT.SESSION_OFFLINE_WEB` | Server → PC | Web/companion offline E2EE session queue |
| `518` | `INIT.OFFLINE_ACK_ONE_ONE` | PC → Server | Acknowledge processed offline 1-1 E2EE session events |
| `519` | `INIT.OFFLINE_ACK_GROUP` | PC → Server | Acknowledge processed offline group E2EE session events |
| `535` | `MSG.PUSH_MISS` | Server → PC | Missed E2EE messages pushed on reconnect |
| `540` | `INIT.SUBMIT_BUNDLE` | PC → Server | `{"delSS": 0\|1, "key": {"deviceId": 2, "preKey": {"iKey": {"id": u32, "key": b64}, "signature": b64, "spKey": {"id": u32, "key": b64}}, "opKeys": [{"id": u32, "key": b64}, ...]}}` |
| `541` | `INIT.GET_BUNDLE` | PC → Server | `{"uids": [{"uid": string, "deviceId": u8}], "isReInit": 0\|1, "msgInfoNoti": 0\|1, "isAutoUpgrade": 0\|1}` *(err `-9` returns `errData` listing missing self subdevices: `0 -> -9.1`, `2 -> -9.2`, `1 -> -9.3`)* |
| `542` | `INIT.SUBMIT_PREKEYS` | PC → Server | `{"deviceId": 2, "id": registrationId, "opKeys": [{"id": u32, "key": b64}, ...]}` |
| `543` | `INIT.INIT_SESSION` | Bidirectional | `{"type": 0\|1, "uidTo": string, "data": [...]}` |
| `544` | `INIT.ACK_INIT_SESSION` | Bidirectional | `{"type": u8, "uidTo": string, "data": [{"deviceId": u8, "ts": u64, ...}]}` |
| `545` | `INIT.DONE_PROCESS_SESSION` | PC → Server | Signal completion of pending session processing |
| `546` | `INIT.ACK_GROUP_SESSIONS_STALE` | Bidirectional | Mark group `SenderKey` sessions stale requiring re-distribution |
| `547` | `INIT.DELETE_KEY` | PC → Server | `{"keyId": u32, "uids": string[], "ts": u64}` (or `"groupIds": string[]`) |
| `548` | `INIT.NOTIFY_DELETE_KEY` | Server → PC | Push notification that a peer/subdevice deleted its key bundle |
| `549` | `INIT.REQUEST_PREKEYS` | Server → PC | Server requests client to replenish one-time PreKeys (`opKeys`) via `542` |
| `550` | `ACK.SUBDEVICE_UNSUPPORT` | Bidirectional | Notify that a subdevice does not support E2EE |
| `551` | `MSG.RECEIVE_ONEONE` | Server → PC | Real-time incoming 1-1 E2EE message (`data.msgs[]`) |
| `552` | `MSG.RECEIVE_GROUP` | Server → PC | Real-time incoming Group E2EE message (`data.groupMsgs[]`) |
| `553` | `ACK.DELIVER_SEEN_ONEONE` | Server → PC | Push E2EE 1-1 delivered/seen receipt |
| `554` | `ACK.DELIVER_SEEN_GROUP` | Server → PC | Push E2EE group delivered/seen receipt |
| `557` | `ACK.DELETE_QUEUE_ACK_ONEONE` | PC → Server | Acknowledge decrypted offline 1-1 E2EE queue items |
| `558` | `ACK.DELETE_QUEUE_ACK_GROUP` | PC → Server | Acknowledge decrypted offline group E2EE queue items |
| `559` | `INIT.ACK_PROCESS_INIT_GROUP_FAILED` | Bidirectional | Report failed `SenderKeyDistributionMessage` decryption (`Bad MAC` / missing session) |
| `560` | `INIT.CHECK_COMPATIBLE` | PC → Server | `{"uids": [string]}` |
| `561` | `INIT.INIT_SESSION_GROUP` | Bidirectional | `{"type": u8, "uidTo": string, "tId": groupId, "data": [{"deviceId": u8, "data": b64Cipher, "ts": u64}]}` |
| `562` | `INIT.CHECK_COMPATIBLE_GROUP` | PC → Server | `{"uids": string[], "groupId": string, "enableEE": 0\|1}` |
| `563` | `INIT.GET_LIST_USER_E2EE` | PC → Server | `{"reqIid": 1, "first": bool, "lastId"?: string}` → returns `{uids, waitUids, threads, pendingUids, hasMore, lastId}` |
| `564` | `INIT.GET_KEY_ID` | PC → Server | `{"platformType": 2}` → returns `{keyId, submitTs}` (or `err = -1` if none registered) |
| `580` | `INIT.DISABLE_E2EE` | Bidirectional | Disable E2EE on a 1-1 conversation (`DISABLE_REASON`: `FRIEND=1, ME=2, INIT_FAILED=3, DOWNGRADE=4`) |
| `581` | `INIT.ENABLE_E2EE` | Bidirectional | Upgrade a 1-1 or group conversation to E2EE |
| `582` | `INIT.DOWNGRADE_E2EE_GROUP` | Bidirectional | Downgrade an E2EE group (`GROUP.MISMATCH = 5`) |

---

### 4.4 Outbound E2EE HTTP Endpoints (`16000..16204`)

All outbound E2EE HTTP calls send `POST` with `params=<url_encoded_AES(JSON)>` (`ParamCipher v2`):

#### 4.4.1 1-1 E2EE (`domainType = CHAT_E2EE`)
Outer JSON `params` envelope:
```json
{
  "listParams": "[{\"userId\":\"<uid>\",\"deviceId\":0,\"ciphers\":[...],\"registrationId\":12345}, ...]",
  "destType": 3,
  "threadId": "<peer_uid>",
  "clientId": 1794100000000
}
```
*(When `action == "forward"`, `"msgType": <u32>` is appended to the outer object.)*

| `commandId` | Endpoint Path | `commandId` | Endpoint Path |
|---:|:---|---:|:---|
| `16000` | `/api/e2ee/pc/message/sms` | `16007` | `/api/e2ee/pc/message/undo` |
| `16001` | `/api/e2ee/pc/message/sticker` | `16008` | `/api/e2ee/pc/message/multi_undo` |
| `16002` | `/api/e2ee/pc/message/link` | `16009` | `/api/e2ee/pc/message/delete` |
| `16003` | `/api/e2ee/pc/message/quote` | `16010` | `/api/e2ee/pc/message/deleteconver` |
| `16004` | `/api/e2ee/pc/message/photo` | `16011` | `/api/e2ee/pc/message/gif` |
| `16005` | `/api/e2ee/pc/message/file` | `16012` | `/api/e2ee/pc/message/forward` |
| `16006` | `/api/e2ee/pc/message/contact` | `16200` / `16201` | `/api/e2ee/pc/t/message/{delivered,seen}` |

#### 4.4.2 Group E2EE (`domainType = CHAT_E2EE_GROUP`)
Outer JSON `params` envelope merges the single `GroupCipher` payload `t[0]` with `"clientId"` (`{ ...t[0], "clientId": <u64> }`):

| `commandId` | Endpoint Path | `commandId` | Endpoint Path |
|---:|:---|---:|:---|
| `16100` | `/api/e2ee/pc/group/sms` | `16107` | `/api/e2ee/pc/group/undoMsg` |
| `16101` | `/api/e2ee/pc/group/sticker` | `16108` | `/api/e2ee/pc/group/multiUndoMsg` |
| `16102` | `/api/e2ee/pc/group/link` | `16109` | `/api/e2ee/pc/group/deleteMsg` |
| `16103` | `/api/e2ee/pc/group/quote` | `16110` | `/api/e2ee/pc/group/deleteConv` |
| `16104` | `/api/e2ee/pc/group/photo` | `16111` | `/api/e2ee/pc/group/mention` |
| `16105` | `/api/e2ee/pc/group/file` | `16112` | `/api/e2ee/pc/group/forward` |
| `16106` | `/api/e2ee/pc/group/contact` | `16113` | `/api/e2ee/pc/group/gif` |
| `16203` / `16204` | `/api/e2ee/pc/t/group/{delivered,seen}` | | |

---

## 5. Residual Unknowns & Architectural Mapping for `alolite`

### 5.1 Explicit Residual Unknowns
1. **Internal KDF/AEAD inside `libzproto_wasm_bg.wasm` (`zprotoSync2*`)**: While all WASM import/export signatures, JS glue marshaling, Zstd framing, and `proto.Sync2.*` schemas are fully recovered (`ev_11f283c2...`), the internal symmetric cipher/KDF construction inside stripped `zprotoSync2CreateMetadataCipher` / `zprotoSync2DecryptMessage` has not been decompiled at the WAT instruction level (`wabt` was removed from the host prior to this pass). Re-implementing Sync V2 in pure Rust would require WAT/oracle differential analysis analogous to [note 11 §2](11_trusted_device_protocol.md).
2. **E2EE Attachment Binary Format**: E2EE photo/file upload (`MSG_PHOTO` / `MSG_FILE`) passes the Base64 symmetric key inside `ciphers[]`, while the exact byte layout (IV/MAC framing) of the encrypted file body uploaded to CDN was outside this static pass and should be verified before implementing E2EE media uploads.

### 5.2 `alolite` Implementation Status (Updated 2026-11-08)
1. **Shipped & Verified in Pure Rust (`270/270` tests passing)**:
   - **Persistent Device Identity (`src/protocol/session.rs`, `src/protocol/qr.rs`)**: `~/.local/share/alolite/device.json` (`0600`) retains stable `imei` (`<uuid_v4>-<md5(ZALO_UA)>`) and comma-joined `known_uids` (`logged_uids` in `reqqr`) across restarts, QR refreshes, and logouts.
   - **Trusted Device Protocol (`src/protocol/trust/`, [note 11](11_trusted_device_protocol.md))**: Pure-Rust QR2 linking & device-list verification ready behind server flag `qr_format_trust_device`.
   - **End-to-End Sync V1 (`src/protocol/history_sync.rs`, `src/protocol/chat.rs`, `src/protocol/endpoints.rs`)**:
     - Pure Sync V1 request gating (`cmd 12000` `pull_mobile_msg` without Sync V2 race).
     - `decrypt_and_unpack_zdb4_container` (64 KiB zero-IV `AES-256-CBC`, Layout A TOC + XZ & Layout B LZMA/XZ, TOC path-traversal sanitization).
     - `parse_bin_net_tlv` + `build_synced_message_content` (full nested TLV `0x06`/`0x07`/`0x08` decoding for photos with `.jxl`→`.jpg`, stickers, files, links, voice, video, and quoted replies).
     - `resolve_znoise_ids` (`ZWID` `https://zwid.api.zalo.me/api/znoise` + `profile` fallback, 5,000-ID batches) resolving both dynamic `is_outgoing` (`rawUid` / `znoise` match, zero hardcoded UIN) and group member `SenderId`s into account-scoped SQLite (`db_path_for_account`) inside a single `spawn_blocking` transaction, followed by `resolve_thread_names` and `clean_sync` (`cmd 12003`).
2. **Not Yet Implemented (Future Scope)**:
   - **Live Multi-Device Signal E2EE (`deviceId = 2`)**: Requires a pure-Rust Signal store + X3DH/Double-Ratchet/`SenderKey` engine implementing WebSocket `cmd 514..582` and HTTP `16000..16204` with the `[type_u8] || MessageProto` Base64 framing and ordered per-field `ciphers[]` vector specified in §4.2–§4.4. (Historical E2EE messages already decrypted on Mobile are imported today via Sync V1.)
   - **Sync V2 (`zprotoSync2`)**: Dormant while production `qr_format_trust_device == 0`; `import_sync_v2_messages` fails closed if an encrypted `zprotoSync2` chunk is received.
