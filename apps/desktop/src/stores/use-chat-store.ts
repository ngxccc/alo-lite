import { create } from "zustand";
import { immer } from "zustand/middleware/immer";
import type {
  AppSettings,
  ChatMessage,
  DeliveryState,
  LinkMetadata,
  PinnedPackMeta,
  StickerItem,
  Thread,
} from "@/shared/types/ipc";

export interface ChatStoreState {
  threads: Thread[];
  selectedThreadId: string | null;
  threadFilter: "all" | "direct" | "groups";
  searchQuery: string;
  unreadCounts: Record<string, number | undefined>;
  historyReachedStart: Record<string, boolean | undefined>;
  renderLimitByThread: Record<string, number | undefined>;

  replyingTo: ChatMessage | null;
  forwardMsg: ChatMessage | null;
  filterStarredOnly: boolean;
  quoteHighlightId: string | null;
  infoDrawerOpen: boolean;

  sendingMsgIds: Record<string, boolean | undefined>;
  pinnedPacks: PinnedPackMeta[];
  composerStickerQuery: string | null;
  composerStickerSuggestions: StickerItem[];
  linkPreviews: Record<string, LinkMetadata | undefined>;

  // Sync state
  isSyncing: boolean;
  syncProgress: { current: number; total: number } | null;
  lastSyncStats: string | null;
  phoneSyncProgress: { current: number; total: number; stage?: string } | null;

  // App settings cache
  settings: AppSettings | null;

  // Actions
  setThreads: (threads: Thread[]) => void;
  selectThread: (threadId: string | null) => void;
  setThreadFilter: (filter: "all" | "direct" | "groups") => void;
  setSearchQuery: (query: string) => void;

  setReplyingTo: (msg: ChatMessage | null) => void;
  setForwardMsg: (msg: ChatMessage | null) => void;
  toggleFilterStarredOnly: () => void;
  setQuoteHighlightId: (id: string | null) => void;
  setInfoDrawerOpen: (open: boolean) => void;
  toggleInfoDrawer: () => void;

  appendMessage: (threadId: string, msg: ChatMessage) => void;
  updateMessageDelivery: (cliMsgId: string, delivery: DeliveryState, serverMsgId?: string) => void;
  setSending: (cliMsgId: string, sending: boolean) => void;
  markThreadRead: (threadId: string) => void;
  setRenderLimit: (threadId: string, limit: number) => void;
  setHistoryReachedStart: (threadId: string, reached: boolean) => void;

  setPinnedPacks: (packs: PinnedPackMeta[]) => void;
  setComposerStickerSuggestions: (items: StickerItem[], query: string | null) => void;
  setLinkPreview: (url: string, meta: LinkMetadata) => void;

  setIsSyncing: (isSyncing: boolean) => void;
  setSyncProgress: (progress: { current: number; total: number } | null) => void;
  setLastSyncStats: (stats: string | null) => void;
  setPhoneSyncProgress: (
    progress: { current: number; total: number; stage?: string } | null,
  ) => void;

  setSettings: (settings: AppSettings | null) => void;
  updateSettings: (partial: Partial<AppSettings>) => void;
}

export const useChatStore = create<ChatStoreState>()(
  immer((set) => ({
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
    quoteHighlightId: null,
    infoDrawerOpen: false,

    sendingMsgIds: {},
    pinnedPacks: [],
    composerStickerQuery: null,
    composerStickerSuggestions: [],
    linkPreviews: {},

    isSyncing: false,
    syncProgress: null,
    lastSyncStats: null,
    phoneSyncProgress: null,

    settings: null,

    setThreads: (threads) =>
      set((state) => {
        state.threads = threads;
      }),

    selectThread: (threadId) =>
      set((state) => {
        state.selectedThreadId = threadId;
        state.replyingTo = null;
        state.quoteHighlightId = null;
        if (threadId) {
          state.unreadCounts[threadId] = 0;
          const target = state.threads.find((t) => t.id === threadId);
          if (target) {
            target.unread.unread_ids = [];
            target.unread.marked_unread = false;
          }
        }
      }),

    setThreadFilter: (filter) =>
      set((state) => {
        state.threadFilter = filter;
      }),

    setSearchQuery: (query) =>
      set((state) => {
        state.searchQuery = query;
      }),

    setReplyingTo: (msg) =>
      set((state) => {
        state.replyingTo = msg;
      }),

    setForwardMsg: (msg) =>
      set((state) => {
        state.forwardMsg = msg;
      }),

    toggleFilterStarredOnly: () =>
      set((state) => {
        state.filterStarredOnly = !state.filterStarredOnly;
      }),

    setQuoteHighlightId: (id) =>
      set((state) => {
        state.quoteHighlightId = id;
      }),

    setInfoDrawerOpen: (open) =>
      set((state) => {
        state.infoDrawerOpen = open;
      }),

    toggleInfoDrawer: () =>
      set((state) => {
        state.infoDrawerOpen = !state.infoDrawerOpen;
      }),

    appendMessage: (threadId, msg) =>
      set((state) => {
        const thread = state.threads.find((t) => t.id === threadId);
        if (thread) {
          // Avoid duplicate messages
          const existingIdx = thread.messages.findIndex(
            (m) =>
              (m.id && m.id === msg.id) ||
              (m.client_message_id &&
                msg.client_message_id &&
                m.client_message_id === msg.client_message_id),
          );
          if (existingIdx !== -1) {
            const existing = thread.messages[existingIdx];
            if (existing) {
              Object.assign(existing, msg);
            }
          } else {
            thread.messages.push(msg);
          }
          thread.preview = msg.text || thread.preview;
          thread.time = msg.time || thread.time;
        }
        if (state.selectedThreadId !== threadId) {
          state.unreadCounts[threadId] = (state.unreadCounts[threadId] ?? 0) + 1;
        }
      }),

    updateMessageDelivery: (cliMsgId, delivery, serverMsgId) =>
      set((state) => {
        for (const thread of state.threads) {
          const msg = thread.messages.find(
            (m) => m.client_message_id === cliMsgId || (serverMsgId && m.id === serverMsgId),
          );
          if (msg) {
            msg.delivery = delivery;
            if (serverMsgId) msg.id = serverMsgId;
            break;
          }
        }
        delete state.sendingMsgIds[cliMsgId];
      }),

    setSending: (cliMsgId, sending) =>
      set((state) => {
        if (sending) {
          state.sendingMsgIds[cliMsgId] = true;
        } else {
          delete state.sendingMsgIds[cliMsgId];
        }
      }),

    markThreadRead: (threadId) =>
      set((state) => {
        state.unreadCounts[threadId] = 0;
        const target = state.threads.find((t) => t.id === threadId);
        if (target) {
          target.unread.unread_ids = [];
          target.unread.marked_unread = false;
        }
      }),

    setRenderLimit: (threadId, limit) =>
      set((state) => {
        state.renderLimitByThread[threadId] = limit;
      }),

    setHistoryReachedStart: (threadId, reached) =>
      set((state) => {
        state.historyReachedStart[threadId] = reached;
      }),

    setPinnedPacks: (packs) =>
      set((state) => {
        state.pinnedPacks = packs;
      }),

    setComposerStickerSuggestions: (items, query) =>
      set((state) => {
        state.composerStickerSuggestions = items;
        state.composerStickerQuery = query;
      }),

    setLinkPreview: (url, meta) =>
      set((state) => {
        state.linkPreviews[url] = meta;
      }),

    setIsSyncing: (isSyncing) =>
      set((state) => {
        state.isSyncing = isSyncing;
      }),

    setSyncProgress: (progress) =>
      set((state) => {
        state.syncProgress = progress;
      }),

    setLastSyncStats: (stats) =>
      set((state) => {
        state.lastSyncStats = stats;
      }),

    setPhoneSyncProgress: (progress) =>
      set((state) => {
        state.phoneSyncProgress = progress;
      }),

    setSettings: (settings) =>
      set((state) => {
        state.settings = settings;
      }),

    updateSettings: (partial) =>
      set((state) => {
        if (state.settings) {
          Object.assign(state.settings, partial);
        }
      }),
  })),
);
