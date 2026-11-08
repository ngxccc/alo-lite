use std::path::PathBuf;
use std::sync::Arc;

use alolite::messaging::{ChatMessage, Thread};
use alolite::services::account_manager::AccountProfile;
use alolite::{
    AppSettings, CoreBootstrapSnapshot, CoreEngine, DevDiagnosticsSnapshot, ImportSummaryDto,
};
use tauri::State;
use tokio::sync::Mutex;

pub struct AppCoreState {
    pub engine: Arc<CoreEngine>,
    pub preloaded_threads: Mutex<Vec<Thread>>,
}

#[tauri::command]
pub async fn bootstrap_state(
    state: State<'_, AppCoreState>,
) -> Result<CoreBootstrapSnapshot, String> {
    let preloaded = {
        let mut guard = state.preloaded_threads.lock().await;
        std::mem::take(&mut *guard)
    };
    Ok(state.engine.bootstrap_snapshot(preloaded).await)
}

#[tauri::command]
pub async fn request_qr(state: State<'_, AppCoreState>) -> Result<(), String> {
    state.engine.request_qr().await;
    Ok(())
}

#[tauri::command]
pub async fn login_password(
    state: State<'_, AppCoreState>,
    phone: String,
    pass: String,
) -> Result<(), String> {
    state.engine.login_password(phone, pass).await;
    Ok(())
}

#[tauri::command]
pub async fn logout(state: State<'_, AppCoreState>) -> Result<(), String> {
    state.engine.logout().await;
    Ok(())
}

#[tauri::command]
pub async fn switch_account(
    state: State<'_, AppCoreState>,
    target_uin: String,
) -> Result<(), String> {
    state.engine.switch_account(&target_uin).await;
    Ok(())
}

#[tauri::command]
pub async fn add_new_account(state: State<'_, AppCoreState>) -> Result<(), String> {
    state.engine.add_new_account().await;
    Ok(())
}

#[tauri::command]
pub async fn get_saved_accounts(
    state: State<'_, AppCoreState>,
) -> Result<Vec<AccountProfile>, String> {
    Ok(state.engine.list_saved_accounts().await)
}

#[tauri::command]
pub async fn delete_saved_account(
    state: State<'_, AppCoreState>,
    uin: String,
) -> Result<(), String> {
    state.engine.delete_saved_account(&uin)
}

#[tauri::command]
pub async fn wipe_local_data(state: State<'_, AppCoreState>) -> Result<(), String> {
    state.engine.wipe_local_data().await;
    Ok(())
}

#[tauri::command]
pub async fn refresh_threads(state: State<'_, AppCoreState>, manual: bool) -> Result<(), String> {
    state.engine.refresh_threads(manual).await;
    Ok(())
}

#[tauri::command]
pub async fn set_known_threads(
    state: State<'_, AppCoreState>,
    threads: Vec<(String, String, bool)>,
) -> Result<(), String> {
    state.engine.set_known_threads(threads);
    Ok(())
}

#[tauri::command]
pub async fn persist_threads(
    state: State<'_, AppCoreState>,
    threads: Vec<Thread>,
) -> Result<(), String> {
    state.engine.persist_threads(threads).await;
    Ok(())
}

#[tauri::command]
pub async fn load_group_history(
    state: State<'_, AppCoreState>,
    group_id: String,
    is_group: bool,
    have: usize,
) -> Result<(), String> {
    state
        .engine
        .load_group_history(group_id, is_group, have)
        .await;
    Ok(())
}

#[tauri::command]
pub async fn sync_history(
    state: State<'_, AppCoreState>,
    threads: Vec<(String, String, bool)>,
) -> Result<(), String> {
    state.engine.sync_history(threads).await;
    Ok(())
}

#[tauri::command]
pub async fn cancel_cloud_sync(state: State<'_, AppCoreState>) -> Result<(), String> {
    state.engine.cancel_cloud_sync();
    Ok(())
}

#[tauri::command]
pub async fn sync_phone_messages(
    state: State<'_, AppCoreState>,
    threads: Vec<Thread>,
    send_phone_prompt: bool,
) -> Result<(), String> {
    state
        .engine
        .sync_phone_messages(threads, send_phone_prompt)
        .await;
    Ok(())
}

#[tauri::command]
pub async fn cancel_phone_sync(state: State<'_, AppCoreState>) -> Result<(), String> {
    state.engine.cancel_phone_sync();
    Ok(())
}

#[tauri::command]
pub async fn send_message(
    state: State<'_, AppCoreState>,
    to_id: String,
    text: String,
    is_group: bool,
) -> Result<(), String> {
    state
        .engine
        .send_message_or_link(to_id, text, is_group)
        .await;
    Ok(())
}

#[tauri::command]
pub async fn send_quote(
    state: State<'_, AppCoreState>,
    to_id: String,
    text: String,
    quoted: ChatMessage,
    is_group: bool,
) -> Result<(), String> {
    state.engine.send_quote(to_id, text, quoted, is_group).await;
    Ok(())
}

#[tauri::command]
pub async fn forward_message(
    state: State<'_, AppCoreState>,
    target_id: String,
    target_is_group: bool,
    msg: ChatMessage,
) -> Result<(), String> {
    state
        .engine
        .forward_message(target_id, target_is_group, msg)
        .await;
    Ok(())
}

#[tauri::command]
pub async fn react_message(
    state: State<'_, AppCoreState>,
    to_id: String,
    is_group: bool,
    msg: ChatMessage,
    reaction: String,
) -> Result<(), String> {
    state.engine.react(to_id, is_group, msg, reaction).await;
    Ok(())
}

#[tauri::command]
pub async fn delete_for_me(
    state: State<'_, AppCoreState>,
    to_id: String,
    is_group: bool,
    msg: ChatMessage,
) -> Result<(), String> {
    state.engine.delete_for_me(to_id, is_group, msg).await;
    Ok(())
}

#[tauri::command]
pub async fn recall_message(
    state: State<'_, AppCoreState>,
    to_id: String,
    is_group: bool,
    msg: ChatMessage,
) -> Result<(), String> {
    state.engine.recall_message(to_id, is_group, msg).await;
    Ok(())
}

#[tauri::command]
pub async fn send_sticker(
    state: State<'_, AppCoreState>,
    to_id: String,
    sticker_id: u64,
    cate_id: u64,
    sticker_type: u32,
    is_group: bool,
) -> Result<(), String> {
    state
        .engine
        .send_sticker(to_id, sticker_id, cate_id, sticker_type, is_group)
        .await;
    Ok(())
}

#[tauri::command]
pub async fn send_attachment_file(
    state: State<'_, AppCoreState>,
    to_id: String,
    is_group: bool,
    path: String,
    caption: String,
) -> Result<(), String> {
    state
        .engine
        .send_attachment_file(to_id, is_group, PathBuf::from(path), caption)
        .await;
    Ok(())
}

#[tauri::command]
pub async fn send_attachment_bytes(
    state: State<'_, AppCoreState>,
    to_id: String,
    is_group: bool,
    file_name: String,
    bytes: Vec<u8>,
    caption: String,
) -> Result<(), String> {
    state
        .engine
        .send_attachment_bytes(to_id, is_group, file_name, bytes, caption)
        .await;
    Ok(())
}

#[tauri::command]
pub async fn fetch_media_data(
    state: State<'_, AppCoreState>,
    key: String,
    url: String,
) -> Result<(), String> {
    state.engine.fetch_sticker_image(key, url).await;
    Ok(())
}

#[tauri::command]
pub async fn fetch_sticker_store(state: State<'_, AppCoreState>) -> Result<(), String> {
    state.engine.fetch_sticker_store().await;
    Ok(())
}

#[tauri::command]
pub async fn fetch_sticker_pack(
    state: State<'_, AppCoreState>,
    cate_id: u64,
    name: String,
) -> Result<(), String> {
    state.engine.fetch_sticker_pack(cate_id, name).await;
    Ok(())
}

#[tauri::command]
pub async fn search_stickers(
    state: State<'_, AppCoreState>,
    keyword: String,
) -> Result<(), String> {
    state.engine.search_stickers(keyword).await;
    Ok(())
}

#[tauri::command]
pub async fn suggest_composer_stickers(
    state: State<'_, AppCoreState>,
    keyword: String,
) -> Result<(), String> {
    state.engine.suggest_composer_stickers(keyword).await;
    Ok(())
}

#[tauri::command]
pub async fn parse_link_preview(state: State<'_, AppCoreState>, url: String) -> Result<(), String> {
    state.engine.parse_link_preview(&url).await;
    Ok(())
}

#[tauri::command]
pub async fn send_typing(
    state: State<'_, AppCoreState>,
    thread_id: String,
    is_group: bool,
) -> Result<(), String> {
    state.engine.send_typing(&thread_id, is_group).await;
    Ok(())
}

#[tauri::command]
pub async fn send_seen(
    state: State<'_, AppCoreState>,
    thread_id: String,
    is_group: bool,
    last_msg_id: String,
) -> Result<(), String> {
    state
        .engine
        .send_seen(&thread_id, is_group, &last_msg_id)
        .await;
    Ok(())
}

#[tauri::command]
pub async fn pin_thread(
    state: State<'_, AppCoreState>,
    thread_id: String,
    is_group: bool,
    pin: bool,
) -> Result<(), String> {
    state.engine.pin_thread(&thread_id, is_group, pin).await;
    Ok(())
}

#[tauri::command]
pub async fn mark_thread_unread(
    state: State<'_, AppCoreState>,
    thread_id: String,
    is_group: bool,
    unread: bool,
) -> Result<(), String> {
    state
        .engine
        .mark_thread_unread(&thread_id, is_group, unread)
        .await;
    Ok(())
}

#[tauri::command]
pub async fn mute_thread(
    state: State<'_, AppCoreState>,
    thread_id: String,
    is_group: bool,
    duration_sec: i64,
    mute: bool,
) -> Result<(), String> {
    state
        .engine
        .mute_thread(&thread_id, is_group, duration_sec, mute)
        .await;
    Ok(())
}

#[tauri::command]
pub async fn hide_thread(
    state: State<'_, AppCoreState>,
    thread_id: String,
    is_group: bool,
    hide: bool,
) -> Result<(), String> {
    state.engine.hide_thread(&thread_id, is_group, hide).await;
    Ok(())
}

#[tauri::command]
pub async fn delete_thread(
    state: State<'_, AppCoreState>,
    thread_id: String,
    is_group: bool,
    last_cli_msg_id: String,
) -> Result<(), String> {
    state
        .engine
        .delete_thread(&thread_id, is_group, &last_cli_msg_id)
        .await;
    Ok(())
}

#[tauri::command]
pub async fn lookup_phone(state: State<'_, AppCoreState>, phone: String) -> Result<(), String> {
    state.engine.lookup_phone(phone).await;
    Ok(())
}

#[tauri::command]
pub async fn add_friend(
    state: State<'_, AppCoreState>,
    user_id: String,
    user_name: String,
) -> Result<(), String> {
    state.engine.add_friend(user_id, user_name).await;
    Ok(())
}

#[tauri::command]
pub async fn check_friend_status(
    state: State<'_, AppCoreState>,
    user_id: String,
) -> Result<(), String> {
    state.engine.check_friend_status(user_id).await;
    Ok(())
}

#[tauri::command]
pub async fn accept_friend(
    state: State<'_, AppCoreState>,
    user_id: String,
    user_name: String,
) -> Result<(), String> {
    state.engine.accept_friend(user_id, user_name).await;
    Ok(())
}

#[tauri::command]
pub async fn reject_friend(
    state: State<'_, AppCoreState>,
    user_id: String,
    user_name: String,
) -> Result<(), String> {
    state.engine.reject_friend(user_id, user_name).await;
    Ok(())
}

#[tauri::command]
pub async fn undo_friend(
    state: State<'_, AppCoreState>,
    user_id: String,
    user_name: String,
) -> Result<(), String> {
    state.engine.undo_friend(user_id, user_name).await;
    Ok(())
}

#[tauri::command]
pub async fn save_settings(
    state: State<'_, AppCoreState>,
    settings: AppSettings,
) -> Result<AppSettings, String> {
    Ok(state.engine.save_settings(settings))
}

#[tauri::command]
pub async fn set_proxy(state: State<'_, AppCoreState>, proxy_url: String) -> Result<(), String> {
    state.engine.set_proxy(&proxy_url).await;
    Ok(())
}

#[tauri::command]
pub async fn download_file(
    state: State<'_, AppCoreState>,
    url: String,
    file_name: String,
    download_dir: String,
) -> Result<(), String> {
    state
        .engine
        .download_file(url, file_name, download_dir)
        .await;
    Ok(())
}

#[tauri::command]
pub async fn import_local_history(
    state: State<'_, AppCoreState>,
    path: String,
) -> Result<ImportSummaryDto, String> {
    state
        .engine
        .import_history_from_path(PathBuf::from(path))
        .await
}

#[tauri::command]
pub async fn auto_detect_import_history(
    state: State<'_, AppCoreState>,
) -> Result<ImportSummaryDto, String> {
    state.engine.auto_detect_and_import_history().await
}

#[tauri::command]
pub async fn check_for_updates(state: State<'_, AppCoreState>, silent: bool) -> Result<(), String> {
    state.engine.check_for_updates(silent).await;
    Ok(())
}

#[tauri::command]
pub async fn download_and_apply_update(
    state: State<'_, AppCoreState>,
    download_url: String,
    asset_name: String,
    target_version: String,
    expected_sha256: Option<String>,
) -> Result<(), String> {
    state
        .engine
        .download_and_apply_update(download_url, asset_name, target_version, expected_sha256)
        .await;
    Ok(())
}

#[tauri::command]
pub fn export_custom_stickers_tpl() -> Result<String, String> {
    alolite::core::stickers::export_custom_stickers_template("custom_stickers.json")
        .map(|_| "custom_stickers.json".to_string())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_autoreply_rules() -> Result<Vec<alolite::mcp::AutoReplyRule>, String> {
    Ok(alolite::mcp::load_autoreply_rules())
}

#[tauri::command]
pub fn save_autoreply_rules(rules: Vec<alolite::mcp::AutoReplyRule>) -> Result<(), String> {
    alolite::mcp::save_autoreply_rules(&rules).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_dev_diagnostics(
    state: State<'_, AppCoreState>,
) -> Result<DevDiagnosticsSnapshot, String> {
    Ok(state.engine.dev_diagnostics())
}

#[tauri::command]
pub fn open_path_in_os(path: String) -> Result<(), String> {
    let p = PathBuf::from(&path);
    let _ = std::fs::create_dir_all(&p);
    #[cfg(target_os = "windows")]
    let mut cmd = {
        let mut c = std::process::Command::new("explorer");
        c.arg(&p);
        c
    };
    #[cfg(target_os = "macos")]
    let mut cmd = {
        let mut c = std::process::Command::new("open");
        c.arg(&p);
        c
    };
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    let mut cmd = {
        let mut c = std::process::Command::new("xdg-open");
        c.arg(&p);
        c
    };
    cmd.spawn().map(|_| ()).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn spawn_multi_profile(profile: String, proxy_url: String) -> Result<(), String> {
    let clean = profile.trim();
    if clean.is_empty() {
        return Err("Tên profile không được để trống".into());
    }
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let mut cmd = std::process::Command::new(exe);
    cmd.arg("--profile").arg(clean);
    if !proxy_url.trim().is_empty() {
        cmd.arg("--proxy").arg(proxy_url.trim());
    }
    cmd.spawn().map(|_| ()).map_err(|e| e.to_string())
}
