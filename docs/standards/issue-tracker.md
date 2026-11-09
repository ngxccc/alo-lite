# Issue Tracking & Triage Standards

Issues, specs and tickets live in **GitHub Issues** for `ngxccc/alolite`. Use the `gh` CLI for every operation.

> Source of truth for ticket state. Local markdown notes under `re-notes/` are evidence, never tickets.

## 1. CLI Command Conventions

| Operation | Command | Notes |
| :--- | :--- | :--- |
| Create issue | `gh issue create --title "..." --body "..."` | Use a heredoc for structured bodies. |
| View issue | `gh issue view <number> --comments` | Description, labels and discussion. |
| List issues | `gh issue list --state open --label "<label>"` | Filter by lifecycle, type or area. |
| Comment | `gh issue comment <number> --body "..."` | Post updates, link PRs, attach evidence. |
| Apply labels | `gh issue edit <number> --add-label "<label>"` | Update type/area/priority. |
| Close issue | `gh issue close <number> --comment "..."` | Always close with a resolution summary. |

## 2. Label Taxonomy

### A. Lifecycle (exceptions only)

State lives in the issue itself (`open`/`closed`), PR links (`Fixes #<id>`) and GitHub Projects. Do not maintain `in-progress` / `needs-review` labels.

| Label | Meaning |
| :--- | :--- |
| `status:blocked` | Waiting on an external dependency, credentials, upstream protocol evidence or another issue. |
| `wontfix` | Closed as out of scope, superseded or rejected. |

### B. Type (Conventional Commits alignment)

`type:feat`, `type:fix`, `type:refactor`, `type:perf`, `type:test`, `type:docs`, `type:chore`, `type:ci`, `type:style`, `type:task`.

### C. Area (domain scopes)

| Label | Scope |
| :--- | :--- |
| `area:core` | Core engine, background runtime, common utilities and services (`crates/alolite-core/src/{core,common,services}`). |
| `area:protocol` | Zalo network protocol, wire codecs, cryptographic handshakes, and message sync (`crates/alolite-core/src/protocol/`). |
| `area:auth` | QR/password login, `session.json`, `zpw_sek` / `zpw_enk`, and trusted device pairing. |
| `area:net` | HTTP endpoints, candidate failover, WebSocket framing, `ParamCipher v2`, and `signkey`. |
| `area:chat` | Threads, message identity, reactions, recall, forwarding, and history sync. |
| `area:store` | SQLite schema, migrations, queries, and local offline cache (`crates/alolite-core/src/storage/`). |
| `area:mcp` | MCP JSON-RPC protocol over stdio/HTTP, local REST daemon, and autoreply rules (`crates/alolite-core/src/mcp/`). |
| `area:ui` | Native Tauri v2 host and React desktop frontend (`apps/desktop/`, `crates/alolite-desktop/`). |
| `area:cli` | Command-line interface, terminal QR login, and desktop launcher (`crates/alolite-cli/`). |
| `area:packaging` | Linux distribution packages (.deb, .rpm, .AppImage, .tar.gz) under `packaging/`. |
| `area:assets` | Desktop icons, SVG branding assets, and reaction graphics under `assets/` and `.github/assets/`. |
| `area:docs` | Documentation book (`docs/`), ADRs, standards, README, and user guides. |
| `area:ci` | GitHub Actions workflows and CI/CD automation (`.github/workflows/`). |
| `area:harness` | `AGENTS.md`, AI agent tooling, `.omp/`, helper scripts, and Makefile. |
### D. Priority

| Label | Meaning |
| :--- | :--- |
| `priority:p0` | Critical: data loss, credential exposure, complete outage. |
| `priority:p1` | Blocks the next release milestone. |
| `priority:p2` | Normal (default). |

## 3. Triage Label Vocabulary

Canonical vocabulary consumed by `/triage`. Do not introduce synonyms.

| Role | Repo representation |
| :--- | :--- |
| `needs-triage` | Default: open issue with no `type:*` / `area:*` label. |
| `needs-info` | `status:blocked` + a comment naming the missing evidence. |
| `ready-for-agent` | `type:*` + `area:*` set, acceptance criteria present, no open blocker. |
| `ready-for-human` | `priority:p0` / `priority:p1`, or requires protocol evidence / physical device testing. |
| `wontfix` | `wontfix` label + closing comment. |

## 4. Native Sub-Issues Protocol

1. Create the child with canonical body format:

   ```bash
   gh issue create --title "task(<area>): <summary>" --label "type:task,area:<area>" --body "$(cat <<'EOF'
   ### Outcome
   <One-sentence observable behaviour or state when complete.>
   
   ### Context
   <Child task context or sub-problem specification.>
   
   ### Acceptance Criteria
   - [ ] <Specific verifiable condition>
   
   ### Non-goals
   - <Explicit boundaries>
   EOF
   )"
   ```

2. Link it to the parent:

   ```bash
   CHILD_ID=$(gh api repos/ngxccc/alolite/issues/<child_number> --jq .id)
   gh api --method POST repos/ngxccc/alolite/issues/<parent_number>/sub_issues -F sub_issue_id=$CHILD_ID
   ```

3. Query children:

   ```bash
   gh api repos/ngxccc/alolite/issues/<parent_number>/sub_issues \
     --jq '.[] | "#\(.number): \(.title)"'
   ```
## 5. Wayfinding

- **Map ticket**: one issue labeled `wayfinder:map`, holding decisions and the fog tracker.
- **Child tickets**: labeled `wayfinder:<research|prototype|grilling|task>` and linked as native sub-issues.
- **Blocking**: use native issue dependencies; a ticket is unblocked when every blocker is closed.

## 6. Ticket Quality Bar

A ticket is actionable only when it adheres to the canonical issue body structure (enforced across `.github/ISSUE_TEMPLATE/` and CLI usage):

### A. Canonical Body Structure

```markdown
### Outcome
<One-sentence observable behaviour or state when complete, not file inventory.>

### Context
<Current limitation, background, discrepancy, or reproduction steps.>

### Acceptance Criteria
- [ ] <Specific verifiable condition>
- [ ] <Verification command or test pass criteria>

### Non-goals
- <Explicit non-goal to prevent scope creep.>
```

### B. Checklist Requirements

- [ ] One-sentence outcome (observable behaviour, not file inventory).
- [ ] Reproduction or acceptance checklist (`- [ ]`).
- [ ] Affected area label (`area:*`) and type label (`type:*`).
- [ ] Evidence for protocol changes (capture path, `re-notes/` reference, server `error_code`).
- [ ] Explicit non-goals when scope is easy to over-read.

### C. CLI Heredoc Creation Template

```bash
gh issue create \
  --title "<type>(<area>): <summary>" \
  --label "type:<type>,area:<area>" \
  --body "$(cat <<'EOF'
### Outcome
...

### Context
...

### Acceptance Criteria
- [ ] ...

### Non-goals
- ...
EOF
)"
```
## 7. Security Rule

Never paste `session.json`, `zpw_sek`, `zpw_enk`, cookies, IMEI or raw captures containing them into an issue. Redact to `zpw_sek=<redacted>` and attach only sanitized excerpts.
