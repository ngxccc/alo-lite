//! Minimal protobuf codec for the trusted-device messages.
//!
//! Behaviour intentionally mirrors `prost` proto3 semantics observed in the
//! reference: unknown fields are skipped, known fields with a mismatched wire
//! type are rejected, scalar fields keep the last value, singular message
//! fields merge, `uint32`/enum varints truncate, and default values are not
//! encoded (an *empty present* sub-message still is).

/// Decoding failure; callers map it to a field-specific `TrustError`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct WireError;

type WireResult<T> = Result<T, WireError>;

/// One decoded field payload.
enum Value<'a> {
    Varint(u64),
    Fixed64(u64),
    Len(&'a [u8]),
    Fixed32,
}

fn read_varint(buf: &mut &[u8]) -> WireResult<u64> {
    let mut value = 0u64;
    for (i, &byte) in buf.iter().enumerate().take(10) {
        if i == 9 && byte > 1 {
            return Err(WireError);
        }
        value |= u64::from(byte & 0x7f) << (7 * i);
        if byte & 0x80 == 0 {
            *buf = &buf[i + 1..];
            return Ok(value);
        }
    }
    Err(WireError)
}

/// Walk every field of `buf`, handing known and unknown fields to `visit`.
fn for_each_field<'a>(
    mut buf: &'a [u8],
    mut visit: impl FnMut(u32, Value<'a>) -> WireResult<()>,
) -> WireResult<()> {
    while !buf.is_empty() {
        let key = read_varint(&mut buf)?;
        if key > u64::from(u32::MAX) {
            return Err(WireError);
        }
        let tag = (key >> 3) as u32;
        if tag == 0 {
            return Err(WireError);
        }
        let value = match key & 7 {
            0 => Value::Varint(read_varint(&mut buf)?),
            1 => {
                if buf.len() < 8 {
                    return Err(WireError);
                }
                let (head, rest) = buf.split_at(8);
                buf = rest;
                Value::Fixed64(u64::from_le_bytes(head.try_into().map_err(|_| WireError)?))
            }
            2 => {
                let len = usize::try_from(read_varint(&mut buf)?).map_err(|_| WireError)?;
                if buf.len() < len {
                    return Err(WireError);
                }
                let (head, rest) = buf.split_at(len);
                buf = rest;
                Value::Len(head)
            }
            5 => {
                buf = buf.get(4..).ok_or(WireError)?;
                Value::Fixed32
            }
            _ => return Err(WireError),
        };
        visit(tag, value)?;
    }
    Ok(())
}

fn utf8(bytes: &[u8]) -> WireResult<String> {
    String::from_utf8(bytes.to_vec()).map_err(|_| WireError)
}

fn put_varint(out: &mut Vec<u8>, mut value: u64) {
    while value >= 0x80 {
        out.push((value as u8) | 0x80);
        value >>= 7;
    }
    out.push(value as u8);
}

fn put_key(out: &mut Vec<u8>, tag: u32, wire: u8) {
    put_varint(out, (u64::from(tag) << 3) | u64::from(wire));
}

fn put_bytes(out: &mut Vec<u8>, tag: u32, bytes: &[u8]) {
    put_key(out, tag, 2);
    put_varint(out, bytes.len() as u64);
    out.extend_from_slice(bytes);
}

/// Encode a non-empty byte string; proto3 omits empty ones.
fn put_bytes_nonempty(out: &mut Vec<u8>, tag: u32, bytes: &[u8]) {
    if !bytes.is_empty() {
        put_bytes(out, tag, bytes);
    }
}

fn put_varint_nonzero(out: &mut Vec<u8>, tag: u32, value: u64) {
    if value != 0 {
        put_key(out, tag, 0);
        put_varint(out, value);
    }
}

fn put_fixed64_nonzero(out: &mut Vec<u8>, tag: u32, value: u64) {
    if value != 0 {
        put_key(out, tag, 1);
        out.extend_from_slice(&value.to_le_bytes());
    }
}

macro_rules! expect {
    ($value:expr, Varint) => {
        match $value {
            Value::Varint(v) => v,
            _ => return Err(WireError),
        }
    };
    ($value:expr, Fixed64) => {
        match $value {
            Value::Fixed64(v) => v,
            _ => return Err(WireError),
        }
    };
    ($value:expr, Len) => {
        match $value {
            Value::Len(v) => v,
            _ => return Err(WireError),
        }
    };
}

/// Companion device platform on the wire (`0` = web, `1` = desktop).
pub(super) type WireDeviceType = i32;
/// Companion device status on the wire (`0` = stale, `1` = active).
pub(super) type WireDeviceStatus = i32;

/// Identity metadata the primary device vouches for.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(super) struct LinkingMetadata {
    pub master_id: String,
    pub primary_ik_public: Vec<u8>,
    pub companion_ik_public: Vec<u8>,
    pub companion_id: u32,
    pub link_id: u64,
    pub platform: WireDeviceType,
}

impl LinkingMetadata {
    fn merge(&mut self, buf: &[u8]) -> WireResult<()> {
        for_each_field(buf, |tag, value| {
            match tag {
                1 => self.master_id = utf8(expect!(value, Len))?,
                2 => self.primary_ik_public = expect!(value, Len).to_vec(),
                3 => self.companion_ik_public = expect!(value, Len).to_vec(),
                4 => self.companion_id = expect!(value, Varint) as u32,
                5 => self.link_id = expect!(value, Varint),
                6 => self.platform = expect!(value, Varint) as i32,
                _ => {}
            }
            Ok(())
        })
    }

    /// Encode, optionally including the platform field.
    pub(super) fn encode(&self, with_platform: bool) -> Vec<u8> {
        let mut out = Vec::new();
        put_bytes_nonempty(&mut out, 1, self.master_id.as_bytes());
        put_bytes_nonempty(&mut out, 2, &self.primary_ik_public);
        put_bytes_nonempty(&mut out, 3, &self.companion_ik_public);
        put_varint_nonzero(&mut out, 4, u64::from(self.companion_id));
        put_varint_nonzero(&mut out, 5, self.link_id);
        if with_platform {
            put_varint_nonzero(&mut out, 6, self.platform as u32 as u64);
        }
        out
    }
}

/// Payload the primary device sends to a companion after scanning its QR.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(super) struct LinkingData {
    pub metadata: Option<LinkingMetadata>,
    pub account_signature: Vec<u8>,
    pub linking_timestamp: u64,
}

impl LinkingData {
    pub(super) fn decode(buf: &[u8]) -> WireResult<Self> {
        let mut out = Self::default();
        for_each_field(buf, |tag, value| {
            match tag {
                1 => out
                    .metadata
                    .get_or_insert_with(Default::default)
                    .merge(expect!(value, Len))?,
                2 => out.account_signature = expect!(value, Len).to_vec(),
                3 => out.linking_timestamp = expect!(value, Fixed64),
                _ => {}
            }
            Ok(())
        })?;
        Ok(out)
    }
}

/// One trusted companion in a device list.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(super) struct CompanionDevice {
    pub companion_id: u32,
    pub ik_public: Vec<u8>,
    pub link_id: u64,
    pub device_type: WireDeviceType,
    pub device_status: WireDeviceStatus,
    pub account_sig: Vec<u8>,
    pub device_sig: Vec<u8>,
    pub expiration: u64,
}

impl CompanionDevice {
    fn merge(&mut self, buf: &[u8]) -> WireResult<()> {
        for_each_field(buf, |tag, value| {
            match tag {
                1 => self.companion_id = expect!(value, Varint) as u32,
                2 => self.ik_public = expect!(value, Len).to_vec(),
                3 => self.link_id = expect!(value, Varint),
                4 => self.device_type = expect!(value, Varint) as i32,
                5 => self.device_status = expect!(value, Varint) as i32,
                6 => self.account_sig = expect!(value, Len).to_vec(),
                7 => self.device_sig = expect!(value, Len).to_vec(),
                8 => self.expiration = expect!(value, Varint),
                _ => {}
            }
            Ok(())
        })
    }

    fn encode(&self) -> Vec<u8> {
        let mut out = Vec::new();
        put_varint_nonzero(&mut out, 1, u64::from(self.companion_id));
        put_bytes_nonempty(&mut out, 2, &self.ik_public);
        put_varint_nonzero(&mut out, 3, self.link_id);
        put_varint_nonzero(&mut out, 4, self.device_type as u32 as u64);
        put_varint_nonzero(&mut out, 5, self.device_status as u32 as u64);
        put_bytes_nonempty(&mut out, 6, &self.account_sig);
        put_bytes_nonempty(&mut out, 7, &self.device_sig);
        put_varint_nonzero(&mut out, 8, self.expiration);
        out
    }
}

/// Signed list of trusted companions, issued by the primary device.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(super) struct DeviceList {
    pub master_id: String,
    pub identity_key: Vec<u8>,
    /// `None` when the field is absent; `Some(vec![])` when present but empty.
    pub companions: Option<Vec<CompanionDevice>>,
    pub timestamp: u64,
    pub signature: Vec<u8>,
}

impl DeviceList {
    pub(super) fn decode(buf: &[u8]) -> WireResult<Self> {
        let mut out = Self::default();
        for_each_field(buf, |tag, value| {
            match tag {
                1 => out.master_id = utf8(expect!(value, Len))?,
                2 => out.identity_key = expect!(value, Len).to_vec(),
                3 => {
                    let devices = out.companions.get_or_insert_with(Vec::new);
                    for_each_field(expect!(value, Len), |inner_tag, inner| {
                        if inner_tag == 1 {
                            let mut device = CompanionDevice::default();
                            device.merge(expect!(inner, Len))?;
                            devices.push(device);
                        }
                        Ok(())
                    })?;
                }
                4 => out.timestamp = expect!(value, Fixed64),
                5 => out.signature = expect!(value, Len).to_vec(),
                _ => {}
            }
            Ok(())
        })?;
        Ok(out)
    }

    /// Serialized `CompanionData` message (the device array as signed).
    pub(super) fn encode_companions(companions: &[CompanionDevice]) -> Vec<u8> {
        let mut out = Vec::new();
        for device in companions {
            put_bytes(&mut out, 1, &device.encode());
        }
        out
    }

    pub(super) fn encode(&self) -> Vec<u8> {
        let mut out = Vec::new();
        put_bytes_nonempty(&mut out, 1, self.master_id.as_bytes());
        put_bytes_nonempty(&mut out, 2, &self.identity_key);
        if let Some(companions) = &self.companions {
            put_bytes(&mut out, 3, &Self::encode_companions(companions));
        }
        put_fixed64_nonzero(&mut out, 4, self.timestamp);
        put_bytes_nonempty(&mut out, 5, &self.signature);
        out
    }
}

/// Contents of the linking QR code.
pub(super) struct LinkingQr<'a> {
    pub token: &'a str,
    pub companion_ik: &'a [u8],
    pub linking_secret: Option<&'a [u8]>,
    pub timestamp: u64,
    pub platform: WireDeviceType,
    pub relink_id: Option<&'a [u8]>,
}

impl LinkingQr<'_> {
    pub(super) fn encode(&self) -> Vec<u8> {
        let mut out = Vec::new();
        put_bytes_nonempty(&mut out, 1, self.token.as_bytes());
        put_bytes_nonempty(&mut out, 2, self.companion_ik);
        if let Some(secret) = self.linking_secret {
            put_bytes(&mut out, 3, secret);
        }
        put_varint_nonzero(&mut out, 4, self.timestamp);
        put_varint_nonzero(&mut out, 5, self.platform as u32 as u64);
        put_bytes_nonempty(&mut out, 6, self.relink_id.unwrap_or_default());
        out
    }
}
