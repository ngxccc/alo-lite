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
  isGifUrl,
  isPhotoMessage,
  isVideoMessage,
  parseCallCard,
  shouldHideMediaCaption,
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
      className="h-30 w-30 object-contain"
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
  const rawUrl = msg.link_url ?? (isGifUrl(msg.text) ? msg.text.trim() : "");
  const isGif = isGifUrl(rawUrl) || isGifUrl(msg.link_title);
  const initialSrc = isGif
    ? rawUrl || jxlToJpgUrl(msg.link_thumb ?? "")
    : jxlToJpgUrl(msg.link_thumb ?? rawUrl);
  const fullUrl = rawUrl || initialSrc;
  const [src, setSrc] = useState(initialSrc);
  const key = `img_${msg.id || msg.ts}`;

  useEffect(() => {
    setSrc(initialSrc);
  }, [initialSrc]);

  return (
    <div className="flex flex-col gap-1.5">
      {src ? (
        <img
          src={src}
          alt={msg.link_title ?? "Hình ảnh"}
          loading="lazy"
          decoding="async"
          onError={() => {
            const fallback = jxlToJpgUrl(rawUrl);
            if (fallback && fallback !== src) {
              setSrc(fallback);
            }
          }}
          onClick={() => onOpenViewer(key, fullUrl)}
          className="max-h-[280px] max-w-[300px] cursor-zoom-in rounded-lg object-contain"
        />
      ) : (
        <Button variant="outline" size="sm" onClick={() => onOpenViewer(key, fullUrl)}>
          Xem hình ảnh
        </Button>
      )}
      {!shouldHideMediaCaption(msg) ? <span className="select-text">{msg.text}</span> : null}
    </div>
  );
}

function VideoMessageMedia({ msg, downloadDir }: { msg: ChatMessage; downloadDir: string }) {
  const videoUrl = msg.link_url ?? "";
  const posterUrl = msg.link_thumb ? jxlToJpgUrl(msg.link_thumb) : undefined;
  const fileName =
    msg.link_title && !msg.link_title.startsWith("[")
      ? msg.link_title.endsWith(".mp4")
        ? msg.link_title
        : `${msg.link_title}.mp4`
      : "video.mp4";

  return (
    <div className="flex flex-col gap-1.5">
      {videoUrl ? (
        <video
          src={videoUrl}
          poster={posterUrl}
          controls
          preload="metadata"
          className="max-h-[300px] max-w-[340px] rounded-lg bg-black/90 object-contain"
        />
      ) : null}
      <div className="flex items-center justify-between gap-2">
        <span className="flex items-center gap-1.5 text-xs text-muted-foreground">
          <Video className="h-3.5 w-3.5 text-primary" />
          {!shouldHideMediaCaption(msg) ? msg.text : "Video"}
        </span>
        {videoUrl ? (
          <Button
            variant="outline"
            size="sm"
            onClick={() => void ipc.downloadFile(videoUrl, fileName, downloadDir)}
          >
            <Download className="h-3.5 w-3.5" /> Tải về
          </Button>
        ) : null}
      </div>
    </div>
  );
}

function isNumericUid(val: string): boolean {
  const trimmed = val.trim();
  return trimmed.length >= 6 && /^\d+$/.test(trimmed);
}

export function ChatArea({ state, dispatch }: ChatAreaProps) {
  const scrollRef = useRef<HTMLDivElement | null>(null);
  const msgRefs = useRef<Record<string, HTMLDivElement | null>>({});
  const [reactPickerFor, setReactPickerFor] = useState<string | null>(null);

  const thread: Thread | undefined = useMemo(
    () => state.threads.find((t) => t.id === state.selectedThreadId),
    [state.threads, state.selectedThreadId],
  );

  const senderLookup = useMemo(() => {
    const byId = new Map<string, { id: string; name: string; avatar?: string | null }>();
    const byName = new Map<string, { id: string; name: string; avatar?: string | null }>();

    if (state.uin) {
      const me = {
        id: state.uin,
        name: state.displayName || "Tôi",
        avatar: state.avatar,
      };
      byId.set(state.uin, me);
      if (state.displayName) byName.set(state.displayName, me);
    }

    for (const t of state.threads) {
      if (!t.is_group) {
        const entry = { id: t.id, name: t.name, avatar: t.avatar };
        byId.set(t.id, entry);
        if (t.name) byName.set(t.name, entry);
      }
      for (const mem of t.presentation.members) {
        const entry = { id: mem.id, name: mem.name, avatar: mem.avatar };
        byId.set(mem.id, entry);
        if (mem.name && !isNumericUid(mem.name)) {
          byName.set(mem.name, entry);
        }
      }
    }

    return { byId, byName };
  }, [state.threads, state.uin, state.displayName, state.avatar]);

  const resolveSender = (msg: ChatMessage, currentThread: Thread) => {
    if (!currentThread.is_group) {
      return {
        id: currentThread.id,
        name: currentThread.name || msg.sender,
        avatar: currentThread.avatar,
      };
    }
    const rawId = msg.sender_id ?? (isNumericUid(msg.sender) ? msg.sender.trim() : "");
    if (rawId) {
      const foundById = senderLookup.byId.get(rawId);
      if (foundById) {
        return {
          id: foundById.id,
          name: !isNumericUid(foundById.name) ? foundById.name : `Thành viên · ${rawId.slice(-4)}`,
          avatar: foundById.avatar,
        };
      }
    }
    const foundByName = senderLookup.byName.get(msg.sender);
    if (foundByName) {
      return foundByName;
    }
    if (isNumericUid(msg.sender)) {
      return {
        id: msg.sender,
        name: `Thành viên · ${msg.sender.slice(-4)}`,
        avatar: null,
      };
    }
    return {
      id: msg.sender_id ?? msg.sender,
      name: msg.sender || "Thành viên",
      avatar: null,
    };
  };

  const resolveReplySenderName = (raw?: string | null): string => {
    if (!raw) return "Trích dẫn";
    if (isNumericUid(raw)) {
      const found = senderLookup.byId.get(raw.trim());
      if (found && !isNumericUid(found.name)) return found.name;
      return `Thành viên · ${raw.trim().slice(-4)}`;
    }
    return raw;
  };

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
        <div className="max-w-sm text-center text-muted-foreground">
          <div className="mb-1.5 text-lg font-bold text-foreground">Chào mừng đến với AloLite</div>
          <div className="text-sm">
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

  const resolveReactionKey = (
    emoji: string,
  ): "like" | "heart" | "smile" | "surprised" | "cry" | "rage" | null => {
    const clean = emoji.trim();
    if (clean === "👍" || clean === "/-strong" || clean === "like" || clean === "3") return "like";
    if (
      clean === "❤️" ||
      clean === "❤" ||
      clean === "/-heart" ||
      clean === "heart" ||
      clean === "5"
    )
      return "heart";
    if (clean === "😆" || clean === ":>" || clean === "smile" || clean === "haha" || clean === "0")
      return "smile";
    if (
      clean === "😮" ||
      clean === ":o" ||
      clean === "surprised" ||
      clean === "wow" ||
      clean === "32"
    )
      return "surprised";
    if (clean === "😢" || clean === ":-((" || clean === "cry" || clean === "buồn" || clean === "2")
      return "cry";
    if (
      clean === "😠" ||
      clean === ":-h" ||
      clean === "rage" ||
      clean === "phẫn nộ" ||
      clean === "20"
    )
      return "rage";
    return null;
  };

  const renderReactionBadge = (raw: string) => {
    const key = resolveReactionKey(raw);
    const asset = key ? state.reactionAssets[key] : "";
    if (asset) {
      return <img src={asset} alt={raw} className="h-4.5 w-4.5 object-contain" />;
    }
    const found = REACTION_LIST.find((r) => r.emoji === raw || r.key === key);
    return <span className="text-sm leading-none">{found?.emoji ?? raw}</span>;
  };

  let prevDay = "";

  return (
    <main className="flex min-w-0 flex-1 flex-col bg-canvas">
      <header className="flex h-15 shrink-0 items-center justify-between border-b border-border bg-sidebar px-4">
        <div className="flex min-w-0 items-center gap-3">
          <Avatar
            id={thread.id}
            name={thread.name}
            avatarUrl={thread.avatar}
            isGroup={thread.is_group}
            size={40}
            mediaCache={state.mediaCache}
          />
          <div className="min-w-0">
            <div className="truncate text-[15.5px] font-bold text-foreground">
              {thread.name || thread.id}
            </div>
            <div
              className={cn(
                "truncate text-xs",
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
            <div className="flex h-9 w-60 items-center gap-2 rounded-md border border-border bg-muted/60 px-3 focus-within:border-ring focus-within:bg-background">
              <Search className="h-4 w-4 shrink-0 text-muted-foreground" />
              <input
                autoFocus
                value={state.inChatSearchQuery}
                onChange={(e) =>
                  dispatch({
                    type: "SET_IN_CHAT_SEARCH_QUERY",
                    query: e.target.value,
                  })
                }
                placeholder="Tìm trong hội thoại…"
                className="w-full bg-transparent text-[13.5px] text-foreground placeholder:text-muted-foreground focus:outline-none"
              />
              <button
                type="button"
                onClick={() => dispatch({ type: "TOGGLE_IN_CHAT_SEARCH", open: false })}
                className="inline-flex h-4.5 w-4.5 cursor-pointer items-center justify-center rounded text-muted-foreground hover:text-foreground"
              >
                <X className="h-3.5 w-3.5" />
              </button>
            </div>
          ) : (
            <Button
              variant="ghost"
              size="icon"
              title="Tìm tin nhắn trong hội thoại"
              onClick={() => dispatch({ type: "TOGGLE_IN_CHAT_SEARCH", open: true })}
            >
              <Search className="h-4.5 w-4.5" />
            </Button>
          )}

          <Button
            variant={state.filterStarredOnly ? "accent" : "ghost"}
            size="icon"
            title="Lọc tin nhắn đã gắn sao"
            onClick={() => dispatch({ type: "TOGGLE_STARRED_FILTER" })}
          >
            <Star className={cn("h-4.5 w-4.5", state.filterStarredOnly && "text-warning")} />
          </Button>

          <Button
            variant={state.infoDrawerOpen ? "accent" : "ghost"}
            size="icon"
            title="Thông tin hội thoại"
            onClick={() => dispatch({ type: "TOGGLE_INFO_DRAWER" })}
          >
            <Info className="h-4.5 w-4.5" />
          </Button>
        </div>
      </header>

      <div ref={scrollRef} className="flex flex-1 flex-col gap-2 overflow-y-auto px-5 py-4">
        {(canShowMoreLocal || !reachedStart) && (
          <div className="mb-2 flex justify-center gap-2">
            {canShowMoreLocal ? (
              <Button
                variant="outline"
                size="sm"
                onClick={() =>
                  dispatch({
                    type: "EXPAND_RENDER_LIMIT",
                    threadId: thread.id,
                    step: 150,
                  })
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

          const prevMsg = !showDivider && idx > 0 ? visibleMessages.at(idx - 1) : undefined;
          const isGroupedWithPrev =
            prevMsg?.is_outgoing === msg.is_outgoing && prevMsg.sender === msg.sender;

          const msgKey = msg.id || `${msg.ts}_${idx}`;
          const isRecalled = msg.text.trim() === "[Tin nhắn đã thu hồi]";
          const callCard = parseCallCard(msg);
          const highlighted = state.quoteHighlightId === msgKey;
          const senderInfo = !msg.is_outgoing ? resolveSender(msg, thread) : null;

          return (
            <div key={msgKey} className="flex flex-col">
              {showDivider ? (
                <div className="my-3 self-center rounded-full border border-border bg-card/90 px-3.5 py-0.5 text-xs font-medium text-muted-foreground shadow-2xs">
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
                  "group relative flex max-w-[76%] items-start gap-2.5 rounded-xl transition-colors",
                  msg.is_outgoing ? "flex-row-reverse self-end" : "self-start",
                  highlighted && "bg-primary/10 p-1 ring-2 ring-primary/60",
                )}
              >
                {!msg.is_outgoing && senderInfo ? (
                  !isGroupedWithPrev ? (
                    <Avatar
                      id={senderInfo.id}
                      name={senderInfo.name}
                      avatarUrl={senderInfo.avatar}
                      size={34}
                      mediaCache={state.mediaCache}
                      className="mt-0.5"
                    />
                  ) : (
                    <div className="w-[34px] shrink-0" />
                  )
                ) : null}

                <div
                  className={cn(
                    "flex min-w-0 flex-col gap-0.5",
                    msg.is_outgoing ? "items-end" : "items-start",
                  )}
                >
                  {!msg.is_outgoing && thread.is_group && !isGroupedWithPrev && senderInfo ? (
                    <div className="px-1 text-xs font-semibold text-muted-foreground">
                      {senderInfo.name}
                    </div>
                  ) : null}

                  <div className="relative flex items-center gap-1.5">
                    <div
                      className={cn(
                        "relative rounded-xl border px-3.5 py-2 text-[14.5px] leading-relaxed break-words whitespace-pre-wrap shadow-2xs",
                        msg.is_outgoing
                          ? "rounded-br-[4px] border-bubble-out-border bg-bubble-out text-bubble-out-foreground"
                          : "rounded-bl-[4px] border-bubble-in-border bg-bubble-in text-bubble-in-foreground",
                        isRecalled && "text-muted-foreground italic opacity-75",
                      )}
                    >
                      {msg.reply_to_text ? (
                        <div
                          onClick={() => jumpToQuoted(msg)}
                          className="mb-1.5 cursor-pointer rounded-md border-l-3 border-primary bg-background/55 px-2.5 py-1.5 text-[13px] hover:bg-background/85"
                        >
                          <div className="text-xs font-bold text-primary">
                            {resolveReplySenderName(msg.reply_to_sender)}
                          </div>
                          <div className="max-w-[300px] truncate text-muted-foreground">
                            {msg.reply_to_text}
                          </div>
                        </div>
                      ) : null}

                      {isRecalled ? (
                        <span>Tin nhắn đã được thu hồi</span>
                      ) : msg.sticker_id ? (
                        <StickerMessageMedia msg={msg} />
                      ) : callCard ? (
                        <div className="flex min-w-[200px] items-center gap-2.5">
                          <div
                            className={cn(
                              "grid h-9 w-9 shrink-0 place-items-center rounded-full",
                              callCard.isMissedOrFailed
                                ? "bg-destructive/15 text-destructive"
                                : "bg-online/15 text-online",
                            )}
                          >
                            {callCard.isVideo ? (
                              <Video className="h-4.5 w-4.5" />
                            ) : (
                              <Phone className="h-4.5 w-4.5" />
                            )}
                          </div>
                          <div>
                            <div className="text-[13.5px] font-bold">{callCard.title}</div>
                            <div className="text-xs text-muted-foreground">{callCard.subtitle}</div>
                          </div>
                        </div>
                      ) : isVideoMessage(msg) && msg.link_url ? (
                        <VideoMessageMedia
                          msg={msg}
                          downloadDir={state.settings?.download_dir ?? ""}
                        />
                      ) : isPhotoMessage(msg) &&
                        Boolean(msg.link_thumb ?? msg.link_url ?? isGifUrl(msg.text)) ? (
                        <PhotoMessageMedia
                          msg={msg}
                          onOpenViewer={(texKey, fullUrl) =>
                            dispatch({
                              type: "SET_IMAGE_VIEWER",
                              viewer: {
                                texKey,
                                fullUrl,
                                title: msg.link_title ?? "Hình ảnh",
                                sender: senderInfo?.name ?? msg.sender,
                                time: msg.time,
                              },
                            })
                          }
                        />
                      ) : isFileMessage(msg) && msg.link_url ? (
                        <div className="flex min-w-[220px] items-center gap-2.5">
                          <FileText className="h-6 w-6 shrink-0 text-primary" />
                          <div className="min-w-0 flex-1">
                            <div className="truncate font-semibold">
                              {msg.link_title ?? msg.text}
                            </div>
                            {msg.link_desc ? (
                              <div className="text-xs text-muted-foreground">{msg.link_desc}</div>
                            ) : null}
                          </div>
                          <Button
                            variant="ghost"
                            size="icon-sm"
                            title="Tải xuống"
                            onClick={() =>
                              void ipc.downloadFile(
                                msg.link_url!,
                                msg.link_title ?? "attachment",
                                state.settings?.download_dir ?? "",
                              )
                            }
                          >
                            <Download className="h-4 w-4" />
                          </Button>
                        </div>
                      ) : (
                        <div>
                          <div className="select-text">{msg.text}</div>
                          {msg.link_url ? (
                            <div
                              onClick={() => void ipc.openExternalUrl(msg.link_url!)}
                              className="mt-1.5 max-w-[340px] cursor-pointer rounded-lg border border-border bg-background/60 p-2.5 hover:bg-background/90"
                            >
                              <div className="flex items-center gap-1.5 text-[13px] font-bold text-foreground">
                                <ExternalLink className="h-3.5 w-3.5 shrink-0 text-primary" />
                                <span className="truncate">{msg.link_title ?? msg.link_url}</span>
                              </div>
                              {msg.link_desc ? (
                                <div className="mt-0.5 line-clamp-2 text-xs text-muted-foreground">
                                  {msg.link_desc}
                                </div>
                              ) : null}
                              {msg.link_src ? (
                                <div className="mt-0.5 truncate text-[11px] text-muted-foreground/80">
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
                          "absolute -top-4 z-10 hidden items-center gap-0.5 rounded-lg border border-border bg-popover p-0.5 shadow-md group-hover:flex",
                          msg.is_outgoing ? "right-2" : "left-2",
                        )}
                      >
                        <Button
                          variant="ghost"
                          size="icon-sm"
                          className="h-6.5 w-6.5"
                          title="Trả lời"
                          onClick={() => dispatch({ type: "SET_REPLYING_TO", msg })}
                        >
                          <CornerUpLeft className="h-3.5 w-3.5" />
                        </Button>

                        <Popover
                          open={reactPickerFor === msgKey}
                          onOpenChange={(open) => setReactPickerFor(open ? msgKey : null)}
                        >
                          <PopoverTrigger asChild>
                            <Button
                              variant="ghost"
                              size="icon-sm"
                              className="h-6.5 w-6.5"
                              title="Thả cảm xúc"
                            >
                              <SmilePlus className="h-3.5 w-3.5" />
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
                                  dispatch({
                                    type: "CORE_EVENT",
                                    event: {
                                      type: "Reacted",
                                      payload: {
                                        thread_id: thread.id,
                                        msg_id: msg.id,
                                        ts: msg.ts,
                                        reaction: r.emoji,
                                      },
                                    },
                                  });
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
                          className="h-6.5 w-6.5"
                          title="Chuyển tiếp"
                          onClick={() => dispatch({ type: "SET_FORWARD_MSG", msg })}
                        >
                          <Forward className="h-3.5 w-3.5" />
                        </Button>

                        <Button
                          variant="ghost"
                          size="icon-sm"
                          className="h-6.5 w-6.5"
                          title="Gắn sao"
                          onClick={() =>
                            dispatch({
                              type: "TOGGLE_LOCAL_STAR",
                              threadId: thread.id,
                              msgId: msg.id,
                            })
                          }
                        >
                          <Star className={cn("h-3.5 w-3.5", msg.is_starred && "text-warning")} />
                        </Button>

                        {msg.is_outgoing ? (
                          <Button
                            variant="ghost"
                            size="icon-sm"
                            className="h-6.5 w-6.5"
                            title="Thu hồi tin nhắn"
                            onClick={() => void ipc.recallMessage(thread.id, thread.is_group, msg)}
                          >
                            <RotateCcw className="h-3.5 w-3.5" />
                          </Button>
                        ) : null}

                        <Button
                          variant="ghost"
                          size="icon-sm"
                          className="h-6.5 w-6.5 text-destructive hover:text-destructive"
                          title="Xóa chỉ ở phía tôi"
                          onClick={() => void ipc.deleteForMe(thread.id, thread.is_group, msg)}
                        >
                          <Trash2 className="h-3.5 w-3.5" />
                        </Button>
                      </div>
                    ) : null}
                  </div>

                  <div className="flex items-center gap-1.5 px-1 text-[11.5px] text-muted-foreground">
                    <span>{msg.time}</span>
                    {msg.is_starred ? <Star className="h-3 w-3 text-warning" /> : null}
                  </div>

                  {msg.reactions && msg.reactions.length > 0 ? (
                    <div
                      className={cn(
                        "absolute -bottom-2 z-10 flex items-center gap-1 rounded-full border border-border bg-card px-2 py-0.5 text-xs shadow-xs select-none",
                        msg.is_outgoing ? "right-2" : "left-2",
                      )}
                    >
                      {Array.from(new Set(msg.reactions))
                        .slice(0, 5)
                        .map((em, i) => (
                          <span key={`${em}_${i}`} className="flex items-center">
                            {renderReactionBadge(em)}
                          </span>
                        ))}
                      {msg.reactions.length > 1 ? (
                        <span className="text-[11px] font-semibold text-muted-foreground">
                          {msg.reactions.length}
                        </span>
                      ) : null}
                    </div>
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
