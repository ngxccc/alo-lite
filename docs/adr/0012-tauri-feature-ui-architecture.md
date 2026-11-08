# ADR-0012: Tauri v2 Feature-Based TypeScript UI Architecture

- **Status:** Accepted (Branch `ui/tauri`)
- **Date:** 2026-11-08
- **Author:** ngxc
- **Deciders:** Alolite Core Engineering

---

## 1. Context

`CoreEngine` (`src/core/`) owns the single live `Session`, WebSocket push loop, long-poll loop, SQLite persistence, and image/sticker decoding without any GUI framework dependency. To deliver a full-featured native WebView UI alongside the lightweight `egui` binary and headless `--mcp` server, the Tauri v2 UI must share `CoreEngine` directly without duplicating Zalo protocol, crypto, or SQLite logic in TypeScript.

---

## 2. Decision

1. **Isolated Tauri Crate (`ui/tauri/src-tauri/`):**
   - `ui/tauri/src-tauri/Cargo.toml` depends on `alolite = { path = "../../.." }` and stays outside the root `Cargo.toml` workspace so headless builds and CI (`--locked`, `--mcp`, `egui`) do not require `webkit2gtk-4.1`.
   - `TauriEventSink` (`src-tauri/src/events.rs`) implements `CoreEventSink` and forwards `CoreEventDto` over Tauri's `"core-event"` channel; `src-tauri/src/commands.rs` exposes 47 thin `#[tauri::command]` wrappers delegating directly to `CoreEngine`.

2. **WebView-Friendly Core Projections (`src/core/ipc.rs`, `settings.rs`, `stickers.rs`):**
   - `CoreImageData::to_png_data_url()` encodes decoded RGBA8 buffers into compact `data:image/png;base64,...` URLs before crossing IPC.
   - `CoreBootstrapSnapshot` hydrates initial logged-in state, threads, `AppSettings`, saved accounts, built-in sticker packs, and embedded PNG reaction/logo assets in one round trip.

3. **Feature-Based TypeScript Frontend (`ui/tauri/src/`):**
   - `src/app/`: application root (`App.tsx`, `main.tsx`), unified reducer (`store/appReducer.ts`), and Atom One Dark / Zalo Light CSS tokens (`styles/app.css`).
   - `src/shared/`: typed IPC wrapper (`api/ipc.ts`), DTO contract (`types/ipc.ts`), reusable `Avatar` / `ImageViewerModal`, and Vietnamese search/date helpers.
   - `src/features/`: domain-sliced UI & logic for `auth`, `navigation`, `threads`, `chat`, `contacts`, `sync`, `settings`, and `devtools`.
   - Built with Vite 8, React 19, Lucide vector icons, and TypeScript `6.0.x` (`>=6.0.3 <7.0.0`).

4. **Linux Runtime Packaging Note:**
   - Tauri v2 (`wry`) links dynamically to `webkit2gtk-4.1` (`2.54.x` on Arch Linux, GTK3 + libsoup3). Linux distributions should ship `.deb` / `.rpm` / `PKGBUILD` with `webkit2gtk-4.1` declared as a package dependency (or `.AppImage` bundling WebKit libs), while `alolite` (`egui` / `--mcp`) remains available for zero-WebKit portable `.tar.gz` targets.
