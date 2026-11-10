# API, IPC & Error Handling Standards

This project is a client, not a server. The "API" surface is three boundaries:

| Boundary | Direction | Module |
| :--- | :--- | :--- |
| **Zalo HTTP API** | outbound | `protocol/session/`, `protocol/registry/`, `protocol/auth/`, `protocol/social/`, `protocol/messaging/` |
| **Zalo WebSocket** | bidirectional | `protocol/socket.rs`, `protocol/registry/ws.rs` |
| **UI ↔ Core IPC** | in-process | `core/` (`CoreEngine` / `CoreEvent`) + `gui/worker/` (`WorkerCmd` / `GuiEvent`) |

Plus local filesystem persistence (`src/storage/`, `protocol/session/persistence.rs`).

---

## 1. Outbound HTTP Conventions

- **Hosts come from `src/protocol/endpoints.rs` only.** No literal `*.zalo.me` / `*.zaloapp.com` URLs elsewhere.
- **Command IDs & paths belong in `src/protocol/registry/`.** Invoke via `Session::api_get` / `Session::api_post`; reuse `src/protocol/wire.rs` for JSON unwrapping, `group_` stripping, and control flattening.
- **Every request goes through one path**: build params → `ParamCipher v2` (`param_cipher.rs`) → `signkey` (`crypto.rs`) → attach session cookies → send.
- **No ad-hoc `reqwest::Client` per call.** Reuse the shared client so cookies and TLS config stay consistent.
- **Timeouts are explicit** on every request; no unbounded awaits.
- **Response parsing is strict**: decode into `serde` structs, not ad-hoc `Value` walks on hot paths.

### Response Envelope

Zalo responses use `{ error_code, error_message, data }`. Parse once, then branch:

| `error_code` | Meaning | Handling |
| :--- | :--- | :--- |
| `0` | Success | Return typed `data`. |
| `18012` | QR scanned, not yet confirmed | Keep long-polling; no user error. |
| `18022` | Invalid signature | Bug: wrong `signkey` parameter set. Do not retry; log and fix. |
| `18060` | Invalid encryption protocol | Bug: request bypassed `ParamCipher v2`. Do not retry. |
| `600` / `-102` | `zpw_sek` missing or expired | Fail closed: clear session, restart QR login. |
| `114` | Invalid parameter type | Bug: malformed nested JSON. Do not retry. |
| `429` | Rate limited | Back off exponentially; surface a retry hint. |

Canonical table lives in `docs/KHAC_PHUC_SU_CO.md`; keep both in sync when a new code is observed.

---

## 2. Error Taxonomy

Use `anyhow::Result` at module boundaries. Add typed errors only where callers must branch on the cause.

| Category | Examples | Retry? | Surface |
| :--- | :--- | :--- | :--- |
| `Network` | DNS, TLS, timeout, socket reset | Yes, with backoff | "Mất kết nối, đang thử lại…" |
| `Auth` | expired `zpw_sek`, invalid credentials | No | Prompt re-login |
| `Protocol` | bad `signkey`, cipher mismatch, decode failure | No (bug) | Log + generic error |
| `RateLimit` | `429` | Yes, honoring delay | "Thao tác quá nhanh, thử lại sau" |
| `Io` | SQLite, file, media read/write | Conditional | Context-specific |
| `Unsupported` | missing `ffmpeg`, missing audio device | No | Actionable hint |

Rules:

- **No `unwrap()` / `expect()` on network, parse, file or user-input paths.** Allowed in tests and in true compile-time invariants with a message.
- **`?` over manual matching**; add `.context("...")` when the error crosses a module boundary.
- **Never swallow errors silently.** `let _ = ...` is only acceptable for best-effort cleanup, and requires a Tier 2 comment.
- **Never include secrets in error text** — no cookies, `zpw_sek`, `zpw_enk`, tokens or raw `session.json` fragments.
- **User-facing messages are Vietnamese; log messages are English.**

---

## 3. Retry & Backoff

- Retry only `Network` and `RateLimit` categories.
- Exponential backoff with jitter: `250ms → 500ms → 1s → 2s`, capped at 8s, max 4 attempts.
- Honor `Retry-After` / `429` delay when present.
- Never retry:
  - Auth failures (must re-authenticate instead).
  - Protocol bugs (`18022`, `18060`, `114`).
  - Non-idempotent operations without a stable idempotency key.
- Background polling (`POLL_SECS`) is not a retry loop; failures must be logged and the next tick scheduled.

---

## 4. Idempotency

- Client-generated message IDs are created **once** per user action and reused across retries; never regenerate on retry.
- Local persistence is idempotent by `msg_id` (`INSERT OR IGNORE`), so re-delivery cannot duplicate history (INV-4).
- Optimistic UI rows reconcile by local message ID; a server echo updates the row, it does not insert a second one.

---

## 5. WebSocket Conventions

- Command IDs are constants in `src/protocol/registry/ws.rs`; never inline magic numbers.
- A push frame is acknowledged/parsed without blocking the read loop; heavy decoding moves to `spawn_blocking`.
- Reconnect uses jittered backoff and re-authenticates before resubscribing.
- Unknown frame types are logged and skipped, never fatal.

---

## 6. UI ↔ Worker IPC

- The UI thread sends `WorkerCmd` and renders `WorkerEvent`; it never performs I/O (INV-3).
- One command variant = one user intent. Do not add a generic `Do(String)` escape hatch.
- Every command that can fail emits a terminal event (success or error); the UI must not hang on a dropped command.
- Long-running tasks report progress events rather than blocking the channel.
- Channels are unbounded today for UI responsiveness; any producer that can flood must rate-limit at the source.

---

## 7. Local Persistence Boundary

- `src/storage/` (`MessageStore`, `sync_ingest`) returns `anyhow::Result`; callers decide whether the failure is fatal.
- A failed history write must never block message delivery or UI rendering.
- Schema and migration rules: `docs/standards/database-and-migrations.md`.

---

## 8. Two-Tier Architecture & Code-as-SSOT

AloLite enforces a strict separation between upstream server wire responses and in-process frontend IPC:

### Tier 1: Upstream Wire Protocol (`protocol/`)
- **Audience**: Core protocol engineers working in `crates/alolite-core/src/protocol/`.
- **SSOT**: Rust `serde::Deserialize` structs (`ZaloWireEnvelope`, `SendMessageReceipt`, `ActionAck`, `PhoneLookupResult`, `ChatMessage`, etc.).
- **Documentation**: Living documentation via `cargo doc --document-private-items` with docstrings (`///`) and sanitized JSON payload fixtures. The consolidated HTTP/WebSocket wire reference lives in `re-notes/14-http-rest-endpoint-catalog.md` (index: `re-notes/README.md`). Upstream schemas are never exposed as public AloLite web documentation to preserve Clean-Room compliance (`docs/standards/compliance-and-ip.md`).
- **Isolation**: The UI thread and frontend React code **never** see or parse raw Zalo wire payloads.

### Tier 2: Frontend IPC & State Contracts (`core/ipc.rs` ↔ `apps/desktop`)
- **Audience**: Desktop frontend engineers working in `apps/desktop/` (React + TypeScript).
- **SSOT**: Rust IPC event projection `CoreEventDto` and snapshot `CoreBootstrapSnapshot`.
- **Automated Codegen**: Generated via `specta` directly from Rust structs to `apps/desktop/src/shared/types/ipc.generated.ts`. JSDoc comments written on Rust structs propagate directly to IDE tooltips.
- **Drift Prevention**: Verified in CI by `cargo test -p alolite-core --test ipc_types_sync`. Any modification to Rust IPC structs without matching TypeScript regeneration causes an immediate test failure.

### Wire struct convention (Tier 1)

Every protocol function that reads an upstream JSON response names its payload with a typed
`*WireData` struct instead of walking `serde_json::Value` inline.

| Rule | Detail |
|:---|:---|
| **Location** | Same module as the endpoint function (e.g. `social/friends.rs`). |
| **Name** | `<Thing>WireData` (e.g. `LastOnlineWireData`, `GroupProfileWireData`). |
| **Derives** | `#[derive(Debug, Clone, Default, Serialize, Deserialize)]` + `#[serde(default)]`. |
| **Field mapping** | Mirror the raw Zalo keys with `#[serde(rename = "...")]` / `#[serde(alias = "...")]`. |
| **Type-inconsistent fields** | Keep the field as `Option<serde_json::Value>` and expose a typed accessor using the existing `protocol::wire` helpers (`val_i64`, `val_u64`, `pick_str`, `unwrap_data`). Never add new ad-hoc coercion. |
| **Docstring** | Four blocks: summary line, `# Wire Response Example` (sanitized JSON), `# Core Transformation`, and `# Errors` on the consuming function. |
| **Constructor** | `pub fn from_value(v: &Value) -> Self { serde_json::from_value(v.clone()).unwrap_or_default() }`; the parse must never fail, so an unexpected scalar cannot silently turn a payload into `Default`. |
| **Envelope head** | Use `wire::ErrorEnvelopeWireData` instead of reading `error_code` / `error_message` inline. `code()` treats a missing code as success (`0`); use `code_opt()` when "absent" must stay distinct from `0`. |
| **Shared coercion** | Multi-alias lookups keep `wire::pick_str`; an already-selected field uses `wire::field_str` so alias order lives in the accessor. |
| **Behaviour** | Behaviour-preserving: no public signature change, no change to emitted values. |

Reference implementations: `protocol/social/friends.rs` (`FriendWireData`,
`PhoneProfileWireData`), `protocol/messaging/polling/parse.rs` (`ChatMessageWireData`,
`QuoteWireData`), `protocol/messaging/unread/mute.rs` (`MuteSnapshotWireData`).

---

## 9. Review Checklist

- [ ] No hardcoded host outside `endpoints.rs`.
- [ ] New request uses the standard cipher/sign path.
- [ ] Error branch added for any new `error_code`.
- [ ] Retries limited to idempotent / transient failures.
- [ ] No secret in logs, errors or issue reports.
- [ ] New `WorkerCmd` has a matching completion event.
- [ ] Rust IPC changes re-generate and pass `cargo test -p alolite-core --test ipc_types_sync`.
