import {
  mergeThreadInto,
  reconcileSentMessage,
  sortThreads,
} from "@/features/threads/utils/threads";
import type { ChatMessage, CoreEventDto } from "@/shared/types/ipc.generated";
import { formatClockNow } from "@/shared/utils/text";
import type { AppState } from "../app-state";
import { nextToast, updateThreadMessage } from "../helpers";

export type MessagesCoreEvent = Extract<
  CoreEventDto,
  | { type: "ThreadsUpdated" }
  | { type: "MessageSent" }
  | { type: "StickerSent" }
  | { type: "AttachmentSent" }
  | { type: "QuoteSent" }
  | { type: "Reacted" }
  | { type: "ReactFailed" }
  | { type: "MessageDeleted" }
  | { type: "MessageRecalled" }
  | { type: "SendFailed" }
  | { type: "HistoryReachedStart" }
>;

export function handleMessagesEvent(state: AppState, event: MessagesCoreEvent): AppState {
  switch (event.type) {
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
            const newIncoming = merged.messages.slice(-added).filter((m) => !m.is_outgoing).length;
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

    case "Reacted": {
      const { thread_id, msg_id, ts, reaction } = event.payload;
      return {
        ...state,
        threads: updateThreadMessage(state.threads, thread_id, (t) => ({
          ...t,
          messages: t.messages.map((m) => {
            const matches = (msg_id && m.id === msg_id) || (!msg_id && m.ts === ts);
            if (!matches) return m;
            const current = m.reactions ?? [];
            const nextReactions = current.includes(reaction) ? current : [...current, reaction];
            return { ...m, reactions: nextReactions };
          }),
        })),
        statusMessage: "Đã thả cảm xúc",
      };
    }
    case "ReactFailed": {
      const { thread_id, msg_id, ts, reaction } = event.payload;
      return {
        ...state,
        threads: updateThreadMessage(state.threads, thread_id, (t) => ({
          ...t,
          messages: t.messages.map((m) => {
            const matches = (msg_id && m.id === msg_id) || (!msg_id && m.ts === ts);
            if (!matches) return m;
            const current = m.reactions ?? [];
            return { ...m, reactions: current.filter((r) => r !== reaction) };
          }),
        })),
        statusMessage: "Thả cảm xúc thất bại",
      };
    }

    case "MessageDeleted":
      return {
        ...state,
        threads: updateThreadMessage(state.threads, event.payload.thread_id, (t) => ({
          ...t,
          messages: t.messages.filter(
            (m) =>
              !(
                (event.payload.msg_id && m.id === event.payload.msg_id) ||
                (!event.payload.msg_id && m.ts === event.payload.ts)
              ),
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

    case "HistoryReachedStart":
      return {
        ...state,
        historyReachedStart: {
          ...state.historyReachedStart,
          [event.payload.thread_id]: true,
        },
        toast: nextToast("Đã hiển thị toàn bộ lịch sử hội thoại này"),
      };
  }
}
