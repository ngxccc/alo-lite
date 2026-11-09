# Git Flow & Pull Request Standards

## 1. Branch Naming

| Prefix | Use |
| :--- | :--- |
| `feature/<kebab-name>` | New capability. |
| `fix/<kebab-name>` | Bug fix. |
| `hotfix/<kebab-name>` | Production patch, branched from the release tag. |
| `core/<kebab-name>` | Architecture, schema, protocol or crypto refactor (Tier 3). |
| `docs/<kebab-name>` | Documentation only. |
| `chore/<kebab-name>` | Tooling, deps, CI. |

- Branch from `main`; keep branches short-lived.
- One logical change per branch. Do not mix formatting sweeps with behaviour changes.
- Rebase on `main` before opening the PR; no merge commits inside a feature branch.

## 2. Conventional Commits

**Format**: `<type>(<scope>): <summary>` — header ≤ 72 characters, imperative mood, no trailing period.

| Type | Use |
| :--- | :--- |
| `feat` | New user-visible capability. |
| `fix` | Bug fix. |
| `refactor` | Behaviour-preserving restructure. |
| `perf` | Performance. |
| `test` | Tests only. |
| `docs` | Documentation. |
| `chore` | Build, deps, tooling. |
| `ci` | Workflows, packaging. |
| `style` | Formatting only. |

**Scopes** (lowercase, module-aligned): `auth`, `core`, `net`, `chat`, `media`, `mcp`, `ui`, `store`, `crypto`, `session`, `ci`, `docs`, `harness`.

```
feat(chat): add quote-reply rendering
fix(session): clear expired zpw_sek before QR re-login
refactor(store): move schema bootstrap into versioned migrations
```

Rules:

- Body explains **why**, not a file list; use concise `-` bullets.
- One commit = one atomic, buildable change.
- Breaking protocol/schema changes use `!` and an ADR link in the body: `feat(store)!: ...`.

## 3. Adaptive 3-Tier PR Matrix

| Tier | Scope & risk | Requirements |
| :--- | :--- | :--- |
| **Tier 1 — Patch** | Typo, comment, docs, dependency bump with no API change. | One-sentence summary, CI green. |
| **Tier 2 — Standard** | Feature, bugfix, refactor, test. | Conventional Commits, 5-section PR body, tests pass, no new clippy warnings. |
| **Tier 3 — Enterprise** | Protocol/cipher/schema change, breaking change, release packaging. | Atomic commits, ADR/RFC link, protocol evidence, human audit, manual verification steps. |

### Standard PR Body Schema (`.github/PULL_REQUEST_TEMPLATE.md`)

Every PR opened against `main` must adhere to the 5-section schema defined in `.github/PULL_REQUEST_TEMPLATE.md`:

```markdown
## Summary
<!-- Concise explanation of what this PR changes and the technical rationale -->

## Context
- Resolves: # <!-- If closing an issue, e.g. Resolves: #123 -->
- Relates to: # <!-- If referencing an issue without closing -->
- None <!-- If standalone change -->

## Type
- [ ] feat: New feature
- [ ] fix: Bug fix
- [ ] refactor: Code refactoring
- [ ] perf: Performance improvement
- [ ] test: Testing updates
- [ ] docs: Documentation update
- [ ] ci: CI/CD or workflow update
- [ ] chore: Maintenance or dependencies

## Verification & Evidence
- **Platform(s) Tested**:
  - Architecture: [ ] x86_64  [ ] aarch64
  - Display Server: [ ] Wayland  [ ] X11
  - Distribution: <!-- e.g. Arch, Ubuntu 24.04, Fedora 40 -->
- **Evidence**:
  - Output / Screenshots / Test logs: <!-- Attach screenshots for UI changes, or paste cargo test output -->

## Checklist
- [ ] `cargo test` passes locally.
- [ ] No proprietary binaries, secrets, or generated files (`target/`, `dist/`, `session*.json`) are committed.
- [ ] No RE code violates the Clean-Room & IP guidelines in `LEGAL.md` and `docs/standards/compliance-and-ip.md`.
- [ ] Documentation updated if introducing new features, flags, or configuration options.
```

Tier 3 additionally requires: ADR link in `## Summary`, protocol capture/evidence reference (`re-notes/...`), and a rollback note for schema or protocol changes under `## Verification & Evidence`.

## 4. Human Audit & Verification Protocol

1. **Inspection & proposal only**: before committing, run `git status` and `git diff`; propose the atomic commit slicing plan and show the formatted Conventional Commit messages.
2. **No secrets in the diff**: verify `session.json`, `*.db`, `dump/`, `qr.png` and logs are untracked.
3. **Explicit confirmation required**: never run `git commit` / `git push` until a human approves.
4. **Commit bodies** use short bullet points, not paragraphs.

## 5. Review Checklist

- [ ] Branch name matches the prefix table.
- [ ] Commits are atomic and conventional; header ≤ 72 chars.
- [ ] `cargo fmt --all` and `cargo clippy` clean for touched code.
- [ ] `cargo test --locked` passes.
- [ ] Docs/standards updated when behaviour or conventions change.
- [ ] ADR added or referenced for Tier 3 decisions.
- [ ] No secrets, logs, databases or capture dumps committed.
- [ ] PR body follows the tier requirement.

## 6. Releases

- Release tags follow `v<major>.<minor>.<patch>`; `v*` triggers `.github/workflows/release.yml`.
- A release PR must list the artifact matrix and the tested platforms.
- Never tag from a dirty worktree or an unsigned-off CI run.
