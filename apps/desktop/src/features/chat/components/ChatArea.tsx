import { useEffect, useMemo, useRef, useState } from "react";
import {
  ChevronUp,
  CornerUpLeft,
  Download,
  ExternalLink,
  FileText,
  Forward,
  Info,
  Phone,
  RotateCcw,
  Search,
  SmilePlus,
  Star,
  Trash2,
  Video,
  X,
} from "lucide-react";
import { ipc, isTauriRuntime } from "../../../shared/api/ipc";
import { REACTION_LIST, type ChatMessage, type Thread } from "../../../shared/types/ipc";
import { Avatar } from "../../../shared/ui/Avatar";
import { Button } from "../../../shared/ui/button";
import { Popover, PopoverContent, PopoverTrigger } from "../../../shared/ui/popover";
import { cn } from "../../../shared/utils/cn";
import { dayKey, formatDayDivider, jxlToJpgUrl, matchesVnQuery } from "../../../shared/utils/text";
import type { AppAction, AppState } from "../../../app/store/appReducer";
import {
  isFileMessage,
  isPhotoMessage,
  isVideoMessage,
  parseCallCard,
  stickerThumbnailUrl,
  uniqueReferenceIndex,
} from "../utils/messages";
import { Composer } from "./Composer";

interface ChatAreaProps {
  state: AppState;
  dispatch: (action: AppAction) => void;
}

function StickerMessageMedia({ msg }: { msg: ChatMessage }) {
  const sid = msg.sticker_id ?? 0;
  const cid = msg.sticker_cate_id ?? 0;
  if (!sid) return null;
  return (
    <img
      src={stickerThumbnailUrl({ sticker_id: sid, cate_id: cid })}
      alt={msg.text}
      loading="lazy"
      decoding="async"
      className="h-28 w-28 object-contain"
    />
  );
}

function PhotoMessageMedia({
  msg,
  onOpenViewer,
}: {
  msg: ChatMessage;
  onOpenViewer: (texKey: string, fullUrl: string) => void;
}) {
  const thumbUrl = jxlToJpgUrl(msg.link_thumb || msg.link_url || "");
  const fullUrl = msg.link_url || thumbUrl;
  const key = `img_${msg.id || msg.ts}`;

  return (
    <div className="flex flex-col gap-1">
      {thumbUrl ? (
        <img
          src={thumbUrl}
          alt={msg.link_title || "Hình ảnh"}
          loading="lazy"
          decoding="async"
          onClick={() => onOpenViewer(key, fullUrl)}
          className="max-h-[260px] max-w-[260px] cursor-zoom-in rounded-lg object-cover"
        />
      ) : (
        <Button variant="outline" size="sm" onClick={() => onOpenViewer(key, fullUrl)}>
          Xem hình ảnh
        </Button>
      )}
      {msg.text && !msg.text.startsWith("[Hình ảnh]") ? <span>{msg.text}</span> : null}
    </div>
  );
}

export function ChatArea({ state, dispatch }: ChatAreaProps) {
  const scrollRef = useRef<HTMLDivElement | null>(null);
  const msgRefs = useRef<Record<string, HTMLDivElement | null>>({});
  const [reactPickerFor, setReactPickerFor] = useState<string | null>(null);

  const thread: Thread | undefined = useMemo(
    () => state.threads.find((t) => t.id === state.selectedThreadId),
    [state.threads, state.selectedThreadId],
  );

  const visibleMessages = useMemo(() => {
    if (!thread) return [];
    let list = thread.messages;
    if (state.filterStarredOnly) {
      list = list.filter((m) => Boolean(m.is_starred));
    }
    if (state.inChatSearchOpen && state.inChatSearchQuery.trim()) {
      list = list.filter((m) => matchesVnQuery(m.text, state.inChatSearchQuery));
    }
    const limit = state.renderLimitByThread[thread.id] ?? 150;
    return list.length > limit ? list.slice(-limit) : list;
  }, [
    thread,
    state.filterStarredOnly,
    state.inChatSearchOpen,
    state.inChatSearchQuery,
    state.renderLimitByThread,
  ]);

  useEffect(() => {
    if (!thread || thread.is_group || thread.name.includes("Cloud của tôi")) return;
    if (!state.friendStatusMap[thread.id] && isTauriRuntime()) {
      void ipc.checkFriendStatus(thread.id);
    }
  }, [thread?.id]);

  useEffect(() => {
    if (scrollRef.current && !state.quoteHighlightId) {
      scrollRef.current.scrollTop = scrollRef.current.scrollHeight;
    }
  }, [thread?.id, visibleMessages.length]);

  if (!thread) {
    return (
      <main className="grid flex-1 place-items-center bg-canvas p-6">
        <div className="max-w-xs text-center text-muted-foreground">
          <div className="mb-1.5 text-base font-bold text-foreground">
            Chào mừng đến với AloLite
          </div>
          <div className="text-xs">
            Chọn một hội thoại bên trái để bắt đầu trò chuyện hoặc tra cứu số điện thoại mới.
          </div>
        </div>
      </main>
    );
  }

  const friendStatus = state.friendStatusMap[thread.id];
  const onlineInfo = state.userOnlineMap[thread.id];
  const totalCount = thread.messages.length;
  const renderLimit = state.renderLimitByThread[thread.id] ?? 150;
  const canShowMoreLocal = totalCount > renderLimit;
  const reachedStart = Boolean(state.historyReachedStart[thread.id]);

  const jumpToQuoted = (msg: ChatMessage) => {
    const ref = msg.reply_to_id || msg.reply_to_text || "";
    const idx = uniqueReferenceIndex(thread.messages, ref);
    if (idx === null) {
      dispatch({
        type: "SHOW_TOAST",
        message: "Không tìm thấy tin nhắn gốc duy nhất trong lịch sử đang tải",
      });
      return;
    }
    const target = thread.messages[idx];
    const targetKey = target.id || String(target.ts);
    dispatch({ type: "SET_QUOTE_HIGHLIGHT", msgId: targetKey });
    requestAnimationFrame(() => {
      msgRefs.current[targetKey]?.scrollIntoView({ behavior: "smooth", block: "center" });
    });
    window.setTimeout(() => dispatch({ type: "SET_QUOTE_HIGHLIGHT", msgId: null }), 2200);
  };

  const renderReactionBadge = (emoji: string) => {
    const found = REACTION_LIST.find((r) => r.emoji === emoji);
    const asset = found ? state.reactionAssets[found.key] : "";
    if (asset) {
      return <img src={asset} alt={emoji} className="h-4 w-4 object-contain" />;
    }
    return <span>{emoji}</span>;
  };

  let prevDay = "";

  return (
    <main className="flex min-w-0 flex-1 flex-col bg-canvas">
      <header className="flex h-14 shrink-0 items-center justify-between border-b border-border bg-sidebar px-4">
        <div className="flex min-w-0 items-center gap-2.5">
          <Avatar
            id={thread.id}
            name={thread.name}
            avatarUrl={thread.avatar}
            isGroup={thread.is_group}
            size={38}
            mediaCache={state.mediaCache}
          />
          <div className="min-w-0">
            <div className="truncate text-sm font-bold text-foreground">
              {thread.name || thread.id}
            </div>
            <div
              className={cn(
                "truncate text-[11.5px]",
                onlineInfo?.[0] ? "font-medium text-online" : "text-muted-foreground",
              )}
            >
              {thread.is_group
                ? thread.presentation.member_count
                  ? `${thread.presentation.member_count} thành viên`
                  : "Nhóm trò chuyện"
                : thread.name.includes("Cloud của tôi")
                  ? "Lưu trữ đám mây cá nhân"
                  : onlineInfo?.[1] || (friendStatus === "Stranger" ? "Người lạ" : "Trực tuyến")}
            </div>
          </div>
        </div>

        <div className="flex items-center gap-1">
          {state.inChatSearchOpen ? (
            <div className="flex h-8 w-56 items-center gap-1.5 rounded-md border border-border bg-muted/60 px-2.5 focus-within:border-ring focus-within:bg-background">
              <Search className="h-3.5 w-3.5 shrink-0 text-muted-foreground" />
              <input
                autoFocus
                value={state.inChatSearchQuery}
                onChange={(e) =>
                  dispatch({ type: "SET_IN_CHAT_SEARCH_QUERY", query: e.target.value })
                }
                placeholder="Tìm trong hội thoại…"
                className="w-full bg-transparent text-xs text-foreground placeholder:text-muted-foreground focus:outline-none"
              />
              <button
                type="button"
                onClick={() => dispatch({ type: "TOGGLE_IN_CHAT_SEARCH", open: false })}
                className="inline-flex h-4 w-4 items-center justify-center rounded text-muted-foreground hover:text-foreground cursor-pointer"
              >
                <X className="h-3 w-3" />
              </button>
            </div>
          ) : (
            <Button
              variant="ghost"
              size="icon"
              title="Tìm tin nhắn trong hội thoại"
              onClick={() => dispatch({ type: "TOGGLE_IN_CHAT_SEARCH", open: true })}
            >
              <Search className="h-4 w-4" />
            </Button>
          )}

          <Button
            variant={state.filterStarredOnly ? "accent" : "ghost"}
            size="icon"
            title="Lọc tin nhắn đã gắn sao"
            onClick={() => dispatch({ type: "TOGGLE_STARRED_FILTER" })}
          >
            <Star className={cn("h-4 w-4", state.filterStarredOnly && "text-warning")} />
          </Button>

          <Button
            variant={state.infoDrawerOpen ? "accent" : "ghost"}
            size="icon"
            title="Thông tin hội thoại"
            onClick={() => dispatch({ type: "TOGGLE_INFO_DRAWER" })}
          >
            <Info className="h-4 w-4" />
          </Button>
        </div>
      </header>

      <div ref={scrollRef} className="flex flex-1 flex-col gap-1.5 overflow-y-auto px-4 py-3">
        {(canShowMoreLocal || !reachedStart) && (
          <div className="mb-1.5 flex justify-center gap-2">
            {canShowMoreLocal ? (
              <Button
                variant="outline"
                size="sm"
                onClick={() =>
                  dispatch({ type: "EXPAND_RENDER_LIMIT", threadId: thread.id, step: 150 })
                }
              >
                <ChevronUp className="h-3.5 w-3.5" /> Xem thêm tin cũ trong máy
              </Button>
            ) : null}
            {!reachedStart && (thread.is_group || thread.name.includes("Cloud của tôi")) ? (
              <Button
                variant="outline"
                size="sm"
                onClick={() =>
                  void ipc.loadGroupHistory(thread.id, thread.is_group, thread.messages.length)
                }
              >
                <ChevronUp className="h-3.5 w-3.5" /> Tải lịch sử cũ từ máy chủ
              </Button>
            ) : null}
          </div>
        )}

        {visibleMessages.map((msg, idx) => {
          const dk = dayKey(msg.ts);
          const showDivider = Boolean(dk && dk !== prevDay);
          if (dk) prevDay = dk;

          const prevMsg = !showDivider && idx > 0 ? visibleMessages[idx - 1] : undefined;
          const isGroupedWithPrev =
            Boolean(prevMsg) &&
            prevMsg?.is_outgoing === msg.is_outgoing &&
            prevMsg?.sender === msg.sender;

          const msgKey = msg.id || `${msg.ts}_${idx}`;
          const isRecalled = msg.text.trim() === "[Tin nhắn đã thu hồi]";
          const callCard = parseCallCard(msg);
          const highlighted = state.quoteHighlightId === msgKey;

          return (
            <div key={msgKey} className="flex flex-col">
              {showDivider ? (
                <div className="my-2.5 self-center rounded-full border border-border bg-card/85 px-3 py-0.5 text-[11px] font-medium text-muted-foreground shadow-2xs">
                  {formatDayDivider(msg.ts)}
                </div>
              ) : null}

              <div
                ref={(el) => {
                  msgRefs.current[msgKey] = el;
                }}
                onDoubleClick={() => {
                  if (state.settings?.double_click_to_reply && !isRecalled) {
                    dispatch({ type: "SET_REPLYING_TO", msg });
                  }
                }}
                className={cn(
                  "group relative flex max-w-[74%] flex-col gap-0.5 rounded-xl transition-colors",
                  msg.is_outgoing ? "self-end items-end" : "self-start items-start",
                  highlighted && "ring-2 ring-primary/60 bg-primary/10 p-1",
                )}
              >
                {!msg.is_outgoing && thread.is_group && !isGroupedWithPrev ? (
                  <div className="px-1 text-[11px] font-semibold text-muted-foreground">
                    {msg.sender || "Thành viên"}
                  </div>
                ) : null}

                <div className="relative flex items-center gap-1.5">
                  <div
                    className={cn(
                      "relative rounded-xl border px-3 py-2 text-[13.5px] leading-relaxed break-words whitespace-pre-wrap shadow-2xs",
                      msg.is_outgoing
                        ? "rounded-br-[4px] border-bubble-out-border bg-bubble-out text-bubble-out-foreground"
                        : "rounded-bl-[4px] border-bubble-in-border bg-bubble-in text-bubble-in-foreground",
                      isRecalled && "italic text-muted-foreground opacity-75",
                    )}
                  >
                    {msg.reply_to_text ? (
                      <div
                        onClick={() => jumpToQuoted(msg)}
                        className="mb-1.5 cursor-pointer rounded-md border-l-3 border-primary bg-background/50 px-2.5 py-1 text-xs hover:bg-background/80"
                      >
                        <div className="text-[11px] font-bold text-primary">
                          {msg.reply_to_sender || "Trích dẫn"}
                        </div>
                        <div className="max-w-[280px] truncate text-muted-foreground">
                          {msg.reply_to_text}
                        </div>
                      </div>
                    ) : null}

                    {isRecalled ? (
                      <span>Tin nhắn đã được thu hồi</span>
                    ) : msg.sticker_id ? (
                      <StickerMessageMedia msg={msg} />
                    ) : callCard ? (
                      <div className="flex min-w-[190px] items-center gap-2.5">
                        <div
                          className={cn(
                            "grid h-8 w-8 shrink-0 place-items-center rounded-full",
                            callCard.isMissedOrFailed
                              ? "bg-destructive/15 text-destructive"
                              : "bg-online/15 text-online",
                          )}
                        >
                          {callCard.isVideo ? (
                            <Video className="h-4 w-4" />
                          ) : (
                            <Phone className="h-4 w-4" />
                          )}
                        </div>
                        <div>
                          <div className="text-xs font-bold">{callCard.title}</div>
                          <div className="text-[11px] text-muted-foreground">
                            {callCard.subtitle}
                          </div>
                        </div>
                      </div>
                    ) : isPhotoMessage(msg) && (msg.link_thumb || msg.link_url) ? (
                      <PhotoMessageMedia
                        msg={msg}
                        onOpenViewer={(texKey, fullUrl) =>
                          dispatch({
                            type: "SET_IMAGE_VIEWER",
                            viewer: {
                              texKey,
                              fullUrl,
                              title: msg.link_title || "Hình ảnh",
                              sender: msg.sender,
                              time: msg.time,
                            },
                          })
                        }
                      />
                    ) : isFileMessage(msg) && msg.link_url ? (
                      <div className="flex min-w-[210px] items-center gap-2.5">
                        <FileText className="h-6 w-6 shrink-0 text-primary" />
                        <div className="min-w-0 flex-1">
                          <div className="truncate font-semibold">
                            {msg.link_title || msg.text}
                          </div>
                          {msg.link_desc ? (
                            <div className="text-[11px] text-muted-foreground">{msg.link_desc}</div>
                          ) : null}
                        </div>
                        <Button
                          variant="ghost"
                          size="icon-sm"
                          title="Tải xuống"
                          onClick={() =>
                            void ipc.downloadFile(
                              msg.link_url!,
                              msg.link_title || "attachment",
                              state.settings?.download_dir ?? "",
                            )
                          }
                        >
                          <Download className="h-3.5 w-3.5" />
                        </Button>
                      </div>
                    ) : isVideoMessage(msg) && msg.link_url ? (
                      <div className="flex items-center gap-2">
                        <Video className="h-4 w-4 text-primary" />
                        <span>{msg.link_title || "Video đính kèm"}</span>
                        <Button
                          variant="outline"
                          size="sm"
                          onClick={() =>
                            void ipc.downloadFile(
                              msg.link_url!,
                              msg.link_title || "video.mp4",
                              state.settings?.download_dir ?? "",
                            )
                          }
                        >
                          Tải về
                        </Button>
                      </div>
                    ) : (
                      <div>
                        <div className="select-text">{msg.text}</div>
                        {msg.link_url ? (
                          <div
                            onClick={() => void ipc.openExternalUrl(msg.link_url!)}
                            className="mt-1.5 max-w-[320px] cursor-pointer rounded-lg border border-border bg-background/60 p-2 hover:bg-background/90"
                          >
                            <div className="flex items-center gap-1.5 text-xs font-bold text-foreground">
                              <ExternalLink className="h-3.5 w-3.5 shrink-0 text-primary" />
                              <span className="truncate">{msg.link_title || msg.link_url}</span>
                            </div>
                            {msg.link_desc ? (
                              <div className="mt-0.5 line-clamp-2 text-[11px] text-muted-foreground">
                                {msg.link_desc}
                              </div>
                            ) : null}
                            {msg.link_src ? (
                              <div className="mt-0.5 truncate text-[10.5px] text-muted-foreground/80">
                                {msg.link_src}
                              </div>
                            ) : null}
                          </div>
                        ) : null}
                      </div>
                    )}
                  </div>

                  {!isRecalled ? (
                    <div
                      className={cn(
                        "absolute -top-3.5 z-10 hidden items-center gap-0.5 rounded-lg border border-border bg-popover p-0.5 shadow-md group-hover:flex",
                        msg.is_outgoing ? "right-2" : "left-2",
                      )}
                    >
                      <Button
                        variant="ghost"
                        size="icon-sm"
                        className="h-6 w-6"
                        title="Trả lời"
                        onClick={() => dispatch({ type: "SET_REPLYING_TO", msg })}
                      >
                        <CornerUpLeft className="h-3 w-3" />
                      </Button>

                      <Popover
                        open={reactPickerFor === msgKey}
                        onOpenChange={(open) => setReactPickerFor(open ? msgKey : null)}
                      >
                        <PopoverTrigger asChild>
                          <Button
                            variant="ghost"
                            size="icon-sm"
                            className="h-6 w-6"
                            title="Thả cảm xúc"
                          >
                            <SmilePlus className="h-3 w-3" />
                          </Button>
                        </PopoverTrigger>
                        <PopoverContent side="top" className="flex items-center gap-1 p-1.5">
                          {REACTION_LIST.map((r) => (
                            <Button
                              key={r.key}
                              variant="ghost"
                              size="icon-sm"
                              title={r.label}
                              onClick={() => {
                                void ipc.reactMessage(thread.id, thread.is_group, msg, r.emoji);
                                setReactPickerFor(null);
                              }}
                            >
                              {renderReactionBadge(r.emoji)}
                            </Button>
                          ))}
                        </PopoverContent>
                      </Popover>

                      <Button
                        variant="ghost"
                        size="icon-sm"
                        className="h-6 w-6"
                        title="Chuyển tiếp"
                        onClick={() => dispatch({ type: "SET_FORWARD_MSG", msg })}
                      >
                        <Forward className="h-3 w-3" />
                      </Button>

                      <Button
                        variant="ghost"
                        size="icon-sm"
                        className="h-6 w-6"
                        title="Gắn sao"
                        onClick={() =>
                          dispatch({ type: "TOGGLE_LOCAL_STAR", threadId: thread.id, msgId: msg.id })
                        }
                      >
                        <Star className={cn("h-3 w-3", msg.is_starred && "text-warning")} />
                      </Button>

                      {msg.is_outgoing ? (
                        <Button
                          variant="ghost"
                          size="icon-sm"
                          className="h-6 w-6"
                          title="Thu hồi tin nhắn"
                          onClick={() => void ipc.recallMessage(thread.id, thread.is_group, msg)}
                        >
                          <RotateCcw className="h-3 w-3" />
                        </Button>
                      ) : null}

                      <Button
                        variant="ghost"
                        size="icon-sm"
                        className="h-6 w-6 text-destructive hover:text-destructive"
                        title="Xóa chỉ ở phía tôi"
                        onClick={() => void ipc.deleteForMe(thread.id, thread.is_group, msg)}
                      >
                        <Trash2 className="h-3 w-3" />
                      </Button>
                    </div>
                  ) : null}
                </div>

                <div className="flex items-center gap-1.5 px-1 text-[10.5px] text-muted-foreground">
                  <span>{msg.time}</span>
                  {msg.is_starred ? <Star className="h-2.5 w-2.5 text-warning" /> : null}
                  {msg.reactions && msg.reactions.length > 0 ? (
                    <span className="inline-flex items-center gap-1 rounded-full border border-border bg-card px-1.5 py-0.5 text-[11px]">
                      {msg.reactions.slice(0, 4).map((em, i) => (
                        <span key={`${em}_${i}`}>{renderReactionBadge(em)}</span>
                      ))}
                    </span>
                  ) : null}
                </div>
              </div>
            </div>
          );
        })}
      </div>

      <Composer state={state} dispatch={dispatch} thread={thread} />
    </main>
  );
}
