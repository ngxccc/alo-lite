//! Trusted device list: signature verification, per-device trust and expiry.

use std::collections::BTreeMap;

use serde::Deserialize;

use super::error::{Result, TrustError};
use super::keys::{PublicKey, SIGNATURE_LEN};
use super::wire::{CompanionDevice, DeviceList, LinkingMetadata};
use super::Platform;

/// How long a device list stays valid after the primary issued it (21 days).
pub const DEVICE_LIST_TTL_MS: u64 = 21 * 24 * 60 * 60 * 1000;

/// Domain tag of the primary's signature over the device list.
const LIST_SIGNATURE_TAG: &[u8] = b"L3";
/// Domain tag of a companion's signature over its own linking metadata.
const DEVICE_SIGNATURE_TAG: &[u8] = b"D2";

/// Lifecycle state of a companion as numbered by the reference client.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceStatus {
    /// Trusted only until its own expiration timestamp.
    Stale = 0,
    /// Trusted for as long as the device list itself is valid.
    Active = 1,
    /// Explicitly revoked (never appears on the wire).
    Deactivated = 2,
}

/// Why a device list was rejected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceListError {
    /// The primary's signature over the list does not verify.
    InvalidDeviceListSignature = 1,
    /// The list is older than [`DEVICE_LIST_TTL_MS`].
    DeviceListHasExpired = 2,
    /// The list was issued by a different primary than expected.
    WrongPrimaryIdentityKey = 3,
    /// The inspected companion's own signature does not verify.
    InvalidCompanionDeviceSignature = 4,
    /// The inspected companion is stale and past its expiration.
    StaleCompanionDeviceHasExpired = 5,
    /// The inspected companion is not in the list.
    CompanionDeviceDoesNotExist = 6,
}

/// Why a single companion identity was rejected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdentityError {
    /// The companion is not in the list.
    CompanionDeviceDoesNotExist = 1,
    /// The companion is stale and past its expiration.
    StaleDeviceHasExpired = 2,
    /// The companion's own signature does not verify.
    InvalidDeviceSignature = 3,
}

/// One companion entry of a device list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompanionDeviceInfo {
    /// Companion slot number.
    pub companion_id: u32,
    /// Link identifier.
    pub link_id: u64,
    /// Companion's 33-byte public identity key.
    pub public_identity_key: Vec<u8>,
    /// Companion platform.
    pub platform: Platform,
    /// Companion status.
    pub status: DeviceStatus,
    /// Expiration (ms); meaningful for [`DeviceStatus::Stale`].
    pub expiration: u64,
    /// Primary's signature binding the companion to the account.
    pub account_signature: Vec<u8>,
    /// Companion's own signature.
    pub device_signature: Vec<u8>,
}

/// Decoded device list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrustedDeviceList {
    /// Account identifier of the primary.
    pub master_id: String,
    /// Primary's 33-byte public identity key.
    pub primary_public_identity_key: Vec<u8>,
    /// Trusted companions.
    pub data: Vec<CompanionDeviceInfo>,
    /// Issue time (ms).
    pub timestamp: u64,
    /// Raw signature bytes as received.
    pub signature: Vec<u8>,
}

/// Outcome of [`verify_device_list_from_bytes`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceListVerification {
    /// Decoded list, returned even when verification fails.
    pub device_list: TrustedDeviceList,
    /// `true` when `error` is `None`.
    pub is_trusted: bool,
    /// First failed check, if any.
    pub error: Option<DeviceListError>,
}

/// Outcome of [`verify_device_list_from_struct`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceStructVerification {
    /// Protobuf serialization of the supplied struct (store this).
    pub device_list_bytes: Vec<u8>,
    /// `true` when `error` is `None`.
    pub is_trusted: bool,
    /// First failed check, if any.
    pub error: Option<DeviceListError>,
}

/// Outcome of [`verify_trusted_identity`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdentityVerification {
    /// Companion key, normalised to 33 bytes.
    pub companion_identity_key: Vec<u8>,
    /// `true` when `error` is `None`.
    pub is_trusted: bool,
    /// First failed check, if any.
    pub error: Option<IdentityError>,
}

/// Outcome of [`check_expiry_time`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckedExpiration {
    /// `true` once `now` reached the expiry (or the companion is unknown).
    pub is_expired: bool,
    /// Milliseconds left; negative once past, `0` when unknown or out of range.
    pub remaining_time: i64,
    /// Status of the companion, `None` when it is not in the list.
    pub companion_device_status: Option<DeviceStatus>,
}

/// One companion in the JSON-shaped input accepted by
/// [`verify_device_list_from_struct`] (field names match the reference client).
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JsCompanionDevice {
    /// Companion slot number.
    pub companion_id: u32,
    /// Decimal `u64` link identifier (a string because JSON numbers are lossy).
    pub link_id: String,
    /// Platform: `1` web, `2` desktop.
    pub platform: i32,
    /// Status: `0` stale, `1` active.
    pub status: i32,
    /// Expiration (ms).
    pub expiration: u64,
    /// Standard padded base64 of the 33-byte public key.
    pub public_identity_key: String,
    /// Base64 account signature.
    pub account_signature: String,
    /// Base64 device signature.
    pub device_signature: String,
}

/// JSON-shaped device list accepted by [`verify_device_list_from_struct`].
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JsDeviceList {
    /// Account identifier of the primary.
    pub master_id: String,
    /// Standard padded base64 of the primary's 33-byte public key.
    pub primary_public_identity_key: String,
    /// Issue time (ms).
    pub timestamp: u64,
    /// Base64 list signature.
    pub signature: String,
    /// Companions.
    pub data: Vec<JsCompanionDevice>,
}

impl JsDeviceList {
    /// Parse the JSON form.
    ///
    /// # Errors
    /// `InvalidInputDeviceListStruct` for any shape or type mismatch.
    pub fn from_json(json: &str) -> Result<Self> {
        serde_json::from_str(json).map_err(|_| TrustError::device_list_struct())
    }

    /// Convert to the wire message.
    ///
    /// Error precedence mirrors the reference: primary key (decode, then parse),
    /// signature, then per device `linkId`, platform/status range, public key
    /// (decode, then parse), account signature, device signature, and finally,
    /// as a second pass over all devices, the platform/status enum mapping. Devices are keyed by `companionId`:
    /// a later entry replaces an earlier one and the output is sorted by id.
    fn to_wire(&self) -> Result<DeviceList> {
        let decode = |field: &str, text: &str| {
            use base64::Engine;
            base64::engine::general_purpose::STANDARD
                .decode(text)
                .map_err(|e| TrustError::base64(field, &e))
        };
        let identity_key = decode(
            "primaryPublicIdentityKey",
            &self.primary_public_identity_key,
        )?;
        PublicKey::parse(&identity_key)?;
        let signature = decode("signature", &self.signature)?;

        struct Converted {
            companion_id: u32,
            link_id: u64,
            platform: i32,
            status: i32,
            ik_public: Vec<u8>,
            account_sig: Vec<u8>,
            device_sig: Vec<u8>,
            expiration: u64,
        }
        let mut converted = Vec::with_capacity(self.data.len());
        for device in &self.data {
            let link_id = device
                .link_id
                .parse::<u64>()
                .map_err(|e| TrustError::parse_int("linkId", &e))?;
            if !(0..=2).contains(&device.platform) {
                return Err(TrustError::device_type(i64::from(device.platform)));
            }
            if !(0..=2).contains(&device.status) {
                return Err(TrustError::device_status(i64::from(device.status)));
            }
            let ik_public = decode("publicIdentityKey", &device.public_identity_key)?;
            PublicKey::parse(&ik_public)?;
            converted.push(Converted {
                companion_id: device.companion_id,
                link_id,
                platform: device.platform,
                status: device.status,
                ik_public,
                account_sig: decode("accountSignature", &device.account_signature)?,
                device_sig: decode("deviceSignature", &device.device_signature)?,
                expiration: device.expiration,
            });
        }
        // Enum mapping is a second pass over all devices in the reference.
        let mut companions = BTreeMap::new();
        for device in converted {
            if device.platform == 0 {
                return Err(TrustError::data_encoding("companion_device.platform"));
            }
            if device.status == 2 {
                return Err(TrustError::data_encoding("companion_device.status"));
            }
            companions.insert(
                device.companion_id,
                CompanionDevice {
                    companion_id: device.companion_id,
                    ik_public: device.ik_public,
                    link_id: device.link_id,
                    device_type: device.platform - 1,
                    device_status: device.status,
                    account_sig: device.account_sig,
                    device_sig: device.device_sig,
                    expiration: device.expiration,
                },
            );
        }
        Ok(DeviceList {
            master_id: self.master_id.clone(),
            identity_key,
            companions: Some(companions.into_values().collect()),
            timestamp: self.timestamp,
            signature,
        })
    }
}

struct ParsedDevice {
    info: CompanionDeviceInfo,
    key: PublicKey,
    wire: CompanionDevice,
}

struct ParsedList {
    list: TrustedDeviceList,
    identity: PublicKey,
    devices: Vec<ParsedDevice>,
    wire: DeviceList,
}

impl ParsedList {
    fn find(&self, key: &PublicKey) -> Option<&ParsedDevice> {
        self.devices.iter().find(|d| d.key == *key)
    }

    /// Bytes the primary signed: `"L3" || master_id || identity || LE64(ts) || companions`.
    fn list_message(&self) -> Vec<u8> {
        let mut message = LIST_SIGNATURE_TAG.to_vec();
        message.extend_from_slice(self.wire.master_id.as_bytes());
        message.extend_from_slice(&self.identity.serialize());
        message.extend_from_slice(&self.wire.timestamp.to_le_bytes());
        let companions = self.wire.companions.as_deref().unwrap_or_default();
        message.extend_from_slice(&DeviceList::encode_companions(companions));
        message
    }

    fn device_signature_ok(&self, device: &ParsedDevice) -> bool {
        let identity = self.identity.serialize();
        let key = device.key.serialize();
        let metadata = LinkingMetadata {
            master_id: self.wire.master_id.clone(),
            primary_ik_public: identity.to_vec(),
            companion_ik_public: key.to_vec(),
            companion_id: device.wire.companion_id,
            link_id: device.wire.link_id,
            platform: device.wire.device_type,
        };
        let mut message = DEVICE_SIGNATURE_TAG.to_vec();
        message.extend_from_slice(&metadata.encode(true));
        message.extend_from_slice(&key);
        message.extend_from_slice(&identity);
        device.key.verify(&message, &device.wire.device_sig)
    }
}

fn parse_list(bytes: &[u8]) -> Result<ParsedList> {
    let mut wire =
        DeviceList::decode(bytes).map_err(|_| TrustError::data_encoding("device_list"))?;
    let received = wire
        .companions
        .take()
        .ok_or_else(|| TrustError::data_encoding("device_list.data"))?;
    // Keyed by companion id like the reference: last entry wins, ascending order.
    let companions: Vec<CompanionDevice> = received
        .into_iter()
        .map(|d| (d.companion_id, d))
        .collect::<BTreeMap<_, _>>()
        .into_values()
        .collect();
    let identity = PublicKey::parse(&wire.identity_key)?;

    let mut devices = Vec::with_capacity(companions.len());
    for device in &companions {
        let platform = Platform::from_companion_wire(device.device_type)
            .ok_or_else(|| TrustError::data_encoding("companion_device.device_type"))?;
        let status = match device.device_status {
            0 => DeviceStatus::Stale,
            1 => DeviceStatus::Active,
            _ => return Err(TrustError::data_encoding("companion_device.device_status")),
        };
        let key = PublicKey::parse(&device.ik_public)?;
        devices.push(ParsedDevice {
            info: CompanionDeviceInfo {
                companion_id: device.companion_id,
                link_id: device.link_id,
                public_identity_key: key.serialize().to_vec(),
                platform,
                status,
                expiration: device.expiration,
                account_signature: device.account_sig.clone(),
                device_signature: device.device_sig.clone(),
            },
            key,
            wire: device.clone(),
        });
    }
    wire.companions = Some(companions);
    let list = TrustedDeviceList {
        master_id: wire.master_id.clone(),
        primary_public_identity_key: identity.serialize().to_vec(),
        data: devices.iter().map(|d| d.info.clone()).collect(),
        timestamp: wire.timestamp,
        signature: wire.signature.clone(),
    };
    Ok(ParsedList {
        list,
        identity,
        devices,
        wire,
    })
}

fn list_expiry(list: &ParsedList) -> Result<u64> {
    list.wire
        .timestamp
        .checked_add(DEVICE_LIST_TTL_MS)
        .ok_or_else(TrustError::time_overflow)
}

fn evaluate_list(
    parsed: &ParsedList,
    primary: &PublicKey,
    verify_at: u64,
    companion: Option<&PublicKey>,
) -> Result<Option<DeviceListError>> {
    if parsed.identity != *primary {
        return Ok(Some(DeviceListError::WrongPrimaryIdentityKey));
    }
    if parsed.wire.signature.len() != SIGNATURE_LEN
        || !parsed
            .identity
            .verify(&parsed.list_message(), &parsed.wire.signature)
    {
        return Ok(Some(DeviceListError::InvalidDeviceListSignature));
    }
    let device = match companion {
        Some(key) => {
            let Some(device) = parsed.find(key) else {
                return Ok(Some(DeviceListError::CompanionDeviceDoesNotExist));
            };
            if !parsed.device_signature_ok(device) {
                return Ok(Some(DeviceListError::InvalidCompanionDeviceSignature));
            }
            Some(device)
        }
        None => None,
    };
    if let Some(device) = device {
        if device.info.status == DeviceStatus::Stale && verify_at > device.info.expiration {
            return Ok(Some(DeviceListError::StaleCompanionDeviceHasExpired));
        }
    }
    if verify_at > list_expiry(parsed)? {
        return Ok(Some(DeviceListError::DeviceListHasExpired));
    }
    Ok(None)
}

/// Verify a serialized device list issued by the primary.
///
/// With `companion_ik`, additionally require that companion to be present with a
/// valid own signature and not stale-expired.
///
/// # Errors
/// Key and decoding failures. Verification failures are reported through
/// [`DeviceListVerification::error`] instead, and the decoded list is still returned.
///
/// # Invariants
/// Check order: wrong primary, list signature, companion lookup, companion
/// signature, stale expiry of that companion, list age (`verify_at > timestamp + 21 days`).
pub fn verify_device_list_from_bytes(
    device_list: &[u8],
    primary_ik: &[u8],
    verify_at: u64,
    companion_ik: Option<&[u8]>,
) -> Result<DeviceListVerification> {
    let primary = PublicKey::parse(primary_ik)?;
    let parsed = parse_list(device_list)?;
    let companion = companion_ik.map(PublicKey::parse).transpose()?;
    let error = evaluate_list(&parsed, &primary, verify_at, companion.as_ref())?;
    Ok(DeviceListVerification {
        device_list: parsed.list,
        is_trusted: error.is_none(),
        error,
    })
}

/// Serialize and verify a JSON-shaped device list received from the server.
///
/// # Errors
/// Conversion errors (`ParseIntError`, `InvalidDataEncoding`, `InvalidDeviceType`,
/// `InvalidDeviceStatus`) plus everything [`verify_device_list_from_bytes`] returns.
pub fn verify_device_list_from_struct(
    device_list: &JsDeviceList,
    primary_ik: &[u8],
    verify_at: u64,
    companion_ik: Option<&[u8]>,
) -> Result<DeviceStructVerification> {
    let bytes = device_list.to_wire()?.encode();
    let verification = verify_device_list_from_bytes(&bytes, primary_ik, verify_at, companion_ik)?;
    Ok(DeviceStructVerification {
        device_list_bytes: bytes,
        is_trusted: verification.is_trusted,
        error: verification.error,
    })
}

/// Check one companion against a stored device list without re-verifying the
/// list signature.
///
/// # Errors
/// Decoding and key failures.
///
/// # Invariants
/// Check order: lookup, companion signature, stale expiry. The list age is not
/// examined here; use [`check_expiry_time`] for that.
pub fn verify_trusted_identity(
    device_list: &[u8],
    companion_ik: &[u8],
    verify_at: u64,
) -> Result<IdentityVerification> {
    let parsed = parse_list(device_list)?;
    let companion = PublicKey::parse(companion_ik)?;
    let error = match parsed.find(&companion) {
        None => Some(IdentityError::CompanionDeviceDoesNotExist),
        Some(device) if !parsed.device_signature_ok(device) => {
            Some(IdentityError::InvalidDeviceSignature)
        }
        Some(device)
            if device.info.status == DeviceStatus::Stale && verify_at > device.info.expiration =>
        {
            Some(IdentityError::StaleDeviceHasExpired)
        }
        Some(_) => None,
    };
    Ok(IdentityVerification {
        companion_identity_key: companion.serialize().to_vec(),
        is_trusted: error.is_none(),
        error,
    })
}

/// Compute how long a companion stays trusted.
///
/// Active companions expire with the list (`timestamp + 21 days`); stale ones at
/// their own `expiration`. An unknown companion counts as already expired.
///
/// # Errors
/// Key and decoding failures, and `TimeOverflow` when `timestamp + 21 days`
/// exceeds `u64`.
pub fn check_expiry_time(
    device_list: &[u8],
    companion_ik: &[u8],
    now: u64,
) -> Result<CheckedExpiration> {
    let companion = PublicKey::parse(companion_ik)?;
    let parsed = parse_list(device_list)?;
    let Some(device) = parsed.find(&companion) else {
        return Ok(CheckedExpiration {
            is_expired: true,
            remaining_time: 0,
            companion_device_status: None,
        });
    };
    let expiry = match device.info.status {
        DeviceStatus::Active => list_expiry(&parsed)?,
        _ => device.info.expiration,
    };
    let remaining = i128::from(expiry) - i128::from(now);
    Ok(CheckedExpiration {
        is_expired: now >= expiry,
        remaining_time: i64::try_from(remaining).unwrap_or(0),
        companion_device_status: Some(device.info.status),
    })
}
