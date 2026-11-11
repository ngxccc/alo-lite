<div align="center">

<img src=".github/assets/logo.svg" alt="AloLite - Lightweight Native Messaging Client for Linux" width="180" />

# AloLite

**A fast, lightweight, native desktop messaging client written in Rust**  
_No Node.js at runtime • No Electron • No Chromium bloat_

[![Rust](https://img.shields.io/badge/Rust-1.75+-DEA584?logo=rust&logoColor=white)](https://www.rust-lang.org)
[![Tauri v2](https://img.shields.io/badge/Tauri-v2.0-24C8DB?logo=tauri&logoColor=white)](https://tauri.app)
[![TypeScript](https://img.shields.io/badge/TypeScript-6.0-007ACC?logo=typescript&logoColor=white)](https://www.typescriptlang.org)
[![React](https://img.shields.io/badge/React-19.3-61DAFB?logo=react&logoColor=black)](https://react.dev)
[![Tailwind CSS v4](https://img.shields.io/badge/Tailwind_CSS-v4.3-38B2AC?logo=tailwindcss&logoColor=white)](https://tailwindcss.com)
[![Bun](https://img.shields.io/badge/Bun-1.4-000000?logo=bun&logoColor=white)](https://bun.sh)
[![Vite](https://img.shields.io/badge/Vite-8.3-646CFF?logo=vite&logoColor=white)](https://vitejs.dev)
[![Tokio](https://img.shields.io/badge/Tokio-Async-FF4088?logo=rust&logoColor=white)](https://tokio.rs)
[![SQLite](https://img.shields.io/badge/SQLite-rusqlite-003B57?logo=sqlite&logoColor=white)](https://www.sqlite.org)
[![Linux](https://img.shields.io/badge/Platform-Linux_x64-FCC624?logo=linux&logoColor=black)](https://www.kernel.org)
[![License: Source-Available](https://img.shields.io/badge/License-Source--Available-blue.svg)](LICENSE)

</div>

AloLite is a standalone, lightweight desktop messaging client written in pure Rust and Tauri v2. It connects directly to upstream messaging endpoints without intermediate proxies. All protocol state machines, cryptographic operations, and SQLite local caches reside in an isolated, UI-independent Rust core — delivering a native desktop experience with **instantaneous startup** and **under 40 MB base memory footprint**.

> [!WARNING]
> **Legal Notice & Trademark Disclaimer**  
> AloLite is an independent third-party open-source project and is **not** affiliated with, sponsored by, or endorsed by VNG Corporation. "Zalo" is a registered trademark of VNG Corporation; all references within this repository are made strictly under **Nominative Fair Use** for technical interoperability research on Linux. See [`docs/LEGAL.md`](docs/LEGAL.md).

---

## Features

**Sign-in & session**

- QR-code or password sign-in; Trusted Device flow implemented in pure Rust.
- Automatic session save/restore on startup; session expiry (`zpw_sek`) is handled without freezing the app.
- Switch between multiple saved accounts.

**Conversations & messages**

- Friends, groups and recent conversations, with diacritic-insensitive search.
- Text, replies (quote), forward, recall, delete-for-me, reactions, stickers, images, file attachments (chunked upload) and link previews.
- Unread counters, mark-as-unread, mute, pin, hide and delete conversations; `@` mention detection.
- Real-time delivery over WebSocket combined with long-polling.

**Local data**

- Messages and threads are stored in SQLite for offline reading.
- History sync from the phone (Sync V1/V2, `ZDB4.0` container) and import from older data directories.
- Data lives in `$XDG_DATA_HOME/alolite` (default `~/.local/share/alolite`) with `0700` directory / `0600` file permissions.

**Integration & automation**

- **MCP** server (JSON-RPC over stdio or HTTP) exposing `zalo_status`, `zalo_list_threads`, `zalo_get_messages`, `zalo_search_messages`, `zalo_send_message` and `zalo_configure_autoreply`.
- **Headless** mode (REST + MCP HTTP) with no GUI.
- Rule-based auto-reply, light/dark/system themes, update checks against GitHub Releases.

**Security**

- Every outbound request goes through one path: params → `ParamCipher v2` → `signkey` → session cookies.
- A Privacy Guard keeps session keys, cookies, IMEI and sensitive content out of logs.

---

## Installation & Runtime Packages

AloLite renders its interface using the system's native **WebKitGTK 4.1** WebView, keeping memory usage minimal with zero bundled Chromium or Electron runtimes.

<details>
<summary><b>Distribution Packages & System Dependencies (Debian, Ubuntu, Fedora, Arch)</b></summary>
<br>

### 1. WebView and GUI libraries (required)

| Distribution                           | Command                                                         |
| :------------------------------------- | :-------------------------------------------------------------- |
| Debian / Ubuntu / Linux Mint / Pop!_OS | `sudo apt install libgtk-3-0 libwebkit2gtk-4.1-0 libsoup-3.0-0` |
| Fedora / RHEL / Rocky                  | `sudo dnf install gtk3 webkit2gtk4.1 libsoup3`                  |
| Arch / Manjaro / EndeavourOS           | `sudo pacman -S gtk3 webkit2gtk-4.1 libsoup3`                   |

> [!NOTE]
> Installing the official `.deb` or `.rpm` release package pulls these dependencies automatically.

### 2. Video playback (GStreamer)

WebKitGTK plays inline `<video>` through GStreamer. AloLite checks for `qtdemux` (MP4 demuxer) and `autoaudiosink` elements at startup and shows a helpful hint in chat when they are absent.

| Distribution                           | Command                                                                                                            |
| :------------------------------------- | :----------------------------------------------------------------------------------------------------------------- |
| Debian / Ubuntu / Linux Mint / Pop!_OS | `sudo apt install gstreamer1.0-plugins-base gstreamer1.0-plugins-good gstreamer1.0-plugins-bad gstreamer1.0-libav` |
| Fedora / RHEL / Rocky                  | `sudo dnf install gstreamer1-plugins-base gstreamer1-plugins-good gstreamer1-plugins-bad-free gstreamer1-libav`    |
| Arch / Manjaro / EndeavourOS           | `sudo pacman -S gst-plugins-base gst-plugins-good gst-plugins-bad gst-libav`                                       |

- `gst-plugins-base` provides core pipeline elements, `gst-plugins-good` and `gst-plugins-bad` provide MP4/WebM demuxers, and `gst-libav` provides H.264/AAC decoders used by chat media.
- On Fedora, stock `gstreamer1-libav` may lack H.264 for patent reasons; enable [RPM Fusion](https://rpmfusion.org/) and install its `ffmpeg`/`gstreamer1-plugin-libav` packages.
- Verify the installation:

  ```bash
  gst-inspect-1.0 --exists qtdemux && echo "mp4 demuxer: ok"
  gst-inspect-1.0 --exists autoaudiosink && echo "audio sink: ok"
  gst-inspect-1.0 avdec_h264 >/dev/null && echo "h264 decoder: ok"
  ```

### 3. Optional desktop helpers

| Purpose                   | Debian / Ubuntu      | Fedora                   | Arch                 |
| :------------------------ | :------------------- | :----------------------- | :------------------- |
| File dialogs              | `zenity`             | `zenity`                 | `zenity`             |
| Clipboard (X11 / Wayland) | `xclip wl-clipboard` | `xclip wl-clipboard`     | `xclip wl-clipboard` |
| Vietnamese-capable fonts  | `fonts-noto-core`    | `google-noto-sans-fonts` | `noto-fonts`         |

These match the `Recommends` of the `.deb`/`.rpm` packages.

</details>

---

## Build from source

```bash
git clone https://github.com/ngxccc/alolite
cd alolite
make dev-desktop        # builds the UI, then runs alolite-desktop
```

<details>
<summary><b>Prerequisites, CLI Modes & Build Matrix</b></summary>
<br>

### Toolchain requirements

- Rust 1.75+ (edition 2021)
- [Bun](https://bun.sh) (only required to compile the static frontend assets; the final binary contains zero Node.js runtimes)
- **Fast Linker**: `mold` and `clang` (configured in `.cargo/config.toml` for sub-second native linking)
- Tauri v2 build dependencies:

| Distribution                                                                                         | Command                                                                                                                                                                                  |
| :--------------------------------------------------------------------------------------------------- | :--------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Debian / Ubuntu                                                                                      | `sudo apt install build-essential pkg-config mold clang libssl-dev libgtk-3-dev libwebkit2gtk-4.1-dev libsoup-3.0-dev librsvg2-dev libayatana-appindicator3-dev libxdo-dev patchelf`     |
| Fedora                                                                                               | `sudo dnf install gcc gcc-c++ pkgconf-pkg-config mold clang openssl-devel gtk3-devel webkit2gtk4.1-devel libsoup3-devel librsvg2-devel libappindicator-gtk3-devel libxdo-devel patchelf` |
| Arch                                                                                                 | `sudo pacman -S --needed base-devel mold clang openssl gtk3 webkit2gtk-4.1 libsoup3 librsvg libappindicator-gtk3 xdotool patchelf`                                                       |
| See the [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/) for the authoritative list. |

### Command-line & headless modes

```bash
cargo run -p alolite-cli --bin alolite -- --qr                          # QR sign-in in the terminal
cargo run -p alolite-cli --bin alolite -- --mcp                         # MCP server over stdio
cargo run -p alolite-cli --bin alolite -- --api-port 18128 --api-only   # REST + MCP HTTP, no GUI
cargo run -p alolite-cli --bin alolite -- --help                        # All options
```

### Build, test, package

| Task                                                    | Command                                                               |
| :------------------------------------------------------ | :-------------------------------------------------------------------- |
| Build CLI and core                                      | `cargo build`                                                         |
| Release build                                           | `cargo build --release --bin alolite --bin alolite-desktop`           |
| Rust tests                                              | `cargo test --locked`                                                 |
| UI tests                                                | `(cd apps/desktop && bun run typecheck && bun test && bun run build)` |
| Format                                                  | `cargo fmt --all`                                                     |
| Lint                                                    | `cargo clippy --workspace --all-targets -- -D warnings`               |
| Linux packages (`.deb`, `.rpm`, `.AppImage`, `.tar.gz`) | `make package`                                                        |
| Install desktop entry into `~/.local`                   | `make install`                                                        |

`make help` lists every available command.

</details>

---

## Architecture

```
crates/alolite-core/      UI-independent core: CoreEngine, protocol/, storage/, mcp/, services/
crates/alolite-cli/       `alolite` binary: --mcp, --api-only, --qr, --phone, desktop launcher
crates/alolite-desktop/   `alolite-desktop` binary: Tauri v2 host wrapping alolite-core
apps/desktop/             React + TypeScript UI (src/{app,shared,features})
docs/                     Engineering standards, ADRs, design, roadmap, legal
packaging/                Linux packaging: .deb, .rpm, .AppImage, .tar.gz
```

Boundaries are enforced strictly:

- `protocol/` is the pure protocol layer (HTTP, WebSocket, crypto); it knows nothing about SQLite or `MessageStore`.
- `storage/` only deals with SQLite and `threads_cache.json`; it never touches the network.
- `CoreEngine` composes the two; the UI only invokes Tauri commands and renders `CoreEventDto`s, never doing heavy work on the UI thread.
- All hosts live in `protocol/endpoints.rs`; all HTTP command IDs live in `protocol/registry/`.

Details: [`docs/adr/`](docs/adr/) and [`docs/standards/`](docs/standards/).

---

## Documentation

> Online documentation book is published at **[https://ngxccc.github.io/alolite/](https://ngxccc.github.io/alolite/)**.

| Document                             | Contents                                                                                |
| :----------------------------------- | :-------------------------------------------------------------------------------------- |
| [`AGENTS.md`](AGENTS.md)             | Operating guide for humans and AI agents: commands, architecture map, non-negotiables   |
| [`docs/standards/`](docs/standards/) | API, SQLite, concurrency, testing, security, commit and PR standards                    |
| [`docs/adr/`](docs/adr/)             | Architecture decisions (Trusted Device, `protocol/` boundaries, Tauri, Cargo workspace) |
| [`docs/ROADMAP.md`](docs/ROADMAP.md) | Scope and roadmap                                                                       |
| [`GLOSSARY.md`](GLOSSARY.md)         | Domain glossary                                                                         |

---

## Contributing

Please read [`CONTRIBUTING.md`](CONTRIBUTING.md), [`AGENTS.md`](AGENTS.md), and [`docs/standards/git-flow-and-pr-matrix.md`](docs/standards/git-flow-and-pr-matrix.md) before opening a PR: prefixed branches (`feature/`, `fix/`, `core/`, …), Conventional Commits, and **never commit** `session.json`, `*.db`, cookies, `zpw_sek`, `zpw_enk`, IMEI or capture dumps.

Any change to protocol parameters, signatures, frame IDs or the cipher flow needs evidence (a capture or note) plus a test or a documented manual verification.

---

## Legal & acceptable use

- Clean-room project: no third-party source code, artwork, icons, stickers or fonts are shipped in this repository. See [`docs/LEGAL.md`](docs/LEGAL.md).
- Use it only with your own lawful account. Automated spam, harassment, scraping other users' data, or circumventing the service's rate limits is prohibited.
- No ads, no telemetry, no monetization.

## License

Licensed under the **[Alolite Source-Available & Non-Commercial License](LICENSE)**. Personal, non-commercial, and technical research use only.
