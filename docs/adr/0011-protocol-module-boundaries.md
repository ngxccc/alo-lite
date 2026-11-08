# ADR-0011: Single-Responsibility Protocol & Storage Module Boundaries

- **Status:** Accepted
- **Date:** 2026-11-08
- **Author:** ngxc
- **Deciders:** Alolite Core Engineering

---

## 1. Context & Problem Statement

Before this refactoring, three monolithic files (`src/protocol/chat.rs` at 8,275 lines / 95 `pub` items, `src/protocol/history_sync.rs` at 2,281 lines, and `src/protocol/session.rs` at 2,191 lines) accumulated ~70% of the entire `src/protocol/` directory. This created three structural defects:

1. **Broken Layering:** `src/protocol/history_sync.rs` opened `rusqlite::Connection` and `MessageStore` directly, while `src/protocol/chat.rs` read/wrote `threads_cache.json` (`load_threads_cache` / `save_threads_cache`), violating the pure-protocol boundary in `AGENTS.md`.
2. **Split Domain Ownership:** Reaction sending (`send_reaction`) and seen/mute REST calls (`send_seen_v2`, `set_unread_mark`, `set_mute_conversation`) lived in `chat.rs` alongside separate `reactions.rs` and `unread.rs` modules.
3. **Duplicated Wire Helpers:** Identical JSON unwrapping (`unwrap_data`, `pick_str`), `group_` prefix stripping, control-event flattening (`controls` / `content` / stringified `data`), monotonic `clientId` generation, and MD5/timestamp helpers were duplicated across multiple files.

---

## 2. Decision

Decompose the protocol and persistence layers into cohesive single-responsibility modules without maintaining legacy re-export shims:

1. **Shared Foundation (`src/protocol/wire.rs` + `src/protocol/registry/`):**
   - `wire.rs` is the single source of truth for `require_login`, `obj`, `unwrap_data`, `pick_str`, `val_i64`, `val_u64`, `wire_identity`, `is_group_flag`, `strip_group_prefix`, `iter_control_items`, `parse_control_data`, `next_client_message_id`, and host helpers `domain_or` / `chat_send_host`.
   - `registry/` splits into `registry/mod.rs` (`ApiDomain`, `CommandSpec`, `lookup_command`, domain command constants), `registry/http_table.rs` (437-entry sorted `COMMAND_REGISTRY`), and `registry/ws.rs` (complete WebSocket `cmd` and close-code catalog).

2. **Session Transport (`src/protocol/session/`):**
   - `session/mod.rs`: `Session` HTTP transport (`reqwest`, `ParamCipher`, candidate failover, `api_get`/`api_post`).
   - `session/domains.rs`: `Domains` routing table, `heal_from_verified`, `apply_service_map`, candidate ranking.
   - `session/persistence.rs`: `SavedSession`, `LoginInfo`, `DeviceIdentity`, `save_to_file`/`load_from_file`, `official_imei`.
   - `session/privacy.rs`: Privacy Guard toggles (`block_typing`/`block_delivered`/`block_seen`), `normalize_proxy_url`, `redact_*`, `urlencode`, `truncate`.
   - `session/status.rs`: `SessionError`, `SessionStatus`, `AUTH_ERROR_CODES`.

3. **Domain-Grouped Protocol Subdirectories (`src/protocol/{auth,social,messaging}/`):**
   - `auth/`: `login.rs` (password & server/login info), `qr.rs` (QR1/QR2 flow), `trust/*` (Curve25519/XEdDSA Trusted Companion Device crypto).
   - `social/`: `friends.rs` (`get_friends`, `FriendshipStatus`, `PhoneLookupResult`, `find_user_by_phone`, friend requests, online status), `groups.rs` (`get_groups`, `fetch_group_profiles`, `get_mini_profiles`, `get_group_history*`, `resolve_thread_names`).
   - `messaging/`: `model.rs` (`WireMessageRef`, `MessageKind`, `DeliveryState`, `UnreadPolicy`), `threads.rs` (`ChatMessage`, `Thread`, `SidebarPresentation`, `MessageIdentityIndex`, `merge_thread_into`, `deduplicate_threads`, pin/hide/delete conversation ops), `send.rs` (outbound text/RTF/quote/forward/recall/delete/typing), `polling.rs` (`PollState`, `QueueCursor`, `lp_chat`, `get_last_msgs`, `get_cloud_messages`, `merge_previews`, `merge_group_history`, `parse_chat_message`), `attachments.rs` (chunked image/asyncfile uploads, `FileDoneRegistry`, CDN URL/filename helpers), `links.rs` (`LinkMetadata`, `extract_first_url`, `/api/message/parselink`, OpenGraph HTML fallback), `stickers.rs` (sticker store, category detail, Vietnamese keyword search, `send_sticker*`), `reactions.rs` (`send_reaction`, `icon_to_emoji`, `reaction_code_and_type`), `unread.rs` (`send_seen_v2`, `set_unread_mark`, `set_mute_conversation`, `get_mute_conversations`, `MuteControlEvent`, `extract_mute_controls`).

4. **Pure Sync vs. SQLite Storage Boundary (`src/protocol/sync/` vs. `src/storage/`):**
   - `src/protocol/sync/` contains **zero** `rusqlite` or `MessageStore` imports: `sync/mod.rs` (sync key/status state & cursor helper), `sync/controls.rs` (`pull_mobile_msg`, `cancel_phone_sync`, `clean_sync`, `/api/znoise`, `MobileSyncControlEvent`), `sync/zdb4.rs` (RSA-2048 sync keys & `ZDB4.0` decryptor), `sync/bin_net.rs` (`BinNet` TLV decoder), `sync/v2_proto.rs` (`proto.Sync2` chunk parser).
   - `src/storage/sync_ingest.rs` and `src/storage/thread_cache.rs` own all SQLite archive ingestion (`sync_thread`, `sync_phone_messages_opts`, `extract_conv_archives_from_entries`, `commit_conv_archives_to_store`, `import_mobile_backup_container*`, `import_sync_v2_*`) and `threads_cache.json` persistence (`load_threads_cache`, `save_threads_cache`).

---

## 3. Mandatory Module & Boundary Rules Going Forward

1. **No `rusqlite` or local disk cache I/O inside `src/protocol/`** (exception: explicit credential persistence in `session/persistence.rs` and read-only media cache lookup in `messaging/stickers.rs`). All SQLite queries, migrations, and `threads_cache.json` writes belong in `src/storage/`.
2. **All service hosts live exclusively in `src/protocol/endpoints.rs`**; outbound HTTP endpoints should be declared in `src/protocol/registry/` and dispatched via `Session::api_get` / `Session::api_post`.
3. **One cohesive responsibility per file under domain subdirectories:** New protocol features must join `auth/`, `social/`, `messaging/`, `session/`, `registry/`, or `sync/` rather than adding root-level files or expanding `send.rs`/`threads.rs`. Internal helpers shared across siblings stay `pub(super)`.
4. **No duplicate wire/JSON helpers:** Reuse `crate::protocol::wire::*` (including `strip_vn_diacritics`), `crate::clock::now_ms`, and `crate::crypto::md5_hex_bytes` instead of reimplementing ad-hoc parsers.
5. **Clean cutover over shims:** Do not re-add deprecated wildcard re-exports; update callers directly to canonical domain paths.
6. **Integration fixtures vs. unit tests:** Keep focused unit tests inline under `#[cfg(test)] mod tests`; place multi-module integration and large binary/JSON fixture tests under `tests/protocol_*.rs`.
