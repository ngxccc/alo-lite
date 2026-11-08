import { useEffect, useMemo, useRef, useState } from "react";
import {
  ChevronDown,
  ChevronUp,
  CornerUpLeft,
  Download,
  ExternalLink,
  FileText,
  Forward,
  PanelRight,
  Phone,
  Pin,
  Play,
  RotateCcw,
  Search,
  SmilePlus,
  Star,
  Trash2,
  UserPlus,
  Users,
  Video,
  X,
} from "lucide-react";
import { ipc, isTauriRuntime } from "@/shared/api/ipc";
import { REACTION_LIST, type ChatMessage, type Thread } from "@/shared/types/ipc";
import { Avatar } from "@/shared/ui/Avatar";
import { Button } from "@/shared/ui/button";
import { Popover, PopoverContent, PopoverTrigger } from "@/shared/ui/popover";
import { cn } from "@/shared/utils/cn";
import { dayKey, formatDayDivider, jxlToJpgUrl, matchesVnQuery } from "@/shared/utils/text";
import type { AppAction, AppState } from "@/app/store/appReducer";
import likeIcon from "@/assets/reactions/like.png";
import heartIcon from "@/assets/reactions/heart.png";
import smileIcon from "@/assets/reactions/smile.png";
import surprisedIcon from "@/assets/reactions/surprised.png";
import cryIcon from "@/assets/reactions/cry.png";
import rageIcon from "@/assets/reactions/rage.png";
import {
  isFileMessage,
  isGifUrl,
  isPhotoMessage,
  isVideoMessage,
  parseCallCard,
  shouldHideMediaCaption,
  stickerThumbnailUrl,
  uniqueReferenceIndex,
} from "@/features/chat/utils/messages";

const REACTION_ICONS = {
  like: likeIcon,
  heart: heartIcon,
  smile: smileIcon,
  surprised: surprisedIcon,
  cry: cryIcon,
  rage: rageIcon,
} as const;
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
      className="h-32 w-32 object-contain select-none transition-transform hover:scale-105"
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

function VideoMessageMedia({
  msg,
  downloadDir,
  inlineVideoSupported,
  inlineVideoReason,
  onNotify,
}: {
  msg: ChatMessage;
  downloadDir: string;
  inlineVideoSupported: boolean;
  inlineVideoReason: string | null;
  onNotify: (message: string) => void;
}) {
  const [playing, setPlaying] = useState(false);
  const [playbackError, setPlaybackError] = useState(false);
  const [blockedClick, setBlockedClick] = useState(false);
  const videoUrl = msg.link_url ?? "";
  const posterUrl = msg.link_thumb ? jxlToJpgUrl(msg.link_thumb) : undefined;
  const fileName =
    msg.link_title && !msg.link_title.startsWith("[")
      ? msg.link_title.endsWith(".mp4")
        ? msg.link_title
        : `${msg.link_title}.mp4`
      : "video.mp4";

  useEffect(() => {
    setPlaying(false);
    setPlaybackError(false);
    setBlockedClick(false);
  }, [videoUrl]);

  const unsupportedMessage =
    inlineVideoReason ??
    "Thiếu GStreamer (gst-plugins-good / gst-libav) nên không thể phát video trực tiếp.";

  return (
    <div className="flex flex-col gap-1.5">
      {videoUrl && inlineVideoSupported && playing && !playbackError ? (
        <video
          src={videoUrl}
          poster={posterUrl}
          controls
          autoPlay
          preload="auto"
          onError={(e) => {
            const err = e.currentTarget.error;
            console.error("Video playback error:", err?.code, err?.message, videoUrl);
            setPlaybackError(true);
            onNotify("Không thể phát trực tiếp video này — hãy thử mở ngoài hoặc tải về.");
          }}
          className="max-h-[300px] max-w-[340px] rounded-lg bg-black/90 object-contain"
        />
      ) : videoUrl ? (
        <button
          type="button"
          onClick={() => {
            if (!inlineVideoSupported) {
              if (blockedClick) {
                void ipc.openExternalUrl(videoUrl);
              } else {
                setBlockedClick(true);
                onNotify(unsupportedMessage);
              }
            } else if (playbackError) {
              void ipc.openExternalUrl(videoUrl);
            } else {
              setPlaying(true);
            }
          }}
          className="group/vid relative flex min-h-[150px] max-w-[320px] min-w-[220px] cursor-pointer items-center justify-center overflow-hidden rounded-lg border border-border bg-black/80"
        >
          {posterUrl ? (
            <img
              src={posterUrl}
              alt={fileName}
              loading="lazy"
              decoding="async"
              className="max-h-[260px] w-full object-cover opacity-85 transition group-hover/vid:opacity-100"
            />
          ) : null}
          <div className="absolute inset-0 flex flex-col items-center justify-center gap-1.5 bg-black/30 p-3 text-white">
            <div className="flex h-11 w-11 items-center justify-center rounded-full bg-black/65 ring-1 ring-white/40 transition group-hover/vid:scale-105 group-hover/vid:bg-primary">
              <Play className="ml-0.5 h-5 w-5 fill-current" />
            </div>
            {!inlineVideoSupported && blockedClick ? (
              <span className="rounded bg-black/80 px-2.5 py-1 text-center text-[11.5px] leading-snug text-amber-200">
                {unsupportedMessage} (Bấm lần nữa để mở ngoài)
              </span>
            ) : playbackError ? (
              <span className="rounded bg-black/80 px-2.5 py-1 text-[11.5px] text-amber-200">
                Không thể phát trực tiếp · Bấm để mở ngoài
              </span>
            ) : !inlineVideoSupported ? (
              <span className="rounded bg-black/65 px-2 py-0.5 text-[11px] text-white/90">
                Thiếu codec GStreamer · Bấm để xem chi tiết
              </span>
            ) : null}
          </div>
        </button>
      ) : null}
      <div className="flex items-center justify-between gap-2">
        <span className="flex items-center gap-1.5 text-xs text-muted-foreground">
          <Video className="h-3.5 w-3.5 text-primary" />
          {!shouldHideMediaCaption(msg) ? msg.text : "Video"}
        </span>
        {videoUrl ? (
          <div className="flex items-center gap-1.5">
            {!inlineVideoSupported || playbackError ? (
              <Button
                variant="outline"
                size="sm"
                onClick={() => void ipc.openExternalUrl(videoUrl)}
              >
                <ExternalLink className="h-3.5 w-3.5" /> Mở ngoài
              </Button>
            ) : null}
            <Button
              variant="outline"
              size="sm"
              onClick={() => void ipc.downloadFile(videoUrl, fileName, downloadDir)}
            >
              <Download className="h-3.5 w-3.5" /> Tải về
            </Button>
          </div>
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
  const pinnedMessages = useMemo(
    () => thread?.messages.filter((m) => m.is_pinned) ?? [],
    [thread?.messages],
  );
  const pinnedMessage =
    pinnedMessages[0] ??
    (thread?.is_group && thread.messages.length > 0
      ? thread.messages.find(
          (m) =>
            m.text?.toLowerCase().includes("lưu ý") ||
            m.text?.toLowerCase().includes("thông báo") ||
            m.text?.toLowerCase().includes("chú ý"),
        )
      : undefined);
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

  const resolveReactionKey = (raw: string): keyof typeof REACTION_ICONS | null => {
    const clean = raw.trim();
    if (clean === "\u{1F44D}" || clean === "/-strong" || clean === "like" || clean === "3") {
      return "like";
    }
    if (
      clean === "\u{2764}\u{FE0F}" ||
      clean === "\u{2764}" ||
      clean === "/-heart" ||
      clean === "heart" ||
      clean === "5"
    ) {
      return "heart";
    }
    if (
      clean === "\u{1F606}" ||
      clean === ":>" ||
      clean === "smile" ||
      clean === "haha" ||
      clean === "0"
    ) {
      return "smile";
    }
    if (
      clean === "\u{1F62E}" ||
      clean === ":o" ||
      clean === "surprised" ||
      clean === "wow" ||
      clean === "32"
    ) {
      return "surprised";
    }
    if (
      clean === "\u{1F622}" ||
      clean === ":-((" ||
      clean === "cry" ||
      clean === "buồn" ||
      clean === "2"
    ) {
      return "cry";
    }
    if (
      clean === "\u{1F620}" ||
      clean === ":-h" ||
      clean === "rage" ||
      clean === "phẫn nộ" ||
      clean === "20"
    ) {
      return "rage";
    }
    return null;
  };

  const renderReactionBadge = (raw: string) => {
    const key = resolveReactionKey(raw);
    if (key) {
      return <img src={REACTION_ICONS[key]} alt={key} className="h-4.5 w-4.5 object-contain" />;
    }
    const found = REACTION_LIST.find((r) => r.emoji === raw);
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
            memberCount={
              thread.presentation.member_count ||
              (thread.is_group ? thread.presentation.members.length : undefined)
            }
            size={42}
            mediaCache={state.mediaCache}
          />
          <div className="min-w-0">
            <div className="truncate text-[15px] font-bold text-foreground">
              {thread.name || thread.id}
            </div>
            <div
              className={cn(
                "flex items-center gap-1.5 truncate text-xs",
                onlineInfo?.[0] ? "font-medium text-online" : "text-muted-foreground",
              )}
            >
              {thread.is_group ? (
                <>
                  <Users className="h-3 w-3 shrink-0" />
                  <span>
                    {thread.presentation.member_count
                      ? `${thread.presentation.member_count} thành viên`
                      : "Nhóm trò chuyện"}
                  </span>
                </>
              ) : thread.name.includes("Cloud của tôi") ? (
                "Lưu trữ đám mây cá nhân"
              ) : (
                onlineInfo?.[1] || (friendStatus === "Stranger" ? "Người lạ" : "Trực tuyến")
              )}
            </div>
          </div>
        </div>

        <div className="flex items-center gap-1.5">
          {thread.is_group ? (
            <Button
              variant="ghost"
              size="icon"
              title="Thêm thành viên vào nhóm"
              className="h-8.5 w-8.5 text-muted-foreground hover:text-foreground"
              onClick={() => dispatch({ type: "SET_PHONE_MODAL", open: true })}
            >
              <UserPlus className="h-4.5 w-4.5" />
            </Button>
          ) : null}

          {state.inChatSearchOpen ? (
            <div className="flex h-8.5 w-60 items-center gap-2 rounded-md border border-border bg-muted/60 px-2.5 focus-within:border-primary/80 focus-within:bg-background">
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
                placeholder="Tìm trong cuộc trò chuyện…"
                className="w-full bg-transparent text-[13px] text-foreground placeholder:text-muted-foreground focus:outline-none"
              />
              <button
                type="button"
                onClick={() => dispatch({ type: "TOGGLE_IN_CHAT_SEARCH", open: false })}
                className="inline-flex h-4 w-4 cursor-pointer items-center justify-center rounded text-muted-foreground hover:text-foreground"
              >
                <X className="h-3.5 w-3.5" />
              </button>
            </div>
          ) : (
            <Button
              variant="ghost"
              size="icon"
              title="Tìm kiếm tin nhắn"
              className="h-8.5 w-8.5 text-muted-foreground hover:text-foreground"
              onClick={() => dispatch({ type: "TOGGLE_IN_CHAT_SEARCH", open: true })}
            >
              <Search className="h-4.5 w-4.5" />
            </Button>
          )}

          <Button
            variant={state.filterStarredOnly ? "accent" : "ghost"}
            size="icon"
            title="Lọc tin nhắn đã gắn sao"
            className="h-8.5 w-8.5 text-muted-foreground hover:text-foreground"
            onClick={() => dispatch({ type: "TOGGLE_STARRED_FILTER" })}
          >
            <Star className={cn("h-4.5 w-4.5", state.filterStarredOnly && "text-warning")} />
          </Button>

          <Button
            variant={state.infoDrawerOpen ? "default" : "ghost"}
            size="icon"
            title="Thông tin hội thoại"
            className={cn(
              "h-8.5 w-8.5 transition-colors",
              state.infoDrawerOpen
                ? "bg-primary text-primary-foreground shadow-xs"
                : "text-muted-foreground hover:text-foreground",
            )}
            onClick={() => dispatch({ type: "TOGGLE_INFO_DRAWER" })}
          >
            <PanelRight className="h-4.5 w-4.5" />
          </Button>
        </div>
      </header>

      {pinnedMessage ? (
        <div className="flex items-center justify-between border-b border-border/70 bg-[#16171b] px-4 py-2 text-xs">
          <div
            onClick={() => jumpToQuoted({ id: pinnedMessage.id, ts: pinnedMessage.ts } as ChatMessage)}
            className="flex min-w-0 flex-1 cursor-pointer items-center gap-2 overflow-hidden hover:opacity-90"
          >
            <Pin className="h-3.5 w-3.5 shrink-0 text-primary rotate-45" />
            <div className="min-w-0 truncate">
              <span className="font-bold text-primary mr-1.5">Tin nhắn ghim</span>
              <span className="truncate text-foreground/90">
                {pinnedMessage.sender ? `${pinnedMessage.sender}: ` : ""}
                {pinnedMessage.text || "[Hình ảnh/Tập tin]"}
              </span>
            </div>
          </div>
          <div className="flex items-center gap-1.5 shrink-0 ml-2">
            {pinnedMessages.length > 1 ? (
              <span className="rounded bg-muted/80 px-2 py-0.5 text-[11px] font-medium text-muted-foreground">
                +{pinnedMessages.length - 1} ghim
              </span>
            ) : null}
            <Button
              variant="ghost"
              size="icon-sm"
              className="h-6 w-6 text-muted-foreground hover:text-foreground"
              title="Mở bảng tin ghim"
              onClick={() => dispatch({ type: "TOGGLE_INFO_DRAWER" })}
            >
              <ChevronDown className="h-3.5 w-3.5" />
            </Button>
          </div>
        </div>
      ) : null}
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
                <div className="my-3 self-center rounded-full border border-border/70 bg-[#1f2126] px-3.5 py-0.5 text-[11.5px] font-medium text-muted-foreground shadow-2xs">
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
                        msg.sticker_id
                          ? "relative bg-transparent p-0 border-0 shadow-none"
                          : cn(
                              "relative rounded-2xl border px-3.5 py-2 text-[14px] leading-relaxed break-words whitespace-pre-wrap shadow-2xs",
                              msg.is_outgoing
                                ? "rounded-br-xs border-bubble-out-border bg-bubble-out text-bubble-out-foreground"
                                : "rounded-bl-xs border-bubble-in-border bg-bubble-in text-bubble-in-foreground",
                            ),
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
                          inlineVideoSupported={state.inlineVideoSupported}
                          inlineVideoReason={state.inlineVideoReason}
                          onNotify={(message) => dispatch({ type: "SHOW_TOAST", message })}
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
                        "absolute -bottom-2.5 z-10 flex cursor-pointer items-center gap-1 rounded-full border border-border/80 bg-[#212328] px-2 py-0.5 text-xs shadow-xs select-none hover:bg-[#282a30] transition-colors",
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
