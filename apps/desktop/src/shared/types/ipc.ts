export type MessageKind =
  | "text"
  | "photo"
  | "video"
  | "voice"
  | "sticker"
  | "file"
  | "link"
  | "location"
  | "contact"
  | "gif"
  | "recall"
  | "system"
  | "action"
  | "web_content"
  | "delete"
  | "unknown";

export type DeliveryState =
  | "sending"
  | "sent"
  | "delivered"
  | "seen"
  | "failed"
  | "unknown";

export interface ReactionSummary {
  emoji: string;
  count: number;
  reacted_by_me?: boolean;
}

export interface ChatMessage {
  kind?: MessageKind;
  delivery?: DeliveryState;
  id: string;
  sender: string;
  text: string;
  time: string;
  is_outgoing: boolean;
  ts: number;
  sticker_id?: number;
  sticker_cate_id?: number;
  sticker_type?: number;
  reply_to_id?: string;
  reply_to_sender?: string;
  reply_to_text?: string;
  reactions?: string[];
  reaction_summaries?: ReactionSummary[];
  client_message_id?: string;
  is_pinned?: boolean;
  is_starred?: boolean;
  link_url?: string;
  link_title?: string;
  link_desc?: string;
  link_thumb?: string;
  link_src?: string;
  resolved_mention_uids?: Record<string, string>;
}

export interface SidebarMember {
  id: string;
  name: string;
  avatar?: string | null;
}

export interface SidebarPresentation {
  members: SidebarMember[];
  member_count?: number | null;
  last_message_ts: number;
  last_message_id: string;
  last_sender: string;
  last_sender_id: string;
  last_outgoing: boolean;
  pinned: boolean;
  muted: boolean;
  hidden: boolean;
}

export interface UnreadState {
  last_seen_id: string | null;
  unread_ids: string[];
  mention_ids?: string[];
  muted: boolean;
  muted_until?: number | null;
  marked_unread: boolean;
}

export interface Thread {
  presentation: SidebarPresentation;
  id: string;
  name: string;
  preview: string;
  time: string;
  avatar: string | null;
  is_group: boolean;
  messages: ChatMessage[];
  unread: UnreadState;
}

export interface AppSettings {
  show_all_contacts: boolean;
  language: string;
  autostart_on_boot: boolean;
  remember_login: boolean;
  download_dir: string;
  show_birthday: string;
  show_online_status: boolean;
  show_seen_status: boolean;
  allow_message_from: string;
  block_typing: boolean;
  block_delivered: boolean;
  block_seen: boolean;
  allow_call_from: string;
  allow_search_by_phone: boolean;
  friend_src_qr: boolean;
  friend_src_group: boolean;
  friend_src_card: boolean;
  theme_mode: "light" | "dark" | "system";
  use_avatar_as_wallpaper: boolean;
  enable_notifications: boolean;
  play_notification_sound: boolean;
  quick_messages_enabled: boolean;
  hidden_chat_pin_enabled: boolean;
  split_priority_inbox: boolean;
  excel_paste_mode: string;
  double_click_to_reply: boolean;
  show_typing_indicator: boolean;
  send_message_key: "enter" | "cmd_enter";
  suggest_stickers_while_typing: boolean;
  suggest_mentions: boolean;
  enable_gif_search: boolean;
  enable_sticker_keyword_search: boolean;
  selected_mic_device: string;
  selected_camera_device: string;
  mic_gain_percent: number;
  pinned_threads: string[];
  muted_threads: Record<string, number>;
  hidden_threads: string[];
  thread_labels: Record<string, string>;
  auto_check_updates: boolean;
  skipped_update_version: string;
  proxy_url: string;
  multi_profile_name: string;
  info_drawer_open: boolean;
}

export interface AccountProfile {
  uin: string;
  display_name: string;
  avatar: string | null;
  last_active_ts: number;
  is_current: boolean;
}

export interface StickerItem {
  sticker_id: number;
  cate_id: number;
  sticker_type: number;
  label: string;
}

export interface StickerPackInfo {
  id: number;
  name: string;
  total_image: number;
  icon_url: string;
}

export interface PinnedPackMeta {
  cate_id: number;
  name: string;
  items: StickerItem[];
}

export interface ReactionAssetMap {
  like: string;
  heart: string;
  smile: string;
  surprised: string;
  cry: string;
  rage: string;
}

export interface LinkMetadata {
  url: string;
  title: string;
  desc: string;
  thumb: string;
  src: string;
}

export type FriendshipStatus =
  | "Unknown"
  | "Friend"
  | "OutgoingPending"
  | "IncomingPending"
  | "Stranger";

export interface PhoneLookupResult {
  user_id: string;
  display_name: string;
  zalo_name: string;
  phone: string;
  avatar: string;
  status: string;
  friendship: FriendshipStatus;
}

export interface UpdateCheckResult {
  current_version: string;
  latest_version: string;
  has_update: boolean;
  release_title: string;
  release_notes: string;
  html_url: string;
  download_url: string | null;
  asset_name: string | null;
  asset_size: number;
  expected_sha256: string | null;
  published_at: string;
}

export interface MuteControlEvent {
  thread_id: string;
  is_group: boolean;
  muted: boolean;
  muted_until: number | null;
  settings_until: number | null;
}

export interface ImportSummaryDto {
  threads_imported: number;
  messages_imported: number;
  source_path: string;
}

export interface LogEntry {
  id: number;
  timestamp: string;
  method: string;
  url: string;
  status: number | null;
  latency_ms: number;
  payload_preview: string;
  decrypted_preview: string | null;
  is_error: boolean;
  error_msg: string | null;
}

export interface ScanFinding {
  level: "Info" | "Warning" | "Critical";
  title: string;
  detail: string;
  suggestion: string;
}

export interface DevCryptoInfo {
  zpw_ver: string;
  zpw_type: string;
  cipher_algo: string;
  sign_key_formula: string;
  param_cipher_key_len: number;
}

export interface DevDiagnosticsSnapshot {
  dev_mode: boolean;
  logs: LogEntry[];
  findings: ScanFinding[];
  crypto: DevCryptoInfo;
}

export interface AutoReplyRule {
  id: string;
  keyword: string;
  match_mode: "contains" | "exact" | "prefix";
  reply_text: string;
  enabled: boolean;
  scope: "all" | "direct" | "group";
}

export interface CoreBootstrapSnapshot {
  app_version: string;
  dev_mode: boolean;
  logged_in: boolean;
  uin: string;
  display_name: string;
  avatar: string | null;
  threads: Thread[];
  settings: AppSettings;
  saved_accounts: AccountProfile[];
  popular_stickers: StickerItem[];
  onion_stickers: StickerItem[];
  pinned_packs: PinnedPackMeta[];
  reaction_assets: ReactionAssetMap;
  app_logo_data_url: string;
  active_db_path: string;
  sync_running: boolean;
  phone_sync_running: boolean;
}

export type CoreEventDto =
  | {
      type: "NotificationAuthorization";
      payload: {
        request_id: number;
        ok: boolean;
        status_label: string | null;
        error: string | null;
      };
    }
  | {
      type: "QrReady";
      payload: { qr_token: string; qr_url: string; qr_data_url: string };
    }
  | { type: "QrScanned"; payload: { name: string; avatar: string | null } }
  | { type: "QrExpired"; payload: { message: string } }
  | {
      type: "LoginSuccess";
      payload: { name: string; uin: string; avatar: string | null };
    }
  | { type: "LoginFailed"; payload: { message: string } }
  | { type: "ThreadsUpdated"; payload: { threads: Thread[] } }
  | {
      type: "MessageSent";
      payload: {
        client_message_id: string;
        thread_id: string;
        text: string;
        msg_id: string;
      };
    }
  | {
      type: "StickerSent";
      payload: {
        client_message_id: string;
        thread_id: string;
        msg_id: string;
        sticker_id: number;
        cate_id: number;
        sticker_type: number;
      };
    }
  | { type: "AttachmentSent"; payload: { thread_id: string; msg: ChatMessage } }
  | { type: "StickerImageReady"; payload: { key: string; data_url: string } }
  | { type: "StickerStoreLoaded"; payload: { packs: StickerPackInfo[] } }
  | {
      type: "StickerPackItemsLoaded";
      payload: { cate_id: number; name: string; items: StickerItem[] };
    }
  | {
      type: "StickerSearchResults";
      payload: { keyword: string; items: StickerItem[] };
    }
  | { type: "StickerSuggestKeywordsLoaded"; payload: { keywords: string[] } }
  | {
      type: "ComposerStickerSuggestions";
      payload: { keyword: string; items: StickerItem[] };
    }
  | { type: "LinkPreviewReady"; payload: { meta: LinkMetadata } }
  | { type: "PhoneLookupSuccess"; payload: { profile: PhoneLookupResult } }
  | { type: "PhoneLookupFailed"; payload: { error: string } }
  | {
      type: "FriendStatusChecked";
      payload: {
        user_id: string;
        status: FriendshipStatus;
        online: [boolean, string] | null;
      };
    }
  | {
      type: "QuoteSent";
      payload: {
        client_message_id: string;
        thread_id: string;
        text: string;
        msg_id: string;
        quoted: ChatMessage;
      };
    }
  | {
      type: "Reacted";
      payload: {
        thread_id: string;
        msg_id: string;
        ts: number;
        reaction: string;
      };
    }
  | {
      type: "MessageDeleted";
      payload: { thread_id: string; msg_id: string; ts: number };
    }
  | {
      type: "MessageRecalled";
      payload: { thread_id: string; msg_id: string; ts: number };
    }
  | {
      type: "SendFailed";
      payload: { thread_id: string; text: string; error: string };
    }
  | { type: "SessionExpired"; payload: { message: string } }
  | { type: "SessionUnreachable"; payload: { message: string } }
  | { type: "LocalDataWiped" }
  | { type: "StatusUpdate"; payload: { message: string } }
  | {
      type: "RefreshFinished";
      payload: {
        threads: number;
        messages: number;
        errors: string[];
        manual: boolean;
      };
    }
  | {
      type: "SyncFinished";
      payload: { added: number; errors: string[]; cancelled: boolean };
    }
  | {
      type: "UpdateCheckFinished";
      payload: {
        silent: boolean;
        ok: boolean;
        info: UpdateCheckResult | null;
        error: string | null;
      };
    }
  | {
      type: "UpdateDownloadStatus";
      payload: { downloading: boolean; message: string };
    }
  | {
      type: "UpdateDownloadProgress";
      payload: { downloaded: number; total: number };
    }
  | { type: "UpdateAppliedAndRestarting"; payload: { version: string } }
  | {
      type: "MuteSnapshotSynced";
      payload: { muted_threads: Record<string, number> };
    }
  | { type: "MuteEventsReceived"; payload: { events: MuteControlEvent[] } }
  | { type: "PhoneSyncProgress"; payload: { progress: number; status: string } }
  | {
      type: "PhoneSyncFinished";
      payload: {
        threads: Thread[];
        added: number;
        active_threads: number;
        cancelled: boolean;
        error: string | null;
      };
    }
  | {
      type: "ProxyUpdated";
      payload: { ok: boolean; proxy_url: string | null; error: string | null };
    }
  | {
      type: "FileDownloaded";
      payload: {
        ok: boolean;
        file_name: string | null;
        path: string | null;
        error: string | null;
      };
    }
  | { type: "LoggedOut" }
  | { type: "HistoryReachedStart"; payload: { thread_id: string } }
  | {
      type: "AccountSwitched";
      payload: {
        uin: string;
        name: string;
        avatar: string | null;
        threads: Thread[];
      };
    };

export const REACTION_LIST = [
  { emoji: "👍", key: "like" as const, label: "Thích" },
  { emoji: "❤️", key: "heart" as const, label: "Yêu thích" },
  { emoji: "😆", key: "smile" as const, label: "Haha" },
  { emoji: "😮", key: "surprised" as const, label: "Wow" },
  { emoji: "😢", key: "cry" as const, label: "Buồn" },
  { emoji: "😠", key: "rage" as const, label: "Phẫn nộ" },
] as const;
