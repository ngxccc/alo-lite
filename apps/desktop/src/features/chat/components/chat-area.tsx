import { useEffect, useState } from "react";
import { ipc, isTauriRuntime } from "@/shared/api/ipc";
import type { Thread } from "@/shared/types/ipc.generated";
import { cn } from "@/shared/utils/cn";
import { dayKey, matchesVnQuery } from "@/shared/utils/text";
import type { AppAction, AppState } from "@/app/store/app-reducer";
import { useAppStore } from "@/app/store/use-app-store";
import { useChatStore } from "@/stores/use-chat-store";
import { ChatMessageList } from "@/shared/ui/chat";
import { Composer } from "./composer";
import { ChatHeader } from "./chat-header";
import { PinnedBar } from "./pinned-bar";
import { DateDivider } from "./date-divider";
import { ChatHistoryBanners } from "./chat-history-banners";
import { MessageBubble } from "./message-bubble";
import { useChatScroll } from "./use-chat-scroll";
import { useSenderLookup } from "./use-sender-lookup";

interface ChatAreaProps {
  state?: AppState;
  dispatch?: (action: AppAction) => void;
}

export function ChatArea({ state: propState, dispatch: propDispatch }: ChatAreaProps = {}) {
  const chat = useChatStore();
  const appStore = useAppStore();
  const state = propState ?? appStore;
  const dispatch = propDispatch ?? appStore.dispatch;
  const [reactPickerFor, setReactPickerFor] = useState<string | null>(null);

  const selectedThreadId = propState?.selectedThreadId ?? chat.selectedThreadId;
  const threads = propState?.threads ?? chat.threads;

  const thread: Thread | undefined = threads.find((t) => t.id === selectedThreadId);
  const pinnedMessages = thread?.messages.filter((m) => m.is_pinned) ?? [];

  const { resolveSender, resolveReplySenderName } = useSenderLookup(state);

  let visibleMessages = thread?.messages ?? [];
  if (state.filterStarredOnly) {
    visibleMessages = visibleMessages.filter((m) => Boolean(m.is_starred));
  }
  if (state.inChatSearchOpen && state.inChatSearchQuery.trim()) {
    visibleMessages = visibleMessages.filter((m) =>
      matchesVnQuery(m.text, state.inChatSearchQuery),
    );
  }
  const limit = thread ? (state.renderLimitByThread[thread.id] ?? 150) : 150;
  if (visibleMessages.length > limit) {
    visibleMessages = visibleMessages.slice(-limit);
  }

  useEffect(() => {
    if (!thread || thread.is_group || thread.name.includes("Cloud của tôi")) return;
    if (!state.friendStatusMap[thread.id] && isTauriRuntime()) {
      void ipc.checkFriendStatus(thread.id);
    }
  }, [thread?.id]);

  const { scrollRef, msgRefs, jumpToQuoted } = useChatScroll({
    thread,
    visibleMessagesCount: visibleMessages.length,
    quoteHighlightId: state.quoteHighlightId,
    dispatch,
  });

  if (!thread) {
    return (
      <main className="grid flex-1 place-items-center bg-canvas p-6">
        <div className="max-w-sm text-center text-muted-foreground">
          <div className="mb-1.5 text-lg font-bold text-foreground">Chào mừng đến với AloLite</div>
          <div className="text-sm">
            Chọn một hội thoại bên trái để bắt đầu trò chuyện hoặc tra cứu số điện thoại mới.
          </div>
        </div>
      </main>
    );
  }

  const totalCount = thread.messages.length;
  const renderLimit = state.renderLimitByThread[thread.id] ?? 150;
  const canShowMoreLocal = totalCount > renderLimit;
  const reachedStart = Boolean(state.historyReachedStart[thread.id]);

  let prevDay = "";

  return (
    <main className="flex min-w-0 flex-1 flex-col bg-canvas">
      {/* ── Chat Header ── 68px per spec ── */}
      <ChatHeader thread={thread} state={state} dispatch={dispatch} />

      {/* ── Pinned message bar ── */}
      <PinnedBar
        pinnedMessages={pinnedMessages}
        onJumpToPinned={jumpToQuoted}
        onToggleInfoDrawer={() => dispatch({ type: "TOGGLE_INFO_DRAWER" })}
      />

      {/* ── Message list (shadcn-chat ChatMessageList) ── */}
      <ChatMessageList ref={scrollRef}>
        {/* Load history banners — styled as quiet chips */}
        <ChatHistoryBanners
          thread={thread}
          canShowMoreLocal={canShowMoreLocal}
          reachedStart={reachedStart}
          dispatch={dispatch}
        />

        {visibleMessages.map((msg, idx) => {
          const dk = dayKey(msg.ts);
          const showDivider = Boolean(dk && dk !== prevDay);
          if (dk) prevDay = dk;

          const prevMsg = !showDivider && idx > 0 ? visibleMessages.at(idx - 1) : undefined;
          const isGroupedWithPrev =
            prevMsg?.is_outgoing === msg.is_outgoing && prevMsg.sender === msg.sender;

          const msgKey = msg.id || `${msg.ts}_${idx}`;
          const highlighted = state.quoteHighlightId === msgKey;
          const senderInfo = !msg.is_outgoing ? resolveSender(msg, thread) : null;

          return (
            <div
              key={msgKey}
              className={cn("flex flex-col", isGroupedWithPrev ? "mt-0.5" : "mt-2")}
            >
              {/* Date divider chip */}
              {showDivider ? <DateDivider ts={msg.ts} /> : null}

              {/* Message row */}
              <MessageBubble
                msg={msg}
                msgKey={msgKey}
                thread={thread}
                isGroupedWithPrev={isGroupedWithPrev}
                highlighted={highlighted}
                senderInfo={senderInfo}
                state={state}
                dispatch={dispatch}
                reactPickerFor={reactPickerFor}
                setReactPickerFor={setReactPickerFor}
                jumpToQuoted={jumpToQuoted}
                resolveReplySenderName={resolveReplySenderName}
                setRef={(el) => {
                  msgRefs.current[msgKey] = el;
                }}
              />
            </div>
          );
        })}
      </ChatMessageList>

      <Composer state={state} dispatch={dispatch} thread={thread} />
    </main>
  );
}
