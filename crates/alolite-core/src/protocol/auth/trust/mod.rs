//! Trusted companion-device protocol.
//!
//! Lets a desktop client link to a primary phone through a QR code, prove the
//! link with a primary-signed proof, and later check that a companion is still
//! trusted according to a primary-signed device list.
//!
//! Everything here is pure computation (no I/O); callers supply the clock.
//! Wire formats, message layouts and verification order are documented in
//! `re-notes/11_trusted_device_protocol.md`.
//!
//! Timestamps are Unix milliseconds. All 33-byte public keys use the Curve25519
//! type byte `0x05`.

mod base91;
mod device_list;
mod error;
mod keys;
mod linking;
mod wire;

pub use device_list::{
    check_expiry_time, verify_device_list_from_bytes, verify_device_list_from_struct,
    verify_trusted_identity, CheckedExpiration, CompanionDeviceInfo, DeviceListError,
    DeviceListVerification, DeviceStatus, DeviceStructVerification, IdentityError,
    IdentityVerification, JsCompanionDevice, JsDeviceList, TrustedDeviceList, DEVICE_LIST_TTL_MS,
};
pub use error::{ErrorCode, Result, TrustError};
pub use linking::{
    create_companion_linking_proof, generate_identity_key_pair, generate_linking_qr,
    generate_linking_secret_key, get_relink_id, CompanionLinkingProof, IdentityKeyPair,
    LinkingSecretKey,
};

/// Device platform as numbered by the reference client.
///
/// Only [`Platform::Web`] and [`Platform::Pc`] can be companions; the wire
/// format numbers them `0` and `1` respectively.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Platform {
    /// Phone (primary device; never valid as a companion).
    Mobile = 0,
    /// Browser companion.
    Web = 1,
    /// Desktop companion.
    Pc = 2,
}

impl Platform {
    /// Wire value of a companion platform.
    ///
    /// # Errors
    /// [`ErrorCode::InvalidDeviceType`] for [`Platform::Mobile`].
    fn companion_wire(self) -> Result<i32> {
        match self {
            Platform::Mobile => Err(TrustError::device_type(0)),
            Platform::Web => Ok(0),
            Platform::Pc => Ok(1),
        }
    }

    fn from_companion_wire(wire: i32) -> Option<Self> {
        match wire {
            0 => Some(Platform::Web),
            1 => Some(Platform::Pc),
            _ => None,
        }
    }
}
