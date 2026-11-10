import type { CoreEventDto } from "@/shared/types/ipc";
import type { AppState } from "../app-state";
import { nextToast, updateThreadMessage } from "../helpers";

export type SettingsCoreEvent = Extract<
  CoreEventDto,
  | { type: "MuteSnapshotSynced" }
  | { type: "MuteEventsReceived" }
  | { type: "ProxyUpdated" }
  | { type: "NotificationAuthorization" }
  | { type: "UpdateCheckFinished" }
  | { type: "UpdateDownloadStatus" }
  | { type: "UpdateDownloadProgress" }
  | { type: "UpdateAppliedAndRestarting" }
>;

export function handleSettingsEvent(state: AppState, event: SettingsCoreEvent): AppState {
  switch (event.type) {
    case "MuteSnapshotSynced": {
      if (!state.settings) return state;
      const muted_threads = {
        ...state.settings.muted_threads,
        ...event.payload.muted_threads,
      };
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

    case "NotificationAuthorization":
      return {
        ...state,
        statusMessage: event.payload.status_label ?? event.payload.error ?? state.statusMessage,
      };

    case "UpdateCheckFinished":
      return {
        ...state,
        updateChecking: false,
        updateInfo: event.payload.ok ? event.payload.info : state.updateInfo,
        updateError: event.payload.ok
          ? null
          : (event.payload.error ?? "Không kiểm tra được cập nhật"),
        toast:
          !event.payload.silent &&
          event.payload.ok &&
          event.payload.info &&
          !event.payload.info.has_update
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
        updateProgress: {
          downloaded: event.payload.downloaded,
          total: event.payload.total,
        },
      };

    case "UpdateAppliedAndRestarting":
      return {
        ...state,
        updateDownloading: false,
        updateDownloadMessage: `Đã cập nhật lên v${event.payload.version} — đang khởi động lại…`,
        toast: nextToast(`Đã cập nhật v${event.payload.version}`),
      };
  }
}
