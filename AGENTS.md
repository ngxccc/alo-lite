# AGENTS.md

Operating guide for AI agents and humans working in this repository.

## Project

`alolite` is a lightweight native desktop messaging client written in Rust: a UI-independent `CoreEngine` (protocol + Tokio sync/WebSocket + SQLite + MCP/REST server), an `egui` GUI adapter (with future Tauri WebView frontend path), and zero Node.js/Electron/Chromium.

- Repository: <https://github.com/ngxccc/alolite>
- Rust 1.75+ / edition 2021
- Product docs are Vietnamese (`docs/`, `README.md`); engineering standards are English (`docs/standards/`)

## Commands

|Task|Command|
|:---|:---|
|Build (debug)|`cargo build`|
|Build (release binary)|`cargo build --release --bin alolite`|
|Test|`cargo test`|
|Format|`cargo fmt --all`|
|Lint|`cargo clippy --all-targets -- -D warnings`|
|Run GUI (egui)|`cargo run -- --gui`|
|Run MCP (stdio)|`cargo run -- --mcp`|
|Run headless REST + MCP HTTP|`cargo run -- --api-port 18128 --api-only`|
|Check Tauri frontend (`ui/tauri/`)|`bun run typecheck && bun test && bun run build`|
|Build Tauri host|`cargo build --manifest-path ui/tauri/src-tauri/Cargo.toml`|

## Architecture Map

|Path|Responsibility|
|:---|:---|
|`src/main.rs`|CLI & Linux desktop entrypoint|
|`src/lib.rs`|Library surface & core/protocol/storage/mcp/services exports|
|`src/core/`|UI-independent `CoreEngine`, `CoreEventDto`, `CoreBootstrapSnapshot`, `AppSettings`, `CoreImageData` shared by GUI, MCP, and Tauri UI|
|`src/protocol/`|Pure network protocol (`client`, `auth/{login,qr,trust}`, `social/{friends,groups}`, `messaging/{model,threads,send,polling,attachments,links,stickers,reactions,unread}`, `session/`, `registry/`, `sync/`, `socket`, `crypto`, `param_cipher`, `endpoints`, `wire`)|
|`src/storage/`|SQLite persistence (`messages`, `threads`, `models`, `importer/`, `sync_ingest`, `thread_cache`)|
|`src/mcp/`|Model Context Protocol JSON-RPC 2.0 (`--mcp`), local HTTP REST API (`--api-port`), and auto-reply rules|
|`src/services/`|Host support services (`account_manager`, `media_resources`, `native_notifications`, `updater`)|
|`src/gui/`|Native egui GUI app shell (`worker/` adapter over `CoreEngine`, messages, workspace, modals)|
|`ui/tauri/`|Feature-based TypeScript (`app/`, `shared/`, `features/`) + Tauri v2 Rust host (`src-tauri/`) over `CoreEngine`|
|`src/common/`|Shared utilities (`clock`, `data_dir`, `logger`, `ui`)|

### Protocol Submodule Map (`src/protocol/`)

|Module|Single Responsibility|
|:---|:---|
|`endpoints.rs` & `registry/{mod,http_table,ws}.rs`|SSOT for all host URLs (`SERVICE_MAP_V3`, fallbacks) + sorted 437 HTTP `commandId` catalog & WS `cmd` constants|
|`wire.rs`, `crypto.rs`, `param_cipher.rs`, `socket.rs`|Shared JSON wire helpers (`require_login`, `unwrap_data`, `pick_str`, `iter_control_items`, `next_client_message_id`, `strip_vn_diacritics`), AES/MD5/`signkey`, `ParamCipher v2`, `zpw_ws` WebSocket framing|
|`session/{mod,domains,persistence,privacy,status}.rs`|HTTP transport & failover (`api_get`/`api_post`), domain healing, `session.json`/`device.json`, Privacy Guard & redaction, `SessionStatus`|
|`auth/{mod,login,qr,trust/*}.rs`|Password/session info auth, QR1/QR2 flow, pure-Rust Curve25519/XEdDSA Trusted Device protocol|
|`social/{mod,friends,groups}.rs`|Friend roster/phone lookup/friendship requests, group profiles/history/name resolution|
|`messaging/{mod,model,threads,send,polling,attachments,links,stickers,reactions,unread}.rs`|`ChatMessage`/`Thread` entities, `MessageIdentityIndex`, merge/dedup, text/RTF/quote/forward/recall, chunked file upload, OpenGraph link preview, stickers, `lp_chat` & `get-last-msgs` polling, reactions, seen/unread/mute|
|`sync/{mod,controls,zdb4,bin_net,v2_proto}.rs`|Pure sync HTTP/WS controls, `/api/znoise`, RSA-2048 + `ZDB4.0` container crypto, `BinNet` TLV, Sync V2 Protobuf (zero `rusqlite`)|

## Non-Negotiables

- **Never commit secrets.** `session.json`, `*.db`, cookies, `zpw_sek`, `zpw_enk`, IMEI and capture dumps stay out of Git (`.gitignore`, `SESSION_SECURITY.md`).
- **No blocking work on the UI thread.** Network, disk, SQLite, and image decoding live inside `CoreEngine` (`src/core/`); UI adapters (`src/gui/worker/`) only send commands and render `CoreEvent`s.
- **All hosts live in `src/protocol/endpoints.rs`.** Never hardcode a host elsewhere; declare HTTP command IDs in `src/protocol/registry/` and invoke via `Session::api_get` / `Session::api_post`.
- **Single responsibility & pure protocol boundary.** Keep `rusqlite`, `MessageStore`, and `threads_cache.json` persistence in `src/storage/` (`sync_ingest`, `thread_cache`) — never import `rusqlite` into `src/protocol/`. Reuse `src/protocol/wire.rs` helpers instead of duplicating JSON coercion, `group_` stripping, or control-envelope parsing.
- **Every outbound request goes through the standard path:** params → `ParamCipher v2` → `signkey` → session cookie attachment.
- **Comments in English, user-facing strings in Vietnamese.** See `docs/standards/code-comment-taxonomy.md`.
- **Do not silently change protocol behaviour.** Any change to params, signing, frame IDs or cipher flow requires evidence (capture, note in `re-notes/`) and a test or documented manual verification.
- **Missing or inaccurate RE documentation:** If reverse-engineering documentation (`re-notes/`, `docs/`) lacks information or deviates from live server behavior, engineers MUST use the `reverse-engineer-anything` skill and REA MCP tools to re-verify against actual application packages/endpoints rather than speculating.
- **UI Icon Standard:** Always use embedded vector icon constants (`lucide::...` from `src/assets/fonts/lucide.ttf`). NEVER use raw keyboard emojis (such as `⚠️`, `✅`, `❌`) in UI code, preventing missing-glyph tofu boxes (`□`) caused by fonts lacking Unicode variation selectors.

## Engineering Standards

MUST read the corresponding standard file under `docs/standards/` before modifying related code or tests:

- **Issue tracking & tickets** → `docs/standards/issue-tracker.md`
- **Domain glossary & ADRs** → `docs/standards/domain-docs.md`
- **Comments & docstrings** → `docs/standards/code-comment-taxonomy.md`
- **API calls, IPC, and error handling** → `docs/standards/api-design-and-error-handling.md`
- **SQLite schema and migrations** → `docs/standards/database-and-migrations.md`
- **Threads, channels, locks, and races** → `docs/standards/concurrency-and-locking.md`
- **Tests, fixtures, and benchmarks** → `docs/standards/testing-and-fixtures.md`
- **Secrets, crypto, and session handling** → `docs/standards/security-and-cryptography.md`
- **Branches, commits, and PRs** → `docs/standards/git-flow-and-pr-matrix.md`
- **Compliance, IP, and Clean-Room** → `docs/standards/compliance-and-ip.md`
