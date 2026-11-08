use super::*;

const GITHUB_REPO: &str = "ngxccc/alolite";

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

    // Rate-limit resilient fallback: fetch static `latest.json` directly from GitHub Release CDN
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

    let mut meta = crate::updater::parse_github_release_value(
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
                    meta.expected_sha256 = crate::updater::parse_sha256sums_for_asset(&txt, b_name);
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

/// Stream-download the release asset from GitHub Releases with real-time progress reporting,
/// verify binary integrity (PE/ELF magic + SHA-256 checksum when provided) via [`crate::updater`],
/// and atomically replace the running executable in-place with automatic rollback on failure.
async fn apply_github_release_update(
    download_url: &str,
    asset_name: &str,
    target_version: &str,
    expected_sha256: Option<&str>,
    gui_tx: &GuiTx,
    ctx: &egui::Context,
) -> Result<bool, String> {
    let client = reqwest::Client::builder()
        .user_agent(format!("alolite/{target_version}"))
        .timeout(Duration::from_secs(240))
        .build()
        .map_err(|e| format!("Khởi tạo HTTP client thất bại: {e}"))?;

    let req = crate::updater::DownloadRequest {
        url: download_url,
        asset_name,
        expected_sha256,
        expected_size: None,
        limits: crate::updater::DownloadLimits::default(),
    };

    let bytes = crate::updater::download_release(&client, &req, |downloaded, total| {
        let _ = gui_tx.send(GuiEvent::UpdateDownloadProgress { downloaded, total });
        ctx.request_repaint();
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
                // On Windows a running .exe cannot be overwritten directly, so rename current exe
                // to `.exe.old` and atomically place the new portable binary at `exe_path`.
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
                if crate::updater::install_release(
                    &bytes,
                    &crate::updater::InstallTarget::Portable(exe_path),
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

    // Fallback / Installer mode: save to Downloads (or current directory) via transactional installer
    let dl_dir = std::env::var("USERPROFILE")
        .ok()
        .map(|p| std::path::PathBuf::from(p).join("Downloads"))
        .or_else(|| {
            std::env::var("HOME")
                .ok()
                .map(|h| std::path::PathBuf::from(h).join("Downloads"))
        })
        .unwrap_or_else(|| std::path::PathBuf::from("."));
    let _ = std::fs::create_dir_all(&dl_dir);
    let dest_path = dl_dir.join(asset_name);

    crate::updater::install_release(
        &bytes,
        &crate::updater::InstallTarget::Installer(dest_path),
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

/// Download a Zalo file attachment (`share.file` / `chat.file` / `/renewMedia`) and save it
/// inside `download_dir` using the sender's original filename instead of `renewMedia.doc` (#74).
async fn download_attachment_with_original_name(
    client: &reqwest::Client,
    url: &str,
    file_name: &str,
    download_dir: &str,
) -> Result<(String, String), String> {
    let clean_name = crate::protocol::messaging::attachments::resolve_attachment_file_name(
        Some(file_name),
        file_name,
        url,
    );
    let resp = client
        .get(url)
        .send()
        .await
        .map_err(|e| format!("Không thể tải tập tin: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("Máy chủ trả về mã HTTP {}", resp.status()));
    }
    let bytes = resp
        .bytes()
        .await
        .map_err(|e| format!("Lỗi đọc dữ liệu tập tin: {e}"))?;

    let target_dir = if !download_dir.trim().is_empty() {
        std::path::PathBuf::from(download_dir.trim())
    } else {
        std::env::var("USERPROFILE")
            .ok()
            .map(|p| {
                std::path::PathBuf::from(p)
                    .join("Downloads")
                    .join("Zalo Received Files")
            })
            .or_else(|| {
                std::env::var("HOME").ok().map(|h| {
                    std::path::PathBuf::from(h)
                        .join("Downloads")
                        .join("Zalo Received Files")
                })
            })
            .unwrap_or_else(|| std::path::PathBuf::from("Downloads"))
    };

    tokio::task::spawn_blocking(move || {
        std::fs::create_dir_all(&target_dir)
            .map_err(|e| format!("Không thể tạo thư mục lưu file: {e}"))?;

        let (stem, ext) = match clean_name.rfind('.') {
            Some(dot_idx) if dot_idx > 0 && dot_idx + 1 < clean_name.len() => {
                (&clean_name[..dot_idx], Some(&clean_name[dot_idx + 1..]))
            }
            _ => (clean_name.as_str(), None),
        };

        let mut dest_path = target_dir.join(&clean_name);
        let mut final_name = clean_name.clone();
        let mut suffix_idx = 1_u32;
        while dest_path.exists() {
            final_name = match ext {
                Some(e) => format!("{stem} ({suffix_idx}).{e}"),
                None => format!("{stem} ({suffix_idx})"),
            };
            dest_path = target_dir.join(&final_name);
            suffix_idx += 1;
        }

        std::fs::write(&dest_path, &bytes).map_err(|e| format!("Không thể ghi tập tin: {e}"))?;
        Ok((final_name, dest_path.display().to_string()))
    })
    .await
    .map_err(|e| format!("Lỗi tác vụ lưu file: {e}"))?
}

pub(super) fn handle_services_cmd(
    cmd: WorkerCmd,
    session: &std::sync::Arc<tokio::sync::Mutex<Session>>,
    call_bridge: &std::sync::Arc<crate::call::CallBridge>,
    gui_tx: &GuiTx,
    egui_ctx: &egui::Context,
) {
    match cmd {
        WorkerCmd::ReadNotificationAuthorization { request_id }
        | WorkerCmd::RequestNotificationAuthorization { request_id } => {
            let request = matches!(cmd, WorkerCmd::RequestNotificationAuthorization { .. });
            let tx = gui_tx.clone();
            tokio::spawn(async move {
                let result = if request {
                    crate::native_notifications::request_if_user_action(true).await
                } else {
                    crate::native_notifications::status().await
                };
                // Closing the UI receiver is normal during shutdown.
                let _ = tx.send(GuiEvent::NotificationAuthorization { request_id, result });
            });
        }
        WorkerCmd::ParseLinkPreview { url } => {
            let session_clone = session.clone();
            let gui_tx_clone = gui_tx.clone();
            let ctx_clone = egui_ctx.clone();
            tokio::spawn(async move {
                let s = session_clone.lock().await.clone();
                if let Ok(meta) =
                    crate::protocol::messaging::links::parse_link_metadata(&s, &url).await
                {
                    let _ = gui_tx_clone.send(GuiEvent::LinkPreviewReady(meta));
                    ctx_clone.request_repaint();
                }
            });
        }
        WorkerCmd::StartCall {
            partner_id,
            partner_name,
            partner_avatar,
            is_video,
            is_group,
        } => {
            let kind_label = if is_video {
                "Gọi video"
            } else {
                "Gọi thoại"
            };
            call_bridge.make_call(
                &partner_id,
                &partner_name,
                partner_avatar.as_deref(),
                is_video,
                is_group,
            );
            let status_msg = format!(
                                "📞 Đang khởi tạo {kind_label} trực tiếp (Pure-Rust) tới {partner_name} ({partner_id})…"
                            );
            let _ = gui_tx.send(GuiEvent::StatusUpdate(status_msg));
            egui_ctx.request_repaint();
        }
        WorkerCmd::AnswerCall {
            accept,
            cam_start_off,
        } => {
            call_bridge.answer_call(accept, cam_start_off);
            egui_ctx.request_repaint();
        }
        WorkerCmd::SetCallMediaState {
            is_on_mic,
            is_on_cam,
        } => {
            call_bridge.set_media_state(is_on_mic, is_on_cam);
            egui_ctx.request_repaint();
        }
        WorkerCmd::EndCall => {
            call_bridge.end_call();
            let _ = gui_tx.send(GuiEvent::CallStateChanged("free".into()));
            let _ = gui_tx.send(GuiEvent::StatusUpdate("Đã kết thúc cuộc gọi".into()));
            egui_ctx.request_repaint();
        }
        WorkerCmd::CheckForUpdates { silent } => {
            let gui_tx_clone = gui_tx.clone();
            let ctx_clone = egui_ctx.clone();
            tokio::spawn(async move {
                let res = fetch_github_latest_release().await;
                let _ = gui_tx_clone.send(GuiEvent::UpdateCheckFinished {
                    result: res,
                    silent,
                });
                ctx_clone.request_repaint();
            });
        }
        WorkerCmd::DownloadAndApplyUpdate {
            download_url,
            asset_name,
            target_version,
            expected_sha256,
        } => {
            let gui_tx_clone = gui_tx.clone();
            let ctx_clone = egui_ctx.clone();
            tokio::spawn(async move {
                let _ = gui_tx_clone.send(GuiEvent::UpdateDownloadStatus {
                    downloading: true,
                    message: format!("Đang tải bản cập nhật v{target_version}…"),
                });
                ctx_clone.request_repaint();
                match apply_github_release_update(
                    &download_url,
                    &asset_name,
                    &target_version,
                    expected_sha256.as_deref(),
                    &gui_tx_clone,
                    &ctx_clone,
                )
                .await
                {
                    Ok(restarting) => {
                        if restarting {
                            let _ = gui_tx_clone
                                .send(GuiEvent::UpdateAppliedAndRestarting(target_version));
                        } else {
                            let _ = gui_tx_clone.send(GuiEvent::UpdateDownloadStatus {
                                downloading: false,
                                message: format!(
                                    "Đã tải và mở bộ cài đặt {asset_name} (v{target_version})"
                                ),
                            });
                        }
                    }
                    Err(e) => {
                        let _ = gui_tx_clone.send(GuiEvent::UpdateDownloadStatus {
                            downloading: false,
                            message: format!("Lỗi cập nhật: {e}"),
                        });
                    }
                }
                ctx_clone.request_repaint();
            });
        }
        WorkerCmd::FetchTimelineFeed {
            friend_uids,
            shared_media_by_uid,
        } => {
            let session_clone = session.clone();
            let gui_tx_clone = gui_tx.clone();
            let ctx_clone = egui_ctx.clone();
            tokio::spawn(async move {
                let s = session_clone.lock().await.clone();
                let res = s
                    .fetch_timeline_feed(&friend_uids, &shared_media_by_uid)
                    .await
                    .map_err(|e| e.to_string());
                let _ = gui_tx_clone.send(GuiEvent::SocialTimelineLoaded(res));
                ctx_clone.request_repaint();
            });
        }
        WorkerCmd::SetProxy { proxy_url } => {
            let session_clone = session.clone();
            let gui_tx_clone = gui_tx.clone();
            let ctx_clone = egui_ctx.clone();
            tokio::spawn(async move {
                let res = {
                    let mut s = session_clone.lock().await;
                    let r = s.set_proxy(Some(&proxy_url)).map_err(|e| e.to_string());
                    if r.is_ok() {
                        let _ = s.save_to_file("session.json");
                    }
                    r
                };
                let _ = gui_tx_clone.send(GuiEvent::ProxyUpdated(res));
                ctx_clone.request_repaint();
            });
        }
        WorkerCmd::DownloadFile {
            url,
            file_name,
            download_dir,
        } => {
            let session_clone = session.clone();
            let gui_tx_clone = gui_tx.clone();
            let ctx_clone = egui_ctx.clone();
            tokio::spawn(async move {
                let client = { session_clone.lock().await.clone().http };
                let res = download_attachment_with_original_name(
                    &client,
                    &url,
                    &file_name,
                    &download_dir,
                )
                .await;
                let _ = gui_tx_clone.send(GuiEvent::FileDownloaded(res));
                ctx_clone.request_repaint();
            });
        }
        _ => unreachable!(),
    }
}
