use eframe::egui::{self, Color32};

pub(crate) fn build_blurred_backdrop_image(
    src: &egui::ColorImage,
    is_dark: bool,
) -> egui::ColorImage {
    let sw = src.size[0].max(1);
    let sh = src.size[1].max(1);
    let scale = 3usize;
    let dw = (sw / scale).max(16);
    let dh = (sh / scale).max(16);

    let mut buf: Vec<[u32; 3]> = vec![[0, 0, 0]; dw * dh];
    for y in 0..dh {
        let sy = (y * scale).min(sh - 1);
        for x in 0..dw {
            let sx = (x * scale).min(sw - 1);
            let px = src.pixels[sy * sw + sx];
            buf[y * dw + x] = [px.r() as u32, px.g() as u32, px.b() as u32];
        }
    }

    let radius = 4isize;
    let mut tmp = vec![[0u32; 3]; dw * dh];
    for _ in 0..3 {
        // Horizontal pass
        for y in 0..dh {
            let row_off = y * dw;
            for x in 0..dw {
                let mut sr = 0u32;
                let mut sg = 0u32;
                let mut sb = 0u32;
                let mut cnt = 0u32;
                for dx in -radius..=radius {
                    let nx = (x as isize + dx).clamp(0, dw as isize - 1) as usize;
                    let c = buf[row_off + nx];
                    sr += c[0];
                    sg += c[1];
                    sb += c[2];
                    cnt += 1;
                }
                tmp[row_off + x] = [sr / cnt, sg / cnt, sb / cnt];
            }
        }
        // Vertical pass
        for y in 0..dh {
            for x in 0..dw {
                let mut sr = 0u32;
                let mut sg = 0u32;
                let mut sb = 0u32;
                let mut cnt = 0u32;
                for dy in -radius..=radius {
                    let ny = (y as isize + dy).clamp(0, dh as isize - 1) as usize;
                    let c = tmp[ny * dw + x];
                    sr += c[0];
                    sg += c[1];
                    sb += c[2];
                    cnt += 1;
                }
                buf[y * dw + x] = [sr / cnt, sg / cnt, sb / cnt];
            }
        }
    }

    let (tr, tg, tb, alpha_num) = if is_dark {
        (8u32, 11u32, 18u32, 56u32) // 56% dark frosted tint
    } else {
        (15u32, 23u32, 42u32, 44u32) // 44% slate frosted tint
    };
    let keep = 100u32 - alpha_num;
    let mut out_pixels = Vec::with_capacity(dw * dh);
    for c in buf {
        let r = ((c[0] * keep + tr * alpha_num) / 100).min(255) as u8;
        let g = ((c[1] * keep + tg * alpha_num) / 100).min(255) as u8;
        let b = ((c[2] * keep + tb * alpha_num) / 100).min(255) as u8;
        out_pixels.push(Color32::from_rgb(r, g, b));
    }
    egui::ColorImage {
        size: [dw, dh],
        pixels: out_pixels,
    }
}
