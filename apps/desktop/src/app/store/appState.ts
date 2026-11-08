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
  StickerItem,
  StickerPackInfo,
  Thread,
  UpdateCheckResult,
} from "@/shared/types/ipc";

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
  activeDbPath: string;
  inlineVideoSupported: boolean;
  inlineVideoReason: string | null;
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
  unreadCounts: Record<string, number | undefined>;
  historyReachedStart: Record<string, boolean | undefined>;
  renderLimitByThread: Record<string, number | undefined>;

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

  mediaCache: Record<string, string | undefined>;
  popularStickers: StickerItem[];
  onionStickers: StickerItem[];
  pinnedPacks: PinnedPackMeta[];
  stickerStorePacks: StickerPackInfo[];
  stickerStoreLoading: boolean;
  stickerPackItems: Record<number, StickerItem[] | undefined>;
  stickerSearchResults: StickerItem[];
  stickerSearchLoading: boolean;
  stickerSuggestKeywords: string[];
  composerStickerSuggestions: StickerItem[];
  linkMetadataCache: Record<string, LinkMetadata | undefined>;

  friendStatusMap: Record<string, FriendshipStatus | undefined>;
  userOnlineMap: Record<string, [boolean, string] | undefined>;
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
  activeDbPath: "zalo_history.db",
  inlineVideoSupported: true,
  inlineVideoReason: null,
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
