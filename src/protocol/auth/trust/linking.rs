//! Linking flow: identity keys, QR payload, relink id and companion proof.

use hmac::{Hmac, Mac};
use sha2::Sha256;
use zeroize::{Zeroize, ZeroizeOnDrop};

use super::base91;
use super::error::{ErrorCode, Result, TrustError};
use super::keys::{fill_random, PrivateKey, PublicKey, PUBLIC_KEY_LEN};
use super::wire::{LinkingData, LinkingQr};
use super::Platform;

type HmacSha256 = Hmac<Sha256>;

/// Length of the shared linking secret.
const LINKING_SECRET_LEN: usize = 32;
/// Prefix of every linking QR text (marker, format version, separator).
const QR_PREFIX: &str = "Z1@";
/// Domain tag of the primary device's signature over the linking metadata.
const ACCOUNT_SIGNATURE_TAG: &[u8] = b"A1";
/// Domain tag of the companion's own signature over the linking metadata.
const DEVICE_SIGNATURE_TAG: &[u8] = b"D2";
const SECRET_NAME: &str = "linking_secret_key";

/// Curve25519 identity key pair of a companion device.
#[derive(Clone, Zeroize, ZeroizeOnDrop)]
pub struct IdentityKeyPair {
    public: [u8; PUBLIC_KEY_LEN],
    private: [u8; 32],
}

impl IdentityKeyPair {
    /// Serialized public key: type byte `0x05` followed by 32 bytes.
    pub fn public_key(&self) -> &[u8; PUBLIC_KEY_LEN] {
        &self.public
    }

    /// Clamped 32-byte private key.
    pub fn private_key(&self) -> &[u8; 32] {
        &self.private
    }
    /// Construct from pre-existing raw key bytes (e.g. loaded from storage).
    pub fn from_raw(public: [u8; PUBLIC_KEY_LEN], private: [u8; 32]) -> Self {
        Self { public, private }
    }
}

impl std::fmt::Debug for IdentityKeyPair {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IdentityKeyPair")
            .field("public", &self.public)
            .field("private", &"<redacted>")
            .finish()
    }
}

/// 32-byte random secret shared between a companion and the primary device
/// through the QR code; it authenticates the primary's reply.
#[derive(Clone, Zeroize, ZeroizeOnDrop)]
pub struct LinkingSecretKey([u8; LINKING_SECRET_LEN]);

impl LinkingSecretKey {
    /// Raw secret bytes.
    pub fn as_bytes(&self) -> &[u8; LINKING_SECRET_LEN] {
        &self.0
    }
    /// Construct from pre-existing raw secret bytes (e.g. for testing or relink).
    pub fn from_bytes(raw: [u8; LINKING_SECRET_LEN]) -> Self {
        Self(raw)
    }

    /// Length in bytes (always 32).
    pub fn key_size(&self) -> usize {
        LINKING_SECRET_LEN
    }
}

impl std::fmt::Debug for LinkingSecretKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("LinkingSecretKey(<redacted>)")
    }
}

/// Generate a companion identity key pair.
///
/// # Errors
/// Fails only if the OS random source is unavailable.
pub fn generate_identity_key_pair() -> Result<IdentityKeyPair> {
    let private = PrivateKey::generate()?;
    Ok(IdentityKeyPair {
        public: private.public_key().serialize(),
        private: private.to_bytes(),
    })
}

/// Generate a fresh linking secret.
///
/// # Errors
/// Fails only if the OS random source is unavailable.
pub fn generate_linking_secret_key() -> Result<LinkingSecretKey> {
    let mut raw = [0u8; LINKING_SECRET_LEN];
    fill_random(&mut raw)?;
    Ok(LinkingSecretKey(raw))
}

fn check_secret(secret: &[u8]) -> Result<()> {
    if secret.len() == LINKING_SECRET_LEN {
        Ok(())
    } else {
        Err(TrustError::bad_secret_size(SECRET_NAME, secret.len()))
    }
}

/// Build the text of the linking QR code.
///
/// `linking_secret` is sent on first link; on a relink pass `None` together with
/// the `relink_id` from [`get_relink_id`].
///
/// # Errors
/// - [`ErrorCode::InvalidKeyType`](super::ErrorCode) / `InvalidKeySize` for a bad `companion_ik`
///   or a secret that is not 32 bytes.
/// - `InvalidDeviceType` when `platform` is [`Platform::Mobile`].
///
/// # Invariants
/// Check order: key, secret, platform.
pub fn generate_linking_qr(
    companion_ik: &[u8],
    linking_secret: Option<&[u8]>,
    token: &str,
    timestamp: u64,
    platform: Platform,
    relink_id: Option<&[u8]>,
) -> Result<String> {
    let companion = PublicKey::parse(companion_ik)?;
    if let Some(secret) = linking_secret {
        check_secret(secret)?;
    }
    let platform = platform.companion_wire()?;
    let companion = companion.serialize();
    let payload = LinkingQr {
        token,
        companion_ik: &companion,
        linking_secret,
        timestamp,
        platform,
        relink_id,
    }
    .encode();
    Ok(format!("{QR_PREFIX}{}", base91::encode(&payload)))
}

/// Derive the identifier a companion presents when relinking to the same
/// primary device: `HMAC-SHA256(secret, LE64(ts) || companion_ik || primary_ik)`.
///
/// # Errors
/// Key errors for `primary_ik` (checked first) or `companion_ik`, then
/// `InvalidKeySize` for a secret that is not 32 bytes.
pub fn get_relink_id(
    primary_ik: &[u8],
    companion_ik: &[u8],
    linking_secret: &[u8],
    linking_timestamp: u64,
) -> Result<[u8; 32]> {
    let primary = PublicKey::parse(primary_ik)?;
    let companion = PublicKey::parse(companion_ik)?;
    check_secret(linking_secret)?;
    let mut mac = new_mac(linking_secret)?;
    mac.update(&linking_timestamp.to_le_bytes());
    mac.update(&companion.serialize());
    mac.update(&primary.serialize());
    Ok(mac.finalize().into_bytes().into())
}

fn new_mac(key: &[u8]) -> Result<HmacSha256> {
    HmacSha256::new_from_slice(key).map_err(|_| TrustError::bad_secret_size(SECRET_NAME, key.len()))
}

/// Result of accepting the primary device's linking reply.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompanionLinkingProof {
    /// Link identifier assigned by the primary.
    pub link_id: u64,
    /// Account identifier of the primary.
    pub master_id: String,
    /// Companion slot number assigned by the primary.
    pub companion_id: u32,
    /// Timestamp (ms) the primary stamped on the link.
    pub linking_timestamp: u64,
    /// Primary's 33-byte public identity key.
    pub primary_public_identity_key: Vec<u8>,
    /// Canonical metadata bytes, including this companion's platform.
    pub linking_metadata: Vec<u8>,
    /// Primary's account signature, passed through verified.
    pub account_signature: Vec<u8>,
    /// This companion's signature over the metadata.
    pub device_signature: Vec<u8>,
}

/// Validate the primary's linking reply and counter-sign it.
///
/// `linking_data` is the serialized reply and `linking_hmac` its MAC, both as
/// delivered by the server (already decoded from their transport encoding).
///
/// Verification order: key, private key, secret, platform; protobuf decode;
/// MAC; primary key; account signature.
///
/// # Errors
/// Any [`TrustError`]; notably `InvalidLinkingMac` and `InvalidAccountSignature`.
///
/// # Invariants
/// The MAC is compared in constant time and covers `linking_data` followed by
/// the primary key bytes declared inside the metadata.
pub fn create_companion_linking_proof(
    companion_ik: &[u8],
    companion_private_key: &[u8],
    linking_secret: &[u8],
    linking_data: &[u8],
    linking_hmac: &[u8],
    platform: Platform,
) -> Result<CompanionLinkingProof> {
    let companion_public = PublicKey::parse(companion_ik)?;
    let companion_private = PrivateKey::parse(companion_private_key)?;
    check_secret(linking_secret)?;
    let wire_platform = platform.companion_wire()?;

    let data = LinkingData::decode(linking_data)
        .map_err(|_| TrustError::data_encoding("primary_linking_data"))?;
    let mut metadata = data
        .metadata
        .ok_or_else(|| TrustError::data_encoding("primary_linking_data.metadata"))?;

    let mut mac = new_mac(linking_secret)?;
    mac.update(linking_data);
    mac.update(&metadata.primary_ik_public);
    // `verify_slice` compares in constant time.
    mac.verify_slice(linking_hmac)
        .map_err(|_| TrustError::linking_mac())?;

    // The reference reports a short primary key with a different message than
    // the other key parsers; keep it so error text stays comparable.
    let primary = PublicKey::parse(&metadata.primary_ik_public).map_err(|e| {
        if e.code() == ErrorCode::InvalidKeySize {
            TrustError::bad_secret_size("ed25519", metadata.primary_ik_public.len())
        } else {
            e
        }
    })?;
    let companion_serialized = companion_public.serialize();

    let mut account_message = ACCOUNT_SIGNATURE_TAG.to_vec();
    account_message.extend_from_slice(&metadata.encode(false));
    account_message.extend_from_slice(&companion_serialized);
    if !primary.verify(&account_message, &data.account_signature) {
        return Err(TrustError::account_signature());
    }

    metadata.platform = wire_platform;
    let canonical = metadata.encode(true);
    let mut device_message = DEVICE_SIGNATURE_TAG.to_vec();
    device_message.extend_from_slice(&canonical);
    device_message.extend_from_slice(&companion_serialized);
    device_message.extend_from_slice(&primary.serialize());
    let device_signature = companion_private.sign(&device_message)?;

    Ok(CompanionLinkingProof {
        link_id: metadata.link_id,
        master_id: metadata.master_id,
        companion_id: metadata.companion_id,
        linking_timestamp: data.linking_timestamp,
        primary_public_identity_key: primary.serialize().to_vec(),
        linking_metadata: canonical,
        account_signature: data.account_signature,
        device_signature: device_signature.to_vec(),
    })
}
