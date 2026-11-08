//! Platform-specific native notification dispatch and executable installation launcher.

use std::fs;
use std::path::PathBuf;
use std::time::Duration;

use super::download::{download_release, DownloadLimits, DownloadRequest};
use super::install::{install_release, InstallTarget};

/// Dispatch a non-blocking native OS desktop notification announcing a new GitHub release.
pub fn notify_os_new_update(latest_version: &str, current_version: &str, release_title: &str) {
    let title = format!("alolite — Có bản cập nhật mới v{latest_version}");
    let body = if release_title.trim().is_empty() {
        format!("Phiên bản mới v{latest_version} đã sẵn sàng trên GitHub (hiện tại: v{current_version}). Mở ứng dụng để cập nhật tự động.")
    } else {
        format!("{release_title} đã sẵn sàng trên GitHub (hiện tại: v{current_version}). Mở ứng dụng để cập nhật ngay!")
    };

    #[cfg(target_os = "macos")]
    {
        let esc_title = title.replace('\\', "\\\\").replace('"', "\\\"");
        let esc_body = body.replace('\\', "\\\\").replace('"', "\\\"");
        let script = format!(
            "display notification \"{esc_body}\" with title \"{esc_title}\" sound name \"Glass\""
        );
        let _ = std::process::Command::new("osascript")
            .args(["-e", &script])
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn();
    }

    #[cfg(target_os = "linux")]
    {
        let _ = std::process::Command::new("notify-send")
            .args([
                "-a",
                "alolite",
                "-i",
                "software-update-available",
                &title,
                &body,
            ])
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn();
    }

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        let esc_title = title.replace('\'', "''");
        let esc_body = body.replace('\'', "''");
        let ps_script = format!(
            "Add-Type -AssemblyName System.Windows.Forms; \
             $n = New-Object System.Windows.Forms.NotifyIcon; \
             $n.Icon = [System.Drawing.SystemIcons]::Information; \
             $n.BalloonTipTitle = '{esc_title}'; \
             $n.BalloonTipText = '{esc_body}'; \
             $n.Visible = $true; \
             $n.ShowBalloonTip(6000); \
             Start-Sleep -Seconds 6; \
             $n.Dispose();"
        );
        let _ = std::process::Command::new("powershell")
            .args([
                "-NoProfile",
                "-NonInteractive",
                "-WindowStyle",
                "Hidden",
                "-Command",
                &ps_script,
            ])
            .creation_flags(CREATE_NO_WINDOW)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn();
    }
}

/// Download, verify, and atomically install/launch a release asset while reporting byte progress.
pub async fn apply_github_release_update<F>(
    download_url: &str,
    asset_name: &str,
    target_version: &str,
    expected_sha256: Option<&str>,
    mut on_progress: F,
) -> Result<bool, String>
where
    F: FnMut(u64, u64),
{
    let client = reqwest::Client::builder()
        .user_agent(format!("alolite/{target_version}"))
        .timeout(Duration::from_secs(240))
        .build()
        .map_err(|e| format!("Khởi tạo HTTP client thất bại: {e}"))?;

    let req = DownloadRequest {
        url: download_url,
        asset_name,
        expected_sha256,
        expected_size: None,
        limits: DownloadLimits::default(),
    };

    let bytes = download_release(&client, &req, |downloaded, total| {
        on_progress(downloaded, total);
    })
    .await?;

    let name_low = asset_name.to_lowercase();
    let is_portable_bin =
        (cfg!(target_os = "windows") && name_low.ends_with(".exe") && !name_low.contains("setup"))
            || (cfg!(target_os = "linux") && name_low.ends_with(".appimage"));

    if is_portable_bin {
        if let Ok(exe_path) = std::env::current_exe() {
            #[cfg(target_os = "windows")]
            {
                let new_path = exe_path.with_extension("exe.new");
                let old_path = exe_path.with_extension("exe.old");
                if std::fs::write(&new_path, &bytes).is_ok() {
                    let _ = std::fs::remove_file(&old_path);
                    if std::fs::rename(&exe_path, &old_path).is_ok() {
                        if std::fs::rename(&new_path, &exe_path).is_ok() {
                            let args: Vec<String> = std::env::args().skip(1).collect();
                            if std::process::Command::new(&exe_path)
                                .args(&args)
                                .spawn()
                                .is_ok()
                            {
                                return Ok(true);
                            }
                            let _ = std::fs::rename(&old_path, &exe_path);
                        } else {
                            let _ = std::fs::rename(&old_path, &exe_path);
                        }
                    }
                    let _ = std::fs::remove_file(&new_path);
                }
            }
            #[cfg(not(target_os = "windows"))]
            {
                let args: Vec<String> = std::env::args().skip(1).collect();
                if install_release(
                    &bytes,
                    &InstallTarget::Portable(exe_path),
                    |installed_path| {
                        std::process::Command::new(installed_path)
                            .args(&args)
                            .spawn()
                            .map(|_| ())
                            .map_err(|e| e.to_string())
                    },
                )
                .is_ok()
                {
                    return Ok(true);
                }
            }
        }
    }

    let dl_dir = std::env::var("USERPROFILE")
        .ok()
        .map(|p| PathBuf::from(p).join("Downloads"))
        .or_else(|| {
            std::env::var("HOME")
                .ok()
                .map(|h| PathBuf::from(h).join("Downloads"))
        })
        .unwrap_or_else(|| PathBuf::from("."));
    let _ = fs::create_dir_all(&dl_dir);
    let dest_path = dl_dir.join(asset_name);

    install_release(
        &bytes,
        &InstallTarget::Installer(dest_path),
        |installer_path| {
            #[cfg(target_os = "windows")]
            {
                std::process::Command::new(installer_path)
                    .spawn()
                    .map(|_| ())
                    .map_err(|e| e.to_string())
            }
            #[cfg(target_os = "macos")]
            {
                std::process::Command::new("open")
                    .arg(installer_path)
                    .spawn()
                    .map(|_| ())
                    .map_err(|e| e.to_string())
            }
            #[cfg(not(any(target_os = "windows", target_os = "macos")))]
            {
                std::process::Command::new("xdg-open")
                    .arg(installer_path)
                    .spawn()
                    .map(|_| ())
                    .map_err(|e| e.to_string())
            }
        },
    )?;
    Ok(false)
}
