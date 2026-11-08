//! basE91 text encoding used for the linking QR payload.
//!
//! Standard basE91 alphabet; 13 or 14 bits are consumed per character pair.

const ALPHABET: &[u8; 91] =
    b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789!#$%&()*+,./:;<=>?@[]^_`{|}~\"";

/// Encode `data` as basE91 text.
pub(super) fn encode(data: &[u8]) -> String {
    let mut out = String::with_capacity(data.len() * 14 / 11 + 2);
    let mut queue = 0u32;
    let mut bits = 0u32;
    for &byte in data {
        queue |= u32::from(byte) << bits;
        bits += 8;
        if bits > 13 {
            let mut value = queue & 8191;
            if value > 88 {
                queue >>= 13;
                bits -= 13;
            } else {
                value = queue & 16383;
                queue >>= 14;
                bits -= 14;
            }
            out.push(ALPHABET[(value % 91) as usize] as char);
            out.push(ALPHABET[(value / 91) as usize] as char);
        }
    }
    if bits > 0 {
        out.push(ALPHABET[(queue % 91) as usize] as char);
        if bits > 7 || queue > 90 {
            out.push(ALPHABET[(queue / 91) as usize] as char);
        }
    }
    out
}

#[cfg(test)]
pub(super) fn decode(text: &str) -> Option<Vec<u8>> {
    let mut queue = 0u32;
    let mut bits = 0u32;
    let mut pending: Option<u32> = None;
    let mut out = Vec::new();
    for ch in text.bytes() {
        let digit = ALPHABET.iter().position(|&c| c == ch)? as u32;
        match pending.take() {
            None => pending = Some(digit),
            Some(low) => {
                let value = low + digit * 91;
                queue |= value << bits;
                bits += if value & 8191 > 88 { 13 } else { 14 };
                while bits > 7 {
                    out.push(queue as u8);
                    queue >>= 8;
                    bits -= 8;
                }
            }
        }
    }
    if let Some(low) = pending {
        out.push((queue | low << bits) as u8);
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_arbitrary_bytes() {
        for len in 0..200usize {
            let data: Vec<u8> = (0..len).map(|i| (i * 37 + 11) as u8).collect();
            assert_eq!(
                decode(&encode(&data)).as_deref(),
                Some(&data[..]),
                "len {len}"
            );
        }
    }

    #[test]
    fn encodes_reference_vector() {
        // Matches the standard basE91 alphabet used by base91-0.1.0 in the reference WASM.
        assert_eq!(encode(b"test"), "fPNKd");
        assert_eq!(decode("fPNKd").as_deref(), Some(&b"test"[..]));
    }
}
