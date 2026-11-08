//! Error type for the trusted-device protocol.
//!
//! Numeric codes and message formats are part of the observable contract of the
//! reference client (see `re-notes/11_trusted_device_protocol.md`), so callers
//! may match on [`ErrorCode`] exactly as the reference JavaScript layer does.

use std::fmt;

/// Stable numeric error codes of the trusted-device protocol.
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ErrorCode {
    /// Decimal integer text could not be parsed.
    ParseIntError = 7,
    /// A timestamp computation overflowed `u64`.
    TimeOverflow = 8,
    /// Invariant violation inside the library.
    InternalError = 10,
    /// A key or secret has the wrong length.
    InvalidKeySize = 101,
    /// A serialized public key carries an unknown type byte.
    InvalidKeyType = 102,
    /// Unknown cryptographic algorithm identifier.
    UnknownAlgorithm = 103,
    /// QR token is not valid UTF-8.
    InvalidQrUtf8Token = 104,
    /// QR text has an unexpected prefix.
    InvalidQrPrefix = 105,
    /// QR text is malformed.
    InvalidQrFormat = 106,
    /// QR text carries an unsupported version.
    UnsupportedQrVersion = 107,
    /// Protobuf, base64 or enum payload could not be decoded.
    InvalidDataEncoding = 109,
    /// The linking MAC does not match.
    InvalidLinkingMac = 111,
    /// The primary device's account signature does not verify.
    InvalidAccountSignature = 112,
    /// A companion's device signature does not verify.
    InvalidDeviceSignature = 113,
    /// Device status value is outside the known range.
    InvalidDeviceStatus = 114,
    /// Device type (platform) value is outside the known range.
    InvalidDeviceType = 116,
    /// The structured device-list input is malformed.
    InvalidInputDeviceListStruct = 117,
}

impl ErrorCode {
    /// Numeric value as seen by the reference client.
    pub fn as_u32(self) -> u32 {
        self as u32
    }
}

/// Failure raised by a trusted-device operation.
///
/// The message never contains key material; it carries only lengths, type
/// bytes and field names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrustError {
    code: ErrorCode,
    message: String,
}

impl TrustError {
    pub(super) fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }

    /// Machine-readable error code.
    pub fn code(&self) -> ErrorCode {
        self.code
    }

    /// Human-readable description (English, matches the reference messages).
    pub fn message(&self) -> &str {
        &self.message
    }

    pub(super) fn no_key_type() -> Self {
        Self::new(ErrorCode::InvalidKeyType, "no key type identifier")
    }

    pub(super) fn bad_key_type(byte: u8) -> Self {
        Self::new(
            ErrorCode::InvalidKeyType,
            format!("bad key type <{byte:#04x}>"),
        )
    }

    pub(super) fn bad_key_length(len: usize) -> Self {
        Self::new(
            ErrorCode::InvalidKeySize,
            format!("bad key length <{len}> for key with type <Djb>"),
        )
    }

    pub(super) fn bad_secret_size(name: &str, len: usize) -> Self {
        Self::new(
            ErrorCode::InvalidKeySize,
            format!("invalid key size: {name} with size <{len}>"),
        )
    }

    pub(super) fn data_encoding(field: &str) -> Self {
        Self::new(
            ErrorCode::InvalidDataEncoding,
            format!("invalid <{field}> data encoding"),
        )
    }

    pub(super) fn base64(field: &str, err: &base64::DecodeError) -> Self {
        Self::new(
            ErrorCode::InvalidDataEncoding,
            format!("invalid base64 encoding for <{field}>, error: {err}"),
        )
    }

    pub(super) fn parse_int(field: &str, err: &std::num::ParseIntError) -> Self {
        Self::new(
            ErrorCode::ParseIntError,
            format!("cannot parse <{field}> into integer, error: {err}"),
        )
    }

    pub(super) fn time_overflow() -> Self {
        Self::new(
            ErrorCode::TimeOverflow,
            "failed to calculate time value due to numeric overflow",
        )
    }

    pub(super) fn device_type(value: i64) -> Self {
        Self::new(
            ErrorCode::InvalidDeviceType,
            format!("invalid device type <{value}>"),
        )
    }

    pub(super) fn device_status(value: i64) -> Self {
        Self::new(
            ErrorCode::InvalidDeviceStatus,
            format!("invalid device status <{value}>"),
        )
    }

    pub(super) fn linking_mac() -> Self {
        Self::new(ErrorCode::InvalidLinkingMac, "invalid linking MAC")
    }

    pub(super) fn account_signature() -> Self {
        Self::new(
            ErrorCode::InvalidAccountSignature,
            "invalid account signature",
        )
    }

    pub(super) fn device_list_struct() -> Self {
        Self::new(
            ErrorCode::InvalidInputDeviceListStruct,
            "invalid jsDeviceList struct",
        )
    }
}

impl fmt::Display for TrustError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for TrustError {}

/// Result alias used across the trusted-device module.
pub type Result<T, E = TrustError> = std::result::Result<T, E>;
