import { sortThreads } from "@/features/threads/utils/threads";
import type { CoreEventDto } from "@/shared/types/ipc";
import type { AppState } from "../app-state";
import { nextToast } from "../helpers";

export type SyncCoreEvent = Extract<
  CoreEventDto,
  | { type: "LocalDataWiped" }
  | { type: "StatusUpdate" }
  | { type: "RefreshFinished" }
  | { type: "SyncFinished" }
  | { type: "PhoneSyncProgress" }
  | { type: "PhoneSyncFinished" }
>;

export function handleSyncEvent(state: AppState, event: SyncCoreEvent): AppState {
  switch (event.type) {
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
  }
}
