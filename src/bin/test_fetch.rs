//! Standalone tool to test fetching real data from Zalo using a saved `session.json`.
//!
//! Run with: `cargo run --release --bin test_fetch`
//! Optional: `cargo run --release --bin test_fetch -- --send <TO_ID> <TEXT>`

use alolite::session::Session;
use anyhow::{bail, Result};

#[tokio::main]
async fn main() -> Result<()> {
    println!("=== ZALO REAL DATA FETCH TEST ===");

    let args: Vec<String> = std::env::args().collect();
    let _ = alolite::data_dir::initialize(&args)?;

    let mut s = Session::new(false, false, false)?;

    if let Err(e) = s.load_from_file("session.json") {
        bail!("Không tìm thấy hoặc không đọc được session.json ({e}). Vui lòng quét QR đăng nhập trước!");
    }
    let li = s.login.as_ref().unwrap();
    println!("✓ Đã tải session.json:");
    println!("  UIN:          {}", li.uin);
    println!(
        "  Tên:          {}",
        if li.display_name.is_empty() {
            "(chưa có)"
        } else {
            &li.display_name
        }
    );
    println!(
        "  zpw_enk:      {}",
        alolite::session::redact_secret(&li.zpw_enk)
    );
    println!(
        "  zpw_sek:      {}",
        alolite::session::redact_secret(&li.zpw_sek)
    );
    println!("  Send2Me ID:   {}", li.send2me_id);
    println!("  Profile Host: {}", s.domains.profile);
    println!("  Convers Host: {}", s.domains.convers_api);
    println!("  Chat Host:    {}", s.domains.chat);

    // 1. Test getfriends
    println!("\n--- [1] Thử gọi /api/social/friend/getfriends ---");
    let mut friends = match alolite::social::friends::get_friends(&s).await {
        Ok(f) => {
            println!("✓ getfriends THÀNH CÔNG! Số lượng: {}", f.len());
            for (i, item) in f.iter().enumerate() {
                println!(
                    "  [{i}] ID: {:<20} Tên: {:<25} Tin nhắn: {}",
                    item.id, item.name, item.preview
                );
            }
            f
        }
        Err(e) => {
            println!(
                "! getfriends lỗi ({e}) -> Khởi tạo danh sách mặc định từ session để test tiếp..."
            );
            let mut list = Vec::new();
            if !li.send2me_id.is_empty() {
                list.push(alolite::messaging::Thread {
                    presentation: Default::default(),
                    unread: Default::default(),
                    id: li.send2me_id.clone(),
                    name: "Cloud của tôi (Lưu trữ)".into(),
                    preview: String::new(),
                    time: "12:00".into(),
                    avatar: None,
                    is_group: false,
                    messages: Vec::new(),
                });
            }
            list.push(alolite::messaging::Thread {
                presentation: Default::default(),
                unread: Default::default(),
                id: "1000000000000000001".into(),
                name: "Zalo".into(),
                preview: String::new(),
                time: "12:00".into(),
                avatar: None,
                is_group: false,
                messages: Vec::new(),
            });
            list
        }
    };

    // 2. Test get-last-msgs
    println!("\n--- [2] Thử gọi poll_new_messages (lp_chat + get-last-msgs + Cloud của tôi) ---");
    let mut ps = alolite::messaging::PollState::default();
    if let Err(e) = alolite::messaging::polling::poll_new_messages(&s, &mut ps, &mut friends).await
    {
        println!("! poll_new_messages lỗi: {e}");
    }
    println!("\n=== KẾT QUẢ ĐỒNG BỘ TIN NHẮN VÀO HỘI THOẠI ===");
    for (i, f) in friends.iter().enumerate() {
        println!(
            "  [{i}] ID: {:<20} Tên: {:<25} Tin mới nhất: '{}' (Tổng tin: {})",
            f.id,
            f.name,
            f.preview,
            f.messages.len()
        );
        for (mi, m) in f.messages.iter().enumerate() {
            println!(
                "      ({mi}) [{}] {}: {}",
                m.time,
                if m.is_outgoing { "Tôi" } else { &m.sender },
                m.text
            );
        }
    }

    // 3. Test get_groups
    println!("\n--- [3] Thử gọi /api/group/getlg/v4 (danh sách nhóm) ---");
    match alolite::social::groups::get_groups(&s).await {
        Ok(groups) => {
            println!("✓ get_groups THÀNH CÔNG! Số lượng nhóm: {}", groups.len());
            for (i, g) in groups.iter().enumerate() {
                println!("  [{i}] Group ID: {:<20} Tên: {}", g.id, g.name);
            }
            // 4. Test get_cloud_messages on group "Cá hộp ba cô gái"
            let target_g = groups
                .iter()
                .find(|g| g.id == "3880896435999729114" || g.name.contains("Cá hộp ba cô gái"))
                .or_else(|| groups.first());
            if let Some(target_g) = target_g {
                let _gid = if target_g.id.starts_with('g') {
                    target_g.id.clone()
                } else {
                    format!("g{}", target_g.id)
                };
                println!(
                    "\n--- [4] Thử gọi get_last_msgs cho Duyên (1-1: 9118042533396610184) ---"
                );
                match alolite::messaging::polling::get_last_msgs(
                    &s,
                    &["9118042533396610184".into()],
                )
                .await
                {
                    Ok(batch) => {
                        println!("✓ get_last_msgs cho Duyên THÀNH CÔNG!");
                        println!(
                            "  Raw response: {}",
                            serde_json::to_string_pretty(batch.raw_value())?
                        );
                    }
                    Err(e) => println!("! get_last_msgs cho Duyên lỗi: {e}"),
                }
                match alolite::messaging::polling::get_last_msgs(&s, &[]).await {
                    Ok(batch) => {
                        println!("✓ get_last_msgs rỗng THÀNH CÔNG!");
                        println!(
                            "  Keys trong data: {:?}",
                            batch.raw_value().get("data").map(|d| d
                                .as_object()
                                .map(|o| o.keys().cloned().collect::<Vec<_>>()))
                        );
                    }
                    Err(e) => println!("! get_last_msgs rỗng lỗi: {e}"),
                }
            }
        }
        Err(e) => {
            println!("! get_groups lỗi: {e}");
        }
    }

    // 5. Cache persistence test
    println!("\n--- [5] Thử import lịch sử từ /home/ngxc/.config/ZaloData ---");
    let zalo_data_dir = std::path::Path::new("/home/ngxc/.config/ZaloData");
    if zalo_data_dir.exists() {
        match alolite::store::importer::import_history_from_path(zalo_data_dir, "zalo_history.db") {
            Ok(rep) => {
                println!("✓ Import THÀNH CÔNG từ ZaloData!");
                println!("  Số hội thoại import: {}", rep.threads_imported);
                println!("  Số tin nhắn import:  {}", rep.messages_imported);
                println!("  Nguồn:               {} files", rep.sources_read.len());
            }
            Err(e) => println!("! Import lỗi: {e}"),
        }
    }
    alolite::store::save_threads_cache(&friends);

    // 6. Optional send test (1-1 or group)
    let args: Vec<String> = std::env::args().skip(1).collect();
    if let Some(pos) = args.iter().position(|a| a == "--send") {
        if let (Some(to_id), Some(text)) = (args.get(pos + 1), args.get(pos + 2)) {
            if to_id != "7694049103535450055" {
                println!("⛔ TỪ CHỐI GỬI: Chỉ cho phép gửi cá nhân tới Cloud của tôi (7694049103535450055)!");
            } else {
                println!("\n--- [6] Thử gửi tin nhắn cá nhân tới {to_id}: '{text}' ---");
                match alolite::messaging::send::send_message(&s, to_id, text).await {
                    Ok(resp) => {
                        println!("✓ Gửi tin nhắn THÀNH CÔNG!");
                        println!("  Response: {}", serde_json::to_string_pretty(&resp)?);
                    }
                    Err(e) => {
                        println!("! Gửi tin nhắn lỗi: {e}");
                    }
                }
            }
        }
    } else if let Some(pos) = args.iter().position(|a| a == "--send-group") {
        if let (Some(grid), Some(text)) = (args.get(pos + 1), args.get(pos + 2)) {
            let raw_grid = grid.trim_start_matches('g');
            if raw_grid != "3880896435999729114" {
                println!("⛔ TỪ CHỐI GỬI: Chỉ cho phép gửi tới nhóm Cá hộp ba cô gái (3880896435999729114)!");
            } else {
                println!("\n--- [6] Thử gửi tin nhắn nhóm tới {grid}: '{text}' ---");
                match alolite::messaging::send::send_group_message(&s, grid, text).await {
                    Ok(resp) => {
                        println!("✓ Gửi tin nhắn nhóm THÀNH CÔNG!");
                        println!("  Response: {}", serde_json::to_string_pretty(&resp)?);
                    }
                    Err(e) => {
                        println!("! Gửi tin nhắn nhóm lỗi: {e}");
                    }
                }
            }
        }
    }

    Ok(())
}
