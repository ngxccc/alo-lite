//! Bounded release downloads and transactional installation, independent of GUI and processes.
//!
//! Submodules isolate release discovery and GitHub API parsing ([`discovery`]),
//! bounded downloads and size/hash limits ([`download`]), transactional installation ([`install`]),
//! and platform-specific notification and launch integration ([`platform`]).

pub mod discovery;
pub mod download;
pub mod install;
pub mod platform;

#[cfg(test)]
#[path = "tests.rs"]
mod github_api_tests;

pub use discovery::{
    fetch_github_latest_release, is_newer_semver, parse_github_release_value,
    parse_sha256sums_for_asset, select_platform_asset_score, GitHubReleaseMetadata,
    UpdateCheckResult,
};
pub use download::{download_release, validate_asset_name, DownloadLimits, DownloadRequest};
pub use install::{
    install_release, stage_portable_release, DeferredPortableInstall, InstallTarget,
};
pub use platform::{apply_github_release_update, notify_os_new_update};
