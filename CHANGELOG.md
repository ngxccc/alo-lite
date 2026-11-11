# Changelog

Generated at release time from merged pull requests, by `bun run release`.
Do not edit by hand; the sections come from the same generator as the GitHub release.

## 0.1.2 - 2026-11-11

<!-- Release notes generated using configuration in .github/release.yml at 1db31ef71926b82d2f5ee30e3a43a5d28ae5924d -->

## What's Changed
### Bug Fixes
* fix(auth): harden the session lifecycle from login to polling by @ngxccc in https://github.com/ngxccc/alolite/pull/49
### Performance
* perf(ui): integrate React Compiler, domain stores, and eliminate prop drilling by @ngxccc in https://github.com/ngxccc/alolite/pull/42
### Refactors
* refactor(social): extract post_friend_api helper and update endpoint by @ngxccc in https://github.com/ngxccc/alolite/pull/37
* refactor(ui): standardize all desktop frontend filenames to kebab-case by @ngxccc in https://github.com/ngxccc/alolite/pull/39
* refactor(protocol): introduce two-tier wire documentation and typed WireData models (#41) by @ngxccc in https://github.com/ngxccc/alolite/pull/46
* refactor(net): remove outbound proxy support by @ngxccc in https://github.com/ngxccc/alolite/pull/48
### CI
* ci(hooks): add native git hooks for formatting and conventional commits by @ngxccc in https://github.com/ngxccc/alolite/pull/18
* ci(infra): optimize CI pipeline, add docker image, pin ubuntu-22.04 by @ngxccc in https://github.com/ngxccc/alolite/pull/20
* ci(container): add .dockerignore, registry cache, and tool auto-skip by @ngxccc in https://github.com/ngxccc/alolite/pull/22
* ci(workflows): fix GitHub Actions cache permissions and sccache gha export by @ngxccc in https://github.com/ngxccc/alolite/pull/29
* ci(setup-rust): route sccache to actions cache v2 endpoint by @ngxccc in https://github.com/ngxccc/alolite/pull/30
* chore(harness): standardize project naming across workspace crates, packages, and UI by @ngxccc in https://github.com/ngxccc/alolite/pull/34

## New Contributors
* @dependabot[bot] made their first contribution in https://github.com/ngxccc/alolite/pull/21

**Full Changelog**: https://github.com/ngxccc/alolite/compare/v0.1.1...v0.1.2
## 0.1.1 - 2026-11-09

<!-- Release notes generated using configuration in .github/release.yml at bacef0e672874466e02927d5459f8dde3907ff50 -->

## What's Changed
### Refactors & Tech Debt
* refactor(assets): relocate branding and icon assets to workspace root by @ngxccc in https://github.com/ngxccc/alo-lite/pull/14
### CI & Tooling
* ci(workflows): add path labeler, security scan, and concurrency filters by @ngxccc in https://github.com/ngxccc/alo-lite/pull/16
### Documentation
* docs(all): renumber ADRs, add contributing guide, and refresh readme by @ngxccc in https://github.com/ngxccc/alo-lite/pull/15

## New Contributors
* @ngxccc made their first contribution in https://github.com/ngxccc/alo-lite/pull/14

**Full Changelog**: https://github.com/ngxccc/alo-lite/compare/v0.1.0...v0.1.1
## 0.1.0 - 2026-11-09

### AloLite Release (v0.1.0)

Lightweight, native messaging desktop client for Linux written in Rust (Zero Node.js, Zero Electron).

#### Available Linux Packages:
- **Debian / Ubuntu / Linux Mint / Pop!_OS:** `alolite_*_amd64.deb`
- **Fedora / RHEL / Rocky / openSUSE:** `alolite-*-1.x86_64.rpm`
- **Universal Single-File Executable:** `alolite-*-x86_64.AppImage`
- **Portable Standalone Archive:** `alolite-*-linux-x86_64.tar.gz`

#### Checksums:
Verify downloaded assets with `sha256sum -c SHA256SUMS.txt`.

<!-- Release notes generated using configuration in .github/release.yml at v0.1.0 -->



**Full Changelog**: https://github.com/ngxccc/alo-lite/commits/v0.1.0
