# Compliance, Intellectual Property & Clean-Room Standards

This document establishes the mandatory intellectual property and legal compliance standards for all human engineers and AI agents contributing to `alolite`.

---

## 1. Core Invariants

### 1.1 Clean-Room Re-implementation

- **Specification-Only Implementation:** Analyze only wire formats, parameter schemas, cryptographic handshakes, and public network protocols.
- **FORBIDDEN:** Committing verbatim decompiled code (from Ghidra, Hopper, IDA, or Wakaru), decompiled C/C++ fragments, or minified JavaScript extracted from `app.asar`.
- **FORBIDDEN:** Distributing or vendoring proprietary binary blobs, dynamic libraries (`.dll`, `.so`, `.dylib`), or compiled helper executables.

### 1.2 Zero Copyrighted Media Assets in Git

- **No Bundled Third-Party Artwork:** Never commit proprietary graphic assets to the Git tree:
  - Official brand logos, favicons, or application icons.
  - Proprietary sticker packs (e.g., Ami Fat Cat, Onion, or artist-copyrighted sticker sets).
  - Proprietary audio assets (ring tones, call alerts, notification sounds).
- **Dynamic Fetching Requirement:** Any user-facing stickers, avatars, or media must be retrieved dynamically over HTTPS into the user's private cache directory (`~/.cache/alolite/`) using active session credentials.

### 1.3 Trademark & Naming Discipline

- **Public Surface:** Use the brand display name **AloLite** exclusively for user-facing surfaces (UI window titles, notifications, desktop entry names, documentation). Technical identifiers (binary names, packages, data directories) strictly use lowercase **alolite** or **alolite-desktop**. Do not use trademarked third-party names.
- **Internal Structure:** Keep protocol logic organized under neutral technical namespaces (`src/protocol/`).
- **Nominative Fair Use:** References to the target service are permitted only in technical comments and interoperability notes where necessary to describe protocol compatibility.

### 1.4 Secret & Session Isolation

- `session.json`, SQLite databases (`*.db`), authentication cookies, device IMEIs, and session encryption keys (`zpw_sek`, `zpw_enk`) are personal user secrets.
- They must remain in `.gitignore` and **must never be committed to Git under any circumstances**.

---

## 2. Pull Request Review Checklist

Every pull request must pass this compliance audit before merging:

| Check             | Requirement                                                             | Violation Action                                    |
| :---------------- | :---------------------------------------------------------------------- | :-------------------------------------------------- |
| **Binaries**      | No `.exe`, `.dll`, `.so`, `.asar`, or proprietary blobs added.          | Immediate rejection.                                |
| **Media Assets**  | No proprietary stickers, brand logos, or copyrighted audio files added. | Immediate rejection; require dynamic CDN retrieval. |
| **Naming**        | No public-facing branding conflicts or misleading trademark usages.     | Require renaming to neutral technical identifiers.  |
| **Legal Notices** | Disclaimers, copyright notices, and license headers remain intact.      | Require re-addition before merge.                   |
