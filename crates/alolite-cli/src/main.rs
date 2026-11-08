//! AloLite headless CLI, MCP server, terminal auth, and desktop launcher.

#![allow(
    dead_code,
    clippy::too_many_arguments,
    clippy::type_complexity,
    clippy::field_reassign_with_default
)]

mod cli;
mod desktop_entry;

use anyhow::Result;
use cli::{setup_panic_hook, CliOptions};
use session::Session;

pub use alolite_core::{
    auth, clock, core, crypto, data_dir, endpoints, logger, mcp, messaging, param_cipher, registry,
    services, session, social, socket, store, sync, ui, wire,
};
pub const POLL_SECS: u64 = 4;

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();

    if args.iter().any(|a| a == "--help" || a == "-h") {
        cli::print_help();
        return Ok(());
    }
    // 1. Instant standalone CLI flags
    if args.iter().any(|a| a == "--version" || a == "-V") {
        println!("alolite v{}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }
    if args.iter().any(|a| a == "--check-update") {
        return cli::handle_check_update().await;
    }
    if args.iter().any(|a| a == "--install-desktop") {
        return desktop_entry::install_desktop_entry();
    }
    if args.iter().any(|a| a == "--uninstall-desktop") {
        return desktop_entry::uninstall_desktop_entry();
    }

    // 2. Initialize XDG storage directory (~/.local/share/alolite)
    let data_path = data_dir::initialize(&args)?;
    if args.iter().any(|a| a == "--print-data-dir") {
        println!("{}", data_path.display());
        return Ok(());
    }

    setup_panic_hook();
    let opts = CliOptions::parse(&args);

    if opts.force_logout {
        let _ = std::fs::remove_file("session.json");
        let _ = std::fs::remove_file("threads_cache.json");
        println!("✓ Đã đăng xuất và xóa phiên lưu trữ cũ (session.json, threads_cache.json).");
        if !opts.force_qr {
            return Ok(());
        }
    }

    logger::init(opts.dev_mode);
    if opts.dev_mode {
        println!("🔧 [DEV MODE] – Quét log, chẩn đoán giao thức và trace chi tiết được bật.");
    }

    let mut s = Session::new(opts.dev_mode, opts.dump, opts.dump_raw)?;
    s.load_or_init_device_identity("device.json");
    if let Some(ref p) = opts.cli_proxy {
        if let Some(norm) = s.set_proxy(Some(p))? {
            println!(
                "🌐 Đã kích hoạt Proxy cho phiên này: {}",
                session::redact_proxy_url(&norm)
            );
        }
    }

    // Attempt to restore persistent session from disk
    let session_restored = if !opts.force_logout {
        if s.load_from_file("session.json").is_ok() {
            if let Some(li) = &s.login {
                let _ = s.save_device_identity("device.json");
                let name = if li.display_name.is_empty() {
                    "Zalo User"
                } else {
                    &li.display_name
                };
                if opts.dev_mode {
                    println!(
                        "✓ Đã khôi phục phiên từ session.json: {name} (UIN: {})",
                        li.uin
                    );
                }
                true
            } else {
                false
            }
        } else {
            false
        }
    } else {
        false
    };

    // If `--mcp` is requested, run Model Context Protocol JSON-RPC server over stdio
    if opts.force_mcp {
        let engine = core::CoreEngine::headless(s, "zalo_history.db");
        return mcp::run_mcp_stdio_server(engine).await;
    }

    // If `--api-port <PORT>` is set without GUI (`--api-only`), run headless REST + MCP HTTP server.
    // When launched with GUI, the port is handed to `CoreEngine` inside the shared worker runtime.
    if let Some(port) = opts.api_port {
        println!("🌐 REST API & MCP HTTP server đang lắng nghe tại http://127.0.0.1:{port}/api/v1/status");
        if args.iter().any(|a| a == "--api-only") {
            let engine = core::CoreEngine::headless(s, "zalo_history.db");
            engine.start_background_loops(None);
            return mcp::run_local_http_server(engine, port).await;
        }
    }

    // If neither terminal login nor headless server was requested, launch the sibling Tauri desktop binary.
    if !opts.force_qr && !args.iter().any(|a| a.starts_with("--phone")) {
        return launch_desktop_app(&args);
    }

    // Terminal fallback login flow
    run_terminal_login_flow(s, opts.force_qr, session_restored, &args).await
}

fn launch_desktop_app(args: &[String]) -> Result<()> {
    let sibling = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|dir| dir.join("alolite-desktop")));
    let candidate = sibling
        .filter(|p| p.exists())
        .unwrap_or_else(|| std::path::PathBuf::from("alolite-desktop"));

    let status = std::process::Command::new(&candidate).args(args).status();
    match status {
        Ok(s) if s.success() => Ok(()),
        Ok(s) => anyhow::bail!("alolite-desktop kết thúc với mã thoát {:?}", s.code()),
        Err(err) => anyhow::bail!(
            "Không tìm thấy binary desktop '{}': {err}. Dùng `alolite --mcp`, `--api-only`, hoặc `--qr` cho chế độ dòng lệnh.",
            candidate.display()
        ),
    }
}
/// Fallback terminal authentication flow (when explicitly running with `--qr` or `--phone`).
async fn run_terminal_login_flow(
    mut s: Session,
    force_qr: bool,
    mut session_restored: bool,
    args: &[String],
) -> Result<()> {
    let _ = auth::login::fetch_and_apply_server_info(&mut s).await;

    println!("Khởi chạy chế độ dòng lệnh…");
    let mut server_ok = false;

    if session_restored && s.is_expired_marked() {
        println!(
            "Phiên trong session.json đã bị máy chủ từ chối ở lần chạy trước. Cần đăng nhập lại."
        );
        session_restored = false;
    } else if session_restored {
        match s.session_status().await {
            session::SessionStatus::Valid => server_ok = true,
            session::SessionStatus::Expired => {
                println!(
                    "Phiên đăng nhập đã hết hạn (máy chủ từ chối zpw_sek). Cần đăng nhập lại."
                );
                s.mark_expired();
                let _ = s.save_to_file("session.json");
                session_restored = false;
            }
            session::SessionStatus::Unreachable => {
                println!("Không kết nối được máy chủ Zalo — giữ nguyên phiên, chạy tiếp với dữ liệu cục bộ.");
            }
        }
    }

    let mut logged_in_now = false;
    if force_qr || !session_restored {
        if force_qr {
            println!("… Đang tạo mã QR");
            let _ = auth::qr::login_via_qr(&mut s, "vi").await?;
            logged_in_now = true;
        } else {
            let arg_val = |name: &str| -> Option<String> {
                args.iter()
                    .position(|a| a == name)
                    .and_then(|i| args.get(i + 1))
                    .cloned()
            };

            let phone = match arg_val("--phone") {
                Some(v) => v,
                None => ui::prompt("Số điện thoại: ")?,
            };
            let password = match arg_val("--pass") {
                Some(v) => v,
                None => ui::prompt_password("Mật khẩu: ")?,
            };
            let iso = match arg_val("--iso") {
                Some(v) => v,
                None => ui::prompt_default("Mã quốc gia", "VN")?,
            };

            if !phone.trim().is_empty() && !password.is_empty() {
                println!("… Đang đăng nhập mật khẩu");
                let v1 = args.iter().any(|a| a == "--v1");
                let _ = if v1 {
                    auth::login::login_by_password_v1(&mut s, phone.trim(), &password).await?
                } else {
                    auth::login::login_by_password(&mut s, phone.trim(), &password, iso.trim())
                        .await?
                };
                logged_in_now = true;
            }
        }
    }

    if logged_in_now {
        let _ = s.save_to_file("session.json");
        let _ = s.save_device_identity("device.json");
        server_ok = true;
    }

    let mut initial_threads = store::load_threads_cache();
    store::load_history_into_threads("zalo_history.db", &mut initial_threads, 200);

    if s.login.is_some() && server_ok {
        if let Ok(list) = social::friends::get_friends(&s).await {
            for ft in list {
                if let Some(ex) = initial_threads.iter_mut().find(|t| t.id == ft.id) {
                    if !ft.name.is_empty() {
                        ex.name = ft.name;
                    }
                    ex.is_group = ex.is_group || ft.is_group;
                } else {
                    initial_threads.push(ft);
                }
            }
        }
    }

    println!(
        "✓ Hoàn tất xử lý phiên. Hãy chạy lại không kèm cờ dòng lệnh để mở giao diện AloLite!"
    );
    Ok(())
}
