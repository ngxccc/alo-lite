//! GitHub Releases API discovery, semantic version parsing, and asset selection.

use std::time::Duration;

use crate::protocol::wire::{opt_str, WireData};
use serde::{Deserialize, Serialize};

/// GitHub Releases API object (`/repos/{owner}/{repo}/releases/latest`) or a `latest.json` manifest.
///
/// # Wire Response Example
/// ```json
/// {
///   "tag_name": "v0.4.2",
///   "name": "alolite v0.4.2",
///   "body": "Ghi chú phát hành",
///   "html_url": "https://github.com/ngxccc/alolite/releases/tag/v0.4.2",
///   "published_at": "2026-11-10T03:32:08Z",
///   "assets": [
///     {
///       "name": "alolite-0.4.2-x86_64.AppImage",
///       "browser_download_url": "https://github.com/ngxc/…/alolite-0.4.2-x86_64.AppImage",
///       "size": 18432000,
///       "digest": "sha256:<redacted>"
///     }
///   ],
///   "draft": false
/// }
/// ```
///
/// # Core Transformation
/// Feeds [`GitHubReleaseMetadata`]: `tag_name` > `version`, `body` > `notes`, `html_url` >
/// `release_url`; a manifest may instead carry a single `sha256` with no `assets`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct GitHubReleaseWireData {
    #[serde(rename = "tag_name")]
    pub tag_name: Option<serde_json::Value>,
    #[serde(rename = "version")]
    pub version: Option<serde_json::Value>,
    #[serde(rename = "name")]
    pub name: Option<serde_json::Value>,
    #[serde(rename = "body")]
    pub body: Option<serde_json::Value>,
    #[serde(rename = "notes")]
    pub notes: Option<serde_json::Value>,
    #[serde(rename = "html_url")]
    pub html_url: Option<serde_json::Value>,
    #[serde(rename = "release_url")]
    pub release_url: Option<serde_json::Value>,
    #[serde(rename = "published_at")]
    pub published_at: Option<serde_json::Value>,
    #[serde(rename = "sha256")]
    pub sha256: Option<serde_json::Value>,
    #[serde(rename = "draft")]
    pub draft: Option<serde_json::Value>,
    #[serde(rename = "assets")]
    pub assets: Option<Vec<serde_json::Value>>,
}

impl GitHubReleaseWireData {
    /// Release tag (`tag_name` > `version`).
    pub fn tag(&self) -> Option<&str> {
        opt_str(self.tag_name.as_ref()).or_else(|| opt_str(self.version.as_ref()))
    }

    /// Release title (`name`), when non-empty after trimming.
    pub fn title(&self) -> Option<&str> {
        opt_str(self.name.as_ref()).filter(|s| !s.trim().is_empty())
    }

    /// Release notes (`body` > `notes`), trimmed.
    pub fn notes(&self) -> &str {
        opt_str(self.body.as_ref())
            .or_else(|| opt_str(self.notes.as_ref()))
            .unwrap_or("")
            .trim()
    }

    /// Human URL (`html_url` > `release_url`), when non-empty.
    pub fn html_url(&self) -> Option<&str> {
        opt_str(self.html_url.as_ref())
            .or_else(|| opt_str(self.release_url.as_ref()))
            .filter(|s| !s.is_empty())
    }

    /// Date part of `published_at`.
    pub fn published_date(&self) -> &str {
        opt_str(self.published_at.as_ref())
            .unwrap_or("")
            .split('T')
            .next()
            .unwrap_or("")
    }

    /// Whether the release is a draft.
    pub fn is_draft(&self) -> bool {
        self.draft
            .as_ref()
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false)
    }

    /// Release assets.
    pub fn assets(&self) -> &[serde_json::Value] {
        self.assets.as_deref().unwrap_or_default()
    }

    /// Whether the payload carried an `assets` key at all (even an empty array).
    pub fn has_assets_key(&self) -> bool {
        self.assets.is_some()
    }

    /// Manifest-level `sha256`, validated as 64 hex digits and lower-cased.
    pub fn manifest_sha256(&self) -> Option<String> {
        opt_str(self.sha256.as_ref())
            .filter(|h| h.len() == 64 && h.bytes().all(|b| b.is_ascii_hexdigit()))
            .map(|h| h.to_ascii_lowercase())
    }
}

/// One asset of a GitHub release.
///
/// # Wire Response Example
/// ```json
/// {
///   "name": "alolite-0.4.2-x86_64.AppImage",
///   "browser_download_url": "https://github.com/ngxc/…/alolite-0.4.2-x86_64.AppImage",
///   "size": 18432000,
///   "digest": "sha256:<redacted>",
///   "sha256": "<redacted>"
/// }
/// ```
///
/// # Core Transformation
/// Yields the best-scoring platform asset plus its checksum; `sha256` (manifest form) wins over
/// the API's `digest: "sha256:…"` form.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct GitHubAssetWireData {
    #[serde(rename = "name")]
    pub name: Option<serde_json::Value>,
    #[serde(rename = "browser_download_url")]
    pub browser_download_url: Option<serde_json::Value>,
    #[serde(rename = "size")]
    pub size: Option<serde_json::Value>,
    #[serde(rename = "digest")]
    pub digest: Option<serde_json::Value>,
    #[serde(rename = "sha256")]
    pub sha256: Option<serde_json::Value>,
}

impl GitHubAssetWireData {
    /// Asset file name (empty when absent).
    pub fn name(&self) -> &str {
        self.name
            .as_ref()
            .and_then(serde_json::Value::as_str)
            .unwrap_or("")
    }

    /// Download URL (empty when absent).
    pub fn download_url(&self) -> &str {
        self.browser_download_url
            .as_ref()
            .and_then(serde_json::Value::as_str)
            .unwrap_or("")
    }

    /// Asset size in bytes.
    pub fn size(&self) -> u64 {
        self.size
            .as_ref()
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(0)
    }

    /// Checksum (`sha256` > `digest` without its `sha256:` prefix), validated as 64 hex digits.
    pub fn sha256(&self) -> Option<String> {
        self.sha256
            .as_ref()
            .and_then(serde_json::Value::as_str)
            .or_else(|| {
                self.digest
                    .as_ref()
                    .and_then(serde_json::Value::as_str)
                    .and_then(|d| d.strip_prefix("sha256:"))
            })
            .filter(|h| h.len() == 64 && h.bytes().all(|b| b.is_ascii_hexdigit()))
            .map(|h| h.to_ascii_lowercase())
    }
}

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
#[derive(Debug, Clone, serde::Serialize, specta::Type)]
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
    let release = GitHubReleaseWireData::from_value(rel);
    let raw_tag = release.tag().unwrap_or(current_ver);
    let latest_ver = raw_tag.trim().trim_start_matches(['v', 'V']).to_string();
    let has_update = is_newer_semver(&latest_ver, current_ver);

    let release_title = release
        .title()
        .map(str::to_string)
        .unwrap_or_else(|| format!("alolite v{latest_ver}"));

    let release_notes = release.notes().to_string();

    let html_url = release
        .html_url()
        .map(str::to_string)
        .unwrap_or_else(|| format!("https://github.com/{repo_slug}/releases/tag/v{latest_ver}"));

    let published_at = release.published_date().to_string();

    let mut best_url: Option<String> = None;
    let mut best_name: Option<String> = None;
    let mut best_size: u64 = 0;
    let mut best_sha256: Option<String> = None;
    let mut best_score: i32 = 0;
    let mut sha256sums_url: Option<String> = None;

    for raw in release.assets() {
        let asset = GitHubAssetWireData::from_value(raw);
        let name = asset.name();
        let dl = asset.download_url();
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
            best_size = asset.size();
            best_sha256 = asset.sha256();
        }
    }

    // Legacy single-asset `latest.json` fallback when `assets` array is absent
    if best_url.is_none() && !release.has_assets_key() {
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
        best_sha256 = release.manifest_sha256();
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
                                .find(|r| !GitHubReleaseWireData::from_value(r).is_draft())
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
