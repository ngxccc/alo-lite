use super::*;

#[test]
fn test_is_newer_semver_comparisons() {
    assert!(is_newer_semver("0.0.4", "0.0.3"));
    assert!(is_newer_semver("v0.0.4", "0.0.3"));
    assert!(is_newer_semver("v0.1.0", "v0.0.9"));
    assert!(!is_newer_semver("0.0.3", "0.0.3"));
    assert!(!is_newer_semver("0.0.2", "0.0.3"));
    assert!(is_newer_semver("0.0.4", "0.0.4-rc.1"));
}

#[test]
fn test_parse_github_release_api_payload_with_native_digest() {
    let api_json = serde_json::json!({
        "tag_name": "v0.0.4",
        "name": "alolite v0.0.4",
        "body": "- Sửa lỗi Windows Defender\n- Tự động thông báo cập nhật qua GitHub API",
        "html_url": "https://github.com/ngxccc/alolite/releases/tag/v0.0.4",
        "published_at": "2026-11-02T03:00:00Z",
        "assets": [
            {
                "name": "SHA256SUMS.txt",
                "size": 512,
                "browser_download_url": "https://github.com/ngxccc/alolite/releases/download/v0.0.4/SHA256SUMS.txt"
            },
            {
                "name": "alolite-0.0.4-setup-windows-x86_64.exe",
                "size": 9500000,
                "digest": "sha256:1111111111111111111111111111111111111111111111111111111111111111",
                "browser_download_url": "https://github.com/ngxccc/alolite/releases/download/v0.0.4/alolite-0.0.4-setup-windows-x86_64.exe"
            },
            {
                "name": "alolite-0.0.4-portable-windows-x86_64.exe",
                "size": 8400000,
                "digest": "sha256:abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789",
                "browser_download_url": "https://github.com/ngxccc/alolite/releases/download/v0.0.4/alolite-0.0.4-portable-windows-x86_64.exe"
            },
            {
                "name": "alolite-0.0.4-macos-arm64.dmg",
                "size": 10200000,
                "digest": "sha256:2222222222222222222222222222222222222222222222222222222222222222",
                "browser_download_url": "https://github.com/ngxccc/alolite/releases/download/v0.0.4/alolite-0.0.4-macos-arm64.dmg"
            }
        ]
    });

    let win_meta =
        parse_github_release_value(&api_json, "0.0.3", "ngxccc/alolite", "windows", "x86_64");
    assert!(win_meta.has_update);
    assert_eq!(win_meta.latest_version, "0.0.4");
    assert_eq!(
        win_meta.asset_name.as_deref(),
        Some("alolite-0.0.4-portable-windows-x86_64.exe")
    );
    assert_eq!(
        win_meta.expected_sha256.as_deref(),
        Some("abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789")
    );
    assert_eq!(win_meta.published_at, "2026-11-02");

    let mac_meta =
        parse_github_release_value(&api_json, "0.0.3", "ngxccc/alolite", "macos", "aarch64");
    assert_eq!(
        mac_meta.asset_name.as_deref(),
        Some("alolite-0.0.4-macos-arm64.dmg")
    );
    assert_eq!(
        mac_meta.expected_sha256.as_deref(),
        Some("2222222222222222222222222222222222222222222222222222222222222222")
    );
}

#[test]
fn test_parse_sha256sums_for_asset() {
    let sums = "\
abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789  alolite-0.0.4-portable-windows-x86_64.exe\n\
2222222222222222222222222222222222222222222222222222222222222222 *./alolite-0.0.4-macos-arm64.dmg\n";
    assert_eq!(
        parse_sha256sums_for_asset(sums, "alolite-0.0.4-macos-arm64.dmg").as_deref(),
        Some("2222222222222222222222222222222222222222222222222222222222222222")
    );
}
