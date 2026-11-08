//! Bounded release asset downloads, resource limits, and integrity verification.

use sha2::{Digest, Sha256};
use std::time::Duration;

/// Resource limits for one release asset.
#[derive(Clone, Copy)]
pub struct DownloadLimits {
    /// Smallest accepted artifact.
    pub min_bytes: u64,
    /// Maximum buffered artifact size.
    pub max_bytes: u64,
    /// Deadline covering headers and the complete body.
    pub timeout: Duration,
}

impl Default for DownloadLimits {
    fn default() -> Self {
        Self {
            min_bytes: 512 * 1024,
            max_bytes: 512 * 1024 * 1024,
            timeout: Duration::from_secs(240),
        }
    }
}

/// Release metadata supplied by the release discovery layer.
pub struct DownloadRequest<'a> {
    /// HTTP asset location, without session cookies.
    pub url: &'a str,
    /// A single filename, never a path.
    pub asset_name: &'a str,
    /// Optional published SHA-256 hex digest.
    pub expected_sha256: Option<&'a str>,
    /// Optional size published by the release API.
    pub expected_size: Option<u64>,
    /// Resource bounds for this transfer.
    pub limits: DownloadLimits,
}

/// Reject absolute paths, traversal and cross-platform path separators.
///
/// # Errors
/// Returns an error unless the input is a single ordinary filename.
pub fn validate_asset_name(name: &str) -> Result<(), String> {
    if name.is_empty()
        || name == "."
        || name == ".."
        || name.contains(['/', '\\', ':', '\0'])
        || name.chars().any(char::is_control)
    {
        return Err("Tên gói cập nhật không hợp lệ".into());
    }
    Ok(())
}

/// Download and validate an asset before any installation destination is touched.
/// The caller supplies a dedicated, cookie-free client and a progress callback.
///
/// # Errors
/// Rejects malformed metadata, HTTP failures, incomplete bodies, resource limits,
/// unsupported executable signatures, and checksum mismatches.
pub async fn download_release(
    client: &reqwest::Client,
    request: &DownloadRequest<'_>,
    mut progress: impl FnMut(u64, u64),
) -> Result<Vec<u8>, String> {
    validate_asset_name(request.asset_name)?;
    let limits = request.limits;
    if limits.min_bytes > limits.max_bytes || limits.max_bytes > usize::MAX as u64 {
        return Err("Giới hạn dung lượng cập nhật không hợp lệ".into());
    }
    if let Some(hash) = request.expected_sha256 {
        if hash.len() != 64 || !hash.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err("Mã SHA-256 của gói cập nhật không hợp lệ".into());
        }
    }
    if request
        .expected_size
        .is_some_and(|size| size > limits.max_bytes)
    {
        return Err("Gói cập nhật vượt giới hạn dung lượng".into());
    }
    let transfer = async {
        let mut response = client
            .get(request.url)
            .timeout(limits.timeout)
            .send()
            .await
            .map_err(|e| format!("Tải gói cập nhật thất bại: {e}"))?
            .error_for_status()
            .map_err(|e| format!("Máy chủ cập nhật trả về lỗi: {e}"))?;
        let length = response.content_length();
        if length.is_some_and(|size| size > limits.max_bytes) {
            return Err("Gói cập nhật vượt giới hạn dung lượng".into());
        }
        if let (Some(expected), Some(actual)) = (request.expected_size, length) {
            if expected != actual {
                return Err("Dung lượng gói cập nhật không khớp metadata".into());
            }
        }
        let total = request.expected_size.or(length).unwrap_or(0);
        let mut bytes = Vec::new();
        let mut hasher = Sha256::new();
        let mut last_progress = std::time::Instant::now();
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|e| format!("Ngắt kết nối khi đang tải cập nhật: {e}"))?
        {
            if (chunk.len() as u64) > limits.max_bytes.saturating_sub(bytes.len() as u64) {
                return Err("Gói cập nhật vượt giới hạn dung lượng".into());
            }
            hasher.update(&chunk);
            bytes.extend_from_slice(&chunk);
            if last_progress.elapsed() >= Duration::from_millis(80) {
                progress(bytes.len() as u64, total);
                last_progress = std::time::Instant::now();
            }
        }
        let actual = bytes.len() as u64;
        if actual < limits.min_bytes {
            return Err("Gói cập nhật tải về quá nhỏ".into());
        }
        if length.is_some_and(|size| size != actual)
            || request.expected_size.is_some_and(|size| size != actual)
        {
            return Err("Gói cập nhật tải về chưa hoàn tất".into());
        }
        if let Some(expected) = request.expected_sha256 {
            let actual = format!("{:x}", hasher.finalize());
            if !actual.eq_ignore_ascii_case(expected) {
                return Err("Mã kiểm tra toàn vẹn SHA-256 không khớp".into());
            }
        }
        let name = request.asset_name.to_ascii_lowercase();
        if name.ends_with(".exe") && !bytes.starts_with(b"MZ") {
            return Err("File cập nhật không có chữ ký PE hợp lệ".into());
        }
        if name.ends_with(".appimage") && !bytes.starts_with(b"\x7fELF") {
            return Err("File cập nhật không có chữ ký ELF hợp lệ".into());
        }
        progress(actual, total.max(actual));
        Ok(bytes)
    };
    tokio::time::timeout(limits.timeout, transfer)
        .await
        .map_err(|_| "Tải gói cập nhật quá thời gian cho phép".to_string())?
}
