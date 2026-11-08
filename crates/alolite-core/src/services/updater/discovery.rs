//! GitHub Releases API discovery, semantic version parsing, and asset selection.

use std::time::Duration;

/// Metadata parsed from GitHub Releases REST API (`/repos/{owner}/{repo}/releases/latest`)
/// or the static `latest.json` release manifest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitHubReleaseMetadata {
    pub current_version: String,
    pub latest_version: String,
    pub has_update: bool,
    pub release_title: String,
    pub release_notes: String,
    pub html_url: String,
    pub download_url: Option<String>,
    pub asset_name: Option<String>,
    pub asset_size: u64,
    pub expected_sha256: Option<String>,
    pub published_at: String,
    pub sha256sums_url: Option<String>,
}

/// UI-agnostic update metadata returned by [`fetch_github_latest_release`].
#[derive(Debug, Clone, serde::Serialize)]
pub struct UpdateCheckResult {
    pub current_version: String,
    pub latest_version: String,
    pub has_update: bool,
    pub release_title: String,
    pub release_notes: String,
    pub html_url: String,
    pub download_url: Option<String>,
    pub asset_name: Option<String>,
    pub asset_size: u64,
    pub expected_sha256: Option<String>,
    pub published_at: String,
}

pub(crate) const GITHUB_REPO: &str = "ngxccc/alolite";

/// Compare semantic versions (`"v0.0.4"` > `"0.0.3"`).
pub fn is_newer_semver(remote: &str, current: &str) -> bool {
    let parse = |s: &str| -> (u64, u64, u64, bool) {
        let trimmed = s.trim().trim_start_matches(['v', 'V']);
        let (core, rest) = trimmed
            .split_once(['-', '+'])
            .map_or((trimmed, ""), |(c, r)| (c, r));
        let is_stable = !trimmed.contains('-') || rest.is_empty();
        let mut parts = core.split('.').filter_map(|p| p.trim().parse::<u64>().ok());
        (
            parts.next().unwrap_or(0),
            parts.next().unwrap_or(0),
            parts.next().unwrap_or(0),
            is_stable,
        )
    };
    parse(remote) > parse(current)
}

/// Score a release asset filename for a target `(os, arch)` pair.
/// Returns `<= 0` when the asset is incompatible with the platform.
pub fn select_platform_asset_score(name: &str, target_os: &str, target_arch: &str) -> i32 {
    let n_low = name.to_ascii_lowercase();
    if n_low.is_empty()
        || n_low.ends_with(".txt")
        || n_low.ends_with(".json")
        || n_low.ends_with(".sig")
        || n_low.ends_with(".asc")
    {
        return -1;
    }
    match target_os {
        "windows" => {
            if n_low.ends_with(".exe") && n_low.contains("portable") {
                110
            } else if n_low.ends_with(".exe") && n_low.contains("setup") {
                90
            } else if n_low.ends_with(".exe") {
                80
            } else {
                0
            }
        }
        "macos" => {
            if !n_low.ends_with(".dmg") {
                return 0;
            }
            if (target_arch == "aarch64" && (n_low.contains("arm64") || n_low.contains("aarch64")))
                || (target_arch == "x86_64"
                    && (n_low.contains("x86_64") || n_low.contains("intel")))
            {
                110
            } else if !n_low.contains("arm64")
                && !n_low.contains("aarch64")
                && !n_low.contains("x86_64")
                && !n_low.contains("intel")
            {
                100
            } else {
                50
            }
        }
        _ => {
            if n_low.ends_with(".appimage") {
                100
            } else if n_low.ends_with(".deb") && !n_low.contains("legacy") {
                90
            } else if n_low.ends_with(".tar.gz") && !n_low.contains("legacy") {
                80
            } else if n_low.ends_with(".rpm") {
                70
            } else {
                0
            }
        }
    }
}

/// Extract a 64-character lowercase SHA-256 hex digest from `SHA256SUMS.txt` content for `asset_name`.
pub fn parse_sha256sums_for_asset(sha256sums_text: &str, asset_name: &str) -> Option<String> {
    for line in sha256sums_text.lines() {
        let mut parts = line.split_whitespace();
        let Some(hash) = parts.next() else { continue };
        let Some(fname) = parts.next() else { continue };
        let clean_fname = fname.trim_start_matches('*').trim_start_matches("./");
        if clean_fname == asset_name
            && hash.len() == 64
            && hash.bytes().all(|b| b.is_ascii_hexdigit())
        {
            return Some(hash.to_ascii_lowercase());
        }
    }
    None
}

/// Parse either a GitHub Releases API JSON object (`/releases/latest`) or a `latest.json`
/// manifest object into [`GitHubReleaseMetadata`].
pub fn parse_github_release_value(
    rel: &serde_json::Value,
    current_ver: &str,
    repo_slug: &str,
    target_os: &str,
    target_arch: &str,
) -> GitHubReleaseMetadata {
    let raw_tag = rel
        .get("tag_name")
        .or_else(|| rel.get("version"))
        .and_then(|v| v.as_str())
        .unwrap_or(current_ver);
    let latest_ver = raw_tag.trim().trim_start_matches(['v', 'V']).to_string();
    let has_update = is_newer_semver(&latest_ver, current_ver);

    let release_title = rel
        .get("name")
        .and_then(|v| v.as_str())
        .filter(|s| !s.trim().is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| format!("alolite v{latest_ver}"));

    let release_notes = rel
        .get("body")
        .or_else(|| rel.get("notes"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim()
        .to_string();

    let html_url = rel
        .get("html_url")
        .or_else(|| rel.get("release_url"))
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| format!("https://github.com/{repo_slug}/releases/tag/v{latest_ver}"));

    let published_at = rel
        .get("published_at")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .split('T')
        .next()
        .unwrap_or("")
        .to_string();

    let mut best_url: Option<String> = None;
    let mut best_name: Option<String> = None;
    let mut best_size: u64 = 0;
    let mut best_sha256: Option<String> = None;
    let mut best_score: i32 = 0;
    let mut sha256sums_url: Option<String> = None;

    if let Some(assets) = rel.get("assets").and_then(|v| v.as_array()) {
        for a in assets {
            let name = a.get("name").and_then(|v| v.as_str()).unwrap_or("");
            let dl = a
                .get("browser_download_url")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let sz = a.get("size").and_then(|v| v.as_u64()).unwrap_or(0);
            if name.eq_ignore_ascii_case("SHA256SUMS.txt") && !dl.is_empty() {
                sha256sums_url = Some(dl.to_string());
                continue;
            }
            if name.is_empty() || dl.is_empty() {
                continue;
            }
            let score = select_platform_asset_score(name, target_os, target_arch);
            if score > best_score {
                best_score = score;
                best_url = Some(dl.to_string());
                best_name = Some(name.to_string());
                best_size = sz;
                // Extract SHA-256 from `latest.json` (`sha256`) or GitHub API asset (`digest: "sha256:..."`)
                best_sha256 = a
                    .get("sha256")
                    .and_then(|v| v.as_str())
                    .or_else(|| {
                        a.get("digest")
                            .and_then(|v| v.as_str())
                            .and_then(|d| d.strip_prefix("sha256:"))
                    })
                    .filter(|h| h.len() == 64 && h.bytes().all(|b| b.is_ascii_hexdigit()))
                    .map(|h| h.to_ascii_lowercase());
            }
        }
    }

    // Legacy single-asset `latest.json` fallback when `assets` array is absent
    if best_url.is_none() && rel.get("assets").is_none() {
        let fallback_name = match target_os {
            "windows" => format!("alolite-{latest_ver}-portable-windows-x86_64.exe"),
            "macos" => match target_arch {
                "aarch64" => format!("alolite-{latest_ver}-macos-arm64.dmg"),
                "x86_64" => format!("alolite-{latest_ver}-macos-x86_64.dmg"),
                _ => format!("alolite-{latest_ver}-macos.dmg"),
            },
            _ => format!("alolite-{latest_ver}-x86_64.AppImage"),
        };
        best_url = Some(format!(
            "https://github.com/{repo_slug}/releases/download/v{latest_ver}/{fallback_name}"
        ));
        best_name = Some(fallback_name);
        best_sha256 = rel
            .get("sha256")
            .and_then(|v| v.as_str())
            .filter(|h| h.len() == 64 && h.bytes().all(|b| b.is_ascii_hexdigit()))
            .map(|h| h.to_ascii_lowercase());
    }

    GitHubReleaseMetadata {
        current_version: current_ver.to_string(),
        latest_version: latest_ver,
        has_update,
        release_title,
        release_notes,
        html_url,
        download_url: best_url,
        asset_name: best_name,
        asset_size: best_size,
        expected_sha256: best_sha256,
        published_at,
        sha256sums_url,
    }
}

/// Query GitHub Releases (falling back to `latest.json`) for the newest release matching this platform.
pub async fn fetch_github_latest_release() -> Result<UpdateCheckResult, String> {
    let current_ver = env!("CARGO_PKG_VERSION").to_string();
    let client = reqwest::Client::builder()
        .user_agent(format!("alolite/{current_ver}"))
        .timeout(Duration::from_secs(12))
        .build()
        .map_err(|e| format!("Khởi tạo HTTP client thất bại: {e}"))?;

    let latest_url = format!("https://api.github.com/repos/{GITHUB_REPO}/releases/latest");
    let mut rel_json: Option<serde_json::Value> = None;

    if let Ok(resp) = client
        .get(&latest_url)
        .header("Accept", "application/vnd.github+json")
        .header("X-GitHub-Api-Version", "2022-11-28")
        .send()
        .await
    {
        if resp.status().is_success() {
            if let Ok(v) = resp.json::<serde_json::Value>().await {
                rel_json = Some(v);
            }
        }
    }

    if rel_json.is_none() {
        let list_url = format!("https://api.github.com/repos/{GITHUB_REPO}/releases?per_page=5");
        if let Ok(resp) = client
            .get(&list_url)
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2022-11-28")
            .send()
            .await
        {
            if resp.status().is_success() {
                if let Ok(arr) = resp.json::<serde_json::Value>().await {
                    if let Some(first) = arr
                        .as_array()
                        .and_then(|a| {
                            a.iter()
                                .find(|r| {
                                    !r.get("draft").and_then(|v| v.as_bool()).unwrap_or(false)
                                })
                                .or_else(|| a.first())
                        })
                        .cloned()
                    {
                        rel_json = Some(first);
                    }
                }
            }
        }
    }

    if rel_json.is_none() {
        let cdn_manifest_url =
            format!("https://github.com/{GITHUB_REPO}/releases/latest/download/latest.json");
        if let Ok(resp) = client.get(&cdn_manifest_url).send().await {
            if resp.status().is_success() {
                if let Ok(manifest) = resp.json::<serde_json::Value>().await {
                    rel_json = Some(manifest);
                }
            }
        }
    }

    let Some(rel) = rel_json else {
        return Err("Không truy cập được máy chủ GitHub Releases API".into());
    };

    let mut meta = parse_github_release_value(
        &rel,
        &current_ver,
        GITHUB_REPO,
        std::env::consts::OS,
        std::env::consts::ARCH,
    );

    if meta.expected_sha256.is_none() {
        if let (Some(ref s_url), Some(ref b_name)) = (&meta.sha256sums_url, &meta.asset_name) {
            if let Ok(resp) = client.get(s_url).send().await {
                if let Ok(txt) = resp.text().await {
                    meta.expected_sha256 = parse_sha256sums_for_asset(&txt, b_name);
                }
            }
        }
    }

    Ok(UpdateCheckResult {
        current_version: meta.current_version,
        latest_version: meta.latest_version,
        has_update: meta.has_update,
        release_title: meta.release_title,
        release_notes: meta.release_notes,
        html_url: meta.html_url,
        download_url: meta.download_url,
        asset_name: meta.asset_name,
        asset_size: meta.asset_size,
        expected_sha256: meta.expected_sha256,
        published_at: meta.published_at,
    })
}
