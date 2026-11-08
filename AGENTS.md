# AGENTS.md

Operating guide for AI agents and humans working in this repository.

## Project

`alolite` is a lightweight native desktop messaging client organized as a clean Cargo workspace (`alolite-core` + `alolite-cli` + `apps/desktop` Tauri v2 + TypeScript), zero Node.js runtime, zero Electron/Chromium.

- Repository: <https://github.com/ngxccc/alolite>
- Rust 1.75+ / edition 2021
- Product docs are Vietnamese (`docs/`, `README.md`); engineering standards are English (`docs/standards/`)

## Commands

|Task|Command|
|:---|:---|
|Build CLI & core|`cargo build`|
|Build Desktop app|`(cd apps/desktop && bun run build) && cargo build --bin alolite-desktop`|
|Build Release binaries|`cargo build --release --bin alolite --bin alolite-desktop`|
|Test Rust core & CLI|`cargo test --locked`|
|Test Desktop UI|`(cd apps/desktop && bun run typecheck && bun test && bun run build)`|
|Format|`cargo fmt --all`|
|Lint|`cargo clippy --workspace --all-targets -- -D warnings`|
|Run Desktop|`cargo run --bin alolite-desktop`|
|Run MCP (stdio)|`cargo run --bin alolite -- --mcp`|
|Run headless REST + MCP HTTP|`cargo run --bin alolite -- --api-port 18128 --api-only`|
## Architecture Map

|Path|Responsibility|
|:---|:---|
|`crates/alolite-core/`|UI-independent `CoreEngine`, `protocol/`, `storage/`, `mcp/`, `services/`, `common/`, and `assets/`|
|`crates/alolite-cli/`|`alolite` binary (`--mcp`, `--api-only`, `--qr`, `--phone`, desktop launcher)|
|`apps/desktop/`|Feature-based TypeScript (`src/{app,shared,features}/`) + Tauri v2 host (`src-tauri/` → `alolite-desktop`) over `alolite-core`|
|`docs/`|Engineering standards, ADRs (`docs/adr/`), `book.toml`, `DESIGN.md`, `ROADMAP.md`, `LEGAL.md`|
|`packaging/`|Linux `.deb`, `.rpm`, `.AppImage`, `.tar.gz` packaging scripts|

### Protocol Submodule Map (`crates/alolite-core/src/protocol/`)

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
- **No blocking work on the UI thread.** Network, disk, SQLite, and image decoding live inside `CoreEngine` (`crates/alolite-core/src/core/`); `apps/desktop` only invokes Tauri commands and renders `CoreEventDto`s.
- **All hosts live in `crates/alolite-core/src/protocol/endpoints.rs`.** Never hardcode a host elsewhere; declare HTTP command IDs in `crates/alolite-core/src/protocol/registry/` and invoke via `Session::api_get` / `Session::api_post`.
- **Single responsibility & pure protocol boundary.** Keep `rusqlite`, `MessageStore`, and `threads_cache.json` persistence in `crates/alolite-core/src/storage/` (`sync_ingest`, `thread_cache`) — never import `rusqlite` into `protocol/`. Reuse `wire.rs` helpers instead of duplicating JSON coercion, `group_` stripping, or control-envelope parsing.
- **Every outbound request goes through the standard path:** params → `ParamCipher v2` → `signkey` → session cookie attachment.
- **Comments in English, user-facing strings in Vietnamese.** See `docs/standards/code-comment-taxonomy.md`.
- **Do not silently change protocol behaviour.** Any change to params, signing, frame IDs or cipher flow requires evidence (capture, note in `re-notes/`) and a test or documented manual verification.
- **Missing or inaccurate RE documentation:** If reverse-engineering documentation (`re-notes/`, `docs/`) lacks information or deviates from live server behavior, engineers MUST use the `reverse-engineer-anything` skill and REA MCP tools to re-verify against actual application packages/endpoints rather than speculating.
- **UI Icon Standard:** Always use embedded SVG/PNG icons (`lucide-react` vector icons and embedded PNG reaction/logo assets). NEVER use raw keyboard emojis (`⚠️`, `✅`, `❌`) in UI code.

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
