//! End-to-end QR code login orchestration.

use anyhow::Result;

use super::builder::request_qr;
use super::login;
use super::polling::{confirm, wait_for_scan};
use super::render::{render_terminal, save_png};
use crate::session::Session;

/// The whole QR login: ticket → code on screen → wait → confirm → session.
pub async fn login_via_qr(
    s: &mut Session,
    language: &str,
) -> Result<crate::protocol::wire::ActionAck> {
    let mut q = request_qr(s, language).await?;

    match render_terminal(&q) {
        Ok(art) => println!("{art}"),
        Err(e) => {
            eprintln!("! không vẽ được QR trong terminal: {e}");
            let path = "qr.png";
            save_png(&q, path)?;
            println!("  ảnh QR đã lưu: {path}");
        }
    }
    if q.companion_state.is_some() {
        println!(
            "Quét mã QR bằng Zalo trên điện thoại (Yêu cầu xác thực sinh trắc học FaceID/Vân tay)…"
        );
    } else {
        println!("Quét mã bằng Zalo trên điện thoại…");
    }

    let scanned = wait_for_scan(s, &mut q).await?;
    if !scanned.display_name.is_empty() {
        println!("✓ Đã quét: {}", scanned.display_name);
    }

    let confirmed = confirm(s, &q).await?;
    confirmed.apply_to_session(s, &q);
    if let Some(cs) = &q.companion_state {
        if let Some(proof) = &cs.proof {
            println!(
                "✓ Đã xác thực sinh trắc học & hoàn tất liên kết thiết bị tin cậy! (Master UID: {}, Link ID: {})",
                proof.master_id, proof.link_id
            );
        }
    }
    println!("✓ Đăng nhập thành công trên điện thoại! Đang đồng bộ khoá phiên…");
    login::fetch_and_apply_login_info(s).await?;
    println!("✓ Đã nạp khoá phiên zpw_enk và cập nhật endpoints!");
    if let Err(e) = s.save_to_file("session.json") {
        eprintln!("! Lỗi lưu session.json: {e}");
    } else {
        println!("✓ Đã lưu phiên vào session.json (tự động khôi phục cho các lần chạy sau)");
    }

    Ok(crate::protocol::wire::ActionAck::ok())
}
