# Contributing to Alolite

Thank you for your interest in contributing to **Alolite**! This guide outlines our engineering standards, architecture, clean-room policies, and pull request workflows.

---

## 1. Principles & Clean-Room Standards

Alolite is an independent, clean-room software client created for platform interoperability on Linux operating systems. All contributors must strictly adhere to our compliance invariants:

- **Clean-Room Re-implementation**: Only reverse-engineered wire specifications, cryptographic handshakes, and public network protocols may be analyzed. **Never** commit decompiled source code, disassembly dumps (Ghidra, IDA, Hopper), or proprietary binaries.
- **Zero Copyrighted Media**: Do not commit official brand logos, proprietary sticker packs, notification sounds, or commercial artwork. All user-facing media must be fetched dynamically at runtime into local cache.
- **Zero Secret Ingestion**: Personal session files (`session.json`), SQLite databases (`*.db`), authentication cookies, device IMEIs, and session encryption keys (`zpw_sek`, `zpw_enk`) are strictly git-ignored and must never enter Git history.
- **License Agreement**: By submitting pull requests or issues, you agree that your contributions are licensed under the **[Alolite Source-Available & Non-Commercial License](LICENSE)**. Contributions must be free of third-party commercial claims.

See [`docs/LEGAL.md`](docs/LEGAL.md) and [`docs/standards/compliance-and-ip.md`](docs/standards/compliance-and-ip.md) for full legal specifications.

---

## 2. Development Setup

### Prerequisites

- **Rust**: 1.75+ (stable toolchain with `cargo`, `rustfmt`, and `clippy`)
- **Bun**: Modern JavaScript/TypeScript runtime and package manager
- **Linux Packages**: `gtk3`, `webkit2gtk-4.1`, `libsoup-3.0` (for Tauri v2 desktop host)

### Common Commands

| Task | Command |
| :--- | :--- |
| **Build CLI & Core** | `cargo build` |
| **Build Desktop App** | `(cd apps/desktop && bun run build) && cargo build --bin alolite-desktop` |
| **Run Desktop App** | `cargo run --bin alolite-desktop` |
| **Run Headless Daemon** | `cargo run --bin alolite -- --api-port 18128 --api-only` |
| **Run MCP (stdio)** | `cargo run --bin alolite -- --mcp` |
| **Test Rust Core & CLI** | `cargo test --locked` |
| **Test Desktop UI** | `(cd apps/desktop && bun run typecheck && bun test && bun run build)` |
| **Format Check** | `cargo fmt --all -- --check` |
| **Lint** | `cargo clippy --workspace --all-targets -- -D warnings` |

---

## 3. Architecture Overview

Alolite is organized as a Cargo workspace with a dedicated web frontend:

```
alolite/
├── crates/
│   ├── alolite-core/      # Protocol state machines, crypto, SQLite storage, MCP runtime
│   ├── alolite-cli/       # CLI launcher, headless daemon (--api-only), MCP stdio
│   └── alolite-desktop/   # Tauri v2 native desktop host embedding frontend dist
├── apps/
│   └── desktop/           # Feature-sliced React 18 + TypeScript frontend
└── docs/
    └── standards/         # Authoritative engineering standards and ADRs
```

### Architectural Non-Negotiables

1. **No UI-Thread Blocking**: Network, disk I/O, SQLite access, and image decoding live exclusively inside `alolite-core` (`CoreEngine`). The desktop frontend communicates asynchronously through Tauri commands and `CoreEventDto` streams.
2. **SSOT Protocol Endpoints**: All server host definitions live in `crates/alolite-core/src/protocol/endpoints.rs`. Never hardcode API hostnames elsewhere.
3. **Pure Protocol Boundary**: Keep `rusqlite` out of `src/protocol/`, and keep HTTP/Session transports out of `src/storage/`.
4. **UI Icon Standards**: Use vector icons (`lucide-react`) or embedded SVG/PNG assets. Never use raw emoji characters as functional UI elements.

---

## 4. Git Flow & Commit Guidelines

We enforce **Conventional Commits 1.0.0** and atomic commit slicing.

### Branch Naming

Branch from `main` using standard prefixes:

- `feature/<kebab-name>`: New capability or enhancement
- `fix/<kebab-name>`: Bug fix
- `core/<kebab-name>`: Architecture, protocol, crypto, or database schema refactor
- `docs/<kebab-name>`: Documentation improvements
- `chore/<kebab-name>`: Tooling, dependencies, or packaging updates

### Commit Message Format

```text
<type>(<scope>): <summary>

- Bulleted rationale explaining why the change is necessary
- Concise summary of implementation details or protocol invariants
```

- **Header**: $\le 72$ characters, imperative mood, lowercase, no trailing period.
- **Allowed Types**: `feat`, `fix`, `refactor`, `perf`, `test`, `docs`, `chore`, `ci`, `style`, `core`, `security`.
- **Allowed Scopes**: `auth`, `core`, `net`, `chat`, `media`, `mcp`, `ui`, `store`, `crypto`, `session`, `ci`, `docs`, `harness`.
- **No Premature Closes**: Do not use `Fixes #...` or `Closes #...` in commit messages. Auto-closing keywords belong exclusively in PR descriptions.

---

## 5. Pull Request Process (3-Tier PR Matrix)

Before opening a PR, determine its tier according to risk and scope:

| Tier | Scope | Requirements |
| :--- | :--- | :--- |
| **Tier 1: Patch** | Typos, docs, non-breaking dependency updates | Single-sentence summary, all CI checks green |
| **Tier 2: Standard** | Features, bug fixes, routine refactors | 5-section PR body, tests pass, zero clippy warnings |
| **Tier 3: Enterprise** | Protocol, cipher, DB schema, or breaking changes | Atomic commits, ADR/RFC reference, protocol evidence, rollback plan |

### 5-Section PR Body Template (Tier 2 & 3)

```markdown
## What
- Concise description of changes made.

## Why
- Problem solved or motivation behind this PR.

## How
- High-level architecture, module changes, or protocol updates.

## Verification
- [ ] `cargo test --locked` passes.
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean.
- [ ] `(cd apps/desktop && bun run typecheck && bun test)` passes.
- [ ] Manual verification flow executed and described.

## Risk & Rollback
- Risk evaluation (blast radius) and rollback strategy.
```

---

## 6. Code Style & Comment Taxonomy

- **Language Policy**:
  - **Source Code Comments**: English only (`///` contracts, `//` rationale).
  - **User-Facing UI Strings**: Vietnamese (UI labels, error toasts, desktop dialogs).
- **Comment Taxonomy**:
  - `///` Rustdoc Contracts: Mandatory for every `pub` function, struct, and module (`# Errors`, `# Panics`, `# Invariants`).
  - `// SAFETY:`: Mandatory above every `unsafe` block explaining sound reasoning.
  - `// TODO(context-or-issue)`: Always associate TODO comments with an issue number or stable context token.
  - **Banned**: Obvious code echoes (`// increment counter`), commented-out dead code, or banner decorations.

Detailed guidelines: [`docs/standards/code-comment-taxonomy.md`](docs/standards/code-comment-taxonomy.md).
