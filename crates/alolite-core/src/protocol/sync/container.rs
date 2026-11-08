//! Pure network download and crypto unpack for mobile backup containers (`.db.crypt`) and Sync V2 chunks.
//!
//! Handles HTTP download, RSA key decryption, AES/LZMA container unpacking, and Zstd decompression
//! returning plain byte entries or parsed protobuf partitions without touching SQLite.

use anyhow::Result;

use super::v2_proto::{parse_sync_chunk, ParsedProtoPartition};
use super::zdb4::{decrypt_and_unpack_zdb4_container, get_sync_keys};
use crate::protocol::session::{Session, ZALO_UA};

/// Unpacked Sync V1 mobile backup container holding extracted `.db` file entries.
#[derive(Debug)]
pub struct DownloadedBackup {
    pub entries: Vec<(String, Vec<u8>)>,
    pub pub_key_b64: String,
}

/// Download, decrypt RSA session key, and unpack a Sync V1 mobile backup container (`.db.crypt`).
///
/// # Errors
/// Returns an error if the RSA sync key is missing, HTTP download fails, or container decryption fails.
pub async fn fetch_mobile_backup_container(
    s: &Session,
    url: &str,
    encrypted_key: &str,
) -> Result<DownloadedBackup> {
    use base64::Engine;
    use rsa::Pkcs1v15Encrypt;

    if url.is_empty() {
        anyhow::bail!("empty backup url");
    }
    if encrypted_key.is_empty() {
        anyhow::bail!("encrypted_key is empty");
    }

    let (priv_key, pub_key_b64) = get_sync_keys()
        .ok_or_else(|| anyhow::anyhow!("Không tìm thấy khoá RSA phiên đồng bộ hiện tại"))?;

    let my_uid = s.login.as_ref().map(|l| l.uin.clone()).unwrap_or_default();
    let sek = s.zpw_sek.clone().unwrap_or_default();
    let cookie_hdr = format!("zpw_sek={sek}; zpw_uin={my_uid}");

    let resp = s
        .http
        .get(url)
        .header("Cookie", cookie_hdr)
        .header("User-Agent", ZALO_UA)
        .send()
        .await?;

    if !resp.status().is_success() {
        anyhow::bail!("failed to download backup: HTTP {}", resp.status());
    }
    let crypt_data = resp.bytes().await?.to_vec();
    let enc_key_owned = encrypted_key.to_string();

    let entries = tokio::task::spawn_blocking(move || -> Result<Vec<(String, Vec<u8>)>> {
        let enc_bytes = base64::engine::general_purpose::STANDARD.decode(enc_key_owned.trim())?;
        let raw_key = priv_key.decrypt(Pkcs1v15Encrypt, &enc_bytes)?;
        decrypt_and_unpack_zdb4_container(&crypt_data, &raw_key)
    })
    .await??;

    Ok(DownloadedBackup {
        entries,
        pub_key_b64,
    })
}

/// Download and acknowledge a Sync V2 metadata batch.
///
/// # Errors
/// Returns an error if the HTTP download fails.
pub async fn fetch_sync_v2_meta(s: &Session, url: &str) -> Result<()> {
    let uin = s.login.as_ref().map(|l| l.uin.clone()).unwrap_or_default();
    let sek = s.zpw_sek.clone().unwrap_or_default();
    let cookie_hdr = format!("zpw_sek={sek}; zpw_uin={uin}");

    let resp = s
        .http
        .get(url)
        .header("Cookie", cookie_hdr)
        .header("User-Agent", ZALO_UA)
        .send()
        .await?;

    if !resp.status().is_success() {
        anyhow::bail!("failed to download v2 meta: HTTP {}", resp.status());
    }
    let _bytes = resp.bytes().await?;
    Ok(())
}

/// Download, decompress, and parse an unencrypted Zstd Sync V2 message batch into protobuf partitions.
///
/// # Errors
/// Returns an error if the chunk is encrypted (`zprotoSync2`), malformed, or HTTP download fails.
pub async fn fetch_sync_v2_messages(s: &Session, url: &str) -> Result<Vec<ParsedProtoPartition>> {
    let uin = s.login.as_ref().map(|l| l.uin.clone()).unwrap_or_default();
    let sek = s.zpw_sek.clone().unwrap_or_default();
    let cookie_hdr = format!("zpw_sek={sek}; zpw_uin={uin}");

    let resp = s
        .http
        .get(url)
        .header("Cookie", cookie_hdr)
        .header("User-Agent", ZALO_UA)
        .send()
        .await?;

    if !resp.status().is_success() {
        anyhow::bail!("failed to download v2 messages: HTTP {}", resp.status());
    }

    let bytes = resp.bytes().await?;
    let decompressed = zstd::decode_all(std::io::Cursor::new(&bytes)).map_err(|_| {
        anyhow::anyhow!(
            "Gói Sync v2 đang mã hoá zprotoSync2 (hãy dùng luồng đồng bộ tiêu chuẩn pull_mobile_msg)"
        )
    })?;
    parse_sync_chunk(&decompressed)
        .ok_or_else(|| anyhow::anyhow!("Không giải mã được cấu trúc SyncChunk của gói Sync v2"))
}
