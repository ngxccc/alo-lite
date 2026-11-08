import { describe, expect, it } from "bun:test";
import type { AppSettings, Thread } from "../../../shared/types/ipc";
import { isThreadMuted, mergeThreadInto, reconcileSentMessage, sortThreads } from "./threads";

const baseSettings: AppSettings = {
  show_all_contacts: false,
  language: "Tiếng Việt",
  autostart_on_boot: true,
  remember_login: true,
  download_dir: "/tmp",
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
  pinned_threads: ["b"],
  muted_threads: { b: -1, c: 100 },
  hidden_threads: [],
  thread_labels: {},
  auto_check_updates: true,
  skipped_update_version: "",
  proxy_url: "",
  multi_profile_name: "acc2",
  info_drawer_open: false,
};

function thread(id: string, ts: number): Thread {
  return {
    id,
    name: id,
    preview: "",
    time: "10:00",
    avatar: null,
    is_group: false,
    messages: [],
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
      last_message_id: "",
      last_sender: "",
      last_sender_id: "",
      last_outgoing: false,
      pinned: false,
      muted: false,
      hidden: false,
    },
  };
}

describe("threads domain utils", () => {
  it("orders pinned threads above newer unpinned threads", () => {
    const sorted = sortThreads([thread("a", 5000), thread("b", 1000)], baseSettings);
    expect(sorted.map((t) => t.id)).toEqual(["b", "a"]);
  });

  it("checks mute timestamps against current unix time", () => {
    expect(isThreadMuted(thread("b", 1), baseSettings, 200)).toBe(true);
    expect(isThreadMuted(thread("c", 1), baseSettings, 200)).toBe(false);
  });

  it("merges and deduplicates messages across sync updates", () => {
    const t1: Thread = {
      ...thread("a", 1000),
      messages: [
        {
          id: "out_1",
          client_message_id: "1",
          sender: "Tôi",
          text: "Hi",
          time: "10:00",
          is_outgoing: true,
          ts: 1000,
        },
      ],
    };
    const reconciled = reconcileSentMessage(t1, {
      id: "srv_1",
      client_message_id: "1",
      sender: "Tôi",
      text: "Hi",
      time: "10:00",
      is_outgoing: true,
      ts: 1005,
    });
    expect(reconciled.messages).toHaveLength(1);
    expect(reconciled.messages[0]?.id).toBe("srv_1");
    const merged = mergeThreadInto(reconciled, {
      ...thread("a", 2000),
      messages: [
        {
          id: "srv_2",
          sender: "A",
          text: "Yo",
          time: "10:01",
          is_outgoing: false,
          ts: 2000,
        },
      ],
    });
    expect(merged.messages.map((m) => m.id)).toEqual(["srv_1", "srv_2"]);
  });
});
