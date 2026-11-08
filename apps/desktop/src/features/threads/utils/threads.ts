import type {
  AppSettings,
  ChatMessage,
  Thread,
} from "../../../shared/types/ipc";
import { normalizeTsMs } from "../../../shared/utils/text";

export function isThreadMuted(
  thread: Thread,
  settings: AppSettings,
  nowUnix = Math.floor(Date.now() / 1000),
): boolean {
  const until = settings.muted_threads[thread.id];
  if (typeof until === "number" && (until === -1 || until > nowUnix))
    return true;
  if (thread.unread.muted) {
    return (
      thread.unread.muted_until == null || thread.unread.muted_until > nowUnix
    );
  }
  return thread.presentation.muted;
}

export function threadUnreadCount(
  thread: Thread,
  localUnread: Record<string, number | undefined>,
): number {
  const serverCount = thread.unread.unread_ids.length;
  const local = localUnread[thread.id] ?? 0;
  return Math.max(serverCount, local);
}

export function threadLastActivityTs(thread: Thread): number {
  const lastMsg = thread.messages.at(-1);
  return Math.max(
    normalizeTsMs(thread.presentation.last_message_ts || 0),
    normalizeTsMs(lastMsg?.ts ?? 0),
  );
}

export function sortThreads(
  threads: Thread[],
  settings: AppSettings,
): Thread[] {
  const pinnedSet = new Set(settings.pinned_threads);
  return [...threads].sort((a, b) => {
    const aPinned = a.presentation.pinned || pinnedSet.has(a.id) ? 1 : 0;
    const bPinned = b.presentation.pinned || pinnedSet.has(b.id) ? 1 : 0;
    if (aPinned !== bPinned) return bPinned - aPinned;
    return threadLastActivityTs(b) - threadLastActivityTs(a);
  });
}

export function reconcileSentMessage(thread: Thread, msg: ChatMessage): Thread {
  const existingIdx = thread.messages.findIndex(
    (m) =>
      (msg.client_message_id &&
        m.client_message_id === msg.client_message_id) ||
      (msg.id && m.id === msg.id),
  );
  const messages = [...thread.messages];
  if (existingIdx >= 0) {
    messages[existingIdx] = { ...messages[existingIdx], ...msg };
  } else {
    messages.push(msg);
  }
  messages.sort((a, b) => normalizeTsMs(a.ts) - normalizeTsMs(b.ts));
  return {
    ...thread,
    preview: msg.text || thread.preview,
    time: msg.time || thread.time,
    messages,
    presentation: {
      ...thread.presentation,
      last_message_ts: Math.max(thread.presentation.last_message_ts, msg.ts),
      last_message_id: msg.id || thread.presentation.last_message_id,
      last_sender: msg.sender,
      last_outgoing: msg.is_outgoing,
    },
  };
}

export function mergeThreadInto(existing: Thread, incoming: Thread): Thread {
  const seen = new Map<string, ChatMessage>();
  for (const m of existing.messages) {
    const key = m.id || m.client_message_id || `${m.ts}:${m.sender}:${m.text}`;
    seen.set(key, m);
  }
  for (const m of incoming.messages) {
    const key = m.id || m.client_message_id || `${m.ts}:${m.sender}:${m.text}`;
    const prev = seen.get(key);
    seen.set(key, prev ? { ...prev, ...m } : m);
  }
  const merged = Array.from(seen.values()).sort(
    (a, b) => normalizeTsMs(a.ts) - normalizeTsMs(b.ts),
  );
  return {
    ...existing,
    name: incoming.name || existing.name,
    avatar: incoming.avatar ?? existing.avatar,
    is_group: existing.is_group || incoming.is_group,
    preview: incoming.preview || existing.preview,
    time: incoming.time || existing.time,
    messages: merged,
    unread: incoming.unread,
    presentation: {
      ...existing.presentation,
      ...incoming.presentation,
      pinned: existing.presentation.pinned || incoming.presentation.pinned,
      muted: existing.presentation.muted || incoming.presentation.muted,
      hidden: existing.presentation.hidden || incoming.presentation.hidden,
    },
  };
}
