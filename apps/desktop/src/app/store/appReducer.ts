import type {
  AccountProfile,
  AppSettings,
  ChatMessage,
  CoreBootstrapSnapshot,
  CoreEventDto,
  FriendshipStatus,
  LinkMetadata,
  PhoneLookupResult,
  PinnedPackMeta,
  ReactionAssetMap,
  StickerItem,
  StickerPackInfo,
  Thread,
  UpdateCheckResult,
} from "../../shared/types/ipc";
import { formatClockNow } from "../../shared/utils/text";
import { mergeThreadInto, reconcileSentMessage, sortThreads } from "../../features/threads/utils/threads";

export type NavSection = "chat" | "contacts";
export type ContactsSubTab = "friends" | "groups" | "friend_requests" | "group_invites";
export type SettingsTab =
  | "general"
  | "account"
  | "privacy"
  | "sync"
  | "appearance"
  | "notifications"
  | "messages"
  | "utilities"
  | "update";

export interface ImageViewerState {
  texKey: string;
  hdTexKey?: string;
  fullUrl: string;
  title: string;
  sender: string;
  time: string;
}

export interface AppState {
  bootstrapped: boolean;
  appVersion: string;
  devMode: boolean;
  appLogoDataUrl: string;
  reactionAssets: ReactionAssetMap;
  activeDbPath: string;

  loggedIn: boolean;
  uin: string;
  displayName: string;
  avatar: string | null;

  qrDataUrl: string | null;
  qrStatus: string;
  qrScannedName: string | null;
  qrScannedAvatar: string | null;
  loginInProgress: boolean;
  loginError: string;

  activeSection: NavSection;
  contactsSubTab: ContactsSubTab;
  contactsSearchQuery: string;

  threads: Thread[];
  selectedThreadId: string | null;
  threadFilter: "all" | "direct" | "groups";
  searchQuery: string;
  unreadCounts: Record<string, number>;
  historyReachedStart: Record<string, boolean>;
  renderLimitByThread: Record<string, number>;

  replyingTo: ChatMessage | null;
  forwardMsg: ChatMessage | null;
  filterStarredOnly: boolean;
  inChatSearchOpen: boolean;
  inChatSearchQuery: string;
  quoteHighlightId: string | null;
  infoDrawerOpen: boolean;

  settings: AppSettings | null;
  savedAccounts: AccountProfile[];
  settingsModalOpen: boolean;
  settingsTab: SettingsTab;

  phoneModalOpen: boolean;
  phoneLookupLoading: boolean;
  phoneLookupResult: PhoneLookupResult | null;
  phoneLookupError: string | null;

  phoneSyncModalOpen: boolean;
  phoneSyncRunning: boolean;
  phoneSyncProgress: number;
  phoneSyncStatus: string;
  syncRunning: boolean;

  wipeConfirmOpen: boolean;
  devModalOpen: boolean;
  imageViewer: ImageViewerState | null;

  mediaCache: Record<string, string>;
  popularStickers: StickerItem[];
  onionStickers: StickerItem[];
  pinnedPacks: PinnedPackMeta[];
  stickerStorePacks: StickerPackInfo[];
  stickerStoreLoading: boolean;
  stickerPackItems: Record<number, StickerItem[]>;
  stickerSearchResults: StickerItem[];
  stickerSearchLoading: boolean;
  stickerSuggestKeywords: string[];
  composerStickerSuggestions: StickerItem[];
  linkMetadataCache: Record<string, LinkMetadata>;

  friendStatusMap: Record<string, FriendshipStatus>;
  userOnlineMap: Record<string, [boolean, string]>;

  updateChecking: boolean;
  updateInfo: UpdateCheckResult | null;
  updateError: string | null;
  updateDownloading: boolean;
  updateDownloadMessage: string;
  updateProgress: { downloaded: number; total: number } | null;

  statusMessage: string;
  toast: { id: number; message: string } | null;
}

export type AppAction =
  | { type: "BOOTSTRAP"; snapshot: CoreBootstrapSnapshot }
  | { type: "CORE_EVENT"; event: CoreEventDto }
  | { type: "SET_NAV_SECTION"; section: NavSection }
  | { type: "SET_CONTACTS_SUBTAB"; tab: ContactsSubTab }
  | { type: "SET_CONTACTS_SEARCH"; query: string }
  | { type: "SELECT_THREAD"; threadId: string | null }
  | { type: "SET_THREAD_FILTER"; filter: "all" | "direct" | "groups" }
  | { type: "SET_SEARCH_QUERY"; query: string }
  | { type: "EXPAND_RENDER_LIMIT"; threadId: string; step?: number }
  | { type: "SET_REPLYING_TO"; msg: ChatMessage | null }
  | { type: "SET_FORWARD_MSG"; msg: ChatMessage | null }
  | { type: "TOGGLE_STARRED_FILTER" }
  | { type: "TOGGLE_IN_CHAT_SEARCH"; open?: boolean }
  | { type: "SET_IN_CHAT_SEARCH_QUERY"; query: string }
  | { type: "SET_QUOTE_HIGHLIGHT"; msgId: string | null }
  | { type: "TOGGLE_INFO_DRAWER"; open?: boolean }
  | { type: "UPDATE_SETTINGS_LOCAL"; settings: AppSettings }
  | { type: "SET_SAVED_ACCOUNTS"; accounts: AccountProfile[] }
  | { type: "OPEN_SETTINGS"; open: boolean; tab?: SettingsTab }
  | { type: "SET_PHONE_MODAL"; open: boolean }
  | { type: "START_PHONE_LOOKUP" }
  | { type: "SET_PHONE_SYNC_MODAL"; open: boolean }
  | { type: "START_CLOUD_SYNC" }
  | { type: "START_PHONE_SYNC" }
  | { type: "SET_WIPE_CONFIRM"; open: boolean }
  | { type: "SET_DEV_MODAL"; open: boolean }
  | { type: "SET_IMAGE_VIEWER"; viewer: ImageViewerState | null }
  | { type: "SET_LOGIN_IN_PROGRESS"; pending: boolean; status?: string }
  | { type: "SET_STICKER_STORE_LOADING"; loading: boolean }
  | { type: "SET_STICKER_SEARCH_LOADING"; loading: boolean }
  | { type: "SET_UPDATE_CHECKING"; checking: boolean }
  | { type: "SHOW_TOAST"; message: string }
  | { type: "CLEAR_TOAST" }
  | { type: "TOGGLE_LOCAL_STAR"; threadId: string; msgId: string };

export const initialAppState: AppState = {
  bootstrapped: false,
  appVersion: "0.0.1",
  devMode: false,
  appLogoDataUrl: "",
  reactionAssets: { like: "", heart: "", smile: "", surprised: "", cry: "", rage: "" },
  activeDbPath: "zalo_history.db",
  loggedIn: false,
  uin: "",
  displayName: "",
  avatar: null,
  qrDataUrl: null,
  qrStatus: "Đang tạo mã QR đăng nhập…",
  qrScannedName: null,
  qrScannedAvatar: null,
  loginInProgress: false,
  loginError: "",
  activeSection: "chat",
  contactsSubTab: "friends",
  contactsSearchQuery: "",
  threads: [],
  selectedThreadId: null,
  threadFilter: "all",
  searchQuery: "",
  unreadCounts: {},
  historyReachedStart: {},
  renderLimitByThread: {},
  replyingTo: null,
  forwardMsg: null,
  filterStarredOnly: false,
  inChatSearchOpen: false,
  inChatSearchQuery: "",
  quoteHighlightId: null,
  infoDrawerOpen: false,
  settings: null,
  savedAccounts: [],
  settingsModalOpen: false,
  settingsTab: "general",
  phoneModalOpen: false,
  phoneLookupLoading: false,
  phoneLookupResult: null,
  phoneLookupError: null,
  phoneSyncModalOpen: false,
  phoneSyncRunning: false,
  phoneSyncProgress: 0,
  phoneSyncStatus: "",
  syncRunning: false,
  wipeConfirmOpen: false,
  devModalOpen: false,
  imageViewer: null,
  mediaCache: {},
  popularStickers: [],
  onionStickers: [],
  pinnedPacks: [],
  stickerStorePacks: [],
  stickerStoreLoading: false,
  stickerPackItems: {},
  stickerSearchResults: [],
  stickerSearchLoading: false,
  stickerSuggestKeywords: [],
  composerStickerSuggestions: [],
  linkMetadataCache: {},
  friendStatusMap: {},
  userOnlineMap: {},
  updateChecking: false,
  updateInfo: null,
  updateError: null,
  updateDownloading: false,
  updateDownloadMessage: "",
  updateProgress: null,
  statusMessage: "Trực tuyến · Sẵn sàng",
  toast: null,
};

let toastSeq = 1;

function nextToast(message: string): { id: number; message: string } {
  toastSeq += 1;
  return { id: toastSeq, message };
}

function updateThreadMessage(
  threads: Thread[],
  threadId: string,
  updater: (t: Thread) => Thread,
): Thread[] {
  return threads.map((t) => (t.id === threadId ? updater(t) : t));
}

function handleCoreEvent(state: AppState, event: CoreEventDto): AppState {
  switch (event.type) {
    case "QrReady":
      return {
        ...state,
        qrDataUrl: event.payload.qr_data_url || null,
        qrStatus: "Mở Zalo trên điện thoại và quét mã QR để đăng nhập",
        qrScannedName: null,
        qrScannedAvatar: null,
        loginInProgress: false,
        loginError: "",
      };

    case "QrScanned":
      return {
        ...state,
        qrScannedName: event.payload.name,
        qrScannedAvatar: event.payload.avatar,
        qrStatus: `Đã quét (${event.payload.name}) — Vui lòng xác nhận đăng nhập trên điện thoại`,
      };

    case "QrExpired":
      return {
        ...state,
        qrDataUrl: null,
        qrScannedName: null,
        qrStatus: event.payload.message,
      };

    case "LoginSuccess":
      return {
        ...state,
        loggedIn: true,
        uin: event.payload.uin,
        displayName: event.payload.name,
        avatar: event.payload.avatar,
        loginInProgress: false,
        loginError: "",
        qrDataUrl: null,
        qrScannedName: null,
        statusMessage: "Đã đăng nhập",
        toast: nextToast(`Chào mừng ${event.payload.name}`),
      };

    case "LoginFailed":
      return {
        ...state,
        loginInProgress: false,
        loginError: event.payload.message,
        statusMessage: event.payload.message,
      };

    case "LoggedOut":
    case "SessionExpired":
      return {
        ...state,
        loggedIn: false,
        uin: "",
        displayName: "",
        avatar: null,
        threads: [],
        selectedThreadId: null,
        unreadCounts: {},
        settingsModalOpen: false,
        qrDataUrl: null,
        qrScannedName: null,
        qrStatus: "Đang xin mã QR mới…",
        statusMessage:
          event.type === "SessionExpired" ? event.payload.message : "Đã đăng xuất",
      };

    case "AccountSwitched": {
      const sorted = state.settings
        ? sortThreads(event.payload.threads, state.settings)
        : event.payload.threads;
      return {
        ...state,
        loggedIn: true,
        uin: event.payload.uin,
        displayName: event.payload.name,
        avatar: event.payload.avatar,
        threads: sorted,
        selectedThreadId: sorted[0]?.id ?? null,
        unreadCounts: {},
        settingsModalOpen: false,
        statusMessage: `Đã chuyển sang tài khoản ${event.payload.name}`,
        toast: nextToast(`Đã chuyển sang ${event.payload.name}`),
      };
    }

    case "ThreadsUpdated": {
      const updates = event.payload.threads;
      if (!state.loggedIn || updates.length === 0) return state;

      const byId = new Map(state.threads.map((t) => [t.id, t]));
      const unreadCounts = { ...state.unreadCounts };

      for (const incoming of updates) {
        if (!incoming.id || incoming.id === "0") continue;
        const existing = byId.get(incoming.id);
        if (existing) {
          const oldLen = existing.messages.length;
          const merged = mergeThreadInto(existing, incoming);
          byId.set(incoming.id, merged);
          const added = merged.messages.length - oldLen;
          if (added > 0) {
            const newIncoming = merged.messages
              .slice(-added)
              .filter((m) => !m.is_outgoing).length;
            if (newIncoming > 0 && incoming.id !== state.selectedThreadId) {
              unreadCounts[incoming.id] = (unreadCounts[incoming.id] ?? 0) + newIncoming;
            }
          }
        } else {
          byId.set(incoming.id, incoming);
        }
      }

      const rawList = Array.from(byId.values());
      const threads = state.settings ? sortThreads(rawList, state.settings) : rawList;
      const selectedThreadId =
        state.selectedThreadId && threads.some((t) => t.id === state.selectedThreadId)
          ? state.selectedThreadId
          : (threads[0]?.id ?? null);

      return {
        ...state,
        threads,
        selectedThreadId,
        unreadCounts,
        statusMessage: `Trực tuyến · Cập nhật ${formatClockNow()}`,
      };
    }

    case "MessageSent": {
      const now = formatClockNow();
      const finalId = event.payload.msg_id || `out_${event.payload.client_message_id}`;
      const msg: ChatMessage = {
        id: finalId,
        client_message_id: event.payload.client_message_id,
        sender: "Tôi",
        text: event.payload.text,
        time: now,
        is_outgoing: true,
        ts: Date.now(),
      };
      return {
        ...state,
        threads: updateThreadMessage(state.threads, event.payload.thread_id, (t) =>
          reconcileSentMessage(t, msg),
        ),
        statusMessage: `Đã gửi tin nhắn lúc ${now}`,
      };
    }

    case "StickerSent": {
      const now = formatClockNow();
      const finalId = event.payload.msg_id || `out_${event.payload.client_message_id}`;
      const msg: ChatMessage = {
        id: finalId,
        client_message_id: event.payload.client_message_id,
        sender: "Tôi",
        text: "[Nhãn dán]",
        time: now,
        is_outgoing: true,
        ts: Date.now(),
        sticker_id: event.payload.sticker_id,
        sticker_cate_id: event.payload.cate_id,
        sticker_type: event.payload.sticker_type,
      };
      return {
        ...state,
        threads: updateThreadMessage(state.threads, event.payload.thread_id, (t) =>
          reconcileSentMessage(t, msg),
        ),
        statusMessage: `Đã gửi nhãn dán lúc ${now}`,
      };
    }

    case "AttachmentSent":
      return {
        ...state,
        threads: updateThreadMessage(state.threads, event.payload.thread_id, (t) =>
          reconcileSentMessage(t, event.payload.msg),
        ),
        statusMessage: `Đã gửi đính kèm lúc ${formatClockNow()}`,
      };

    case "QuoteSent": {
      const now = formatClockNow();
      const finalId = event.payload.msg_id || `out_${event.payload.client_message_id}`;
      const msg: ChatMessage = {
        id: finalId,
        client_message_id: event.payload.client_message_id,
        sender: "Tôi",
        text: event.payload.text,
        time: now,
        is_outgoing: true,
        ts: Date.now(),
        reply_to_id: event.payload.quoted.id,
        reply_to_sender: event.payload.quoted.sender,
        reply_to_text: event.payload.quoted.text,
      };
      return {
        ...state,
        threads: updateThreadMessage(state.threads, event.payload.thread_id, (t) =>
          reconcileSentMessage(t, msg),
        ),
        statusMessage: `Đã gửi trả lời lúc ${now}`,
      };
    }

    case "Reacted":
      return {
        ...state,
        threads: updateThreadMessage(state.threads, event.payload.thread_id, (t) => ({
          ...t,
          messages: t.messages.map((m) =>
            (m.id && m.id === event.payload.msg_id) || (!m.id && m.ts === event.payload.ts)
              ? { ...m, reactions: [event.payload.reaction] }
              : m,
          ),
        })),
        statusMessage: "Đã thả cảm xúc",
      };

    case "MessageDeleted":
      return {
        ...state,
        threads: updateThreadMessage(state.threads, event.payload.thread_id, (t) => ({
          ...t,
          messages: t.messages.filter(
            (m) =>
              !((event.payload.msg_id && m.id === event.payload.msg_id) ||
                (!event.payload.msg_id && m.ts === event.payload.ts)),
          ),
        })),
        statusMessage: "Đã xóa tin nhắn khỏi lịch sử máy này",
      };

    case "MessageRecalled":
      return {
        ...state,
        threads: updateThreadMessage(state.threads, event.payload.thread_id, (t) => ({
          ...t,
          messages: t.messages.map((m) =>
            (event.payload.msg_id && m.id === event.payload.msg_id) ||
            (!event.payload.msg_id && m.ts === event.payload.ts)
              ? {
                  ...m,
                  text: "[Tin nhắn đã thu hồi]",
                  sticker_id: undefined,
                  link_url: undefined,
                  link_thumb: undefined,
                }
              : m,
          ),
        })),
        statusMessage: "Đã thu hồi tin nhắn",
      };

    case "SendFailed":
      return {
        ...state,
        statusMessage: event.payload.error,
        toast: nextToast(event.payload.error),
      };

    case "StickerImageReady":
      return {
        ...state,
        mediaCache: { ...state.mediaCache, [event.payload.key]: event.payload.data_url },
      };

    case "StickerStoreLoaded":
      return {
        ...state,
        stickerStorePacks: event.payload.packs,
        stickerStoreLoading: false,
      };

    case "StickerPackItemsLoaded":
      return {
        ...state,
        stickerPackItems: {
          ...state.stickerPackItems,
          [event.payload.cate_id]: event.payload.items,
        },
      };

    case "StickerSearchResults":
      return {
        ...state,
        stickerSearchResults: event.payload.items,
        stickerSearchLoading: false,
      };

    case "StickerSuggestKeywordsLoaded":
      return {
        ...state,
        stickerSuggestKeywords: event.payload.keywords,
      };

    case "ComposerStickerSuggestions":
      return {
        ...state,
        composerStickerSuggestions: event.payload.items,
      };

    case "LinkPreviewReady":
      return {
        ...state,
        linkMetadataCache: {
          ...state.linkMetadataCache,
          [event.payload.meta.url]: event.payload.meta,
        },
      };

    case "PhoneLookupSuccess":
      return {
        ...state,
        phoneLookupLoading: false,
        phoneLookupResult: event.payload.profile,
        phoneLookupError: null,
        friendStatusMap: {
          ...state.friendStatusMap,
          [event.payload.profile.user_id]: event.payload.profile.friendship,
        },
      };

    case "PhoneLookupFailed":
      return {
        ...state,
        phoneLookupLoading: false,
        phoneLookupResult: null,
        phoneLookupError: event.payload.error,
      };

    case "FriendStatusChecked":
      return {
        ...state,
        friendStatusMap: {
          ...state.friendStatusMap,
          [event.payload.user_id]: event.payload.status,
        },
        userOnlineMap: event.payload.online
          ? { ...state.userOnlineMap, [event.payload.user_id]: event.payload.online }
          : state.userOnlineMap,
      };

    case "LocalDataWiped":
      return {
        ...state,
        threads: state.threads.map((t) => ({ ...t, messages: [] })),
        toast: nextToast("Đã xóa dữ liệu tin nhắn cục bộ"),
        statusMessage: "Đã xóa dữ liệu tin nhắn cục bộ",
      };

    case "StatusUpdate":
      return {
        ...state,
        statusMessage: event.payload.message,
      };

    case "RefreshFinished": {
      const msg =
        event.payload.errors.length === 0
          ? `Đã làm mới ${event.payload.threads} hội thoại (${event.payload.messages} tin nhắn)`
          : `Làm mới ${event.payload.threads} hội thoại · ${event.payload.errors.length} lỗi`;
      return {
        ...state,
        statusMessage: msg,
        toast: event.payload.manual ? nextToast(msg) : state.toast,
      };
    }

    case "SyncFinished": {
      const msg = event.payload.cancelled
        ? `Đã dừng đồng bộ (đã thêm ${event.payload.added} tin nhắn)`
        : `Đồng bộ hoàn tất: +${event.payload.added} tin nhắn`;
      return {
        ...state,
        syncRunning: false,
        statusMessage: msg,
        toast: nextToast(msg),
      };
    }

    case "PhoneSyncProgress":
      return {
        ...state,
        phoneSyncRunning: true,
        phoneSyncProgress: event.payload.progress,
        phoneSyncStatus: event.payload.status,
      };

    case "PhoneSyncFinished": {
      const msg = event.payload.error
        ? `Đồng bộ điện thoại lỗi: ${event.payload.error}`
        : event.payload.cancelled
          ? `Đã dừng đồng bộ điện thoại (+${event.payload.added} tin nhắn)`
          : `Đồng bộ điện thoại hoàn tất (+${event.payload.added} tin nhắn từ ${event.payload.active_threads} hội thoại)`;
      const threads = state.settings
        ? sortThreads(event.payload.threads, state.settings)
        : event.payload.threads;
      return {
        ...state,
        phoneSyncRunning: false,
        phoneSyncProgress: event.payload.error ? 0 : 1,
        phoneSyncStatus: msg,
        threads: threads.length > 0 ? threads : state.threads,
        toast: nextToast(msg),
      };
    }

    case "HistoryReachedStart":
      return {
        ...state,
        historyReachedStart: { ...state.historyReachedStart, [event.payload.thread_id]: true },
        toast: nextToast("Đã hiển thị toàn bộ lịch sử hội thoại này"),
      };

    case "MuteSnapshotSynced": {
      if (!state.settings) return state;
      const muted_threads = { ...state.settings.muted_threads, ...event.payload.muted_threads };
      return {
        ...state,
        settings: { ...state.settings, muted_threads },
      };
    }

    case "MuteEventsReceived": {
      if (!state.settings) return state;
      const muted_threads = { ...state.settings.muted_threads };
      let threads = state.threads;
      for (const ev of event.payload.events) {
        if (ev.muted) {
          if (ev.settings_until != null) muted_threads[ev.thread_id] = ev.settings_until;
        } else {
          delete muted_threads[ev.thread_id];
        }
        threads = updateThreadMessage(threads, ev.thread_id, (t) => ({
          ...t,
          presentation: { ...t.presentation, muted: ev.muted },
          unread: { ...t.unread, muted: ev.muted, muted_until: ev.muted_until },
        }));
      }
      return {
        ...state,
        threads,
        settings: { ...state.settings, muted_threads },
      };
    }

    case "ProxyUpdated":
      return {
        ...state,
        toast: nextToast(
          event.payload.ok
            ? event.payload.proxy_url
              ? `Đã áp dụng proxy: ${event.payload.proxy_url}`
              : "Đã xóa cấu hình proxy"
            : `Lỗi cấu hình proxy: ${event.payload.error ?? "không xác định"}`,
        ),
      };

    case "FileDownloaded":
      return {
        ...state,
        toast: nextToast(
          event.payload.ok
            ? `Đã lưu tệp: ${event.payload.file_name} (${event.payload.path})`
            : `Tải tệp thất bại: ${event.payload.error ?? "không xác định"}`,
        ),
      };

    case "UpdateCheckFinished":
      return {
        ...state,
        updateChecking: false,
        updateInfo: event.payload.ok ? event.payload.info : state.updateInfo,
        updateError: event.payload.ok ? null : (event.payload.error ?? "Không kiểm tra được cập nhật"),
        toast:
          !event.payload.silent && event.payload.ok && event.payload.info && !event.payload.info.has_update
            ? nextToast("Bạn đang dùng phiên bản mới nhất")
            : state.toast,
      };

    case "UpdateDownloadStatus":
      return {
        ...state,
        updateDownloading: event.payload.downloading,
        updateDownloadMessage: event.payload.message,
      };

    case "UpdateDownloadProgress":
      return {
        ...state,
        updateProgress: { downloaded: event.payload.downloaded, total: event.payload.total },
      };

    case "UpdateAppliedAndRestarting":
      return {
        ...state,
        updateDownloading: false,
        updateDownloadMessage: `Đã cập nhật lên v${event.payload.version} — đang khởi động lại…`,
        toast: nextToast(`Đã cập nhật v${event.payload.version}`),
      };

    case "SessionUnreachable":
      return {
        ...state,
        statusMessage: `Mất kết nối tạm thời: ${event.payload.message}`,
      };

    case "NotificationAuthorization":
      return {
        ...state,
        statusMessage: event.payload.status_label ?? event.payload.error ?? state.statusMessage,
      };
  }
}

export function appReducer(state: AppState, action: AppAction): AppState {
  switch (action.type) {
    case "BOOTSTRAP": {
      const snap = action.snapshot;
      const sorted = sortThreads(snap.threads, snap.settings);
      const packMap: Record<number, StickerItem[]> = {};
      for (const p of snap.pinned_packs) packMap[p.cate_id] = p.items;
      return {
        ...state,
        bootstrapped: true,
        appVersion: snap.app_version,
        devMode: snap.dev_mode,
        appLogoDataUrl: snap.app_logo_data_url,
        reactionAssets: snap.reaction_assets,
        activeDbPath: snap.active_db_path,
        loggedIn: snap.logged_in,
        uin: snap.uin,
        displayName: snap.display_name,
        avatar: snap.avatar,
        threads: sorted,
        selectedThreadId: sorted[0]?.id ?? null,
        infoDrawerOpen: snap.settings.info_drawer_open,
        settings: snap.settings,
        savedAccounts: snap.saved_accounts,
        popularStickers: snap.popular_stickers,
        onionStickers: snap.onion_stickers,
        pinnedPacks: snap.pinned_packs,
        stickerPackItems: packMap,
        syncRunning: snap.sync_running,
        phoneSyncRunning: snap.phone_sync_running,
      };
    }
    case "CORE_EVENT":
      return handleCoreEvent(state, action.event);
    case "SET_NAV_SECTION":
      return { ...state, activeSection: action.section };
    case "SET_CONTACTS_SUBTAB":
      return { ...state, contactsSubTab: action.tab };
    case "SET_CONTACTS_SEARCH":
      return { ...state, contactsSearchQuery: action.query };
    case "SELECT_THREAD": {
      const nextUnread = { ...state.unreadCounts };
      if (action.threadId) delete nextUnread[action.threadId];
      return {
        ...state,
        activeSection: "chat",
        selectedThreadId: action.threadId,
        unreadCounts: nextUnread,
        replyingTo: null,
        inChatSearchOpen: false,
        inChatSearchQuery: "",
        quoteHighlightId: null,
      };
    }
    case "SET_THREAD_FILTER":
      return { ...state, threadFilter: action.filter };
    case "SET_SEARCH_QUERY":
      return { ...state, searchQuery: action.query };
    case "EXPAND_RENDER_LIMIT": {
      const cur = state.renderLimitByThread[action.threadId] ?? 150;
      return {
        ...state,
        renderLimitByThread: {
          ...state.renderLimitByThread,
          [action.threadId]: cur + (action.step ?? 150),
        },
      };
    }
    case "SET_REPLYING_TO":
      return { ...state, replyingTo: action.msg };
    case "SET_FORWARD_MSG":
      return { ...state, forwardMsg: action.msg };
    case "TOGGLE_STARRED_FILTER":
      return { ...state, filterStarredOnly: !state.filterStarredOnly };
    case "TOGGLE_IN_CHAT_SEARCH":
      return {
        ...state,
        inChatSearchOpen: action.open ?? !state.inChatSearchOpen,
        inChatSearchQuery: action.open === false ? "" : state.inChatSearchQuery,
      };
    case "SET_IN_CHAT_SEARCH_QUERY":
      return { ...state, inChatSearchQuery: action.query };
    case "SET_QUOTE_HIGHLIGHT":
      return { ...state, quoteHighlightId: action.msgId };
    case "TOGGLE_INFO_DRAWER": {
      const open = action.open ?? !state.infoDrawerOpen;
      return {
        ...state,
        infoDrawerOpen: open,
        settings: state.settings ? { ...state.settings, info_drawer_open: open } : null,
      };
    }
    case "UPDATE_SETTINGS_LOCAL":
      return {
        ...state,
        settings: action.settings,
        threads: sortThreads(state.threads, action.settings),
      };
    case "SET_SAVED_ACCOUNTS":
      return { ...state, savedAccounts: action.accounts };
    case "OPEN_SETTINGS":
      return {
        ...state,
        settingsModalOpen: action.open,
        settingsTab: action.tab ?? state.settingsTab,
      };
    case "SET_PHONE_MODAL":
      return {
        ...state,
        phoneModalOpen: action.open,
        ...(action.open ? {} : { phoneLookupError: null }),
      };
    case "START_PHONE_LOOKUP":
      return { ...state, phoneLookupLoading: true, phoneLookupError: null };
    case "SET_PHONE_SYNC_MODAL":
      return { ...state, phoneSyncModalOpen: action.open };
    case "START_CLOUD_SYNC":
      return { ...state, syncRunning: true };
    case "START_PHONE_SYNC":
      return {
        ...state,
        phoneSyncRunning: true,
        phoneSyncProgress: 0.05,
        phoneSyncStatus: "Đang gửi yêu cầu đồng bộ tin nhắn tới điện thoại…",
      };
    case "SET_WIPE_CONFIRM":
      return { ...state, wipeConfirmOpen: action.open };
    case "SET_DEV_MODAL":
      return { ...state, devModalOpen: action.open };
    case "SET_IMAGE_VIEWER":
      return { ...state, imageViewer: action.viewer };
    case "SET_LOGIN_IN_PROGRESS":
      return {
        ...state,
        loginInProgress: action.pending,
        loginError: action.pending ? "" : state.loginError,
        statusMessage: action.status ?? state.statusMessage,
      };
    case "SET_STICKER_STORE_LOADING":
      return { ...state, stickerStoreLoading: action.loading };
    case "SET_STICKER_SEARCH_LOADING":
      return { ...state, stickerSearchLoading: action.loading };
    case "SET_UPDATE_CHECKING":
      return { ...state, updateChecking: action.checking, updateError: null };
    case "SHOW_TOAST":
      return { ...state, toast: nextToast(action.message) };
    case "CLEAR_TOAST":
      return { ...state, toast: null };
    case "TOGGLE_LOCAL_STAR":
      return {
        ...state,
        threads: updateThreadMessage(state.threads, action.threadId, (t) => ({
          ...t,
          messages: t.messages.map((m) =>
            m.id === action.msgId ? { ...m, is_starred: !m.is_starred } : m,
          ),
        })),
      };
  }
}
