# ADR-0005: Retain Tauri v2 Over Qt6 and Native Rust GUI Toolkits

- **Status:** Accepted
- **Date:** 2026-11-11
- **Author:** ngxc
- **Deciders:** AloLite Core Engineering

---

## 1. Context

Following the migration from legacy `egui` (`ce798e6`) and the Cargo workspace separation in [ADR-0003](./0003-tauri-feature-ui-architecture.md) and [ADR-0004](./0004-cargo-workspace-and-desktop-separation.md), `alolite-desktop` runs as a Tauri v2 host over `alolite-core` (`CoreEngine`) with a feature-based React 19 + TypeScript UI (`apps/desktop/`).

Because `alolite-desktop` dynamically links against the system WebView (`webkit2gtk-4.1` on Linux, `WebView2` on Windows) and spawns three processes totaling ~374 MiB `PSS` (`alolite-desktop` + `WebKitNetworkProcess` + `WebKitWebProcess`), we evaluated whether to replace Tauri v2 with **Qt6** (`cxx-qt` or C++ `QtWidgets`) or an alternative native GUI toolkit (**GPUI**, **Iced**, **Slint**, **GTK4/Relm4**) to reduce RAM usage and eliminate system WebView runtime dependencies.

---

## 2. Evaluation of Alternatives

### 2.1 Why Pure Rust GPU Toolkits (`GPUI`, `egui`, `Iced`) Fail for Rich Messaging

A modern messaging client (`AloLite`) is fundamentally a **rich document, internationalized text input, and multimedia layout engine**:

1. **Vietnamese IME & Multi-Bubble Text Selection:**
   - Custom GPU glyph renderers (`GPUI`, `Iced`, `egui`) bypass native toolkit text widgets and implement their own X11/Wayland text-input protocols. In prior empirical trials with both `egui` and `GPUI`, Vietnamese input method editors (`fcitx5`, `ibus`, preedit composition underlines) and multi-line / cross-bubble text selection exhibited severe regressions on Linux Wayland/X11 compared to browser engines.
2. **Variable-Height Virtualized Timelines:**
   - Chat timelines mix 1-line text bubbles, 50-line formatted messages, quote replies, multi-image grids, animated stickers, OpenGraph link cards, and inline video players. Browser engines provide battle-tested `Flexbox`/`Grid` layout, `ResizeObserver`, and `@tanstack/react-virtual` measurement. Implementing jitter-free variable-height virtualization manually in `GPUI` or `Iced` requires thousands of lines of custom layout measurement code.
3. **Multimedia Decoding, Caching & VRAM Management:**
   - System WebViews natively handle `<img loading="lazy">`, HTTP/2 media caching, animated WebP/GIF playback, and hardware-accelerated `<video>` pipelines. In `GPUI` or `egui`, the application must manually decode animation frames, manage GPU texture atlases, and evict textures to prevent VRAM exhaustion during fast timeline scrolling.

### 2.2 Why Qt6 (`cxx-qt` / QML or C++ `QtWidgets`) Is Not Viable for `AloLite`

1. **Linux Distribution ABI Skew (Does Not Solve Portable Packaging):**
   - Dynamically linking Qt6 (`libQt6Core.so.6`, `libQt6Gui.so.6`, `libQt6Quick.so.6`, `libQt6WaylandClient.so.6`) shifts the dependency problem from `webkit2gtk-4.1` to Qt6 minor-version symbol mismatches across Linux distributions (e.g., Ubuntu 22.04 ships Qt 6.2, Debian 12 ships Qt 6.4, Rolling/Arch ships Qt 6.7–6.8; binaries compiled against `Qt_6.7` symbols fail at dynamic link time on older distros).
2. **Static Linking & LGPLv3 Constraints:**
   - Statically linking Qt6 to produce a self-contained Linux binary requires building Qt6 from source in CI (+30–60 minutes) and complying with **LGPLv3** object-relinking obligations (or relicensing the entire workspace under GPLv3). Bundling shared Qt6 libraries + ICU + platform/Wayland plugins inside an AppImage adds ~50–80 MiB of payload.
3. **Rust↔C++ Bridge Friction (`cxx-qt`):**
   - Rust lacks a first-party pure-Rust Qt6 binding. Using KDAB's `cxx-qt` introduces a hybrid Rust + C++17 + CMake + `moc` build graph and requires wrapping `Thread` and `ChatMessage` collections inside C++-compatible `QAbstractListModel` subclasses, losing the direct ergonomic type sharing currently provided by `specta` + `serde`.
4. **Full C++ `QtWidgets` Rewrite Cost:**
   - Achieving Telegram Desktop's memory profile (~60–100 MiB RAM) requires writing custom C++ `QPainter` delegates for every chat bubble, media grid, sticker picker, and modal from scratch and bridging `alolite-core` over a C ABI (`cbindgen`), halting feature development for months.

### 2.3 Comparative Matrix

| Dimension                        | Tauri v2 + React 19 (Current)             | Qt6 + Rust (`cxx-qt` / QML)                   | Qt6 C++ (`QtWidgets`)                | Pure Rust GPU (`GPUI` / `Iced` / `egui`) | Slint (Rust + DSL)       | GTK4 / Libadwaita (`relm4`)         |
| :------------------------------- | :---------------------------------------- | :-------------------------------------------- | :----------------------------------- | :--------------------------------------- | :----------------------- | :---------------------------------- |
| **Release Binary Size**          | **~17 MiB** (uses OS WebView)             | ~25 MiB (dynamic) / ~80 MiB (bundled Qt)      | ~20 MiB (dynamic) / ~65 MiB (static) | **~15–20 MiB** (single binary)           | **~12–18 MiB**           | ~18 MiB (Linux) / ~50 MiB (Win)     |
| **Runtime Memory (`PSS`)**       | ~250–370 MiB (3 processes)                | ~120–180 MiB (QML JIT + scene graph)          | **~65–110 MiB**                      | **~75–120 MiB**                          | **~50–90 MiB**           | ~90–140 MiB                         |
| **Vietnamese IME (X11/Wayland)** | **Excellent** (Native WebKit/Blink)       | **Excellent**                                 | **Excellent**                        | **Poor / Fragile**                       | Fair                     | **Excellent** (on Linux)            |
| **Variable-Height Virtual Chat** | **Excellent** (`@tanstack/react-virtual`) | Good (`ListView` + C++ model)                 | Manual custom `QPainter`             | Manual measurement / scroll jump         | Limited for rich chat    | Good (`GtkListView`)                |
| **Rich Media (GIF/WebP/Video)**  | **Built-in** (`<img>`, `<video>`)         | Requires `QtMultimedia` + GStreamer           | Requires custom FFmpeg/WebP          | Manual texture atlas & decoding          | Manual frame pushing     | Requires `GtkMediaFile` / GStreamer |
| **Build Toolchain**              | `cargo` + `bun` (Vite)                    | `cargo` + `cmake` + `g++` + `qt6-declarative` | `cmake` + `g++` + `cbindgen`         | `cargo` only                             | `cargo` + Slint compiler | `cargo` + `pkg-config` + GTK4 dev   |
| **Cross-Platform Fidelity**      | **Identical** (Linux / Win / macOS)       | High                                          | High                                 | High                                     | High                     | **Poor on Windows/macOS**           |
| **Single-Maintainer Velocity**   | **Very High** (Tailwind v4 + Radix UI)    | Low (FFI + `QAbstractListModel` boilerplate)  | Very Low (100% C++ UI rewrite)       | Low (custom widgets from scratch)        | Medium                   | Medium                              |

---

## 3. Decision

1. **Retain Tauri v2 + React 19 (`crates/alolite-desktop` + `apps/desktop`) as the canonical desktop UI architecture.**
   - Do not migrate to Qt6, GPUI, Iced, Slint, or GTK4. The `CoreEngine` (`crates/alolite-core/src/core/`) boundary remains strictly UI-independent (`INV-3`), while `apps/desktop/` leverages the browser engine for text shaping, Vietnamese IME, variable-height virtualization, and media rendering.
2. **Eliminate IPC & Heap Bottlenecks in the Existing Tauri v2 Layer:**
   - **Authoritative Rust Store for Phone Sync (`sync_phone_messages`):** Stop serializing `Vec<Thread>` from JavaScript back to Rust over JSON IPC when triggering `sync_phone_messages`. Instead, `crates/alolite-desktop/src/commands.rs` hydrates the authoritative thread list directly from `CoreEngine::list_threads` in Rust.
   - **Prune Legacy `egui`-Era IPC Commands:** Remove unused IPC commands (`persist_threads`, `set_known_threads`, `fetch_media_data`) that previously allowed full-thread-list round-trips or redundant PNG Base64 polling.
   - **Compact Binary Attachment State (`PendingUpload`):** Keep pasted/dropped file buffers as typed `Uint8Array` in React state (`apps/desktop/src/features/chat/components/composer.tsx`) rather than eagerly boxing every byte into a 64-bit JavaScript `number[]` on `paste`/`drop`.

---

## 4. Consequences

- **Positive:**
  - Zero feature freeze or multi-month UI rewrite; retains the full React 19 Compiler + Tailwind CSS v4 + Radix UI + TanStack Virtual stack.
  - Eliminates multi-megabyte `Thread[]` JSON serialization from `WebKitWebProcess` to `alolite-desktop` during phone sync.
  - Reduces JavaScript heap allocation by ~8× while composing messages with pasted/dropped attachments.
  - Preserves a clean pure-Rust Cargo workspace without C++, CMake, `moc`, or LGPLv3 static-linking encumbrances.
- **Negative / Accepted Trade-offs:**
  - `alolite-desktop` continues to require the host WebView (`webkit2gtk-4.1` on Linux, `WebView2` on Windows), consuming ~150–250 MiB more `PSS` than a custom-painted C++ `QtWidgets` client.
  - Headless and ultra-low-memory environments (<15 MiB RAM, zero GUI libs) remain served by `alolite --mcp` and `alolite --api-only`.
