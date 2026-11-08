import { useState, type MouseEvent } from "react";
import {
  BellOff,
  EyeOff,
  MailCheck,
  MailOpen,
  Pin,
  PinOff,
  RefreshCw,
  Search,
  Trash2,
  UserPlus,
  X,
} from "lucide-react";
import { ipc } from "../../../shared/api/ipc";
import type { Thread } from "../../../shared/types/ipc";
import { Avatar } from "../../../shared/ui/Avatar";
import { matchesVnQuery } from "../../../shared/utils/text";
import type { AppAction, AppState } from "../../../app/store/appReducer";
import { isThreadMuted, threadUnreadCount } from "../utils/threads";

interface ThreadsSidebarProps {
  state: AppState;
  dispatch: (action: AppAction) => void;
}

interface ContextState {
  x: number;
  y: number;
  thread: Thread;
}

export function ThreadsSidebar({ state, dispatch }: ThreadsSidebarProps) {
  const [ctxMenu, setCtxMenu] = useState<ContextState | null>(null);
  const settings = state.settings;

  const hiddenSet = new Set(settings?.hidden_threads ?? []);
  const pinnedSet = new Set(settings?.pinned_threads ?? []);

  const visibleThreads = state.threads.filter((t) => {
    if (hiddenSet.has(t.id) || t.presentation.hidden) return false;
    if (state.threadFilter === "direct" && t.is_group) return false;
    if (state.threadFilter === "groups" && !t.is_group) return false;
    if (!state.searchQuery.trim()) return true;
    return (
      matchesVnQuery(t.name, state.searchQuery) ||
      matchesVnQuery(t.preview, state.searchQuery) ||
      t.messages.some((m) => matchesVnQuery(m.text, state.searchQuery))
    );
  });

  const handleSearchKeyDown = (key: string) => {
    if (key !== "Enter") return;
    const digits = state.searchQuery.replace(/\D/g, "");
    if (digits.length >= 8) {
      dispatch({ type: "SET_PHONE_MODAL", open: true });
      dispatch({ type: "START_PHONE_LOOKUP" });
      void ipc.lookupPhone(state.searchQuery.trim());
    }
  };

  const openContextMenu = (e: MouseEvent, thread: Thread) => {
    e.preventDefault();
    setCtxMenu({ x: e.clientX, y: e.clientY, thread });
  };

  const togglePin = (thread: Thread) => {
    if (!settings) return;
    const isPinned = thread.presentation.pinned || pinnedSet.has(thread.id);
    const nextPinned = isPinned
      ? settings.pinned_threads.filter((id) => id !== thread.id)
      : [...settings.pinned_threads, thread.id];
    const nextSettings = { ...settings, pinned_threads: nextPinned };
    dispatch({ type: "UPDATE_SETTINGS_LOCAL", settings: nextSettings });
    void ipc.saveSettings(nextSettings);
    void ipc.pinThread(thread.id, thread.is_group, !isPinned);
    setCtxMenu(null);
  };

  const toggleMute = (thread: Thread, durationSec: number, mute: boolean) => {
    if (!settings) return;
    const nextMuted = { ...settings.muted_threads };
    if (mute) {
      nextMuted[thread.id] = durationSec > 0 ? Math.floor(Date.now() / 1000) + durationSec : -1;
    } else {
      delete nextMuted[thread.id];
    }
    const nextSettings = { ...settings, muted_threads: nextMuted };
    dispatch({ type: "UPDATE_SETTINGS_LOCAL", settings: nextSettings });
    void ipc.saveSettings(nextSettings);
    void ipc.muteThread(thread.id, thread.is_group, durationSec, mute);
    setCtxMenu(null);
  };

  const toggleHide = (thread: Thread) => {
    if (!settings) return;
    const nextHidden = [...settings.hidden_threads, thread.id];
    const nextSettings = { ...settings, hidden_threads: nextHidden };
    dispatch({ type: "UPDATE_SETTINGS_LOCAL", settings: nextSettings });
    void ipc.saveSettings(nextSettings);
    void ipc.hideThread(thread.id, thread.is_group, true);
    setCtxMenu(null);
  };

  return (
    <section className="sidebar" onClick={() => setCtxMenu(null)}>
      <div className="sidebar-top">
        <div className="search-row">
          <div className="search-box">
            <Search size={14} color="var(--text-muted)" />
            <input
              type="text"
              value={state.searchQuery}
              onChange={(e) => dispatch({ type: "SET_SEARCH_QUERY", query: e.target.value })}
              onKeyDown={(e) => handleSearchKeyDown(e.key)}
              placeholder="Tìm hội thoại, tin nhắn, SĐT…"
            />
            {state.searchQuery ? (
              <button
                type="button"
                className="icon-btn"
                style={{ width: 20, height: 20 }}
                onClick={() => dispatch({ type: "SET_SEARCH_QUERY", query: "" })}
                title="Xóa tìm kiếm"
              >
                <X size={12} />
              </button>
            ) : null}
          </div>
          <button
            type="button"
            className="icon-btn"
            title="Tra cứu SĐT & Kết bạn"
            onClick={() => dispatch({ type: "SET_PHONE_MODAL", open: true })}
          >
            <UserPlus size={16} />
          </button>
        </div>

        <div className="filter-tabs">
          {[
            { id: "all" as const, label: "Tất cả" },
            { id: "direct" as const, label: "Cá nhân" },
            { id: "groups" as const, label: "Nhóm" },
          ].map((item) => (
            <button
              key={item.id}
              type="button"
              className={`filter-tab ${state.threadFilter === item.id ? "active" : ""}`}
              onClick={() => dispatch({ type: "SET_THREAD_FILTER", filter: item.id })}
            >
              {item.label}
            </button>
          ))}
        </div>
      </div>

      <div className="thread-list">
        {visibleThreads.length === 0 ? (
          <div style={{ padding: 24, textAlign: "center", color: "var(--text-muted)", fontSize: 12.5 }}>
            Không có hội thoại phù hợp
          </div>
        ) : (
          visibleThreads.map((t) => {
            const selected = t.id === state.selectedThreadId;
            const muted = settings ? isThreadMuted(t, settings) : false;
            const pinned = t.presentation.pinned || pinnedSet.has(t.id);
            const unread = threadUnreadCount(t, state.unreadCounts);
            const hasDot = unread > 0 || t.unread.marked_unread;

            return (
              <div
                key={t.id}
                className={`thread-row ${selected ? "selected" : ""}`}
                onClick={() => {
                  dispatch({ type: "SELECT_THREAD", threadId: t.id });
                  const last = t.messages[t.messages.length - 1];
                  if (last && !last.is_outgoing && last.id) {
                    void ipc.sendSeen(t.id, t.is_group, last.id);
                  }
                }}
                onContextMenu={(e) => openContextMenu(e, t)}
              >
                <Avatar
                  id={t.id}
                  name={t.name}
                  avatarUrl={t.avatar}
                  isGroup={t.is_group}
                  size={40}
                  mediaCache={state.mediaCache}
                />
                <div className="thread-meta">
                  <div className="thread-title-line">
                    <span
                      className="thread-name"
                      style={{ fontWeight: hasDot ? 700 : 600 }}
                    >
                      {t.name || t.id}
                    </span>
                    <span className="thread-time">{t.time}</span>
                  </div>
                  <div className="thread-preview-line">
                    <span className="thread-preview">{t.preview || "Chưa có tin nhắn"}</span>
                    <div style={{ display: "inline-flex", alignItems: "center", gap: 4, flexShrink: 0 }}>
                      {pinned ? <Pin size={11} color="var(--primary)" /> : null}
                      {muted ? <BellOff size={11} color="var(--text-muted)" /> : null}
                      {unread > 0 ? (
                        <span className="unread-pill">{unread > 99 ? "99+" : unread}</span>
                      ) : t.unread.marked_unread ? (
                        <span className="unread-pill" style={{ minWidth: 10, height: 10, padding: 0 }} />
                      ) : null}
                    </div>
                  </div>
                </div>
              </div>
            );
          })
        )}
      </div>

      <div className="sidebar-footer">
        <span style={{ overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
          {state.statusMessage}
        </span>
        <button
          type="button"
          className="icon-btn"
          style={{ width: 24, height: 24 }}
          title="Làm mới"
          onClick={() => void ipc.refreshThreads(true)}
        >
          <RefreshCw size={12} />
        </button>
      </div>

      {ctxMenu ? (
        <div
          className="context-menu"
          style={{ left: ctxMenu.x, top: ctxMenu.y }}
          onClick={(e) => e.stopPropagation()}
        >
          <button type="button" onClick={() => togglePin(ctxMenu.thread)}>
            {ctxMenu.thread.presentation.pinned || pinnedSet.has(ctxMenu.thread.id) ? (
              <>
                <PinOff size={14} /> Bỏ ghim hội thoại
              </>
            ) : (
              <>
                <Pin size={14} /> Ghim hội thoại
              </>
            )}
          </button>
          <button
            type="button"
            onClick={() => {
              const markUnread = !ctxMenu.thread.unread.marked_unread;
              void ipc.markThreadUnread(ctxMenu.thread.id, ctxMenu.thread.is_group, markUnread);
              setCtxMenu(null);
            }}
          >
            {ctxMenu.thread.unread.marked_unread ? (
              <>
                <MailOpen size={14} /> Đánh dấu đã đọc
              </>
            ) : (
              <>
                <MailCheck size={14} /> Đánh dấu chưa đọc
              </>
            )}
          </button>
          {settings && isThreadMuted(ctxMenu.thread, settings) ? (
            <button type="button" onClick={() => toggleMute(ctxMenu.thread, 0, false)}>
              <BellOff size={14} /> Bật lại thông báo
            </button>
          ) : (
            <>
              <button type="button" onClick={() => toggleMute(ctxMenu.thread, 3600, true)}>
                <BellOff size={14} /> Tắt thông báo 1 giờ
              </button>
              <button type="button" onClick={() => toggleMute(ctxMenu.thread, -1, true)}>
                <BellOff size={14} /> Tắt đến khi bật lại
              </button>
            </>
          )}
          <button type="button" onClick={() => toggleHide(ctxMenu.thread)}>
            <EyeOff size={14} /> Ẩn trò chuyện
          </button>
          <button
            type="button"
            style={{ color: "var(--danger)" }}
            onClick={() => {
              const last = ctxMenu.thread.messages[ctxMenu.thread.messages.length - 1];
              void ipc.deleteThread(
                ctxMenu.thread.id,
                ctxMenu.thread.is_group,
                last?.client_message_id || last?.id || "",
              );
              setCtxMenu(null);
            }}
          >
            <Trash2 size={14} /> Xóa hội thoại
          </button>
        </div>
      ) : null}
    </section>
  );
}
