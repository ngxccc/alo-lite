import { useEffect, useMemo, useRef, useState } from "react";
import {
  ChevronDown,
  ChevronUp,
  CornerUpLeft,
  Download,
  ExternalLink,
  FileText,
  Forward,
  MessageSquare,
  PanelRight,
  Phone,
  Play,
  RotateCcw,
  Search,
  SmilePlus,
  Star,
  Tag,
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
import { threadMemberCount } from "@/features/threads/utils/threads";
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
      className="h-32 w-32 object-contain transition-transform select-none hover:scale-105"
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
        <div className="relative">
          {/* HD badge as in Zalo */}
          <span className="absolute top-2 left-2 z-10 rounded bg-black/60 px-1.5 py-0.5 text-[11px] font-bold text-white">
            HD
          </span>
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
            className="max-h-[320px] max-w-[480px] min-w-[80px] cursor-zoom-in rounded-[6px] object-contain"
          />
        </div>
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
          className="max-h-[300px] max-w-[340px] rounded-[6px] bg-black/90 object-contain"
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
          className="group/vid relative flex min-h-[150px] max-w-[320px] min-w-[220px] cursor-pointer items-center justify-center overflow-hidden rounded-[6px] border border-border bg-black/80"
        >
          {/* HD badge */}
          <span className="absolute top-2 left-2 z-10 rounded bg-black/60 px-1.5 py-0.5 text-[11px] font-bold text-white">
            HD
          </span>
          {posterUrl ? (
            <img
              src={posterUrl}
              alt={fileName}
              loading="lazy"
              decoding="async"
              className="max-h-[260px] w-full object-cover opacity-85 transition group-hover/vid:opacity-100"
            />
          ) : null}
          <div className="absolute inset-0 flex items-center justify-center">
            <div className="flex h-10 w-10 items-center justify-center rounded-full bg-black/55 shadow-md backdrop-blur-sm transition group-hover/vid:bg-black/70">
              <Play className="h-5 w-5 fill-white text-white" />
            </div>
          </div>
          {!inlineVideoSupported || playbackError ? (
            <div className="absolute right-0 bottom-2 left-0 flex justify-center">
              <span className="rounded bg-black/60 px-2 py-0.5 text-[11px] text-white/80">
                {playbackError ? "Nhấn để mở ngoài" : "Nhấn để xem"}
              </span>
            </div>
          ) : null}
        </button>
      ) : null}
      {!shouldHideMediaCaption(msg) ? (
        <span className="text-[15px] select-text">{msg.text}</span>
      ) : null}
      {/* Download button for videos */}
      {videoUrl ? (
        <button
          type="button"
          className="flex w-fit items-center gap-1 text-[12px] text-muted-foreground hover:text-foreground"
          onClick={() => void ipc.downloadFile(videoUrl, fileName, downloadDir)}
        >
          <Download className="h-3.5 w-3.5" />
          Tải xuống
        </button>
      ) : null}
    </div>
  );
}

function isNumericUid(val: string): boolean {
  const trimmed = val.trim();
  return trimmed.length >= 6 && /^\d+$/.test(trimmed);
}

/** Derive file extension color for the file icon badge */
function fileIconColor(fileName?: string): string {
  const ext = (fileName ?? "").split(".").pop()?.toLowerCase() ?? "";
  if (["pdf"].includes(ext)) return "bg-red-500";
  if (["doc", "docx"].includes(ext)) return "bg-blue-600";
  if (["xls", "xlsx"].includes(ext)) return "bg-green-600";
  if (["ppt", "pptx"].includes(ext)) return "bg-orange-500";
  if (["zip", "rar", "7z"].includes(ext)) return "bg-yellow-600";
  if (["mp3", "ogg", "wav", "aac"].includes(ext)) return "bg-purple-500";
  return "bg-primary";
}

/** Format file size from bytes string */
function formatFileSize(desc?: string): string {
  if (!desc) return "";
  // If desc already has KB/MB, just return it
  if (/[KMG]B/i.test(desc)) return desc;
  const n = parseFloat(desc);
  if (isNaN(n)) return desc;
  if (n < 1024) return `${n} B`;
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KB`;
  return `${(n / 1024 / 1024).toFixed(1)} MB`;
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
  const pinnedMessage = pinnedMessages.at(0);
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
      clean === ":-" ||
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
      {/* ── Chat Header ── 68px per spec ── */}
      <header className="flex h-[68px] shrink-0 items-center justify-between border-b border-border bg-sidebar px-4">
        <div className="flex min-w-0 items-center gap-3">
          <Avatar
            id={thread.id}
            name={thread.name}
            avatarUrl={thread.avatar}
            isGroup={thread.is_group}
            memberCount={thread.is_group ? threadMemberCount(thread) : undefined}
            size={48}
            mediaCache={state.mediaCache}
          />
          <div className="min-w-0">
            <div className="truncate text-[18px] font-semibold text-foreground">
              {thread.name || thread.id}
            </div>
            <div
              className={cn(
                "flex items-center gap-1 text-[14px]",
                onlineInfo?.[0] ? "font-medium text-online" : "text-muted-foreground",
              )}
            >
              {thread.is_group ? (
                <>
                  <Users className="h-3.5 w-3.5 shrink-0" />
                  <span className="truncate">
                    {thread.presentation.member_count
                      ? `${thread.presentation.member_count} thành viên`
                      : "Nhóm trò chuyện"}
                  </span>
                  <Tag className="ml-0.5 h-3.5 w-3.5 shrink-0 opacity-60" />
                </>
              ) : thread.name.includes("Cloud của tôi") ? (
                "Lưu trữ đám mây cá nhân"
              ) : (
                onlineInfo?.[1] || (friendStatus === "Stranger" ? "Người lạ" : "Trực tuyến")
              )}
            </div>
          </div>
        </div>

        {/* Right-side icon buttons — 32px each */}
        <div className="flex items-center gap-1">
          {thread.is_group ? (
            <Button
              variant="ghost"
              size="icon"
              title="Thêm thành viên vào nhóm"
              className="h-8 w-8 text-muted-foreground hover:text-foreground"
              onClick={() => dispatch({ type: "SET_PHONE_MODAL", open: true })}
            >
              <UserPlus className="h-5 w-5" />
            </Button>
          ) : null}

          {state.inChatSearchOpen ? (
            <div className="flex h-8 w-56 items-center gap-2 rounded-md border border-border bg-muted px-2.5 focus-within:border-primary/80">
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
              className="h-8 w-8 text-muted-foreground hover:text-foreground"
              onClick={() => dispatch({ type: "TOGGLE_IN_CHAT_SEARCH", open: true })}
            >
              <Search className="h-5 w-5" />
            </Button>
          )}

          <Button
            variant={state.filterStarredOnly ? "accent" : "ghost"}
            size="icon"
            title="Lọc tin nhắn đã gắn sao"
            className="h-8 w-8 text-muted-foreground hover:text-foreground"
            onClick={() => dispatch({ type: "TOGGLE_STARRED_FILTER" })}
          >
            <Star className={cn("h-5 w-5", state.filterStarredOnly && "text-warning")} />
          </Button>

          {/* Info panel toggle — filled blue square when active */}
          <button
            type="button"
            title="Thông tin hội thoại"
            className={cn(
              "flex h-8 w-8 cursor-pointer items-center justify-center rounded-md transition-colors",
              state.infoDrawerOpen
                ? "bg-primary text-primary-foreground"
                : "text-muted-foreground hover:bg-[rgba(255,255,255,0.05)] hover:text-foreground",
            )}
            onClick={() => dispatch({ type: "TOGGLE_INFO_DRAWER" })}
          >
            <PanelRight className="h-5 w-5" />
          </button>
        </div>
      </header>

      {/* ── Pinned message bar ── */}
      {pinnedMessage ? (
        <div className="flex shrink-0 items-center justify-between border-b border-border/40 bg-sidebar px-4 py-1.5">
          <div
            onClick={() => jumpToQuoted(pinnedMessage)}
            className="flex min-w-0 flex-1 cursor-pointer items-center gap-2.5 overflow-hidden hover:opacity-90"
          >
            <div className="flex h-7 w-7 shrink-0 items-center justify-center rounded-full bg-primary/10 text-primary">
              <MessageSquare className="h-4 w-4" />
            </div>
            <div className="min-w-0 flex-1">
              <div className="text-[13px] leading-tight font-semibold text-foreground">
                Tin nhắn
              </div>
              <div className="truncate text-[12px] leading-tight text-muted-foreground">
                {pinnedMessage.sender ? `${pinnedMessage.sender}: ` : ""}
                {pinnedMessage.text || "[Hình ảnh/Tập tin]"}
              </div>
            </div>
          </div>
          <div className="ml-2 flex shrink-0 items-center gap-1.5">
            {pinnedMessages.length > 1 ? (
              <button
                type="button"
                onClick={() => dispatch({ type: "TOGGLE_INFO_DRAWER" })}
                className="flex items-center gap-1 rounded-full bg-muted/80 px-2.5 py-0.5 text-[11px] font-medium text-muted-foreground transition hover:bg-muted"
              >
                +{pinnedMessages.length - 1} ghim
                <ChevronDown className="h-3 w-3" />
              </button>
            ) : (
              <Button
                variant="ghost"
                size="icon-sm"
                className="h-6 w-6 text-muted-foreground hover:text-foreground"
                title="Xem tất cả tin ghim"
                onClick={() => dispatch({ type: "TOGGLE_INFO_DRAWER" })}
              >
                <ChevronDown className="h-3.5 w-3.5" />
              </Button>
            )}
          </div>
        </div>
      ) : null}

      {/* ── Message list ── */}
      <div ref={scrollRef} className="flex flex-1 flex-col gap-0.5 overflow-y-auto px-4 py-3">
        {/* Load history banners — styled as quiet chips */}
        {(canShowMoreLocal || !reachedStart) && (
          <div className="mb-3 flex flex-col items-center gap-2">
            {canShowMoreLocal ? (
              <button
                type="button"
                className="flex cursor-pointer items-center gap-1.5 self-center rounded-full bg-[rgba(255,255,255,0.05)] px-4 py-1 text-[12px] text-muted-foreground transition hover:bg-[rgba(255,255,255,0.09)] active:scale-95"
                onClick={() =>
                  dispatch({
                    type: "EXPAND_RENDER_LIMIT",
                    threadId: thread.id,
                    step: 150,
                  })
                }
              >
                <ChevronUp className="h-3 w-3" />
                Xem thêm tin cũ trong máy
              </button>
            ) : null}
            {!reachedStart && (thread.is_group || thread.name.includes("Cloud của tôi")) ? (
              <button
                type="button"
                className="flex cursor-pointer items-center gap-1.5 self-center rounded-full bg-[rgba(255,255,255,0.05)] px-4 py-1 text-[12px] text-muted-foreground transition hover:bg-[rgba(255,255,255,0.09)] active:scale-95"
                onClick={() =>
                  void ipc.loadGroupHistory(thread.id, thread.is_group, thread.messages.length)
                }
              >
                <ChevronUp className="h-3 w-3" />
                Tải lịch sử cũ từ máy chủ
              </button>
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

          // Whether this message has bubble container (not bare sticker/special)
          const isBareSticker = Boolean(msg.sticker_id);
          const isTextOrRich = !isBareSticker;

          return (
            <div
              key={msgKey}
              className={cn("flex flex-col", isGroupedWithPrev ? "mt-0.5" : "mt-2")}
            >
              {/* Date divider chip */}
              {showDivider ? (
                <div className="my-3 self-center rounded-xl bg-[rgba(255,255,255,0.05)] px-3 py-0.5 text-[13px] text-muted-foreground">
                  {formatDayDivider(msg.ts)}
                </div>
              ) : null}

              {/* Message row */}
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
                  "group relative flex max-w-[76%] items-end gap-2 transition-colors",
                  msg.is_outgoing ? "flex-row-reverse self-end" : "self-start",
                  highlighted && "rounded-lg bg-primary/10 p-1 ring-2 ring-primary/60",
                )}
              >
                {/* Incoming avatar — 32px */}
                {!msg.is_outgoing && senderInfo ? (
                  !isGroupedWithPrev ? (
                    <Avatar
                      id={senderInfo.id}
                      name={senderInfo.name}
                      avatarUrl={senderInfo.avatar}
                      size={32}
                      mediaCache={state.mediaCache}
                      className="mb-1 shrink-0"
                    />
                  ) : (
                    <div className="w-8 shrink-0" />
                  )
                ) : null}

                {/* Bubble column */}
                <div
                  className={cn(
                    "relative flex min-w-0 flex-col",
                    msg.is_outgoing ? "items-end" : "items-start",
                  )}
                >
                  {/* Bubble */}
                  <div
                    className={cn(
                      isBareSticker
                        ? "relative"
                        : cn(
                            "relative max-w-[640px] rounded-[6px] text-[15px] leading-relaxed break-words whitespace-pre-wrap shadow-sm",
                            isRecalled
                              ? "bg-bubble-in px-3 py-2.5 text-muted-foreground italic opacity-70"
                              : msg.is_outgoing
                                ? "bg-bubble-out px-3 py-2.5 text-bubble-out-foreground"
                                : "bg-bubble-in px-3 py-2.5 text-bubble-in-foreground",
                          ),
                    )}
                  >
                    {/* Sender name inside bubble top — groups only, incoming, first in run */}
                    {!msg.is_outgoing &&
                    thread.is_group &&
                    !isGroupedWithPrev &&
                    senderInfo &&
                    isTextOrRich &&
                    !isRecalled ? (
                      <div className="mb-1 text-[13px] font-semibold text-accent-foreground">
                        {senderInfo.name}
                      </div>
                    ) : null}

                    {/* Reply quote */}
                    {msg.reply_to_text ? (
                      <div
                        onClick={() => jumpToQuoted(msg)}
                        className="mb-2 cursor-pointer rounded-md border-l-[3px] border-primary bg-background/40 px-2.5 py-1.5 text-[13px] hover:bg-background/70"
                      >
                        <div className="text-[12px] font-bold text-primary">
                          {resolveReplySenderName(msg.reply_to_sender)}
                        </div>
                        <div className="max-w-[300px] truncate text-muted-foreground">
                          {msg.reply_to_text}
                        </div>
                      </div>
                    ) : null}

                    {/* Message body */}
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
                          <div className="text-[13px] text-muted-foreground">
                            {callCard.subtitle}
                          </div>
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
                      /* File card — 40px colored icon */
                      <div className="flex max-w-[340px] min-w-[220px] items-center gap-3">
                        <div
                          className={cn(
                            "flex h-10 w-10 shrink-0 items-center justify-center rounded-lg",
                            fileIconColor(msg.link_title ?? msg.text),
                          )}
                        >
                          <FileText className="h-5 w-5 text-white" />
                        </div>
                        <div className="min-w-0 flex-1">
                          <div className="truncate text-[14px] font-medium">
                            {msg.link_title ?? msg.text}
                          </div>
                          {msg.link_desc ? (
                            <div className="text-[12px] text-muted-foreground">
                              {formatFileSize(msg.link_desc)}
                            </div>
                          ) : null}
                        </div>
                        <Button
                          variant="ghost"
                          size="icon-sm"
                          title="Tải xuống"
                          className="h-8 w-8 shrink-0"
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
                      /* Plain text + optional link card */
                      <div>
                        <div className="select-text">{msg.text}</div>
                        {msg.link_url ? (
                          <div
                            onClick={() => void ipc.openExternalUrl(msg.link_url!)}
                            className="mt-2 max-w-[360px] cursor-pointer rounded-[6px] border border-border/60 bg-background/50 p-2.5 transition-colors hover:bg-background/80"
                          >
                            {msg.link_thumb ? (
                              <img
                                src={msg.link_thumb}
                                alt={msg.link_title ?? ""}
                                loading="lazy"
                                className="mb-2 w-full rounded-[4px] object-cover"
                                style={{ maxHeight: 140 }}
                              />
                            ) : null}
                            <div className="flex items-start gap-1.5">
                              {!msg.link_thumb ? (
                                <ExternalLink className="mt-0.5 h-3.5 w-3.5 shrink-0 text-primary" />
                              ) : null}
                              <div className="min-w-0">
                                <div className="truncate text-[14px] font-semibold text-foreground">
                                  {msg.link_title ?? msg.link_url}
                                </div>
                                {msg.link_desc ? (
                                  <div className="mt-0.5 line-clamp-2 text-[13px] text-muted-foreground">
                                    {msg.link_desc}
                                  </div>
                                ) : null}
                                {msg.link_src ? (
                                  <div className="mt-1 truncate text-[12px] text-primary">
                                    {msg.link_src}
                                  </div>
                                ) : null}
                              </div>
                            </div>
                          </div>
                        ) : null}
                      </div>
                    )}

                    {/* Timestamp inside bubble — bottom, after content */}
                    {isTextOrRich && !isRecalled ? (
                      <div
                        className={cn(
                          "mt-1.5 flex items-center gap-1 text-[12px]",
                          msg.is_outgoing
                            ? "justify-end text-bubble-out-foreground/60"
                            : "text-muted-foreground",
                        )}
                      >
                        <span>{msg.time}</span>
                        {msg.is_starred ? <Star className="h-3 w-3 text-warning" /> : null}
                        {msg.is_outgoing && msg.delivery === "seen" ? (
                          <span className="opacity-80">· Đã xem</span>
                        ) : msg.is_outgoing && msg.delivery === "sent" ? (
                          <span className="opacity-60">· Đã gửi</span>
                        ) : null}
                      </div>
                    ) : null}

                    {/* Quick reaction/like button — small circle, bottom-right corner of bubble, appears on group hover */}
                    {isTextOrRich && !isRecalled ? (
                      <Popover
                        open={reactPickerFor === `quick_${msgKey}`}
                        onOpenChange={(open) => setReactPickerFor(open ? `quick_${msgKey}` : null)}
                      >
                        <PopoverTrigger asChild>
                          <button
                            type="button"
                            title="Thả cảm xúc"
                            className={cn(
                              "absolute -right-2.5 -bottom-2.5 z-10 flex h-6 w-6 cursor-pointer items-center justify-center rounded-full border border-border bg-sidebar shadow-sm transition-opacity",
                              "opacity-0 group-hover:opacity-100",
                            )}
                          >
                            <SmilePlus className="h-3.5 w-3.5 text-muted-foreground" />
                          </button>
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
                    ) : null}
                  </div>

                  {/* Sticker time below */}
                  {isBareSticker || isRecalled ? (
                    <div className="flex items-center gap-1 px-1 pt-0.5 text-[11.5px] text-muted-foreground">
                      <span>{msg.time}</span>
                      {msg.is_starred ? <Star className="h-3 w-3 text-warning" /> : null}
                    </div>
                  ) : null}

                  {/* Reactions chip — overlaps bubble bottom edge */}
                  {msg.reactions && msg.reactions.length > 0 ? (
                    <div
                      className={cn(
                        "-mt-1.5 flex cursor-pointer items-center gap-0.5 rounded-full border border-border/80 bg-sidebar px-2 py-0.5 text-xs shadow-sm transition-colors select-none hover:bg-secondary",
                        msg.is_outgoing ? "mr-1 self-end" : "ml-1 self-start",
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
                        <span className="ml-0.5 text-[11px] font-semibold text-muted-foreground">
                          {msg.reactions.length}
                        </span>
                      ) : null}
                    </div>
                  ) : null}
                </div>

                {/* Hover action toolbar — floating above bubble */}
                {!isRecalled ? (
                  <div
                    className={cn(
                      "absolute -top-9 z-20 hidden items-center gap-0.5 rounded-lg border border-border bg-popover p-0.5 shadow-md group-hover:flex",
                      msg.is_outgoing ? "right-0" : "left-10",
                    )}
                  >
                    <Button
                      variant="ghost"
                      size="icon-sm"
                      className="h-7 w-7"
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
                          className="h-7 w-7"
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
                      className="h-7 w-7"
                      title="Chuyển tiếp"
                      onClick={() => dispatch({ type: "SET_FORWARD_MSG", msg })}
                    >
                      <Forward className="h-3.5 w-3.5" />
                    </Button>

                    <Button
                      variant="ghost"
                      size="icon-sm"
                      className="h-7 w-7"
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
                        className="h-7 w-7"
                        title="Thu hồi tin nhắn"
                        onClick={() => void ipc.recallMessage(thread.id, thread.is_group, msg)}
                      >
                        <RotateCcw className="h-3.5 w-3.5" />
                      </Button>
                    ) : null}

                    <Button
                      variant="ghost"
                      size="icon-sm"
                      className="h-7 w-7 text-destructive hover:text-destructive"
                      title="Xóa chỉ ở phía tôi"
                      onClick={() => void ipc.deleteForMe(thread.id, thread.is_group, msg)}
                    >
                      <Trash2 className="h-3.5 w-3.5" />
                    </Button>
                  </div>
                ) : null}
              </div>
            </div>
          );
        })}
      </div>

      <Composer state={state} dispatch={dispatch} thread={thread} />
    </main>
  );
}
