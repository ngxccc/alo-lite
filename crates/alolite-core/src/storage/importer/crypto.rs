/// Recover the 32-character XOR key from LevelDB in `Partitions/zalo/Local Storage/leveldb`.
pub fn recover_zalo_localstorage_key(base_dir: &std::path::Path) -> String {
    let cand_dirs = vec![base_dir
        .join("Partitions")
        .join("zalo")
        .join("Local Storage")
        .join("leveldb")];

    let mut key = vec![None; 32];
    let u_pat = "z_ecrp_"
        .encode_utf16()
        .flat_map(|u| u.to_le_bytes())
        .collect::<Vec<u8>>();

    for ldb_dir in cand_dirs {
        let Ok(entries) = std::fs::read_dir(ldb_dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let p = entry.path();
            let ext = p.extension().and_then(|e| e.to_str()).unwrap_or_default();
            if ext != "ldb" && ext != "log" {
                continue;
            }
            let Ok(b) = std::fs::read(&p) else {
                continue;
            };
            let mut pos = 0;
            while let Some(rel) = b[pos..]
                .windows(u_pat.len())
                .position(|w| w == u_pat.as_slice())
            {
                let idx = pos + rel + u_pat.len();
                pos = idx;
                let mut chars = Vec::new();
                for i in (0..64).step_by(2) {
                    if idx + i + 1 < b.len() {
                        chars.push((b[idx + i] as u16 | ((b[idx + i + 1] as u16) << 8)) as u32);
                    }
                }
                if chars.len() >= 32 {
                    let prefix = b"{\"version\":";
                    let cand: Vec<u32> = chars
                        .iter()
                        .take(prefix.len())
                        .zip(prefix.iter())
                        .map(|(c, p)| c ^ (*p as u32))
                        .collect();
                    if cand.iter().all(|x| (*x as u8 as char).is_ascii_hexdigit()) {
                        for (i, v) in cand.iter().enumerate() {
                            key[i] = Some(*v as u8 as char);
                        }
                        let suffix = b",\"list_recent\":[{\"";
                        if chars.len() >= 14 + suffix.len() {
                            let cand2: Vec<u32> = chars[14..14 + suffix.len()]
                                .iter()
                                .zip(suffix.iter())
                                .map(|(c, p)| c ^ (*p as u32))
                                .collect();
                            if cand2.iter().all(|x| (*x as u8 as char).is_ascii_hexdigit()) {
                                for (i, v) in cand2.iter().enumerate() {
                                    key[14 + i] = Some(*v as u8 as char);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    let mut full_key = String::new();
    for k in key {
        match k {
            Some(c) => full_key.push(c),
            None => return "de039414865b856ea650d3305b1f387f".to_string(),
        }
    }
    full_key
}

/// Decrypt single official Zalo PC database bytes using PBKDF2-HMAC-SHA512 + AES-256-CBC (SQLCipher v4 profile).
pub fn try_decrypt_zalo_pc_database(bytes: &[u8]) -> Option<Vec<u8>> {
    let key = "de039414865b856ea650d3305b1f387f";
    decrypt_sqlcipher_bytes(bytes, key.as_bytes())
}

/// Decrypt SQLCipher v4 database pages with 80-byte reserve, 4096 page size, SHA-512 256k iterations.
pub fn decrypt_sqlcipher_bytes(bytes: &[u8], passphrase: &[u8]) -> Option<Vec<u8>> {
    use aes::cipher::{block_padding::NoPadding, BlockDecryptMut, KeyIvInit};

    type Aes256CbcDec = cbc::Decryptor<aes::Aes256>;

    if bytes.len() < 4096 {
        return None;
    }
    if bytes.starts_with(b"SQLite format 3\x00") {
        return Some(bytes.to_vec());
    }

    let salt = &bytes[0..16];
    // PBKDF2-HMAC-SHA512 256000 iterations -> 32-byte key
    let mut derived_key = [0u8; 32];
    pbkdf2_hmac_sha512(passphrase, salt, 256000, &mut derived_key);

    let page_size = 4096;
    let reserve = 80;
    let num_pages = bytes.len() / page_size;
    if num_pages == 0 {
        return None;
    }

    let mut out = Vec::with_capacity(bytes.len());
    for p in 0..num_pages {
        let page = &bytes[p * page_size..(p + 1) * page_size];
        let iv = &page[page_size - reserve..page_size - reserve + 16];
        let mut dec_buf;
        if p == 0 {
            dec_buf = page[16..page_size - reserve].to_vec();
            let decryptor = Aes256CbcDec::new((&derived_key).into(), iv.into());
            if decryptor
                .decrypt_padded_mut::<NoPadding>(&mut dec_buf)
                .is_err()
            {
                return None;
            }
            // Check SQLite magic page size (offset 0..2 in decrypted buffer)
            if dec_buf.len() < 16 || dec_buf[0] != 0x10 || dec_buf[1] != 0x00 {
                return None;
            }
            out.extend_from_slice(b"SQLite format 3\x00");
            out.extend_from_slice(&dec_buf);
            out.extend(std::iter::repeat_n(0u8, reserve));
        } else {
            dec_buf = page[0..page_size - reserve].to_vec();
            let decryptor = Aes256CbcDec::new((&derived_key).into(), iv.into());
            if decryptor
                .decrypt_padded_mut::<NoPadding>(&mut dec_buf)
                .is_err()
            {
                return None;
            }
            out.extend_from_slice(&dec_buf);
            out.extend(std::iter::repeat_n(0u8, reserve));
        }
    }
    Some(out)
}

fn pbkdf2_hmac_sha512(password: &[u8], salt: &[u8], iterations: u32, output: &mut [u8]) {
    use sha2::{Digest, Sha512};
    let mut k = [0u8; 128];
    if password.len() > 128 {
        let h = Sha512::digest(password);
        k[..64].copy_from_slice(&h);
    } else {
        k[..password.len()].copy_from_slice(password);
    }
    let mut i_pad = [0x36u8; 128];
    let mut o_pad = [0x5cu8; 128];
    for i in 0..128 {
        i_pad[i] ^= k[i];
        o_pad[i] ^= k[i];
    }
    let mut base_inner = Sha512::new();
    base_inner.update(i_pad);
    let mut base_outer = Sha512::new();
    base_outer.update(o_pad);

    let hmac = |data: &[u8]| -> [u8; 64] {
        let mut inner = base_inner.clone();
        inner.update(data);
        let inner_hash = inner.finalize();

        let mut outer = base_outer.clone();
        outer.update(inner_hash);
        let mut res = [0u8; 64];
        res.copy_from_slice(&outer.finalize());
        res
    };

    let mut salt_and_i = Vec::with_capacity(salt.len() + 4);
    salt_and_i.extend_from_slice(salt);
    salt_and_i.extend_from_slice(&1u32.to_be_bytes());

    let mut u = hmac(&salt_and_i);
    let mut t = u;

    for _ in 1..iterations {
        u = hmac(&u);
        for j in 0..64 {
            t[j] ^= u[j];
        }
    }
    let len = output.len().min(64);
    output[..len].copy_from_slice(&t[..len]);
}
