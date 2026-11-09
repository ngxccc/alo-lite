# ADR-0003: Tauri v2 Feature-Based TypeScript UI Architecture

- **Status:** Accepted (Superseded in workspace layout by ADR-0004)
- **Date:** 2026-11-08
- **Author:** ngxc
- **Deciders:** Alolite Core Engineering

---

## 1. Context

`CoreEngine` (`src/core/`) owns the single live `Session`, WebSocket push loop, long-poll loop, SQLite persistence, and image/sticker decoding without any GUI framework dependency. To deliver a full-featured native WebView UI alongside the lightweight `egui` binary and headless `--mcp` server, the Tauri v2 UI must share `CoreEngine` directly without duplicating Zalo protocol, crypto, or SQLite logic in TypeScript.

---

## 2. Decision

1. **Tauri Host (`crates/alolite-desktop/`):**
   - `crates/alolite-desktop/Cargo.toml` depends exclusively on `alolite-core` inside the root Cargo workspace, keeping headless CLI/MCP builds free of `webkit2gtk-4.1`.
   - `TauriEventSink` (`crates/alolite-desktop/src/events.rs`) implements `CoreEventSink` and forwards `CoreEventDto` over `"core-event"`; `crates/alolite-desktop/src/commands.rs` exposes 47 thin `#[tauri::command]` wrappers delegating directly to `CoreEngine`.

2. **WebView-Friendly Core Projections (`src/core/ipc.rs`, `settings.rs`, `stickers.rs`):**
   - `CoreImageData::to_png_data_url()` encodes decoded RGBA8 buffers into compact `data:image/png;base64,...` URLs before crossing IPC.
   - `CoreBootstrapSnapshot` hydrates initial logged-in state, threads, `AppSettings`, saved accounts, built-in sticker packs, and embedded PNG reaction/logo assets in one round trip.

3. **Feature-Based TypeScript Frontend (`apps/desktop/src/`):**
   - `src/app/styles/globals.css`: single-source-of-truth Tailwind CSS v4 (`@tailwindcss/vite`) `@theme inline` token stylesheet with 3 layers (`--rail`/`--sidebar`/`--canvas` shell, `shadcn/ui` semantic tokens, and `--bubble-in`/`--bubble-out`/`--online`/`--warning` chat tokens) for Light (`Zalo Slate`) and Dark (`Zinc Dark`) modes.
   - `src/app/store/`: pure `appReducer.ts` state transitions backed by `useAppStore.ts` (`zustand` v5 selector store).
   - `src/shared/ui/`: centralized `shadcn/ui` component library built on `@radix-ui/*` + `class-variance-authority` + `clsx` + `tailwind-merge` (`button`, `input`, `badge`, `dialog`, `dropdown-menu`, `context-menu`, `popover`, `tabs`, `switch`, `scroll-area`, `tooltip`, `separator`, `Avatar`, `ImageViewerModal`).
   - `src/features/`: domain-sliced UI & logic for `auth`, `navigation`, `threads` (virtualized via `@tanstack/react-virtual`), `chat`, `contacts`, `sync`, `settings`, and `devtools`.
   - Built with Vite 8, React 19, Tailwind CSS v4, Radix UI / `shadcn/ui`, Zustand 5, TanStack Virtual 3, Lucide vector icons, and TypeScript `6.0.x` (`>=6.0.3 <7.0.0`).
4. **Linux Runtime Packaging Note:**
   - Tauri v2 (`wry`) links dynamically to `webkit2gtk-4.1` (`2.54.x` on Arch Linux, GTK3 + libsoup3). Linux distributions should ship `.deb` / `.rpm` / `PKGBUILD` with `webkit2gtk-4.1` declared as a package dependency (or `.AppImage` bundling WebKit libs), while `alolite` (`egui` / `--mcp`) remains available for zero-WebKit portable `.tar.gz` targets.
