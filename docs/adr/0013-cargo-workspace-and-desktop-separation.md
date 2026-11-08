# ADR-0013: Unified Cargo Workspace (`alolite-core`, `alolite-cli`, `apps/desktop`)

- **Status:** Accepted (Branch `ui/tauri`)
- **Date:** 2026-11-08
- **Author:** ngxc
- **Deciders:** Alolite Core Engineering

---

## 1. Context

During initial Tauri integration, the repository carried two separate Rust graphs (`Cargo.toml` at root bundling `eframe`/`egui` together with core protocol/storage, plus an isolated `ui/tauri/src-tauri/Cargo.toml` depending back up on root). That caused:
- Duplicate `Cargo.lock` and duplicate multi-gigabyte `target/` directories.
- `alolite-tauri` transitively compiling `eframe`, `egui`, `glutin`, and `winit` despite running WebKitGTK (~442 MB debug binary).
- Three competing `src/` trees (`src/`, `ui/tauri/src/`, `ui/tauri/src-tauri/src/`), leftover `src/bin/` probe binaries, and root-level documentation clutter.

---

## 2. Decision

1. **Single Root Cargo Workspace (`Cargo.toml`):**
   - `members = ["crates/alolite-core", "crates/alolite-cli", "apps/desktop/src-tauri"]`
   - `default-members = ["crates/alolite-core", "crates/alolite-cli"]` so headless CI and `--mcp` servers compile/test cleanly without requiring `webkit2gtk-4.1`, while sharing a single `Cargo.lock` and `target/` directory when `alolite-desktop` is built.
2. **UI-Free Core (`crates/alolite-core/`):**
   - Owns `core/`, `protocol/`, `storage/`, `mcp/`, `services/`, `common/`, and `assets/` with zero `eframe`/`egui` or WebKit dependencies.
3. **Dual Focused Binaries:**
   - `crates/alolite-cli/` (`alolite`): lightweight headless CLI, `--mcp` stdio server, `--api-only` REST server, `--qr` terminal login, and desktop launcher.
   - `apps/desktop/` (`alolite-desktop`): feature-based TypeScript (`src/{app,shared,features}/`) + Tauri v2 Rust host (`src-tauri/`).
4. **Root Hygiene:**
   - Remove legacy `src/gui/` (`egui`, preserved on `main` at `ce798e6`), legacy `src/bin/` probes, and `egui`-only fonts (`BeVietnamPro`, `lucide.ttf`).
   - Move `book.toml`, `DESIGN.md`, `ROADMAP.md`, and `LEGAL.md` under `docs/`.

---

## 3. Measured Release Binary & RAM Benchmark (Arch Linux x86_64, 32 Threads / 961 Messages)

| Mode / Architecture | Release Binary | Processes / Threads (`t=10s`) | Total `PSS` (`t=10s`) | Total `Private_Dirty` (`t=10s`) | Total `VmRSS` (`t=10s`) |
| :--- | ---: | ---: | ---: | ---: | ---: |
| **Headless MCP (`alolite --mcp`)** | **8.9 MiB** | 1 proc / 13 threads | **7.87 MiB** | **1.90 MiB** | **10.36 MiB** |
| **Legacy `egui` (`main` @ `ce798e6`)** | 16.1 MiB | 1 proc / 43 threads | 85.20 MiB | 47.00 MiB | 118.40 MiB |
| **Tauri Desktop (`alolite-desktop`, total 3 procs)** | **17.0 MiB** | 3 procs / 113 threads | **373.96 MiB** | **228.15 MiB** | **571.51 MiB** |
| ↳ `alolite-desktop` (Rust CoreEngine + GTK3 host) | — | 44 threads | 107.99 MiB | 66.30 MiB | 189.70 MiB |
| ↳ `WebKitNetworkProcess` (HTTP/TLS cache) | — | 18 threads | 48.73 MiB | 33.56 MiB | 84.90 MiB |
| ↳ `WebKitWebProcess` (DOM/CSS/JS + GPU tiles) | — | 51 threads | 217.24 MiB | 128.30 MiB | 296.91 MiB |
