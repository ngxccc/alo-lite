import { useEffect, useRef } from "react";
import type { ChatMessage, Thread } from "@/shared/types/ipc";
import type { AppAction } from "@/app/store/app-reducer";
import { uniqueReferenceIndex } from "@/features/chat/utils/messages";

interface UseChatScrollOptions {
  thread?: Thread;
  visibleMessagesCount: number;
  quoteHighlightId: string | null;
  dispatch: (action: AppAction) => void;
}

export function useChatScroll({
  thread,
  visibleMessagesCount,
  quoteHighlightId,
  dispatch,
}: UseChatScrollOptions) {
  const scrollRef = useRef<HTMLDivElement | null>(null);
  const msgRefs = useRef<Record<string, HTMLDivElement | null>>({});

  useEffect(() => {
    if (scrollRef.current && !quoteHighlightId) {
      scrollRef.current.scrollTop = scrollRef.current.scrollHeight;
    }
  }, [thread?.id, visibleMessagesCount, quoteHighlightId]);

  const jumpToQuoted = (msg: ChatMessage) => {
    if (!thread) return;
    const ref = msg.reply_to_id ?? msg.reply_to_text ?? "";
    const idx = uniqueReferenceIndex(thread.messages, ref);
    if (idx === null) {
      dispatch({
        type: "SHOW_TOAST",
        message: "Không tìm thấy tin nhắn gốc duy nhất trong lịch sử đang tải",
      });
      return;
    }
    const target = thread.messages[idx];
    if (!target) return;
    const targetKey = target.id || String(target.ts);
    dispatch({ type: "SET_QUOTE_HIGHLIGHT", msgId: targetKey });
    requestAnimationFrame(() => {
      msgRefs.current[targetKey]?.scrollIntoView({
        behavior: "smooth",
        block: "center",
      });
    });
    window.setTimeout(() => dispatch({ type: "SET_QUOTE_HIGHLIGHT", msgId: null }), 2200);
  };

  return {
    scrollRef,
    msgRefs,
    jumpToQuoted,
  };
}
