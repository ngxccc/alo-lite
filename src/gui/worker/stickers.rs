use super::*;

/// Immediately decode any disk-cached sticker thumbnails (`stickers_cache/stk_{cid}_{sid}.png`)
/// and push `GuiEvent::StickerImageReady` *before* sending sticker list metadata, returning any
/// missing `(key, url, cache_path)` entries for concurrent network prefetching.
pub fn preload_disk_sticker_thumbnails(
    items: &[crate::protocol::messaging::StickerItem],
    gui_tx: &GuiTx,
    limit: usize,
) -> Vec<(String, String, std::path::PathBuf)> {
    let cache_dir = std::path::Path::new("stickers_cache");
    let _ = std::fs::create_dir_all(cache_dir);

    let mut missing = Vec::new();
    for item in items.iter().take(limit) {
        let key = format!("stk_{}_{}", item.cate_id, item.sticker_id);
        let cache_path = cache_dir.join(format!("{key}.png"));
        if let Ok(disk_bytes) = std::fs::read(&cache_path) {
            if let Ok(dyn_img) = image::load_from_memory(&disk_bytes) {
                let rgba = dyn_img.to_rgba8();
                let size = [rgba.width() as usize, rgba.height() as usize];
                let pixels = rgba.as_flat_samples();
                let color_img = egui::ColorImage::from_rgba_unmultiplied(size, pixels.as_slice());
                let _ = gui_tx.send(GuiEvent::StickerImageReady {
                    key,
                    image: color_img,
                });
                continue;
            } else {
                let _ = std::fs::remove_file(&cache_path);
            }
        }
        let url = if item.cate_id == 0 && (1..=30).contains(&item.sticker_id) {
            format!(
                "https://stc-chat.zdn.vn/images/stickers/default/thumb/{}.png",
                item.sticker_id
            )
        } else {
            format!(
                "https://api.zaloapp.com/api/emoticon/sticker/webpc?eid={}&size=130",
                item.sticker_id
            )
        };
        missing.push((key, url, cache_path));
    }
    missing
}

/// Concurrently download, cache, and decode missing sticker thumbnails so the popover and search
/// grid render smoothly without waiting for frame-by-frame lazy requests.
async fn prefetch_missing_sticker_thumbnails(
    http: &reqwest::Client,
    missing: Vec<(String, String, std::path::PathBuf)>,
    gui_tx: &GuiTx,
    egui_ctx: &egui::Context,
) {
    if missing.is_empty() {
        return;
    }
    let futs = missing.into_iter().map(|(key, url, cache_path)| {
        let client = http.clone();
        let tx = gui_tx.clone();
        let ctx = egui_ctx.clone();
        async move {
            if let Ok(resp) = client.get(&url).send().await {
                if resp.status().is_success() {
                    if let Ok(b) = resp.bytes().await {
                        let vec = b.to_vec();
                        if let Ok(dyn_img) = image::load_from_memory(&vec) {
                            let _ = std::fs::write(&cache_path, &vec);
                            let rgba = dyn_img.to_rgba8();
                            let size = [rgba.width() as usize, rgba.height() as usize];
                            let pixels = rgba.as_flat_samples();
                            let color_img =
                                egui::ColorImage::from_rgba_unmultiplied(size, pixels.as_slice());
                            let _ = tx.send(GuiEvent::StickerImageReady {
                                key,
                                image: color_img,
                            });
                            ctx.request_repaint();
                        }
                    }
                }
            }
        }
    });
    futures_util::future::join_all(futs).await;
}

pub(super) fn handle_stickers_cmd(
    cmd: WorkerCmd,
    session: &std::sync::Arc<tokio::sync::Mutex<Session>>,
    gui_tx: &GuiTx,
    egui_ctx: &egui::Context,
) {
    match cmd {
        WorkerCmd::FetchStickerImage { key, url } => {
            let session_clone = session.clone();
            let gui_tx_clone = gui_tx.clone();
            let ctx_clone = egui_ctx.clone();
            if key.starts_with("stk_99901_") {
                if let Some(sid) = key
                    .strip_prefix("stk_99901_")
                    .and_then(|s| s.parse::<u64>().ok())
                {
                    if let Some((_lbl, png_bytes, _gif)) = get_ami_fat_cat_gif(sid) {
                        if let Ok(dyn_img) = image::load_from_memory(png_bytes) {
                            let rgba = dyn_img.to_rgba8();
                            let size = [rgba.width() as usize, rgba.height() as usize];
                            let pixels = rgba.as_flat_samples();
                            let color_img =
                                egui::ColorImage::from_rgba_unmultiplied(size, pixels.as_slice());
                            let _ = gui_tx.send(GuiEvent::StickerImageReady {
                                key,
                                image: color_img,
                            });
                            egui_ctx.request_repaint();
                        }
                        return;
                    }
                }
            }
            tokio::spawn(async move {
                let cache_dir = std::path::Path::new("stickers_cache");
                let _ = std::fs::create_dir_all(cache_dir);
                let cache_path = cache_dir.join(format!("{key}.png"));

                // Bytes come from the disk cache, a local file or the network. They
                // used to be decoded once to validate and again to use; now once, on
                // the blocking pool so a large photo cannot stall the 2-thread runtime.
                let (bytes_opt, from_network) = if let Ok(disk_bytes) = std::fs::read(&cache_path) {
                    (Some(disk_bytes), false)
                } else if !url.starts_with("http://") && !url.starts_with("https://") {
                    (std::fs::read(&url).ok(), false)
                } else {
                    let client = { session_clone.lock().await.clone().http };
                    let fetch_url = crate::store::jxl_to_jpg_url(&url);
                    let body = match client.get(&fetch_url).send().await {
                        Ok(resp) if resp.status().is_success() => resp.bytes().await.ok(),
                        _ => None,
                    };
                    (body.map(|b| b.to_vec()), true)
                };
                let Some(bytes) = bytes_opt else {
                    return;
                };

                let _ = tokio::task::spawn_blocking(move || {
                    let Ok(mut dyn_img) = image::load_from_memory(&bytes) else {
                        if !from_network {
                            // Corrupt cache entry: drop it so the next request refetches.
                            let _ = std::fs::remove_file(&cache_path);
                        }
                        return;
                    };
                    if from_network {
                        let _ = std::fs::write(&cache_path, &bytes);
                    }
                    // Bubbles draw images up to 360 pt wide; resize large photos to 720px max
                    // so inline thumbnails stay crisp on Retina/HiDPI displays (#72).
                    // The viewer's `hd_` textures stay full size.
                    if !key.starts_with("ava_")
                        && !key.starts_with("hd_")
                        && (dyn_img.width() > 720 || dyn_img.height() > 720)
                    {
                        dyn_img = dyn_img.thumbnail(720, 720);
                    }
                    {
                        let mut rgba = dyn_img.to_rgba8();
                        // Apply smooth anti-aliased circular alpha mask for avatar textures (`ava_*`)
                        if key.starts_with("ava_") {
                            let side = rgba.width().min(rgba.height()).clamp(32, 160);
                            let x0 = (rgba.width() - side) / 2;
                            let y0 = (rgba.height() - side) / 2;
                            let cropped =
                                image::imageops::crop_imm(&rgba, x0, y0, side, side).to_image();
                            rgba = if side > 128 {
                                image::imageops::resize(
                                    &cropped,
                                    128,
                                    128,
                                    image::imageops::FilterType::Triangle,
                                )
                            } else {
                                cropped
                            };
                            let w = rgba.width() as f32;
                            let h = rgba.height() as f32;
                            let cx = (w - 1.0) * 0.5;
                            let cy = (h - 1.0) * 0.5;
                            let r = w.min(h) * 0.5;
                            for (x, y, px) in rgba.enumerate_pixels_mut() {
                                let dx = x as f32 - cx;
                                let dy = y as f32 - cy;
                                let dist = (dx * dx + dy * dy).sqrt();
                                if dist > r {
                                    px[3] = 0;
                                } else if dist > r - 1.4 {
                                    let cov = ((r - dist) / 1.4).clamp(0.0, 1.0);
                                    px[3] = (px[3] as f32 * cov) as u8;
                                }
                            }
                        }
                        let size = [rgba.width() as usize, rgba.height() as usize];
                        let pixels = rgba.as_flat_samples();
                        let color_img =
                            egui::ColorImage::from_rgba_unmultiplied(size, pixels.as_slice());
                        let _ = gui_tx_clone.send(GuiEvent::StickerImageReady {
                            key,
                            image: color_img,
                        });
                        ctx_clone.request_repaint();
                    }
                })
                .await;
            });
        }
        WorkerCmd::FetchStickerStore => {
            let session_clone = session.clone();
            let gui_tx_clone = gui_tx.clone();
            let ctx_clone = egui_ctx.clone();
            tokio::spawn(async move {
                let s = session_clone.lock().await.clone();
                match crate::protocol::messaging::stickers::fetch_sticker_store_packs(&s).await {
                    Ok(packs) => {
                        let _ = gui_tx_clone.send(GuiEvent::StickerStoreLoaded(packs));
                    }
                    Err(e) => {
                        crate::logger::log_error("gui::fetch_sticker_store", &e.to_string());
                        let _ = gui_tx_clone.send(GuiEvent::StickerStoreLoaded(Vec::new()));
                        let _ = gui_tx_clone.send(GuiEvent::StatusUpdate(format!(
                            "Không tải được kho nhãn dán: {e}"
                        )));
                    }
                }
                ctx_clone.request_repaint();
            });
        }
        WorkerCmd::FetchStickerPack { cate_id, name } => {
            let session_clone = session.clone();
            let gui_tx_clone = gui_tx.clone();
            let ctx_clone = egui_ctx.clone();
            tokio::spawn(async move {
                let s = session_clone.lock().await.clone();
                match crate::protocol::messaging::stickers::fetch_sticker_pack_items(
                    &s, cate_id, &name,
                )
                .await
                {
                    Ok(items) => {
                        let _ = gui_tx_clone.send(GuiEvent::StickerPackItemsLoaded {
                            cate_id,
                            name,
                            items,
                        });
                    }
                    Err(e) => {
                        crate::logger::log_error("gui::fetch_sticker_pack", &e.to_string());
                        let _ = gui_tx_clone.send(GuiEvent::StatusUpdate(format!(
                            "Không tải được bộ nhãn \"{name}\": {e}"
                        )));
                    }
                }
                ctx_clone.request_repaint();
            });
        }
        WorkerCmd::SearchStickers { keyword } => {
            let session_clone = session.clone();
            let gui_tx_clone = gui_tx.clone();
            let ctx_clone = egui_ctx.clone();
            tokio::spawn(async move {
                let s = session_clone.lock().await.clone();
                match crate::protocol::messaging::stickers::search_stickers_by_keyword(&s, &keyword)
                    .await
                {
                    Ok(items) => {
                        let missing = preload_disk_sticker_thumbnails(&items, &gui_tx_clone, 24);
                        let _ =
                            gui_tx_clone.send(GuiEvent::StickerSearchResults { keyword, items });
                        ctx_clone.request_repaint();
                        prefetch_missing_sticker_thumbnails(
                            &s.http,
                            missing,
                            &gui_tx_clone,
                            &ctx_clone,
                        )
                        .await;
                    }
                    Err(e) => {
                        crate::logger::log_error("gui::search_stickers", &e.to_string());
                        let _ = gui_tx_clone.send(GuiEvent::StickerSearchResults {
                            keyword,
                            items: Vec::new(),
                        });
                        ctx_clone.request_repaint();
                    }
                }
            });
        }
        WorkerCmd::SuggestComposerStickers { keyword } => {
            let session_clone = session.clone();
            let gui_tx_clone = gui_tx.clone();
            let ctx_clone = egui_ctx.clone();
            tokio::spawn(async move {
                let s = session_clone.lock().await.clone();
                let items =
                    crate::protocol::messaging::stickers::search_stickers_by_keyword(&s, &keyword)
                        .await
                        .unwrap_or_default();
                let missing = preload_disk_sticker_thumbnails(&items, &gui_tx_clone, 16);
                let _ = gui_tx_clone.send(GuiEvent::ComposerStickerSuggestions { keyword, items });
                ctx_clone.request_repaint();
                prefetch_missing_sticker_thumbnails(&s.http, missing, &gui_tx_clone, &ctx_clone)
                    .await;
            });
        }
        _ => unreachable!(),
    }
}
