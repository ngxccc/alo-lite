//! End-to-end Sync V1 ZDB4.0 + BinNet TLV + SQLite store archive ingest integration tests.

use std::collections::HashMap;
use std::io::Cursor;

use aes::cipher::{BlockEncryptMut, KeyIvInit};
use alolite::messaging::Thread;
use alolite::store::{
    commit_conv_archives_to_store, extract_conv_archives_from_entries, MessageStore, StoredMessage,
    StoredThread,
};
use alolite::sync::{
    build_synced_message_content, clear_sync_keys, decrypt_and_unpack_zdb4_container,
    generate_sync_public_key, get_sync_keys, parse_bin_net_tlv,
};
use base64::Engine;
use rsa::pkcs8::DecodePublicKey;
use rsa::{Pkcs1v15Encrypt, RsaPublicKey};
use rusqlite::{params, Connection};

fn push_tlv_le(buf: &mut Vec<u8>, tag: u32, payload: &[u8]) {
    buf.extend_from_slice(&tag.to_le_bytes());
    buf.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    buf.extend_from_slice(payload);
}

#[test]
fn parse_bin_net_tlv_and_hydrate_roundtrips_photo_sticker_and_quote() {
    let mut photo_att = Vec::new();
    push_tlv_le(
        &mut photo_att,
        0x30,
        b"https://f10-zpcloud.zdn.vn/jxl/photo1.jxl?jxlstatus=1",
    );
    push_tlv_le(
        &mut photo_att,
        0x31,
        b"https://f10-zpcloud.zdn.vn/jxl/thumb1.jxl?jxlstatus=1",
    );
    push_tlv_le(&mut photo_att, 0x2F, "Ảnh chuyến đi".as_bytes());

    let mut quote_tlv = Vec::new();
    push_tlv_le(&mut quote_tlv, 0x50, &777_888_i32.to_le_bytes());
    push_tlv_le(&mut quote_tlv, 0x52, &9_001_002_003_i64.to_le_bytes());
    push_tlv_le(&mut quote_tlv, 0x56, "Tin nhắn gốc được trả lời".as_bytes());

    let mut mention_tlv = Vec::new();
    push_tlv_le(&mut mention_tlv, 0x65, &777_888_i32.to_le_bytes());
    push_tlv_le(&mut mention_tlv, 0x66, &0_i32.to_le_bytes());
    push_tlv_le(&mut mention_tlv, 0x67, &5_i32.to_le_bytes());

    let mut bin_net = Vec::new();
    push_tlv_le(&mut bin_net, 0x06, &photo_att);
    push_tlv_le(&mut bin_net, 0x07, &quote_tlv);
    push_tlv_le(&mut bin_net, 0x08, &mention_tlv);

    let parsed = parse_bin_net_tlv(&bin_net).expect("valid BinNet TLV");
    assert_eq!(parsed.attachs.len(), 1);
    assert_eq!(parsed.mentions.len(), 1);
    assert_eq!(parsed.mentions[0].uid, Some(777_888));

    let mut fid_map = HashMap::new();
    fid_map.insert(777_888_i64, "noised_friend_777".to_string());

    let (text, wire_type, raw_json) =
        build_synced_message_content(3, Some(""), Some(&bin_net), &fid_map);
    assert_eq!(wire_type, "chat.photo");
    assert_eq!(text, "Ảnh chuyến đi");

    let mut store = MessageStore::open(":memory:").expect("in-memory store");
    store
        .upsert_thread(&StoredThread {
            thread_id: "noised_friend_777".into(),
            name: "Minh".into(),
            is_group: false,
            last_ts: 1_794_000_000_000,
        })
        .expect("upsert thread");
    let sm = StoredMessage {
        msg_id: "9001002099".into(),
        thread_id: "noised_friend_777".into(),
        sender_id: "noised_friend_777".into(),
        sender_name: "Minh".into(),
        text,
        msg_type: wire_type,
        ts: 1_794_000_000_000,
        is_outgoing: false,
        raw_json,
    };
    assert!(store.upsert_message(&sm).expect("first insert")); // (INV-4)
    assert!(!store.upsert_message(&sm).expect("idempotent re-insert")); // (INV-4)

    let mut threads = Vec::new();
    assert_eq!(store.hydrate_threads(&mut threads, 10).expect("hydrate"), 1);
    let hydrated = &threads[0].messages[0];
    assert_eq!(
        hydrated.link_url.as_deref(),
        Some("https://f10-zpcloud.zdn.vn/jpg/photo1.jpg")
    );
    assert_eq!(
        hydrated.link_thumb.as_deref(),
        Some("https://f10-zpcloud.zdn.vn/jpg/thumb1.jpg")
    );
    assert_eq!(hydrated.reply_to_id.as_deref(), Some("9001002003"));
    assert_eq!(
        hydrated.reply_to_sender.as_deref(),
        Some("noised_friend_777")
    );
    assert_eq!(
        hydrated.reply_to_text.as_deref(),
        Some("Tin nhắn gốc được trả lời")
    );

    let mut sticker_att = Vec::new();
    push_tlv_le(&mut sticker_att, 0x29, &10420_i32.to_le_bytes());
    push_tlv_le(&mut sticker_att, 0x2A, &88991_i32.to_le_bytes());
    push_tlv_le(&mut sticker_att, 0x28, b"7");
    let mut sticker_bin = Vec::new();
    push_tlv_le(&mut sticker_bin, 0x06, &sticker_att);
    let (st_text, st_wire, st_json) =
        build_synced_message_content(10, None, Some(&sticker_bin), &fid_map);
    assert_eq!(st_wire, "chat.sticker");
    assert_eq!(st_text, "[Nhãn dán]");
    let st_val: serde_json::Value = serde_json::from_str(&st_json).expect("valid json");
    assert_eq!(st_val["stickerId"], 88991);
    assert_eq!(st_val["cateId"], 10420);
    assert_eq!(st_val["type"], 7);
}

#[test]
fn zdb4_sqlite_archive_ingest_resolves_dynamic_my_uid_and_group_member_senders() {
    let pub_b64 = generate_sync_public_key();
    let (priv_key, _) = get_sync_keys().expect("sync keys stored");
    let pub_der = base64::engine::general_purpose::STANDARD
        .decode(&pub_b64)
        .expect("spki der");
    let pub_key = RsaPublicKey::from_public_key_der(&pub_der).expect("valid rsa pubkey");
    let mobile_raw_key: [u8; 16] =
        *b"\xaa\xbb\xcc\xdd\x11\x22\x33\x44\x55\x66\x77\x88\x99\x00\xab\xcd";
    let encrypted_key_bytes = pub_key
        .encrypt(&mut rsa::rand_core::OsRng, Pkcs1v15Encrypt, &mobile_raw_key)
        .expect("rsa pkcs1v15 encrypt");
    let decrypted_raw = priv_key
        .decrypt(Pkcs1v15Encrypt, &encrypted_key_bytes)
        .expect("rsa pkcs1v15 decrypt");
    assert_eq!(decrypted_raw.as_slice(), &mobile_raw_key);

    let tmp = std::env::temp_dir().join(format!(
        "alolite_test_zdb4_{}",
        alolite::param_cipher::random_hex(8)
    ));
    std::fs::create_dir_all(&tmp).unwrap();
    let dm_db_path = tmp.join("111222.db");
    {
        let conn = Connection::open(&dm_db_path).unwrap();
        conn.execute_batch(
            "CREATE TABLE ChatContent (
                SenderId INTEGER, CliMsgId INTEGER, GlbMsgId INTEGER,
                MsgStatus INTEGER, MsgType INTEGER, TimeStamp INTEGER,
                MsgContent TEXT, BinNet BLOB
            );",
        )
        .unwrap();
        conn.execute(
            "INSERT INTO ChatContent VALUES (?1, ?2, ?3, 1, 0, ?4, ?5, NULL)",
            params![
                987_654_321_i64,
                1001_i64,
                50001_i64,
                1_794_000_001_000_i64,
                "Chào cậu nhé"
            ],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO ChatContent VALUES (?1, ?2, ?3, 1, 20, ?4, ?5, NULL)",
            params![
                111_222_i64,
                1002_i64,
                50002_i64,
                1_794_000_002_000_i64,
                "system"
            ],
        )
        .unwrap();
    }
    let grp_db_path = tmp.join("group_333444.db");
    {
        let conn = Connection::open(&grp_db_path).unwrap();
        conn.execute_batch(
            "CREATE TABLE ChatContent (
                SenderId INTEGER, CliMsgId INTEGER, GlbMsgId INTEGER,
                MsgStatus INTEGER, MsgType INTEGER, TimeStamp INTEGER,
                MsgContent TEXT, BinNet BLOB
            );",
        )
        .unwrap();
        let mut file_att = Vec::new();
        push_tlv_le(&mut file_att, 0x2F, "bao-cao.pdf".as_bytes());
        push_tlv_le(&mut file_att, 0x30, b"https://f.zalo.me/bao-cao.pdf");
        let mut bin_net = Vec::new();
        push_tlv_le(&mut bin_net, 0x06, &file_att);
        conn.execute(
            "INSERT INTO ChatContent VALUES (?1, ?2, ?3, 1, 22, ?4, '', ?5)",
            params![
                555_666_i64,
                2001_i64,
                60001_i64,
                1_794_000_003_000_i64,
                bin_net
            ],
        )
        .unwrap();
    }
    let dm_bytes = std::fs::read(&dm_db_path).unwrap();
    let grp_bytes = std::fs::read(&grp_db_path).unwrap();
    let _ = std::fs::remove_dir_all(&tmp);

    let mut raw_concat = Vec::new();
    raw_concat.extend_from_slice(&dm_bytes);
    raw_concat.extend_from_slice(&grp_bytes);
    let mut xz_stream = Vec::new();
    lzma_rs::xz_compress(&mut Cursor::new(&raw_concat), &mut xz_stream).unwrap();

    let mut container = Vec::new();
    container.extend_from_slice(b"ZDB4.0\0");
    container.extend_from_slice(&[0u8; 7]);
    container.extend_from_slice(&2u32.to_be_bytes());
    for (fname, sz) in [
        (&b"111222.db"[..], dm_bytes.len()),
        (&b"group_333444.db"[..], grp_bytes.len()),
    ] {
        container.extend_from_slice(&(fname.len() as u32).to_be_bytes());
        container.extend_from_slice(fname);
        container.extend_from_slice(&(sz as u32).to_be_bytes());
    }
    container.extend_from_slice(&xz_stream);
    let pad = (16 - (container.len() % 16)) % 16;
    container.extend(std::iter::repeat_n(pad as u8, pad));

    let hex_key: String = mobile_raw_key.iter().map(|b| format!("{b:02X}")).collect();
    let key_32: &[u8; 32] = hex_key.as_bytes().try_into().unwrap();
    let mut crypt = container.clone();
    cbc::Encryptor::<aes::Aes256>::new(key_32.into(), (&[0u8; 16]).into())
        .encrypt_padded_mut::<cipher::block_padding::NoPadding>(&mut crypt, container.len())
        .unwrap();

    let unpacked = decrypt_and_unpack_zdb4_container(&crypt, &decrypted_raw).unwrap();
    let (archives, fids, gids) = extract_conv_archives_from_entries(unpacked, None).unwrap();
    assert!(fids.contains(&111_222));
    assert!(fids.contains(&987_654_321));
    assert!(fids.contains(&555_666));
    assert_eq!(gids, vec![333_444]);

    let mut fid_map = HashMap::new();
    fid_map.insert(987_654_321, "noised_self_987".into());
    fid_map.insert(111_222, "noised_friend_111".into());
    fid_map.insert(555_666, "noised_member_555".into());
    let mut gid_map = HashMap::new();
    gid_map.insert(333_444, "noised_group_333".into());

    let mut store = MessageStore::open(":memory:").unwrap();
    let mut threads = vec![Thread {
        id: "noised_member_555".into(),
        name: "Lan Anh".into(),
        is_group: false,
        ..Default::default()
    }];
    let imported = commit_conv_archives_to_store(
        &mut store,
        &mut threads,
        archives,
        &fid_map,
        &gid_map,
        "noised_self_987",
        "Ngọc",
        None,
    )
    .unwrap();
    assert_eq!(imported, 2);

    let dm_thread = threads
        .iter()
        .find(|t| t.id == "noised_friend_111")
        .expect("1-1 thread hydrated");
    assert_eq!(dm_thread.messages.len(), 1);
    assert!(dm_thread.messages[0].is_outgoing);
    assert_eq!(dm_thread.messages[0].sender, "Ngọc");
    assert_eq!(dm_thread.messages[0].text, "Chào cậu nhé");

    let grp_thread = threads
        .iter()
        .find(|t| t.id == "noised_group_333")
        .expect("group thread hydrated");
    assert!(grp_thread.is_group);
    assert_eq!(grp_thread.messages.len(), 1);
    assert!(!grp_thread.messages[0].is_outgoing);
    assert_eq!(grp_thread.messages[0].sender, "Lan Anh");
    assert_eq!(grp_thread.messages[0].text, "bao-cao.pdf");
    assert_eq!(
        grp_thread.messages[0].link_url.as_deref(),
        Some("https://f.zalo.me/bao-cao.pdf")
    );
    clear_sync_keys();
}
