import type { CoreEventDto } from "@/shared/types/ipc.generated";
import type { AppState } from "../app-state";
import { handleContactsEvent } from "./contacts";
import { handleLoginEvent } from "./login";
import { handleMediaEvent } from "./media";
import { handleMessagesEvent } from "./messages";
import { handleSettingsEvent } from "./settings";
import { handleSyncEvent } from "./sync";

export function handleCoreEvent(state: AppState, event: CoreEventDto): AppState {
  switch (event.type) {
    case "QrReady":
    case "QrScanned":
    case "QrExpired":
    case "LoginSuccess":
    case "LoginFailed":
    case "CaptchaRequired":
    case "LoggedOut":
    case "SessionExpired":
    case "AccountSwitched":
    case "SessionUnreachable":
      return handleLoginEvent(state, event);

    case "ThreadsUpdated":
    case "MessageSent":
    case "StickerSent":
    case "AttachmentSent":
    case "QuoteSent":
    case "Reacted":
    case "ReactFailed":
    case "MessageDeleted":
    case "MessageRecalled":
    case "SendFailed":
    case "HistoryReachedStart":
      return handleMessagesEvent(state, event);

    case "StickerImageReady":
    case "StickerStoreLoaded":
    case "StickerPackItemsLoaded":
    case "StickerSearchResults":
    case "StickerSuggestKeywordsLoaded":
    case "ComposerStickerSuggestions":
    case "LinkPreviewReady":
    case "FileDownloaded":
      return handleMediaEvent(state, event);

    case "LocalDataWiped":
    case "StatusUpdate":
    case "RefreshFinished":
    case "SyncFinished":
    case "PhoneSyncProgress":
    case "PhoneSyncFinished":
      return handleSyncEvent(state, event);

    case "MuteSnapshotSynced":
    case "MuteEventsReceived":
    case "NotificationAuthorization":
    case "UpdateCheckFinished":
    case "UpdateDownloadStatus":
    case "UpdateDownloadProgress":
    case "UpdateAppliedAndRestarting":
      return handleSettingsEvent(state, event);

    case "PhoneLookupSuccess":
    case "PhoneLookupFailed":
    case "FriendStatusChecked":
      return handleContactsEvent(state, event);
  }
}

export {
  handleContactsEvent,
  handleLoginEvent,
  handleMediaEvent,
  handleMessagesEvent,
  handleSettingsEvent,
  handleSyncEvent,
};
