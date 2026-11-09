# ADR-0001: Pure-Rust Trusted Device Protocol Implementation

- **Status:** Accepted
- **Date:** 2026-11-07
- **Author:** ngxc
- **Deciders:** Alolite Core Engineering

---

## 1. Context & Problem Statement

To support direct 1-1 end-to-end messaging and resolve Zalo server `error_code: 10002` (Group not found / Companion authorization required), Alolite requires the **Trusted Device Protocol**.

The official Zalo client ships this logic inside an isolated WebAssembly binary:
- File: `trusted_device_protocol_bg.97af2bf51fb002cefc5a888e932bcab2.wasm` (SHA-256: `716f0a0637...`)
- Size: 346 KB, 331 functions, stripped symbols.
- Analysis across official Linux, Windows (`Zalo-Win-26.11.10.7z`), and macOS Universal (`Zalo 26.11.10.dmg`) packages revealed that **no native binary library exists**; all desktop platforms run this protocol via WebAssembly.

Per Alolite core principles (Zero Node.js, Zero Electron, pure native Rust executable) and Clean-Room IP Standards (`docs/standards/compliance-and-ip.md`), Alolite cannot bundle or execute proprietary WASM runtimes or embed proprietary WASM blobs.

---

## 2. Decision

Re-implement the entire Trusted Device Protocol in clean-room pure Rust under `src/protocol/auth/trust/` without external code generators or runtime WASM dependencies:

1. **Curve25519 & XEdDSA:** Use `curve25519-dalek 4` (with `digest` feature) and `sha2` to implement Signal-compatible 33-byte Curve25519 public keys (`0x05` Djb type) and 64-byte XEdDSA signatures ($R \parallel s$) with the Edwards sign-bit packed into bit 7 of the last byte.
2. **Protobuf Codec:** Implement a zero-dependency, zero-allocation-where-possible protobuf reader and writer in `wire.rs` strictly matching the wire semantics observed via differential fuzzing against the reference WASM.
3. **basE91:** Pure Rust implementation of the standard basE91 text codec for the `"Z1@"` QR payload in `base91.rs`.
4. **Security & Zeroization:** All private scalars and linking secrets implement `Zeroize` and `ZeroizeOnDrop`. Constant-time comparison (`subtle::ConstantTimeEq`) is mandatory for MAC and signature checks.
5. **Testing & Verification:** Keep deterministic known-answer test vectors in `tests/fixtures/trusted_device_kat.json` and integration tests in `tests/protocol_trusted_device.rs`.

---

## 3. Consequences

### Positive
- **Zero WASM / JS Dependency:** Native execution speed, instant startup, zero runtime memory overhead from WASM interpreters.
- **Strict Clean-Room:** Clean implementation derived exclusively from black-box differential testing and wire observations.
- **Portability:** Seamless compilation across Linux (x86_64, aarch64), Windows, and macOS without extra native C dependencies.

### Negative / Trade-offs
- Future protocol updates from upstream Zalo require re-verifying against updated official packages using the established REA harness (`re-notes/11_trusted_device_protocol.md`).

---

## 4. Compliance & Invariants

- **INV-1:** All companion public keys must start with byte `0x05` and measure 33 bytes.
- **INV-2:** Private keys must be clamped per RFC 7748 before scalar operations.
- **INV-3:** Linking MACs must be verified in constant time before parsing inner metadata keys.
- **INV-4:** Device list companions are strictly ordered by `companion_id` ascending; duplicates are resolved as last-write-wins.
- **INV-5:** The QR2 companion flow MUST only be offered when the login-time server flag
  `getServerInfo` → `setttings.authen.trusted_device.qr_format_trust_device` is `1`; with `0`
  (all live auth hosts, 2026-11-07) the client MUST fall back to plain QR1, matching the reference
  client. `ZALO_QR_TRUSTED=1|0` is the only sanctioned override, for protocol experiments.

---

## 5. Deployment Status (2026-11-07)

The module is complete and offline-verified (KAT + integration tests), but **inactive in production**:
Zalo's servers report `qr_format_trust_device = 0`, so the companion-linking QR (and the mobile
biometric authorization it triggers) cannot be exercised end to end. Evidence and the probe matrix:
`re-notes/10_trusted_device_and_bio_sync.md` §1.2.1.

