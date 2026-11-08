//! CLI options parsing and command handlers for Alolite.

use anyhow::Result;

/// Parsed command line flags and options.
#[derive(Clone, Debug, Default)]
pub struct CliOptions {
    pub dev_mode: bool,
    pub dump: bool,
    pub dump_raw: bool,
    pub force_gui: bool,
    pub force_qr: bool,
    pub force_logout: bool,
    pub force_mcp: bool,
    pub api_port: Option<u16>,
    pub cli_proxy: Option<String>,
    pub maximized: bool,
}

impl CliOptions {
    /// Parse flags from process argument strings.
    pub fn parse(args: &[String]) -> Self {
        let dev_mode = args
            .iter()
            .any(|a| a == "--dev" || a == "-v" || a == "--verbose");
        let dump = args.iter().any(|a| a == "-d" || a == "--dump");
        let dump_raw = args.iter().any(|a| a == "--dump-raw");
        let force_gui = args.iter().any(|a| a == "--gui");
        let force_qr = args.iter().any(|a| a == "--qr");
        let force_logout = args.iter().any(|a| a == "--logout");
        let force_mcp = args.iter().any(|a| a == "--mcp");
        let maximized = args.iter().any(|a| a == "--maximized");
        let api_port = args
            .iter()
            .position(|a| a == "--api-port")
            .and_then(|i| args.get(i + 1))
            .and_then(|v| v.parse::<u16>().ok())
            .or_else(|| {
                std::env::var("ALOLITE_API_PORT")
                    .ok()
                    .and_then(|v| v.parse::<u16>().ok())
            });

        let cli_proxy = args
            .iter()
            .position(|a| a == "--proxy")
            .and_then(|i| args.get(i + 1).cloned())
            .or_else(|| {
                args.iter()
                    .find_map(|a| a.strip_prefix("--proxy=").map(str::to_string))
            });

        Self {
            dev_mode,
            dump,
            dump_raw,
            force_gui,
            force_qr,
            force_logout,
            force_mcp,
            api_port,
            cli_proxy,
            maximized,
        }
    }
}

/// Displays formatted CLI help message.
pub fn print_help() {
    println!("AloLite v{}", env!("CARGO_PKG_VERSION"));
    println!("Trình nhắn tin Zalo native nhẹ, bảo mật và hiệu năng cao dành cho Linux.\n");
    println!("CÁCH DÙNG:");
    println!("    alolite [CỜ] [TÙY CHỌN]\n");
    println!("CÁC CỜ LỆNH:");
    println!("    -h, --help               Hiển thị thông tin trợ giúp này và thoát");
    println!("    -V, --version            Hiển thị phiên bản ứng dụng");
    println!("        --gui                Khởi chạy giao diện đồ họa GUI (mặc định)");
    println!("        --qr                 Khởi chạy luồng quét mã QR trên terminal");
    println!("        --logout             Đăng xuất và xóa session.json cũ trên máy");
    println!(
        "        --dev, -v, --verbose Bật chế độ lập trình viên (hiển thị log chẩn đoán chi tiết)"
    );
    println!("        --check-update       Kiểm tra bản phát hành mới nhất từ GitHub");
    println!("        --install-desktop    Cài đặt desktop entry và icon ứng dụng vào ~/.local");
    println!("        --uninstall-desktop  Gỡ bỏ desktop entry và icon khỏi ~/.local");
    println!("        --print-data-dir     In đường dẫn thư mục lưu trữ XDG và thoát");
    println!("        --maximized          Khởi chạy cửa sổ ở chế độ phóng to (maximized)");
    println!("        --mcp                Khởi chạy Model Context Protocol JSON-RPC qua stdio\n");
    println!("TÙY CHỌN:");
    println!("        --proxy <URL>        Thiết lập HTTP/HTTPS/SOCKS5 proxy cho phiên");
    println!(
        "        --api-port <CỔNG>    Mở cổng REST API & MCP HTTP server (ví dụ: --api-port 18128)"
    );
}

/// Sets up a panic hook that logs unhandled panics to `crash.log`.
pub fn setup_panic_hook() {
    std::panic::set_hook(Box::new(|info| {
        use std::io::Write;
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let msg = format!("[{ts}] PANIC: {info}\n");
        eprint!("{msg}");
        if let Ok(mut f) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open("crash.log")
        {
            let _ = f.write_all(msg.as_bytes());
        }
    }));
}

/// Check for updates against GitHub Releases API and display information.
pub async fn handle_check_update() -> Result<()> {
    match crate::gui::check_github_release_cli().await {
        Ok(info) => {
            if info.has_update {
                println!(
                    "Có bản cập nhật mới v{} trên GitHub (hiện tại: v{})",
                    info.latest_version, info.current_version
                );
                println!("   Tiêu đề: {}", info.release_title);
                println!("   Trang phát hành: {}", info.html_url);
                if let Some(ref dl) = info.download_url {
                    println!("   Tải xuống: {dl}");
                }
                if !info.release_notes.trim().is_empty() {
                    println!("\n📋 Nội dung cập nhật & sửa lỗi (GitHub Releases API):");
                    for line in info.release_notes.lines() {
                        let trimmed = line.trim();
                        if trimmed.starts_with("- ")
                            || trimmed.starts_with("* ")
                            || trimmed.starts_with("### ")
                        {
                            println!("   {trimmed}");
                        }
                    }
                }
                crate::updater::notify_os_new_update(
                    &info.latest_version,
                    &info.current_version,
                    &info.release_title,
                );
            } else {
                println!(
                    "Bạn đang sử dụng phiên bản mới nhất (v{})",
                    info.current_version
                );
            }
            Ok(())
        }
        Err(e) => {
            eprintln!("Không kiểm tra được bản cập nhật từ GitHub API: {e}");
            std::process::exit(1);
        }
    }
}
