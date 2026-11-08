import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { openUrl } from "@tauri-apps/plugin-opener";
import type {
  AccountProfile,
  AppSettings,
  AutoReplyRule,
  ChatMessage,
  CoreBootstrapSnapshot,
  CoreEventDto,
  DevDiagnosticsSnapshot,
  ImportSummaryDto,
  Thread,
} from "@/shared/types/ipc";

declare global {
  interface Window {
    __TAURI_INTERNALS__?: unknown;
  }
}

export function isTauriRuntime(): boolean {
  return typeof window !== "undefined" && Boolean(window.__TAURI_INTERNALS__);
}

async function call<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (!isTauriRuntime()) {
    throw new Error("Ứng dụng cần chạy bên trong cửa sổ Tauri AloLite.");
  }
  return invoke<T>(cmd, args);
}

export const ipc = {
  onCoreEvent(handler: (event: CoreEventDto) => void): Promise<UnlistenFn> {
    if (!isTauriRuntime()) {
      return Promise.reject(new Error("Ứng dụng cần chạy bên trong cửa sổ Tauri AloLite."));
    }
    return listen<CoreEventDto>("core-event", (ev) => handler(ev.payload));
  },

  bootstrapState: () => call<CoreBootstrapSnapshot>("bootstrap_state"),
  requestQr: () => call<void>("request_qr"),
  loginPassword: (phone: string, pass: string) => call<void>("login_password", { phone, pass }),
  logout: () => call<void>("logout"),
  switchAccount: (targetUin: string) => call<void>("switch_account", { targetUin }),
  addNewAccount: () => call<void>("add_new_account"),
  listSavedAccounts: () => call<AccountProfile[]>("get_saved_accounts"),
  deleteSavedAccount: (uin: string) => call<void>("delete_saved_account", { uin }),
  wipeLocalData: () => call<void>("wipe_local_data"),
  refreshThreads: (manual = false) => call<void>("refresh_threads", { manual }),
  setKnownThreads: (threads: [string, string, boolean][]) =>
    call<void>("set_known_threads", { threads }),
  persistThreads: (threads: Thread[]) => call<void>("persist_threads", { threads }),
  loadGroupHistory: (groupId: string, isGroup: boolean, have: number) =>
    call<void>("load_group_history", { groupId, isGroup, have }),
  syncHistory: (threads: [string, string, boolean][]) => call<void>("sync_history", { threads }),
  cancelCloudSync: () => call<void>("cancel_cloud_sync"),
  syncPhoneMessages: (threads: Thread[], sendPhonePrompt: boolean) =>
    call<void>("sync_phone_messages", { threads, sendPhonePrompt }),
  cancelPhoneSync: () => call<void>("cancel_phone_sync"),
  sendMessage: (toId: string, text: string, isGroup: boolean) =>
    call<void>("send_message", { toId, text, isGroup }),
  sendQuote: (toId: string, text: string, quoted: ChatMessage, isGroup: boolean) =>
    call<void>("send_quote", { toId, text, quoted, isGroup }),
  forwardMessage: (targetId: string, targetIsGroup: boolean, msg: ChatMessage) =>
    call<void>("forward_message", { targetId, targetIsGroup, msg }),
  reactMessage: (toId: string, isGroup: boolean, msg: ChatMessage, reaction: string) =>
    call<void>("react_message", { toId, isGroup, msg, reaction }),
  deleteForMe: (toId: string, isGroup: boolean, msg: ChatMessage) =>
    call<void>("delete_for_me", { toId, isGroup, msg }),
  recallMessage: (toId: string, isGroup: boolean, msg: ChatMessage) =>
    call<void>("recall_message", { toId, isGroup, msg }),
  sendSticker: (
    toId: string,
    stickerId: number,
    cateId: number,
    stickerType: number,
    isGroup: boolean,
  ) =>
    call<void>("send_sticker", {
      toId,
      stickerId,
      cateId,
      stickerType,
      isGroup,
    }),
  sendAttachmentFile: (toId: string, isGroup: boolean, path: string, caption: string) =>
    call<void>("send_attachment_file", { toId, isGroup, path, caption }),
  sendAttachmentBytes: (
    toId: string,
    isGroup: boolean,
    fileName: string,
    bytes: number[],
    caption: string,
  ) =>
    call<void>("send_attachment_bytes", {
      toId,
      isGroup,
      fileName,
      bytes,
      caption,
    }),
  fetchMediaData: (key: string, url: string) => call<void>("fetch_media_data", { key, url }),
  fetchStickerStore: () => call<void>("fetch_sticker_store"),
  fetchStickerPack: (cateId: number, name: string) =>
    call<void>("fetch_sticker_pack", { cateId, name }),
  searchStickers: (keyword: string) => call<void>("search_stickers", { keyword }),
  suggestComposerStickers: (keyword: string) =>
    call<void>("suggest_composer_stickers", { keyword }),
  parseLinkPreview: (url: string) => call<void>("parse_link_preview", { url }),
  sendTyping: (threadId: string, isGroup: boolean) =>
    call<void>("send_typing", { threadId, isGroup }),
  sendSeen: (threadId: string, isGroup: boolean, lastMsgId: string) =>
    call<void>("send_seen", { threadId, isGroup, lastMsgId }),
  pinThread: (threadId: string, isGroup: boolean, pin: boolean) =>
    call<void>("pin_thread", { threadId, isGroup, pin }),
  markThreadUnread: (threadId: string, isGroup: boolean, unread: boolean) =>
    call<void>("mark_thread_unread", { threadId, isGroup, unread }),
  muteThread: (threadId: string, isGroup: boolean, durationSec: number, mute: boolean) =>
    call<void>("mute_thread", { threadId, isGroup, durationSec, mute }),
  hideThread: (threadId: string, isGroup: boolean, hide: boolean) =>
    call<void>("hide_thread", { threadId, isGroup, hide }),
  deleteThread: (threadId: string, isGroup: boolean, lastCliMsgId: string) =>
    call<void>("delete_thread", { threadId, isGroup, lastCliMsgId }),
  lookupPhone: (phone: string) => call<void>("lookup_phone", { phone }),
  addFriend: (userId: string, userName: string) => call<void>("add_friend", { userId, userName }),
  checkFriendStatus: (userId: string) => call<void>("check_friend_status", { userId }),
  acceptFriend: (userId: string, userName: string) =>
    call<void>("accept_friend", { userId, userName }),
  rejectFriend: (userId: string, userName: string) =>
    call<void>("reject_friend", { userId, userName }),
  undoFriend: (userId: string, userName: string) => call<void>("undo_friend", { userId, userName }),
  saveSettings: (settings: AppSettings) => call<AppSettings>("save_settings", { settings }),
  setProxy: (proxyUrl: string) => call<void>("set_proxy", { proxyUrl }),
  downloadFile: (url: string, fileName: string, downloadDir: string) =>
    call<void>("download_file", { url, fileName, downloadDir }),
  importLocalHistory: (path: string) => call<ImportSummaryDto>("import_local_history", { path }),
  autoDetectImportHistory: () => call<ImportSummaryDto>("auto_detect_import_history"),
  checkForUpdates: (silent: boolean) => call<void>("check_for_updates", { silent }),
  downloadAndApplyUpdate: (
    downloadUrl: string,
    assetName: string,
    targetVersion: string,
    expectedSha256: string | null,
  ) =>
    call<void>("download_and_apply_update", {
      downloadUrl,
      assetName,
      targetVersion,
      expectedSha256,
    }),
  exportCustomStickersTemplate: () => call<string>("export_custom_stickers_tpl"),
  getAutoreplyRules: () => call<AutoReplyRule[]>("get_autoreply_rules"),
  saveAutoreplyRules: (rules: AutoReplyRule[]) => call<void>("save_autoreply_rules", { rules }),
  getDevDiagnostics: () => call<DevDiagnosticsSnapshot>("get_dev_diagnostics"),
  openPathInOs: (path: string) => call<void>("open_path_in_os", { path }),
  spawnMultiProfile: (profile: string, proxyUrl: string) =>
    call<void>("spawn_multi_profile", { profile, proxyUrl }),

  async openExternalUrl(url: string): Promise<void> {
    if (isTauriRuntime()) {
      await openUrl(url);
    } else {
      window.open(url, "_blank", "noopener,noreferrer");
    }
  },

  async pickFile(options?: {
    imagesOnly?: boolean;
    directory?: boolean;
    title?: string;
  }): Promise<string | null> {
    if (!isTauriRuntime()) {
      throw new Error("Ứng dụng cần chạy bên trong cửa sổ Tauri AloLite.");
    }
    const res = await openDialog({
      multiple: false,
      directory: Boolean(options?.directory),
      title: options?.title,
      filters: options?.imagesOnly
        ? [
            {
              name: "Hình ảnh",
              extensions: ["png", "jpg", "jpeg", "gif", "webp"],
            },
          ]
        : undefined,
    });
    return typeof res === "string" ? res : null;
  },
};
