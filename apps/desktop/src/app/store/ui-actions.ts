import { sortThreads } from "@/features/threads/utils/threads";
import type { StickerItem } from "@/shared/types/ipc";
import type { AppAction, AppState } from "./app-state";
import { nextToast, updateThreadMessage } from "./helpers";

export type UiAction = Exclude<AppAction, { type: "CORE_EVENT" }>;

export function handleUiAction(state: AppState, action: UiAction): AppState {
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
        activeDbPath: snap.active_db_path,
        inlineVideoSupported: snap.inline_video_supported,
        inlineVideoReason: snap.inline_video_reason ?? null,
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
