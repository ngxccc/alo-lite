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
import { dayKey, formatDayDivider, matchesVnQuery } from "../../../shared/utils/text";
import type { AppAction, AppState } from "../../../app/store/appReducer";
import {
  isFileMessage,
  isPhotoMessage,
  isVideoMessage,
  parseCallCard,
  stickerCacheKey,
  stickerThumbnailUrl,
  uniqueReferenceIndex,
} from "../utils/messages";
import { Composer } from "./Composer";

interface ChatAreaProps {
  state: AppState;
  dispatch: (action: AppAction) => void;
}

function StickerMessageMedia({
  msg,
  mediaCache,
}: {
  msg: ChatMessage;
  mediaCache: Record<string, string>;
}) {
  const sid = msg.sticker_id ?? 0;
  const cid = msg.sticker_cate_id ?? 0;
  const key = stickerCacheKey(sid, cid);
  const dataUrl = mediaCache[key];

  useEffect(() => {
    if (!sid || dataUrl || !isTauriRuntime()) return;
    void ipc.fetchMediaData(key, stickerThumbnailUrl({ sticker_id: sid, cate_id: cid }));
  }, [sid, cid, key, dataUrl]);

  if (dataUrl) {
    return <img src={dataUrl} alt={msg.text} style={{ width: 112, height: 112, objectFit: "contain" }} />;
  }
  return (
    <div style={{ width: 100, height: 100, display: "grid", placeItems: "center", color: "var(--text-muted)", fontSize: 11.5 }}>
      Đang tải nhãn dán…
    </div>
  );
}

function PhotoMessageMedia({
  msg,
  mediaCache,
  onOpenViewer,
}: {
  msg: ChatMessage;
  mediaCache: Record<string, string>;
  onOpenViewer: (texKey: string, fullUrl: string) => void;
}) {
  const thumbUrl = msg.link_thumb || msg.link_url || "";
  const fullUrl = msg.link_url || thumbUrl;
  const key = `img_${msg.id || msg.ts}`;
  const dataUrl = mediaCache[key];

  useEffect(() => {
    if (!thumbUrl || dataUrl || !isTauriRuntime()) return;
    void ipc.fetchMediaData(key, thumbUrl);
  }, [key, thumbUrl, dataUrl]);

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 4 }}>
      {dataUrl ? (
        <img
          src={dataUrl}
          alt={msg.link_title || "Hình ảnh"}
          onClick={() => onOpenViewer(key, fullUrl)}
          style={{
            maxWidth: 260,
            maxHeight: 260,
            borderRadius: 8,
            cursor: "zoom-in",
            objectFit: "cover",
          }}
        />
      ) : (
        <button type="button" className="btn" onClick={() => onOpenViewer(key, fullUrl)}>
          Xem hình ảnh
        </button>
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
  }, [thread, state.filterStarredOnly, state.inChatSearchOpen, state.inChatSearchQuery, state.renderLimitByThread]);

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
      <main className="chat-main" style={{ display: "grid", placeItems: "center" }}>
        <div style={{ textAlign: "center", color: "var(--text-muted)", maxWidth: 320 }}>
          <div style={{ fontSize: 16, fontWeight: 700, color: "var(--text-primary)", marginBottom: 6 }}>
            Chào mừng đến với AloLite
          </div>
          <div style={{ fontSize: 13 }}>
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
      dispatch({ type: "SHOW_TOAST", message: "Không tìm thấy tin nhắn gốc duy nhất trong lịch sử đang tải" });
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
      return <img src={asset} alt={emoji} style={{ width: 15, height: 15, objectFit: "contain" }} />;
    }
    return <span>{emoji}</span>;
  };

  let prevDay = "";

  return (
    <main className="chat-main" onClick={() => setReactPickerFor(null)}>
      <header className="chat-header">
        <div className="chat-header-left">
          <Avatar
            id={thread.id}
            name={thread.name}
            avatarUrl={thread.avatar}
            isGroup={thread.is_group}
            size={38}
            mediaCache={state.mediaCache}
          />
          <div style={{ minWidth: 0 }}>
            <div style={{ fontWeight: 700, fontSize: 14.2, whiteSpace: "nowrap", overflow: "hidden", textOverflow: "ellipsis" }}>
              {thread.name || thread.id}
            </div>
            <div style={{ fontSize: 11.8, color: onlineInfo?.[0] ? "var(--accent-online)" : "var(--text-secondary)" }}>
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

        <div className="chat-header-actions">
          {state.inChatSearchOpen ? (
            <div className="search-box" style={{ width: 220, height: 30 }}>
              <Search size={13} color="var(--text-muted)" />
              <input
                autoFocus
                value={state.inChatSearchQuery}
                onChange={(e) => dispatch({ type: "SET_IN_CHAT_SEARCH_QUERY", query: e.target.value })}
                placeholder="Tìm trong hội thoại…"
              />
              <button
                type="button"
                className="icon-btn"
                style={{ width: 20, height: 20 }}
                onClick={() => dispatch({ type: "TOGGLE_IN_CHAT_SEARCH", open: false })}
              >
                <X size={12} />
              </button>
            </div>
          ) : (
            <button
              type="button"
              className="icon-btn"
              title="Tìm tin nhắn trong hội thoại"
              onClick={() => dispatch({ type: "TOGGLE_IN_CHAT_SEARCH", open: true })}
            >
              <Search size={17} />
            </button>
          )}

          <button
            type="button"
            className="icon-btn"
            title="Lọc tin nhắn đã gắn sao"
            style={{ color: state.filterStarredOnly ? "var(--warning)" : undefined }}
            onClick={() => dispatch({ type: "TOGGLE_STARRED_FILTER" })}
          >
            <Star size={17} />
          </button>

          <button
            type="button"
            className="icon-btn"
            title="Thông tin hội thoại"
            style={{ color: state.infoDrawerOpen ? "var(--primary)" : undefined }}
            onClick={() => dispatch({ type: "TOGGLE_INFO_DRAWER" })}
          >
            <Info size={17} />
          </button>
        </div>
      </header>

      <div ref={scrollRef} className="chat-scroll">
        {(canShowMoreLocal || !reachedStart) && (
          <div style={{ display: "flex", justifyContent: "center", gap: 8, marginBottom: 6 }}>
            {canShowMoreLocal ? (
              <button
                type="button"
                className="btn"
                style={{ fontSize: 11.8, padding: "4px 10px" }}
                onClick={() => dispatch({ type: "EXPAND_RENDER_LIMIT", threadId: thread.id, step: 150 })}
              >
                <ChevronUp size={13} /> Xem thêm tin cũ trong máy
              </button>
            ) : null}
            {!reachedStart && (thread.is_group || thread.name.includes("Cloud của tôi")) ? (
              <button
                type="button"
                className="btn"
                style={{ fontSize: 11.8, padding: "4px 10px" }}
                onClick={() => void ipc.loadGroupHistory(thread.id, thread.is_group, thread.messages.length)}
              >
                <ChevronUp size={13} /> Tải lịch sử cũ từ máy chủ
              </button>
            ) : null}
          </div>
        )}

        {visibleMessages.map((msg, idx) => {
          const dk = dayKey(msg.ts);
          const showDivider = Boolean(dk && dk !== prevDay);
          if (dk) prevDay = dk;

          const msgKey = msg.id || `${msg.ts}_${idx}`;
          const isRecalled = msg.text.trim() === "[Tin nhắn đã thu hồi]";
          const callCard = parseCallCard(msg);
          const highlighted = state.quoteHighlightId === msgKey;

          return (
            <div key={msgKey}>
              {showDivider ? <div className="date-divider">{formatDayDivider(msg.ts)}</div> : null}

              <div
                ref={(el) => {
                  msgRefs.current[msgKey] = el;
                }}
                className={`msg-row ${msg.is_outgoing ? "outgoing" : "incoming"} ${highlighted ? "highlighted" : ""}`}
                onDoubleClick={() => {
                  if (state.settings?.double_click_to_reply && !isRecalled) {
                    dispatch({ type: "SET_REPLYING_TO", msg });
                  }
                }}
              >
                {!msg.is_outgoing && thread.is_group ? (
                  <div className="msg-sender">{msg.sender || "Thành viên"}</div>
                ) : null}

                <div className="bubble-wrap">
                  <div className={`bubble ${isRecalled ? "recalled" : ""}`}>
                    {msg.reply_to_text ? (
                      <div className="quote-box" onClick={() => jumpToQuoted(msg)}>
                        <div style={{ fontWeight: 700, fontSize: 11.2, color: "var(--primary)" }}>
                          {msg.reply_to_sender || "Trích dẫn"}
                        </div>
                        <div style={{ color: "var(--text-secondary)", overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap", maxWidth: 280 }}>
                          {msg.reply_to_text}
                        </div>
                      </div>
                    ) : null}

                    {isRecalled ? (
                      <span>Tin nhắn đã được thu hồi</span>
                    ) : msg.sticker_id ? (
                      <StickerMessageMedia msg={msg} mediaCache={state.mediaCache} />
                    ) : callCard ? (
                      <div style={{ display: "flex", alignItems: "center", gap: 10, minWidth: 190 }}>
                        <div
                          style={{
                            width: 34,
                            height: 34,
                            borderRadius: "50%",
                            display: "grid",
                            placeItems: "center",
                            background: callCard.isMissedOrFailed
                              ? "color-mix(in srgb, var(--danger) 18%, transparent)"
                              : "color-mix(in srgb, var(--accent-online) 18%, transparent)",
                            color: callCard.isMissedOrFailed ? "var(--danger)" : "var(--accent-online)",
                          }}
                        >
                          {callCard.isVideo ? <Video size={16} /> : <Phone size={16} />}
                        </div>
                        <div>
                          <div style={{ fontWeight: 700 }}>{callCard.title}</div>
                          <div style={{ fontSize: 11.8, color: "var(--text-muted)" }}>{callCard.subtitle}</div>
                        </div>
                      </div>
                    ) : isPhotoMessage(msg) && (msg.link_thumb || msg.link_url) ? (
                      <PhotoMessageMedia
                        msg={msg}
                        mediaCache={state.mediaCache}
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
                      <div style={{ display: "flex", alignItems: "center", gap: 10, minWidth: 210 }}>
                        <FileText size={24} color="var(--primary)" />
                        <div style={{ flex: 1, minWidth: 0 }}>
                          <div style={{ fontWeight: 600, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
                            {msg.link_title || msg.text}
                          </div>
                          {msg.link_desc ? (
                            <div style={{ fontSize: 11.5, color: "var(--text-muted)" }}>{msg.link_desc}</div>
                          ) : null}
                        </div>
                        <button
                          type="button"
                          className="icon-btn"
                          title="Tải xuống"
                          onClick={() =>
                            void ipc.downloadFile(
                              msg.link_url!,
                              msg.link_title || "attachment",
                              state.settings?.download_dir ?? "",
                            )
                          }
                        >
                          <Download size={15} />
                        </button>
                      </div>
                    ) : isVideoMessage(msg) && msg.link_url ? (
                      <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
                        <Video size={18} color="var(--primary)" />
                        <span>{msg.link_title || "Video đính kèm"}</span>
                        <button
                          type="button"
                          className="btn"
                          style={{ padding: "3px 8px", fontSize: 11.5 }}
                          onClick={() =>
                            void ipc.downloadFile(
                              msg.link_url!,
                              msg.link_title || "video.mp4",
                              state.settings?.download_dir ?? "",
                            )
                          }
                        >
                          Tải về
                        </button>
                      </div>
                    ) : (
                      <div>
                        <div>{msg.text}</div>
                        {msg.link_url ? (
                          <div
                            onClick={() => void ipc.openExternalUrl(msg.link_url!)}
                            style={{
                              marginTop: 6,
                              padding: 8,
                              borderRadius: 8,
                              background: "color-mix(in srgb, var(--bg-app) 45%, transparent)",
                              border: "1px solid var(--border)",
                              cursor: "pointer",
                              maxWidth: 320,
                            }}
                          >
                            <div style={{ display: "flex", alignItems: "center", gap: 6, fontWeight: 700, fontSize: 12.5 }}>
                              <ExternalLink size={13} color="var(--primary)" />
                              <span>{msg.link_title || msg.link_url}</span>
                            </div>
                            {msg.link_desc ? (
                              <div style={{ fontSize: 11.5, color: "var(--text-secondary)", marginTop: 3 }}>
                                {msg.link_desc}
                              </div>
                            ) : null}
                            {msg.link_src ? (
                              <div style={{ fontSize: 11, color: "var(--text-muted)", marginTop: 2 }}>
                                {msg.link_src}
                              </div>
                            ) : null}
                          </div>
                        ) : null}
                      </div>
                    )}
                  </div>

                  {!isRecalled ? (
                    <div className="msg-hover-bar" onClick={(e) => e.stopPropagation()}>
                      <button
                        type="button"
                        className="icon-btn"
                        style={{ width: 26, height: 26 }}
                        title="Trả lời"
                        onClick={() => dispatch({ type: "SET_REPLYING_TO", msg })}
                      >
                        <CornerUpLeft size={13} />
                      </button>
                      <button
                        type="button"
                        className="icon-btn"
                        style={{ width: 26, height: 26 }}
                        title="Thả cảm xúc"
                        onClick={() => setReactPickerFor((cur) => (cur === msgKey ? null : msgKey))}
                      >
                        <SmilePlus size={13} />
                      </button>
                      <button
                        type="button"
                        className="icon-btn"
                        style={{ width: 26, height: 26 }}
                        title="Chuyển tiếp"
                        onClick={() => dispatch({ type: "SET_FORWARD_MSG", msg })}
                      >
                        <Forward size={13} />
                      </button>
                      <button
                        type="button"
                        className="icon-btn"
                        style={{ width: 26, height: 26, color: msg.is_starred ? "var(--warning)" : undefined }}
                        title="Gắn sao"
                        onClick={() => dispatch({ type: "TOGGLE_LOCAL_STAR", threadId: thread.id, msgId: msg.id })}
                      >
                        <Star size={13} />
                      </button>
                      {msg.is_outgoing ? (
                        <button
                          type="button"
                          className="icon-btn"
                          style={{ width: 26, height: 26 }}
                          title="Thu hồi tin nhắn"
                          onClick={() => void ipc.recallMessage(thread.id, thread.is_group, msg)}
                        >
                          <RotateCcw size={13} />
                        </button>
                      ) : null}
                      <button
                        type="button"
                        className="icon-btn"
                        style={{ width: 26, height: 26, color: "var(--danger)" }}
                        title="Xóa chỉ ở phía tôi"
                        onClick={() => void ipc.deleteForMe(thread.id, thread.is_group, msg)}
                      >
                        <Trash2 size={13} />
                      </button>
                    </div>
                  ) : null}

                  {reactPickerFor === msgKey ? (
                    <div
                      className="popover-panel"
                      style={{
                        bottom: "100%",
                        left: msg.is_outgoing ? "auto" : 0,
                        right: msg.is_outgoing ? 0 : "auto",
                        padding: "5px 8px",
                        display: "flex",
                        gap: 6,
                      }}
                      onClick={(e) => e.stopPropagation()}
                    >
                      {REACTION_LIST.map((r) => (
                        <button
                          key={r.key}
                          type="button"
                          className="icon-btn"
                          style={{ width: 28, height: 28 }}
                          title={r.label}
                          onClick={() => {
                            void ipc.reactMessage(thread.id, thread.is_group, msg, r.emoji);
                            setReactPickerFor(null);
                          }}
                        >
                          {renderReactionBadge(r.emoji)}
                        </button>
                      ))}
                    </div>
                  ) : null}
                </div>

                <div className="msg-meta">
                  <span>{msg.time}</span>
                  {msg.is_starred ? <Star size={10} color="var(--warning)" /> : null}
                  {msg.reactions && msg.reactions.length > 0 ? (
                    <span className="reaction-strip">
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
