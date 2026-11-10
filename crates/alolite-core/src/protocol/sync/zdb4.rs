//! Ephemeral RSA-2048 Sync V1 keypair and `ZDB4.0` AES-256-CBC + XZ/LZMA container unpacker.
//!
//! Pure byte-to-byte cryptographic and container parser (`re-notes/12-message-sync-and-e2ee-protocol.md` §3.2).

use std::io::Cursor;
use std::sync::Mutex;

use anyhow::Result;

static CURRENT_SYNC_KEYS: Mutex<Option<(rsa::RsaPrivateKey, String)>> = Mutex::new(None);

/// Retrieve the active RSA-2048 keypair used for mobile sync decryption without consuming it.
pub fn get_sync_keys() -> Option<(rsa::RsaPrivateKey, String)> {
    CURRENT_SYNC_KEYS.lock().ok().and_then(|g| g.clone())
}

/// Retrieve the active RSA-2048 SPKI Base64 public key.
pub fn get_sync_public_key() -> Option<String> {
    CURRENT_SYNC_KEYS
        .lock()
        .ok()
        .and_then(|g| g.as_ref().map(|(_, pub_k)| pub_k.clone()))
}

/// Clear the active ephemeral RSA sync keys once sync is finalized.
pub fn clear_sync_keys() {
    if let Ok(mut g) = CURRENT_SYNC_KEYS.lock() {
        *g = None;
    }
}

/// Generate a 2048-bit RSA SubjectPublicKeyInfo (`SPKI`) DER Base64 key (`294 bytes` -> `392 chars`)
/// matching `sync-message-key-generator` for `pullMobileMsg` (`cmd 12000`).
pub fn generate_sync_public_key() -> String {
    use base64::Engine;
    use rsa::pkcs8::EncodePublicKey;
    use rsa::{RsaPrivateKey, RsaPublicKey};

    let mut rng = rsa::rand_core::OsRng;
    if let Ok(priv_key) = RsaPrivateKey::new(&mut rng, 2048) {
        let pub_key = RsaPublicKey::from(&priv_key);
        if let Ok(der) = pub_key.to_public_key_der() {
            let b64 = base64::engine::general_purpose::STANDARD.encode(der.as_bytes());
            if let Ok(mut g) = CURRENT_SYNC_KEYS.lock() {
                *g = Some((priv_key, b64.clone()));
            }
            return b64;
        }
    }
    String::new()
}

fn strip_pkcs7_padding(buf: &[u8]) -> &[u8] {
    let Some(&last) = buf.last() else {
        return buf;
    };
    let pad = last as usize;
    if (1..=16).contains(&pad)
        && buf.len() >= pad
        && buf[buf.len() - pad..].iter().all(|&b| b == last)
    {
        &buf[..buf.len() - pad]
    } else {
        buf
    }
}

fn decompress_zdb4_stream(compressed: &[u8]) -> Result<Vec<u8>> {
    if compressed.starts_with(b"\xFD7zXZ\x00") {
        let xz_end = compressed
            .windows(2)
            .rposition(|w| w == b"YZ")
            .map(|pos| pos + 2)
            .unwrap_or(compressed.len());
        let mut cursor = Cursor::new(&compressed[..xz_end]);
        let mut out = Vec::new();
        lzma_rs::xz_decompress(&mut cursor, &mut out)
            .map_err(|e| anyhow::anyhow!("XZ decompress failed: {e}"))?;
        return Ok(out);
    }
    let unpadded = strip_pkcs7_padding(compressed);
    let mut cursor = Cursor::new(unpadded);
    let mut out = Vec::new();
    if lzma_rs::lzma_decompress(&mut cursor, &mut out).is_ok() {
        return Ok(out);
    }
    let mut xz_cursor = Cursor::new(unpadded);
    out.clear();
    lzma_rs::xz_decompress(&mut xz_cursor, &mut out)
        .map_err(|e| anyhow::anyhow!("LZMA/XZ decompress failed: {e}"))?;
    Ok(out)
}

/// Decrypt and unpack a Sync V1 `ZDB4.0` `.db.crypt` container into `(entry_name, sqlite_bytes)` pairs.
///
/// Each `65,536`-byte ciphertext chunk is decrypted independently with `AES-256-CBC` and a zero IV
/// using the 32-byte uppercase hex representation of `raw_key`.
///
/// # Errors
/// Returns an error if `raw_key` is shorter than 16 bytes, `crypt_data` lacks the `ZDB4.0` header,
/// TOC bounds are invalid, or LZMA/XZ decompression fails.
pub fn decrypt_and_unpack_zdb4_container(
    crypt_data: &[u8],
    raw_key: &[u8],
) -> Result<Vec<(String, Vec<u8>)>> {
    use aes::cipher::{BlockDecryptMut, KeyIvInit};

    if raw_key.len() < 16 {
        anyhow::bail!("decrypted RSA sync key too short: {} bytes", raw_key.len());
    }
    if crypt_data.len() < 32 {
        anyhow::bail!("backup file too small: {} bytes", crypt_data.len());
    }

    let hex_key: String = raw_key
        .iter()
        .take(16)
        .map(|b| format!("{b:02X}"))
        .collect();
    let key_32: &[u8; 32] = hex_key
        .as_bytes()
        .try_into()
        .map_err(|_| anyhow::anyhow!("derived ZDB4.0 key length mismatch"))?;

    type Aes256CbcDec = cbc::Decryptor<aes::Aes256>;
    const CHUNK_SIZE: usize = 65_536;
    let iv_zero = [0u8; 16];
    let mut decrypted = Vec::with_capacity(crypt_data.len());

    for chunk in crypt_data.chunks(CHUNK_SIZE) {
        let aligned_len = chunk.len() - (chunk.len() % 16);
        if aligned_len == 0 {
            break;
        }
        let mut buf = chunk[..aligned_len].to_vec();
        let dec = Aes256CbcDec::new(key_32.into(), &iv_zero.into());
        dec.decrypt_padded_mut::<cipher::block_padding::NoPadding>(&mut buf)
            .map_err(|e| anyhow::anyhow!("AES-256-CBC chunk decrypt failed: {e:?}"))?;
        decrypted.extend_from_slice(&buf);
    }

    if !decrypted.starts_with(b"ZDB4.0") {
        let preview = &decrypted[..decrypted.len().min(7)];
        anyhow::bail!("invalid backup container magic: {preview:?}");
    }

    for hdr_len in [6usize, 7usize] {
        if decrypted.len() > hdr_len && decrypted[hdr_len..].starts_with(b"\xFD7zXZ\x00") {
            let payload = decompress_zdb4_stream(&decrypted[hdr_len..])?;
            return Ok(vec![("backup.db".to_string(), payload)]);
        }
    }

    if decrypted.len() < 18 || decrypted[6] != 0 {
        let payload = decompress_zdb4_stream(&decrypted[6..])?;
        return Ok(vec![("backup.db".to_string(), payload)]);
    }

    let mut offset = 14usize;
    let n_files = u32::from_be_bytes(decrypted[offset..offset + 4].try_into()?) as usize;
    offset += 4;
    if n_files > 100_000 {
        anyhow::bail!("unreasonable ZDB4.0 TOC file count: {n_files}");
    }

    let mut toc = Vec::with_capacity(n_files.min(4096));
    for _ in 0..n_files {
        if offset + 4 > decrypted.len() {
            anyhow::bail!("truncated ZDB4.0 TOC entry name length");
        }
        let name_len = u32::from_be_bytes(decrypted[offset..offset + 4].try_into()?) as usize;
        offset += 4;
        if name_len == 0 || name_len > 256 || offset + name_len + 4 > decrypted.len() {
            anyhow::bail!("invalid ZDB4.0 TOC entry name length: {name_len}");
        }
        let raw_name = String::from_utf8_lossy(&decrypted[offset..offset + name_len]);
        offset += name_len;
        let size = u32::from_be_bytes(decrypted[offset..offset + 4].try_into()?) as usize;
        offset += 4;

        let safe_name = std::path::Path::new(raw_name.as_ref())
            .file_name()
            .and_then(|n| n.to_str())
            .filter(|n| !n.is_empty() && *n != "." && *n != "..")
            .ok_or_else(|| anyhow::anyhow!("unsafe ZDB4.0 TOC entry name"))?
            .to_string();
        toc.push((safe_name, size));
    }

    if offset >= decrypted.len() {
        anyhow::bail!("corrupted ZDB4.0 container: TOC extends beyond payload");
    }

    let payload = decompress_zdb4_stream(&decrypted[offset..])?;
    let mut entries = Vec::with_capacity(toc.len());
    let mut curr = 0usize;
    for (name, size) in toc {
        if curr + size > payload.len() {
            break;
        }
        entries.push((name, payload[curr..curr + size].to_vec()));
        curr += size;
    }
    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_sync_public_key_produces_valid_rsa2048_spki_der_base64() {
        use base64::Engine;
        let k1 = generate_sync_public_key();
        let k2 = generate_sync_public_key();
        assert_ne!(k1, k2);
        assert!(k1.starts_with("MIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEA"));
        let raw = base64::engine::general_purpose::STANDARD
            .decode(&k1)
            .expect("valid base64");
        assert_eq!(raw.len(), 294);
        assert_eq!(&raw[raw.len() - 5..], &[0x02, 0x03, 0x01, 0x00, 0x01]);
        clear_sync_keys();
    }

    #[test]
    fn decrypt_and_unpack_zdb4_container_handles_multi_chunk_iv_reset_and_sanitizes_toc() {
        use aes::cipher::{BlockEncryptMut, KeyIvInit};

        let entry_a: Vec<u8> = (0..70_000u32)
            .map(|i| ((i.wrapping_mul(1_664_525).wrapping_add(1_013_904_223)) >> 24) as u8)
            .collect();
        let entry_b = b"SQLite format 3\0synthetic_group_payload".to_vec();

        let mut concat_raw = Vec::with_capacity(entry_a.len() + entry_b.len());
        concat_raw.extend_from_slice(&entry_a);
        concat_raw.extend_from_slice(&entry_b);

        let mut xz_compressed = Vec::new();
        lzma_rs::xz_compress(&mut Cursor::new(&concat_raw), &mut xz_compressed).unwrap();

        let mut plain_container = Vec::new();
        plain_container.extend_from_slice(b"ZDB4.0\0");
        plain_container.extend_from_slice(&[0u8; 7]);
        plain_container.extend_from_slice(&2u32.to_be_bytes());

        let name_a = b"../../123456.db";
        plain_container.extend_from_slice(&(name_a.len() as u32).to_be_bytes());
        plain_container.extend_from_slice(name_a);
        plain_container.extend_from_slice(&(entry_a.len() as u32).to_be_bytes());

        let name_b = b"group_999888.db";
        plain_container.extend_from_slice(&(name_b.len() as u32).to_be_bytes());
        plain_container.extend_from_slice(name_b);
        plain_container.extend_from_slice(&(entry_b.len() as u32).to_be_bytes());

        plain_container.extend_from_slice(&xz_compressed);
        let pad = (16 - (plain_container.len() % 16)) % 16;
        plain_container.extend(std::iter::repeat_n(pad as u8, pad));
        assert!(plain_container.len() > 65_536);

        let raw_key: [u8; 16] =
            *b"\x01\x23\x45\x67\x89\xab\xcd\xef\x10\x32\x54\x76\x98\xba\xdc\xfe";
        let hex_key: String = raw_key.iter().map(|b| format!("{b:02X}")).collect();
        let key_32: &[u8; 32] = hex_key.as_bytes().try_into().unwrap();
        let iv_zero = [0u8; 16];

        type Aes256CbcEnc = cbc::Encryptor<aes::Aes256>;
        let mut ciphertext = Vec::with_capacity(plain_container.len());
        for chunk in plain_container.chunks(65_536) {
            let mut buf = chunk.to_vec();
            let len = buf.len();
            let enc = Aes256CbcEnc::new(key_32.into(), &iv_zero.into());
            enc.encrypt_padded_mut::<cipher::block_padding::NoPadding>(&mut buf, len)
                .unwrap();
            ciphertext.extend_from_slice(&buf);
        }

        let unpacked = decrypt_and_unpack_zdb4_container(&ciphertext, &raw_key).unwrap();
        assert_eq!(unpacked.len(), 2);
        assert_eq!(unpacked[0].0, "123456.db");
        assert_eq!(unpacked[0].1, entry_a);
        assert_eq!(unpacked[1].0, "group_999888.db");
        assert_eq!(unpacked[1].1, entry_b);
    }
}
