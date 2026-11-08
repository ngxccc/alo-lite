//! Terminal and image rendering utilities for QR login payloads and server PNGs.

use anyhow::{bail, Result};

use super::types::QrLogin;

/// Draw the QR as terminal art.
///
/// With `self_g = 1` the payload is rendered by `Dense1x2`, which packs two module rows into
/// one character cell so the code keeps its square proportions. With `self_g = 0` the code
/// only exists as the server's PNG, so its pixels are downscaled to the terminal grid and
/// drawn with half-block characters.
pub fn render_terminal(q: &QrLogin) -> Result<String> {
    use qrcode::render::unicode::Dense1x2;

    if q.payload().is_none() {
        return render_terminal_png(q, 88);
    }

    let payload = q
        .payload()
        .ok_or_else(|| anyhow::anyhow!("không có payload để vẽ"))?;
    let code = qrcode::QrCode::new(payload.as_bytes())?;
    Ok(code
        .render::<Dense1x2>()
        .quiet_zone(true)
        .module_dimensions(1, 1)
        .min_dimensions(25, 25)
        .build())
}

/// Half-block rendering of the server PNG, scaled with nearest-neighbour so the modules stay
/// crisp. `cols` is the maximum character width the terminal is assumed to have.
fn render_terminal_png(q: &QrLogin, cols: u32) -> Result<String> {
    let (w, h, rgba) = q.rgba()?;
    let scale = (w.max(1)).div_ceil(cols.max(1));
    let out_w = (w / scale).max(1) as usize;
    // Each cell holds two vertical pixels.
    let out_h_px = (h / scale).max(2);

    let mut art = String::new();
    let at = |x: u32, y: u32| -> bool {
        let sx = (x * scale).min(w - 1);
        let sy = (y * scale).min(h - 1);
        let idx = ((sy * w + sx) * 4) as usize;
        rgba.get(idx).copied().unwrap_or(255) < 128
    };

    let mut y = 0;
    while y + 1 < out_h_px {
        for x in 0..out_w as u32 {
            let top = at(x, y);
            let bottom = at(x, y + 1);
            art.push(match (top, bottom) {
                (true, true) => '█',
                (true, false) => '▀',
                (false, true) => '▄',
                (false, false) => ' ',
            });
        }
        art.push('\n');
        y += 2;
    }
    Ok(art)
}

/// Write the server-rendered PNG to disk, when there is one.
pub fn save_png(q: &QrLogin, path: &str) -> Result<()> {
    use base64::{engine::general_purpose::STANDARD as B64, Engine};
    let Some(b64) = q.base64_png.as_deref() else {
        bail!("không có ảnh PNG để lưu");
    };
    let raw = B64.decode(b64.trim())?;
    std::fs::write(path, raw)?;
    Ok(())
}
