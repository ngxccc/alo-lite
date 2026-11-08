//! Integration tests for message preview/cloud merging, thread deduplication, and unauthenticated guards.

use alolite_core::messaging::polling::{get_last_msgs, merge_group_history, merge_previews};
use alolite_core::messaging::send::send_message;
use alolite_core::messaging::threads::{deduplicate_threads, get_pinned, ChatMessage, Thread};
use alolite_core::session::Session;
use alolite_core::social::friends::get_friends;
use alolite_core::social::groups::get_group_history;
use serde_json::json;

fn sample_thread(id: &str) -> Thread {
    Thread {
        id: id.into(),
        name: format!("Name {id}"),
        ..Default::default()
    }
}

#[tokio::test]
async fn unauthenticated_calls_fail_cleanly() {
    let s = Session::new(false, false, false).unwrap();
    for res in [
        get_friends(&s).await.map(|_| ()),
        get_last_msgs(&s, &["test_tid".into()]).await.map(|_| ()),
        send_message(&s, "test_to", "hello").await.map(|_| ()),
        get_pinned(&s).await.map(|_| ()),
        get_group_history(&s, "test_gid", 20).await.map(|_| ()),
    ] {
        assert_eq!(res.unwrap_err().to_string(), "chưa đăng nhập");
    }
}

#[test]
fn merge_previews_handles_single_last_msg_and_ordered_batches() {
    let mut threads = vec![
        sample_thread("u1"),
        sample_thread("u2"),
        sample_thread("u3"),
    ];
    merge_previews(
        &mut threads,
        &json!({
            "error_code": 0,
            "data": [
                {
                    "idTo": "u1",
                    "message": { "msgId": "m_single", "content": "Single message text", "ts": 1729501000000_i64, "uidFrom": "u1" }
                },
                {
                    "idTo": "u2",
                    "lastMsg": { "msgId": "m_last", "content": "LastMsg text", "ts": 1729502000000_i64, "uidFrom": "u2" }
                },
                {
                    "idTo": "u3",
                    "messages": [
                        { "msgId": "m_item1", "content": "First message", "ts": 1000_i64, "uidFrom": "u3" },
                        { "msgId": "m_item3", "content": "Third message", "ts": 3000_i64, "uidFrom": "u3" },
                        { "msgId": "m_item2", "content": "Second message", "ts": 2000_i64, "uidFrom": "0" }
                    ]
                }
            ]
        }),
    );

    assert_eq!(threads[0].preview, "Single message text");
    assert_eq!(threads[1].preview, "LastMsg text");
    assert_eq!(threads[2].messages.len(), 3);
    assert!(threads[2].messages[1].is_outgoing);
    assert_eq!(threads[2].preview, "Third message");
}

#[test]
fn merge_previews_routes_incoming_1_1_group_and_cloud_accurately() {
    let mut cloud = sample_thread("9000000000000000001");
    cloud.name = "Cloud của tôi (Lưu trữ)".into();
    let friend = sample_thread("1000000000000000001");
    let mut group = sample_thread("888999000");
    group.is_group = true;
    let mut threads = vec![cloud, friend, group];

    merge_previews(
        &mut threads,
        &json!({
            "error_code": 0,
            "data": {
                "msgs": [
                    { "msgId": "in_1", "content": "Tin nhan den tu ban be", "idTo": "0", "uidFrom": "1000000000000000001", "ts": "1729505000000" },
                    { "msgId": "cloud_1", "content": "Tin nhan tu gui vao Cloud", "idTo": "0", "uidFrom": "0", "ts": "1729506000000" }
                ],
                "groupMsgs": [
                    { "msgId": "grp_msg_1", "content": "Tin nhan trong nhom", "idTo": "888999000", "uidFrom": "111222333", "dName": "Nguyen Van B", "ts": "1729507000000" }
                ]
            }
        }),
    );

    assert_eq!(threads[0].messages[0].text, "Tin nhan tu gui vao Cloud");
    assert!(threads[0].messages[0].is_outgoing);
    assert_eq!(threads[1].messages[0].text, "Tin nhan den tu ban be");
    assert!(!threads[1].messages[0].is_outgoing);
    assert_eq!(threads[2].messages[0].sender, "Nguyen Van B");
}

#[test]
fn merge_group_history_parses_stringified_cloud_data_and_prevents_pollution() {
    let mut cloud_thread = Thread {
        id: "9000000000000000002".into(),
        name: "Cloud của tôi (Lưu trữ)".into(),
        ..Default::default()
    };
    let raw_cloud_resp = json!({
        "error_code": 0,
        "data": r#"{"error":0,"groupMsgs":[{"msgId":"1000000000001","cliMsgId":"1700000000001","msgType":"chat.recommended","uidFrom":"0","idTo":"9000000000000000002","dName":"User Test","ts":"1700000000001","content":{"title":"","description":"Ghi chú đám mây","href":"https://zalo.me/test","action":"recommened.link"}}]}"#
    });
    merge_group_history(&mut cloud_thread, &raw_cloud_resp);
    assert_eq!(cloud_thread.messages.len(), 1);
    assert_eq!(
        cloud_thread.messages[0].link_url.as_deref(),
        Some("https://zalo.me/test")
    );

    let mut contact = sample_thread("1000000000000000009");
    let foreign_cloud = json!({
        "error_code": 0,
        "data": {
            "groupMsgs": [{
                "msgId": "1000000000002", "msgType": "webchat", "uidFrom": "0",
                "idTo": "9000000000000000001", "ts": "1700000000002", "content": "Ghi chú Cloud của tôi"
            }]
        }
    });
    merge_group_history(&mut contact, &foreign_cloud);
    assert!(contact.messages.is_empty());

    let cloud_owner = Thread {
        id: "9000000000000000001".into(),
        name: "Cloud của tôi (Lưu trữ)".into(),
        messages: vec![ChatMessage {
            id: "1000000000002".into(),
            text: "Ghi chú Cloud của tôi".into(),
            is_outgoing: true,
            ts: 1700000000002,
            ..Default::default()
        }],
        ..Default::default()
    };
    let polluted_contact = Thread {
        id: "1000000000000000009".into(),
        name: "Người dùng Test".into(),
        messages: vec![ChatMessage {
            id: "1000000000002".into(),
            text: "Ghi chú Cloud của tôi".into(),
            is_outgoing: true,
            ts: 1700000000002,
            ..Default::default()
        }],
        ..Default::default()
    };
    let mut list = vec![cloud_owner, polluted_contact];
    deduplicate_threads(&mut list, None);
    assert_eq!(list[0].messages.len(), 1);
    assert!(list[1].messages.is_empty());
}
