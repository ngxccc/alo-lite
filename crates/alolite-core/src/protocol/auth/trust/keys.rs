//! Curve25519 identity keys and the signature scheme used by trusted devices.
//!
//! Public keys travel as 33 bytes: type byte `0x05` followed by the X25519
//! u-coordinate. Signatures are 64 bytes (`R || s`) over the Edwards image of
//! that key; the sign bit of the Edwards point is parked in the top bit of the
//! last signature byte, so verification needs only the Montgomery key.

use curve25519_dalek::constants::{ED25519_BASEPOINT_TABLE, X25519_BASEPOINT};
use curve25519_dalek::montgomery::MontgomeryPoint;
use curve25519_dalek::scalar::{clamp_integer, Scalar};
use sha2::{Digest, Sha512};
use subtle::ConstantTimeEq;
use zeroize::{Zeroize, ZeroizeOnDrop};

use super::error::{Result, TrustError};

/// Type byte of a serialized Curve25519 public key.
const KEY_TYPE_DJB: u8 = 0x05;
/// Length of a serialized public key (type byte + 32).
pub(super) const PUBLIC_KEY_LEN: usize = 33;
/// Length of a private key.
pub(super) const PRIVATE_KEY_LEN: usize = 32;
/// Length of a signature.
pub(super) const SIGNATURE_LEN: usize = 64;

/// A validated public identity key.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) struct PublicKey([u8; 32]);

impl PublicKey {
    /// Parse a serialized key. Bytes after the 33rd are ignored.
    pub(super) fn parse(bytes: &[u8]) -> Result<Self> {
        let Some(&kind) = bytes.first() else {
            return Err(TrustError::no_key_type());
        };
        if kind != KEY_TYPE_DJB {
            return Err(TrustError::bad_key_type(kind));
        }
        if bytes.len() < PUBLIC_KEY_LEN {
            return Err(TrustError::bad_key_length(bytes.len()));
        }
        let mut raw = [0u8; 32];
        raw.copy_from_slice(&bytes[1..PUBLIC_KEY_LEN]);
        Ok(Self(raw))
    }

    /// Canonical 33-byte serialization.
    pub(super) fn serialize(&self) -> [u8; PUBLIC_KEY_LEN] {
        let mut out = [0u8; PUBLIC_KEY_LEN];
        out[0] = KEY_TYPE_DJB;
        out[1..].copy_from_slice(&self.0);
        out
    }

    /// Verify `signature` over `message`.
    ///
    /// Returns `false` for any malformed signature instead of an error, which
    /// matches how the reference treats it.
    pub(super) fn verify(&self, message: &[u8], signature: &[u8]) -> bool {
        let Ok(signature) = <&[u8; SIGNATURE_LEN]>::try_from(signature) else {
            return false;
        };
        let sign_bit = (signature[63] & 0x80) >> 7;
        let Some(edwards) = MontgomeryPoint(self.0).to_edwards(sign_bit) else {
            return false;
        };
        let compressed_a = edwards.compress();

        let mut r = [0u8; 32];
        r.copy_from_slice(&signature[..32]);
        let mut s = [0u8; 32];
        s.copy_from_slice(&signature[32..]);
        s[31] &= 0x7f;
        if s[31] & 0xe0 != 0 {
            return false;
        }

        let h = Scalar::from_hash(
            Sha512::new()
                .chain_update(r)
                .chain_update(compressed_a.as_bytes())
                .chain_update(message),
        );
        let check = curve25519_dalek::edwards::EdwardsPoint::vartime_double_scalar_mul_basepoint(
            &h,
            &-edwards,
            &Scalar::from_bytes_mod_order(s),
        )
        .compress();
        bool::from(check.as_bytes().ct_eq(&r))
    }
}

/// A validated private identity key.
#[derive(Zeroize, ZeroizeOnDrop)]
pub(super) struct PrivateKey([u8; 32]);

impl PrivateKey {
    /// Parse a 32-byte private key (no type byte).
    pub(super) fn parse(bytes: &[u8]) -> Result<Self> {
        let raw = <[u8; PRIVATE_KEY_LEN]>::try_from(bytes)
            .map_err(|_| TrustError::bad_key_length(bytes.len()))?;
        Ok(Self(raw))
    }

    /// Generate a fresh clamped private key from the OS CSPRNG.
    pub(super) fn generate() -> Result<Self> {
        let mut raw = [0u8; 32];
        fill_random(&mut raw)?;
        let key = Self(clamp_integer(raw));
        raw.zeroize();
        Ok(key)
    }

    pub(super) fn to_bytes(&self) -> [u8; PRIVATE_KEY_LEN] {
        self.0
    }

    /// Public key matching this private key.
    pub(super) fn public_key(&self) -> PublicKey {
        PublicKey(X25519_BASEPOINT.mul_clamped(self.0).to_bytes())
    }

    /// Sign `message` with a caller-supplied 64-byte nonce seed.
    ///
    /// The seed must be fresh randomness for real signatures; tests pass a
    /// fixed value to get a reproducible output.
    pub(super) fn sign_with_nonce(
        &self,
        message: &[u8],
        nonce_seed: &[u8; 64],
    ) -> [u8; SIGNATURE_LEN] {
        let a = Scalar::from_bytes_mod_order(clamp_integer(self.0));
        let ed_public = (&a * ED25519_BASEPOINT_TABLE).compress();
        let sign_bit = ed_public.as_bytes()[31] & 0x80;

        let mut prefix = [0xffu8; 32];
        prefix[0] = 0xfe;
        let r = Scalar::from_hash(
            Sha512::new()
                .chain_update(prefix)
                .chain_update(self.0)
                .chain_update(message)
                .chain_update(nonce_seed),
        );
        let big_r = (&r * ED25519_BASEPOINT_TABLE).compress();
        let h = Scalar::from_hash(
            Sha512::new()
                .chain_update(big_r.as_bytes())
                .chain_update(ed_public.as_bytes())
                .chain_update(message),
        );
        let s = h * a + r;

        let mut out = [0u8; SIGNATURE_LEN];
        out[..32].copy_from_slice(big_r.as_bytes());
        out[32..].copy_from_slice(s.as_bytes());
        out[63] &= 0x7f;
        out[63] |= sign_bit;
        out
    }

    /// Sign `message` using fresh randomness.
    pub(super) fn sign(&self, message: &[u8]) -> Result<[u8; SIGNATURE_LEN]> {
        let mut seed = [0u8; 64];
        fill_random(&mut seed)?;
        let signature = self.sign_with_nonce(message, &seed);
        seed.zeroize();
        Ok(signature)
    }
}

/// Fill `buf` from the operating-system CSPRNG.
pub(super) fn fill_random(buf: &mut [u8]) -> Result<()> {
    getrandom::fill(buf).map_err(|_| {
        TrustError::new(
            super::error::ErrorCode::InternalError,
            "random source unavailable",
        )
    })
}
