# AGENTS.md

Operating guide for AI agents and humans working in this repository.

## Project

`alolite` is a lightweight native desktop messaging client written in Rust: `egui` GUI, `ratatui` TUI, a Tokio worker for network I/O, and a local SQLite message store. Zero Node.js, zero Electron, zero Chromium.

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
|Run GUI|`cargo run -- --gui`|
|Run TUI|`cargo run -- --tui --qr`|

`CMAKE_POLICY_VERSION_MINIMUM=3.5` is required when building against CMake 4.x (see `docs/KHAC_PHUC_SU_CO.md`).

## Architecture Map

|Path|Responsibility|
|:---|:---|
|`src/main.rs`|CLI & Linux desktop entrypoint|
|`src/lib.rs`|Library surface & feature module re-exports|
|`src/protocol/`|Pure network protocol (`client`, `auth/{login,qr,trust}`, `social/{friends,groups}`, `messaging/{model,threads,send,polling,attachments,links,stickers,reactions,unread}`, `session/`, `registry/`, `sync/`, `socket`, `crypto`, `param_cipher`, `endpoints`, `wire`)|
|`src/storage/`|SQLite persistence (`messages`, `threads`, `models`, `importer/`, `sync_ingest`, `thread_cache`)|
|`src/gui/`|Native egui GUI app shell, messages, workspace, worker channel|
|`src/common/`|Shared utilities (`clock`, `data_dir`, `logger`, `ui`)|
|`src/legacy/`|Quarantined out-of-scope modules (VoIP, Feed, Scheduler)|

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
- **No blocking work on the UI thread.** Network, disk and crypto go through `WorkerCmd`; the UI only renders and sends commands.
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
