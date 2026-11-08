//! Standalone real-time probe to monitor mobile message sync packets from phone.
//!
//! Run with: `cargo run --bin test_phone_sync`

use alolite::session::Session;
use alolite::{data_dir, messaging::polling, socket, store, sync};
use anyhow::{bail, Result};
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<()> {
    println!("============================================================");
    println!("     ALOLITE - PROBE ĐỒNG BỘ TIN NHẮN TỪ ĐIỆN THOẠI (REALTIME)");
    println!("============================================================\n");

    let args: Vec<String> = std::env::args().collect();
    let data_path = data_dir::initialize(&args)?;
    println!(
        "[1/5] Đang sử dụng thư mục dữ liệu: {}",
        data_path.display()
    );

    let mut s = Session::new(true, false, false)?;
    if let Err(e) = s.load_from_file("session.json") {
        bail!("Không tìm thấy hoặc không đọc được session.json tại {} ({e})! Hãy chạy GUI và đăng nhập trước.", data_path.display());
    }
    let li = s.login.as_ref().expect("Session thiếu thông tin đăng nhập");
    println!("      ✓ UIN: {}", li.uin);
    println!("      ✓ Tên: {}", li.display_name);
    println!("      ✓ Send2Me ID: {}", li.send2me_id);

    println!("\n[2/5] Đang kết nối WebSocket tới máy chủ Zalo...");
    let mut ws_rx = socket::spawn(s.clone());
    tokio::time::sleep(Duration::from_millis(1500)).await;
    println!("      ✓ WebSocket đã kết nối & xác thực thành công.");

    println!("\n[3/5] Dọn dẹp phiên đồng bộ cũ còn treo (clean_sync)...");
    let pub_key = sync::generate_sync_public_key();
    match sync::clean_sync(&s, &pub_key).await {
        Ok(v) => println!(
            "      ✓ clean_sync phản hồi: {}",
            serde_json::to_string(&v)?
        ),
        Err(e) => println!("      ⚠️ clean_sync: {e}"),
    }

    println!("\n[4/5] Đang phát yêu cầu đồng bộ mới tới điện thoại...");
    match sync::request_phone_sync(&s, &pub_key).await {
        Ok(v) => println!(
            "      ✓ HTTP pull_mobile_msg: {}",
            serde_json::to_string(&v)?
        ),
        Err(e) => println!("      ⚠️ HTTP pull_mobile_msg lỗi: {e}"),
    }

    println!("\n============================================================");
    println!(">>> BÂY GIỜ HÃY MỞ ZALO TRÊN ĐIỆN THOẠI VÀ BẤM 'ĐỒNG BỘ NGAY' <<<");
    println!(">>> Đang chờ tín hiệu phản hồi từ điện thoại...              <<<");
    println!("============================================================\n");

    let mut poll_state = polling::PollState::default();
    let mut poll_interval = tokio::time::interval(Duration::from_secs(2));
    poll_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

    let deadline = std::time::Instant::now() + Duration::from_secs(300);

    while std::time::Instant::now() < deadline {
        tokio::select! {
            Some(ev) = ws_rx.recv() => {
                let now = chrono::Local::now().format("%H:%M:%S%.3f");
                let data_node = ev.payload.get("data").unwrap_or(&ev.payload);

                let sync_evs = sync::extract_mobile_sync_controls(data_node);
                if !sync_evs.is_empty() {
                    println!("\n[{now}] 🎯 >>> PHÁT HIỆN SỰ KIỆN TỪ ĐIỆN THOẠI (WS cmd: {}): <<<", ev.cmd);
                    for sev in &sync_evs {
                        match sev {
                            sync::MobileSyncControlEvent::WaitingConfirmOnPhone { sync_id } => {
                                println!("  📲 Điện thoại đang hiện thông báo xác nhận (syncId: {sync_id})");
                            }
                            sync::MobileSyncControlEvent::UserConfirmed => {
                                println!("  ✅ Người dùng đã bấm ĐỒNG BỘ trên điện thoại!");
                            }
                            sync::MobileSyncControlEvent::UserRejected => {
                                println!("  ❌ Người dùng đã bấm TỪ CHỐI trên điện thoại!");
                            }
                            sync::MobileSyncControlEvent::MobileBusyRestoring => {
                                println!("  ⏳ Điện thoại báo đang bận / đang chuẩn bị gói dữ liệu (status 8 hoặc action 2)");
                            }
                            sync::MobileSyncControlEvent::BackupReady { url, file_name, encrypted_key, raw_uid, .. } => {
                                println!("  📦 ĐÃ NHẬN GÓI SAO LƯU: {file_name} (raw_uid: {raw_uid:?})");
                                println!("     URL tải về: {url}");
                                println!("     Khóa mã hóa: {}...", &encrypted_key[..32.min(encrypted_key.len())]);

                                println!("     -> Đang tiến hành tải và giải mã gói tin nhắn...");
                                match store::import_mobile_backup_container_with_uid(&s, url, encrypted_key, *raw_uid).await {
                                    Ok(count) => {
                                        println!("\n  THÀNH CÔNG! ĐÃ GIẢI MÃ {count} TIN NHẮN TỪ ĐIỆN THOẠI!");
                                    }
                                    Err(e) => {
                                        println!("     Lỗi giải mã: {e}");
                                    }
                                }
                            }
                            sync::MobileSyncControlEvent::SyncV2Batch { url, batch_type, sync_id, is_last } => {
                                println!("  📦 Gói Sync v2: batch_type={batch_type}, last={is_last}, sync_id={sync_id}");
                                println!("     URL: {url}");
                                if *batch_type == 2 {
                                    println!("     -> Đang tải và ghi nhận gói metadata...");
                                    match store::import_sync_v2_meta(&s, url).await {
                                        Ok(()) => println!("        ✓ Gói metadata đã tải thành công!"),
                                        Err(e) => println!("        ⚠️ Lỗi tải metadata: {e}"),
                                    }
                                } else if *batch_type == 1 {
                                    match store::import_sync_v2_messages(&s, url).await {
                                        Ok(count) => println!("\n  🎉🎉🎉 THÀNH CÔNG! ĐÃ ĐỒNG BỘ {count} TIN NHẮN SYNC V2!"),
                                        Err(e) => println!("     ⚠️ Lỗi Sync v2: {e}"),
                                    }
                                }
                            }
                            sync::MobileSyncControlEvent::TransferAfterLogin { temp_key } => {
                                println!("  🔑 Nhận temp_key: {}...", &temp_key[..16.min(temp_key.len())]);
                            }
                        }
                    }
                    println!("     Raw Payload: {}\n", serde_json::to_string(data_node)?);
                } else if ev.cmd == 590 || ev.cmd == 591 || ev.cmd == 592 || ev.cmd == 601 || ev.cmd == 602 || ev.cmd == 603 {
                    println!("[{now}] [WS MSG] cmd: {}, sub_cmd: {}, payload: {}", ev.cmd, ev.sub_cmd, serde_json::to_string(data_node)?);
                }
            }
            _ = poll_interval.tick() => {
                if let Ok(resp) = polling::get_chat_from_polling(&s, &mut poll_state).await {
                    let data_node = resp.get("data").unwrap_or(&resp);
                    let sync_evs = sync::extract_mobile_sync_controls(data_node);
                    if !sync_evs.is_empty() {
                        let now = chrono::Local::now().format("%H:%M:%S%.3f");
                        println!("\n[{now}] 🎯 >>> PHÁT HIỆN SỰ KIỆN TRÊN LP_CHAT POLLING: <<<");
                        for sev in &sync_evs {
                            println!("  -> {:?}", sev);
                        }
                        println!("     Raw: {}\n", serde_json::to_string(data_node)?);
                    }
                }
            }
        }
    }

    println!("\n[Hết thời gian chờ 300s (5 phút)].");
    Ok(())
}
