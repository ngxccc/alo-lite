import { create } from "zustand";
import { appReducer, initialAppState, type AppAction, type AppState } from "./app-reducer";
import { useAuthStore } from "@/stores/use-auth-store";
import { useUIStore } from "@/stores/use-ui-store";
import { useChatStore } from "@/stores/use-chat-store";
import { useContactsStore } from "@/stores/use-contacts-store";

export interface AppStore extends AppState {
  dispatch: (action: AppAction) => void;
}

export function syncDomainStores(next: AppState): void {
  // Sync to useAuthStore
  useAuthStore.setState({
    bootstrapped: next.bootstrapped,
    appVersion: next.appVersion,
    loggedIn: next.loggedIn,
    uin: next.uin,
    displayName: next.displayName,
    avatar: next.avatar,
    qrDataUrl: next.qrDataUrl,
    qrStatus: next.qrStatus,
    qrScannedName: next.qrScannedName,
    qrScannedAvatar: next.qrScannedAvatar,
    loginInProgress: next.loginInProgress,
    loginError: next.loginError,
    savedAccounts: next.savedAccounts,
  });

  // Sync to useUIStore
  useUIStore.setState({
    themeMode: next.settings?.theme_mode ?? "dark",
    activeSection: next.activeSection,
    devMode: next.devMode,
    appLogoDataUrl: next.appLogoDataUrl,
    activeDbPath: next.activeDbPath,
    inlineVideoSupported: next.inlineVideoSupported,
    inlineVideoReason: next.inlineVideoReason,
    devDiagnostics: null,
    settingsOpen: next.settingsModalOpen,
    activeSettingsTab: next.settingsTab,
    devToolsOpen: next.devModalOpen,
    phoneSyncOpen: next.phoneSyncModalOpen,
    phoneLookupOpen: next.phoneModalOpen,
    wipeConfirmOpen: next.wipeConfirmOpen,
    imageViewerState: next.imageViewer,
    inChatSearchOpen: next.inChatSearchOpen,
    inChatSearchQuery: next.inChatSearchQuery,
    toast: next.toast ? { message: next.toast.message } : null,
  });

  // Sync to useChatStore
  useChatStore.setState({
    threads: next.threads,
    selectedThreadId: next.selectedThreadId,
    threadFilter: next.threadFilter,
    searchQuery: next.searchQuery,
    unreadCounts: next.unreadCounts,
    historyReachedStart: next.historyReachedStart,
    renderLimitByThread: next.renderLimitByThread,
    replyingTo: next.replyingTo,
    forwardMsg: next.forwardMsg,
    filterStarredOnly: next.filterStarredOnly,
    quoteHighlightId: next.quoteHighlightId,
    infoDrawerOpen: next.infoDrawerOpen,
    sendingMsgIds: {},
    pinnedPacks: next.pinnedPacks,
    composerStickerQuery: null,
    composerStickerSuggestions: next.composerStickerSuggestions,
    linkPreviews: next.linkMetadataCache,
    isSyncing: next.syncRunning,
    syncProgress: null,
    lastSyncStats: null,
    phoneSyncProgress: next.phoneSyncProgress
      ? { current: next.phoneSyncProgress, total: 100, stage: next.phoneSyncStatus }
      : null,
    settings: next.settings,
  });

  // Sync to useContactsStore
  useContactsStore.setState({
    contactsSubTab: next.contactsSubTab,
    contactsSearchQuery: next.contactsSearchQuery,
    phoneLookupQuery: "",
    phoneLookupLoading: next.phoneLookupLoading,
    phoneLookupResult: next.phoneLookupResult,
    phoneLookupError: next.phoneLookupError,
    friendStatusByUid: next.friendStatusMap,
  });
}

export const useAppStore = create<AppStore>((set) => ({
  ...initialAppState,
  dispatch: (action: AppAction) =>
    set((state) => {
      const next = appReducer(state, action);
      syncDomainStores(next);
      return next;
    }),
}));
