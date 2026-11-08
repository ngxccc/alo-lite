//! Sticker retrieval, sticker search/store, link preview parsing, and file downloads.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use super::CoreEngine;
use crate::core::events::{CoreEvent, CoreImageData};
use crate::protocol::messaging::links::parse_link_metadata;
use crate::protocol::messaging::stickers::{
    fetch_sticker_pack_items, fetch_sticker_store_packs, search_stickers_by_keyword,
};
use crate::storage::jxl_to_jpg_url;

impl CoreEngine {
    pub async fn fetch_sticker_image(&self, key: String, url: String) {
        let cache_dir = Path::new("stickers_cache");
        let _ = std::fs::create_dir_all(cache_dir);
        let cache_path = cache_dir.join(format!("{key}.png"));

        let (bytes_opt, from_network) = if let Ok(disk_bytes) = std::fs::read(&cache_path) {
            (Some(disk_bytes), false)
        } else if !url.starts_with("http://") && !url.starts_with("https://") {
            (std::fs::read(&url).ok(), false)
        } else {
            let client = { self.session.lock().await.clone().http };
            let fetch_url = jxl_to_jpg_url(&url);
            let body = match client.get(&fetch_url).send().await {
                Ok(resp) if resp.status().is_success() => resp.bytes().await.ok(),
                _ => None,
            };
            (body.map(|b| b.to_vec()), true)
        };
        let Some(bytes) = bytes_opt else {
            return;
        };

        let sink = Arc::clone(&self.sink);
        let _ = tokio::task::spawn_blocking(move || {
            let Ok(mut dyn_img) = image::load_from_memory(&bytes) else {
                if !from_network {
                    let _ = std::fs::remove_file(&cache_path);
                }
                return;
            };
            if from_network {
                let _ = std::fs::write(&cache_path, &bytes);
            }
            if !key.starts_with("ava_")
                && !key.starts_with("hd_")
                && (dyn_img.width() > 720 || dyn_img.height() > 720)
            {
                dyn_img = dyn_img.thumbnail(720, 720);
            }
            let mut rgba = dyn_img.to_rgba8();
            if key.starts_with("ava_") {
                let side = rgba.width().min(rgba.height()).clamp(32, 160);
                let x0 = (rgba.width() - side) / 2;
                let y0 = (rgba.height() - side) / 2;
                let cropped = image::imageops::crop_imm(&rgba, x0, y0, side, side).to_image();
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
                    let cov = (r + 0.5 - dist).clamp(0.0, 1.0);
                    px.0[3] = ((px.0[3] as f32) * cov).round() as u8;
                }
            }
            let width = rgba.width();
            let height = rgba.height();
            let pixels = rgba.into_raw();
            sink.emit(CoreEvent::StickerImageReady {
                key,
                image: CoreImageData {
                    width,
                    height,
                    rgba: pixels,
                },
            });
        })
        .await;
    }

    pub async fn fetch_sticker_store(&self) {
        let s = self.session.lock().await.clone();
        match fetch_sticker_store_packs(&s).await {
            Ok(packs) => self.emit(CoreEvent::StickerStoreLoaded(packs)),
            Err(e) => {
                crate::logger::log_error("core::fetch_sticker_store", &e.to_string());
                self.emit(CoreEvent::StickerStoreLoaded(Vec::new()));
                self.emit(CoreEvent::StatusUpdate(format!(
                    "Không tải được kho nhãn dán: {e}"
                )));
            }
        }
    }

    pub async fn fetch_sticker_pack(&self, cate_id: u64, name: String) {
        let s = self.session.lock().await.clone();
        match fetch_sticker_pack_items(&s, cate_id, &name).await {
            Ok(items) => self.emit(CoreEvent::StickerPackItemsLoaded {
                cate_id,
                name,
                items,
            }),
            Err(e) => {
                crate::logger::log_error("core::fetch_sticker_pack", &e.to_string());
                self.emit(CoreEvent::StatusUpdate(format!(
                    "Không tải được bộ nhãn \"{name}\": {e}"
                )));
            }
        }
    }

    pub async fn search_stickers(&self, keyword: String) {
        let s = self.session.lock().await.clone();
        match search_stickers_by_keyword(&s, &keyword).await {
            Ok(items) => {
                self.emit(CoreEvent::StickerSearchResults { keyword, items });
            }
            Err(e) => {
                crate::logger::log_error("core::search_stickers", &e.to_string());
                self.emit(CoreEvent::StickerSearchResults {
                    keyword,
                    items: Vec::new(),
                });
            }
        }
    }

    pub async fn suggest_composer_stickers(&self, keyword: String) {
        let s = self.session.lock().await.clone();
        let items = search_stickers_by_keyword(&s, &keyword)
            .await
            .unwrap_or_default();
        self.emit(CoreEvent::ComposerStickerSuggestions { keyword, items });
    }

    pub async fn parse_link_preview(&self, url: &str) {
        let s = self.session.lock().await.clone();
        if let Ok(meta) = parse_link_metadata(&s, url).await {
            self.emit(CoreEvent::LinkPreviewReady(meta));
        }
    }

    pub async fn download_file(&self, url: String, file_name: String, download_dir: String) {
        let client = { self.session.lock().await.clone().http };
        let res =
            download_attachment_with_original_name(&client, &url, &file_name, &download_dir).await;
        self.emit(CoreEvent::FileDownloaded(res));
    }
}

async fn download_attachment_with_original_name(
    client: &reqwest::Client,
    url: &str,
    file_name: &str,
    download_dir: &str,
) -> std::result::Result<(String, String), String> {
    let resp = client
        .get(url)
        .send()
        .await
        .map_err(|e| format!("Không thể kết nối tải tệp: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("Máy chủ trả về mã lỗi HTTP {}", resp.status()));
    }

    // Try to extract original filename from Content-Disposition header if available
    let mut resolved_name = file_name.to_string();
    if let Some(cd) = resp.headers().get(reqwest::header::CONTENT_DISPOSITION) {
        if let Ok(cd_str) = cd.to_str() {
            for part in cd_str.split(';') {
                let part = part.trim();
                if let Some(val) = part
                    .strip_prefix("filename*=")
                    .or_else(|| part.strip_prefix("filename="))
                {
                    let clean = val
                        .trim_matches('"')
                        .trim_matches('\'')
                        .split("''")
                        .last()
                        .unwrap_or(val)
                        .trim();
                    if !clean.is_empty() {
                        resolved_name = clean.to_string();
                        break;
                    }
                }
            }
        }
    }

    let bytes = resp
        .bytes()
        .await
        .map_err(|e| format!("Lỗi nhận dữ liệu tệp: {e}"))?;

    let dir = PathBuf::from(download_dir);
    let resolved_name_clone = resolved_name.clone();
    tokio::task::spawn_blocking(move || {
        let _ = std::fs::create_dir_all(&dir);
        let mut dest_path = dir.join(&resolved_name_clone);
        let mut counter = 1;
        let file_stem = Path::new(&resolved_name_clone)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("file");
        let extension = Path::new(&resolved_name_clone)
            .extension()
            .and_then(|e| e.to_str());

        while dest_path.exists() {
            let next_name = match extension {
                Some(ext) => format!("{file_stem}_{counter}.{ext}"),
                None => format!("{file_stem}_{counter}"),
            };
            dest_path = dir.join(next_name);
            counter += 1;
        }

        let final_name = dest_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(&resolved_name_clone)
            .to_string();

        std::fs::write(&dest_path, &bytes).map_err(|e| format!("Lỗi ghi tệp xuống máy: {e}"))?;
        Ok((final_name, dest_path.display().to_string()))
    })
    .await
    .map_err(|e| format!("Lỗi tác vụ lưu file: {e}"))?
}
