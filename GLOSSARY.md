# Glossary

Ubiquitous language for `alolite`. Use these terms verbatim in issues, ADRs, tests and code identifiers; do not drift to synonyms.

> Purpose: keep protocol vocabulary, UI vocabulary and storage vocabulary aligned. Update via `/domain-modeling` when a term is resolved or renamed.

## Session & Authentication

| Term | Definition | Avoid |
| :--- | :--- | :--- |
| **Session** | Authenticated Zalo identity persisted in `session.json`: cookies, `zpw_sek`, `zpw_enk`, IMEI, user info. | "login state", "auth blob" |
| **`zpw_sek`** | Server-issued session cookie proving an authenticated session. Attached to every `*.zaloapp.com` / `*.chat.zalo.me` request. | "session key" |
| **`zpw_enk`** | Base64-encoded encryption key used to derive the AES key for `params`. | "encryption key" (ambiguous) |
| **QR login** | Login by scanning a QR code, then long-polling `pc-status` until the phone confirms. | "scan login" |
| **Password login** | Login with phone number + password via `PasswordCipher`. | "manual login" |

## Protocol

| Term | Definition | Avoid |
| :--- | :--- | :--- |
| **ParamCipher v2** | Request parameter encoding scheme (`src/protocol/param_cipher.rs`) required by the server. | "param encryption" |
| **`signkey`** | MD5-based signature over selected request parameters (`src/protocol/crypto.rs`), bound to a timestamp/tag. | "checksum", "hash" |
| **Command registry** | Sorted 437-endpoint HTTP `commandId` catalog and WebSocket `cmd` catalog in `src/protocol/registry/` dispatched via `Session::api_get` / `Session::api_post`. | "inline endpoint string" |
| **Service map** | Server-provided `zpw_service_map_v3` mapping logical services (`chat`, `group`, `file`, …) to hosts; managed by `Domains` (`src/protocol/session/domains.rs`) and `src/protocol/endpoints.rs`. | "host list" |
| **`error_code`** | Numeric status inside Zalo JSON responses, classified into `SessionError` / `SessionStatus` (`src/protocol/session/status.rs`). `0` means success. | "status" |
| **Push / Pull** | WebSocket message classes (`src/protocol/registry/ws.rs`): `501/521` push, `510/511` pull, `601–604` control. | "socket message" |

## Chat Domain

| Term | Definition | Avoid |
| **Thread** | A conversation (`src/protocol/messaging/threads.rs`): 1:1 user thread or group thread. Identified by `thread_id`. | "chat", "conversation" (when the ID matters) |
| **Message** | One chat entry (`ChatMessage`) with `msg_id`, `thread_id`, sender, timestamp, type and raw JSON. | "text" |
| **Local message ID** | Monotonic client-generated ID (`wire::next_client_message_id`) used to reconcile optimistic UI rows (`MessageIdentityIndex`) with server echoes. | "temp id" |
| **Recall** | Server-side removal of a message for all participants (`messaging::send::recall_message`). | "delete" (that is local-only) |
| **Delete for me** | Local-only removal (`messaging::send::delete_message_for_me`) mirrored into SQLite. | "recall" |
| **ZDB4.0 / BinNet** | Mobile Sync V1 encrypted archive container (`sync::zdb4`) and SQLite attachment/quote TLV blob (`sync::bin_net`), ingested into SQLite by `storage::sync_ingest`. | "raw backup dump" |
## Runtime

| Term | Definition | Avoid |
| :--- | :--- | :--- |
| **CoreEngine** | UI-independent Tokio runtime (`crates/alolite-core/src/core/`) owning `Session`, WebSocket/polling, SQLite, and MCP/REST; `crates/alolite-desktop` and `alolite-cli` act as thin adapters. | "backend", "service" |
| **UI thread** | The Tauri WebView event loop (`apps/desktop/src/`); must never block on I/O. | "frontend" |
| **Store** | Local SQLite database managed by `MessageStore` (`crates/alolite-core/src/storage/`), plus fast-startup JSON snapshot in `storage::thread_cache`. | "cache" (cache is non-authoritative) |

## Product & Workspace Naming

| Term | Definition | Avoid |
| :--- | :--- | :--- |
| **`AloLite`** | Canonical product display name for user-facing UI, window title, desktop entry name, notifications, and documentation headers. | "Alolite", "AloLite Linux", "Alolite Linux", "Alo-Lite" |
| **`alolite`** | Headless CLI binary, system desktop entry executable/file name, base data directory (`~/.local/share/alolite`), and root npm package name. | "alo-lite", "alolite-cli" (when referring to the binary) |
| **`alolite-desktop`** | GUI binary name (`target/release/alolite-desktop`), Tauri host crate name (`crates/alolite-desktop`), and desktop UI package name (`apps/desktop/package.json`). | "alolite-ui", "@alolite/desktop", "AloLite Desktop" (as binary name) |
| **`alolite-core`** | UI-independent Rust core crate (`crates/alolite-core`). | "core", "alolite_core" |
| **`alolite-cli`** | Headless CLI/launcher crate (`crates/alolite-cli`) that compiles to the `alolite` binary. | "alolite-bin" |

## Invariants

Domain invariants are tagged `INV-N` here and referenced verbatim from code and tests.

- **INV-1** — No request leaves the process without `ParamCipher v2` encoding and a valid `signkey`.
- **INV-2** — `session.json` is never logged, committed or transmitted outside Zalo endpoints.
- **INV-3** — The UI thread never performs blocking I/O; all such work runs in the worker (SQLite & `ZDB4.0` unpacking execute on `spawn_blocking`).
- **INV-4** — A message row is idempotent by `msg_id`; re-delivery never duplicates history.
- **INV-5** — `crates/alolite-core/src/protocol/` remains pure protocol, crypto, and wire parsing; SQLite (`rusqlite`) and `threads_cache.json` persistence live strictly under `crates/alolite-core/src/storage/`.
