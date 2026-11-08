# ROADMAP: Alolite Linux (Minimal Native Messaging Client)

**Objective**: Build a minimal, ultra-lightweight, native Linux messaging client in Rust that starts instantaneously, uses minimal system resources, and eliminates Electron, Node.js, and out-of-scope non-core features (VoIP, Video Feed, Multi-platform bloat).

---

## 1. Layered Architecture & Separation of Concerns

The protocol layer is isolated in an independent module to ensure modularity, clean maintenance, and independent testability:

```
src/
├── main.rs                   # Single Linux CLI/GUI entrypoint
├── lib.rs                    # Crate root & module re-exports
├── protocol/                 # [FEATURE 1] Pure network protocol, auth & crypto
│   ├── mod.rs, crypto.rs, session.rs, login.rs, qr.rs, chat.rs, socket.rs, ...
├── storage/                  # [FEATURE 2] Local SQLite persistence (messages, threads)
│   └── mod.rs
├── gui/                      # [FEATURE 3] Native egui desktop GUI (Wayland & X11)
│   ├── mod.rs, workspace.rs, messages.rs, modals.rs, theme.rs, stickers.rs, worker.rs
├── common/                   # [SHARED] Shared utilities (clock, data_dir, logger, ui)
│   └── mod.rs, clock.rs, data_dir.rs, logger.rs, ui.rs
└── legacy/                   # [QUARANTINED] Out-of-scope modules (VoIP, Feed, Scheduler)
```

---

## 2. Feature Scope Boundaries

### ✅ In Scope (Core Linux MVP):
1. **Authentication**: QR code scanning, persistent session and cookie storage, automatic session restore on startup.
2. **Conversations**: Friend list, group list, and recent conversation threads.
3. **Messaging**:
   - Real-time text, basic sticker, and image receipt via long-lived WebSocket connection.
   - Text message, basic emoji, and image transmission via HTTP endpoints.
4. **Local Persistence**: SQLite local message and thread caching, enabling offline message reading.
5. **Platform**: Native Linux execution on Wayland and X11, compatible with Vietnamese input methods (ibus, fcitx5).

### ❌ Out of Scope (Explicit Non-Goals):
1. **VoIP / Audio & Video Calls**: Omit WebRTC signaling, FFmpeg camera probing, and Opus streaming (`src/call.rs`, `src/audio.rs`).
2. **Zalo Video & Social Feed**: Omit timeline feed, video stories, and social profiles (`src/feed.rs`).
3. **TUI Terminal Interface**: Completely deleted (`src/tui.rs` removed, `ratatui` and `crossterm` dropped).
4. **Multi-Account Management**: Focus on a single reliable, hardened user session.
5. **Windows / macOS Legacy**: Remove Windows PE resource compilation, manifests, Inno Setup scripts, and macOS notification helpers.

---

## 3. Execution Roadmap (4 Phases)

### Phase 1: Codebase Pruning & Linux Build Simplification [COMPLETED]
- [x] **Empirical Verification**: Validated `ParamCipher v2` and `reqqr` authentication against live servers (`code: 0`).
- [x] **Protocol Decoupling**: Extracted all protocol logic into the isolated `src/protocol/` module.
- [x] **Build Script Cleanup**: Stripped 250+ lines of Windows PE COFF `.res` generator from `build.rs`.
- [x] **UI Streamlining**: Hid feed navigation from the left sidebar; removed Windows console attachment from `src/main.rs`.
- [x] **TUI Deletion**: Completely removed `src/tui.rs`, dropped `ratatui` and `crossterm` dependencies, streamlined client to pure `egui` desktop.
- [x] **Verification**: Confirmed `cargo check --all-targets` (0 warnings) and all 204 unit tests passing.

### Phase 2: QR Authentication & Persistent Session Management [COMPLETED]
- [x] **XDG Standard Compliance**: Standardized session and database storage at `~/.local/share/alolite/` (or `$XDG_DATA_HOME/alolite`) with private `0700` directory and `0600` file permissions (`src/common/data_dir.rs`).
- [x] **Seamless Session Restore**: Validated instant session resume from `session.json` on startup without blocking the UI thread; non-expired sessions navigate directly into chat workspace.
- [x] **Graceful Expiration & Re-authentication**: Handled token expiry (`zpw_sek` invalidation / error code 600 / -102) by marking expiration, notifying UI (`GuiEvent::SessionExpired`), and generating fresh QR without application crash.
- [x] **QR Lifecycle & Timeout Handling**: Added `GuiEvent::QrExpired` when long-poll scan or confirmation times out, clearly updating the login UI state and providing manual and automatic refresh options.

### Phase 3: Core Messaging & WebSocket Synchronization
- [ ] Fetch recent conversations (`get_friends`, `get_groups`) and persist into SQLite `store.rs`.
- [ ] Sync conversation message history (`get_group_history` / 1-1 chats).
- [ ] Implement text message sending (`/api/message/sms`) with optimistic UI update.
- [ ] Maintain a stable long-lived WebSocket connection (`socket.rs`) for push notifications and unread badge updates.
- [ ] Implement basic image viewing and photo sending.

### Phase 4: Linux Polish & Packaging
- [ ] Validate font rendering (`BeVietnamPro`) and Vietnamese input methods (ibus, fcitx5) in `egui`.
- [ ] Polish UI with responsive dark/light themes.
- [ ] Produce standalone Linux distribution artifacts: stripped binary, `alolite.desktop` entry, and `.deb` package.
