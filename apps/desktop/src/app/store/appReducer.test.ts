import { describe, expect, it } from "bun:test";
import type { AppSettings, ChatMessage, CoreBootstrapSnapshot, Thread } from "@/shared/types/ipc";
import { matchesVnQuery } from "@/shared/utils/text";
import { uniqueReferenceIndex } from "@/features/chat/utils/messages";
import { appReducer, initialAppState } from "./appReducer";

function makeSettings(): AppSettings {
  return {
    show_all_contacts: false,
    language: "Tiếng Việt",
    autostart_on_boot: true,
    remember_login: true,
    download_dir: "/tmp/dl",
    show_birthday: "Không hiện",
    show_online_status: true,
    show_seen_status: true,
    allow_message_from: "Tất cả mọi người",
    block_typing: false,
    block_delivered: false,
    block_seen: false,
    allow_call_from: "Bạn bè",
    allow_search_by_phone: true,
    friend_src_qr: true,
    friend_src_group: true,
    friend_src_card: true,
    theme_mode: "dark",
    use_avatar_as_wallpaper: false,
    enable_notifications: true,
    play_notification_sound: false,
    quick_messages_enabled: true,
    hidden_chat_pin_enabled: true,
    split_priority_inbox: false,
    excel_paste_mode: "ask",
    double_click_to_reply: true,
    show_typing_indicator: true,
    send_message_key: "enter",
    suggest_stickers_while_typing: true,
    suggest_mentions: true,
    enable_gif_search: true,
    enable_sticker_keyword_search: true,
    selected_mic_device: "",
    selected_camera_device: "",
    mic_gain_percent: 100,
    pinned_threads: [],
    muted_threads: {},
    hidden_threads: [],
    thread_labels: {},
    auto_check_updates: true,
    skipped_update_version: "",
    proxy_url: "",
    multi_profile_name: "acc2",
    info_drawer_open: false,
  };
}

function makeThread(id: string, name: string, ts: number, msgs: ChatMessage[] = []): Thread {
  return {
    id,
    name,
    preview: msgs[msgs.length - 1]?.text ?? "",
    time: "12:00",
    avatar: null,
    is_group: false,
    messages: msgs,
    unread: {
      last_seen_id: null,
      unread_ids: [],
      muted: false,
      marked_unread: false,
    },
    presentation: {
      members: [],
      member_count: null,
      last_message_ts: ts,
      last_message_id: msgs[msgs.length - 1]?.id ?? "",
      last_sender: name,
      last_sender_id: id,
      last_outgoing: false,
      pinned: false,
      muted: false,
      hidden: false,
    },
  };
}

describe("appReducer + feature domain invariants", () => {
  it("folds Vietnamese diacritics accurately in search", () => {
    expect(matchesVnQuery("Nguyễn Văn Đông", "nguyen van dong")).toBe(true);
    expect(matchesVnQuery("Nhóm Đồ Án KLTN", "do an kltn")).toBe(true);
    expect(matchesVnQuery("Trần Minh", "pham")).toBe(false);
  });

  it("preserves exact selectedThreadId across ThreadsUpdated refreshes even with duplicate names (#63)", () => {
    const settings = makeSettings();
    const t1 = makeThread("u_100", "Minh Anh", 1000, [
      {
        id: "m1",
        sender: "Minh Anh",
        text: "Hi",
        time: "10:00",
        is_outgoing: false,
        ts: 1000,
      },
    ]);
    const t2 = makeThread("u_200", "Minh Anh", 900, [
      {
        id: "m2",
        sender: "Minh Anh",
        text: "Hello",
        time: "09:00",
        is_outgoing: false,
        ts: 900,
      },
    ]);

    const snapshot: CoreBootstrapSnapshot = {
      app_version: "0.0.1",
      dev_mode: false,
      logged_in: true,
      uin: "999",
      display_name: "Test",
      avatar: null,
      threads: [t1, t2],
      settings,
      saved_accounts: [],
      popular_stickers: [],
      onion_stickers: [],
      pinned_packs: [],
      app_logo_data_url: "",
      active_db_path: "test.db",
      sync_running: false,
      phone_sync_running: false,
      inline_video_supported: true,
      inline_video_reason: null,
    };

    let state = appReducer(initialAppState, { type: "BOOTSTRAP", snapshot });
    state = appReducer(state, { type: "SELECT_THREAD", threadId: "u_200" });
    expect(state.selectedThreadId).toBe("u_200");

    state = appReducer(state, {
      type: "CORE_EVENT",
      event: {
        type: "ThreadsUpdated",
        payload: {
          threads: [
            makeThread("u_100", "Minh Anh", 2000, [
              {
                id: "m3",
                sender: "Minh Anh",
                text: "Ping",
                time: "10:05",
                is_outgoing: false,
                ts: 2000,
              },
            ]),
          ],
        },
      },
    });

    expect(state.selectedThreadId).toBe("u_200");
    expect(state.unreadCounts.u_100).toBe(1);
    expect(state.threads[0]?.id).toBe("u_100");
  });

  it("reconciles MessageSent, Reacted, MessageRecalled, and MessageDeleted cleanly", () => {
    const settings = makeSettings();
    const baseThread = makeThread("u_1", "Alice", 100);
    let state = appReducer(initialAppState, {
      type: "BOOTSTRAP",
      snapshot: {
        app_version: "0.0.1",
        dev_mode: false,
        logged_in: true,
        uin: "999",
        display_name: "Me",
        avatar: null,
        threads: [baseThread],
        settings,
        saved_accounts: [],
        popular_stickers: [],
        onion_stickers: [],
        pinned_packs: [],
        app_logo_data_url: "",
        active_db_path: "test.db",
        sync_running: false,
        phone_sync_running: false,
        inline_video_supported: true,
        inline_video_reason: null,
      },
    });

    state = appReducer(state, {
      type: "CORE_EVENT",
      event: {
        type: "MessageSent",
        payload: {
          client_message_id: "c1",
          thread_id: "u_1",
          text: "Xin chào",
          msg_id: "srv_1",
        },
      },
    });
    expect(state.threads[0]?.messages).toHaveLength(1);
    expect(state.threads[0]?.messages[0]?.id).toBe("srv_1");

    state = appReducer(state, {
      type: "CORE_EVENT",
      event: {
        type: "Reacted",
        payload: { thread_id: "u_1", msg_id: "srv_1", ts: 0, reaction: "\u{1F44D}" },
      },
    });
    expect(state.threads[0]?.messages[0]?.reactions).toEqual(["\u{1F44D}"]);
    state = appReducer(state, {
      type: "CORE_EVENT",
      event: {
        type: "MessageRecalled",
        payload: { thread_id: "u_1", msg_id: "srv_1", ts: 0 },
      },
    });
    expect(state.threads[0]?.messages[0]?.text).toBe("[Tin nhắn đã thu hồi]");

    state = appReducer(state, {
      type: "CORE_EVENT",
      event: {
        type: "MessageDeleted",
        payload: { thread_id: "u_1", msg_id: "srv_1", ts: 0 },
      },
    });
    expect(state.threads[0]?.messages).toHaveLength(0);
  });

  it("refuses ambiguous quote-jump text references just like Rust unique_reference_index", () => {
    const dupMessages: ChatMessage[] = [
      {
        id: "1",
        sender: "A",
        text: "ok",
        time: "10:00",
        is_outgoing: false,
        ts: 1,
      },
      {
        id: "2",
        sender: "B",
        text: "ok",
        time: "10:01",
        is_outgoing: true,
        ts: 2,
      },
    ];
    expect(uniqueReferenceIndex(dupMessages, "ok")).toBeNull();
    expect(uniqueReferenceIndex(dupMessages, "2")).toBe(1);
  });
});
