#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod events;

use std::sync::Arc;

use alolite_core::session::Session;
use alolite_core::{data_dir, logger, store, CoreEngine};
use commands::AppCoreState;
use events::TauriEventSink;
use tauri::Manager;
use tokio::sync::Mutex;

#[tokio::main(flavor = "multi_thread", worker_threads = 2)]
async fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    data_dir::initialize(&args)?;

    let dev_mode = args
        .iter()
        .any(|a| a == "--dev" || a == "-v" || a == "--verbose");
    let dump = args.iter().any(|a| a == "-d" || a == "--dump");
    let dump_raw = args.iter().any(|a| a == "--dump-raw");
    logger::init(dev_mode);

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

    let mut session = Session::new(dev_mode, dump, dump_raw)?;
    session.load_or_init_device_identity("device.json");
    if let Err(e) = session.load_from_file("session.json") {
        if std::path::Path::new("session.json").exists() {
            eprintln!("! nạp session.json thất bại: {e}");
        }
    }

    let mut initial_threads = store::load_threads_cache();
    let uin = session
        .login
        .as_ref()
        .map(|l| l.uin.as_str())
        .unwrap_or_default();
    let db_path = store::db_path_for_account(uin);
    store::load_history_into_threads(
        &db_path,
        &mut initial_threads,
        store::MessageStore::PRELOAD_MSGS,
    );

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .setup(move |app| {
            let sink = Arc::new(TauriEventSink {
                app: app.handle().clone(),
            });
            let engine = CoreEngine::new(session, sink);
            engine.start_background_loops(api_port);
            app.manage(AppCoreState {
                engine,
                preloaded_threads: Mutex::new(initial_threads),
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::bootstrap_state,
            commands::request_qr,
            commands::login_password,
            commands::refresh_captcha,
            commands::logout,
            commands::switch_account,
            commands::add_new_account,
            commands::get_saved_accounts,
            commands::delete_saved_account,
            commands::wipe_local_data,
            commands::refresh_threads,
            commands::set_known_threads,
            commands::persist_threads,
            commands::load_group_history,
            commands::sync_history,
            commands::cancel_cloud_sync,
            commands::sync_phone_messages,
            commands::cancel_phone_sync,
            commands::send_message,
            commands::send_quote,
            commands::forward_message,
            commands::react_message,
            commands::delete_for_me,
            commands::recall_message,
            commands::send_sticker,
            commands::send_attachment_file,
            commands::send_attachment_bytes,
            commands::fetch_media_data,
            commands::fetch_sticker_store,
            commands::fetch_sticker_pack,
            commands::search_stickers,
            commands::suggest_composer_stickers,
            commands::parse_link_preview,
            commands::send_typing,
            commands::send_seen,
            commands::pin_thread,
            commands::mark_thread_unread,
            commands::mute_thread,
            commands::hide_thread,
            commands::delete_thread,
            commands::lookup_phone,
            commands::add_friend,
            commands::check_friend_status,
            commands::accept_friend,
            commands::reject_friend,
            commands::undo_friend,
            commands::save_settings,
            commands::download_file,
            commands::import_local_history,
            commands::auto_detect_import_history,
            commands::check_for_updates,
            commands::download_and_apply_update,
            commands::export_custom_stickers_tpl,
            commands::get_autoreply_rules,
            commands::save_autoreply_rules,
            commands::get_dev_diagnostics,
            commands::open_path_in_os,
            commands::spawn_multi_profile,
        ])
        .run(tauri::generate_context!())
        .map_err(|e| anyhow::anyhow!("Không thể khởi chạy AloLite Tauri: {e}"))?;

    Ok(())
}
