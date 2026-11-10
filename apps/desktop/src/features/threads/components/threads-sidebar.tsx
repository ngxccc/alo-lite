import { useRef, useState } from "react";
import { useVirtualizer } from "@tanstack/react-virtual";
import {
  BellOff,
  CheckCheck,
  ChevronDown,
  EyeOff,
  MailCheck,
  MailOpen,
  MoreHorizontal,
  Pin,
  PinOff,
  Search,
  Trash2,
  UserPlus,
  Users,
  X,
} from "lucide-react";
import { ipc } from "@/shared/api/ipc";
import type { Thread } from "@/shared/types/ipc";
import { Avatar } from "@/shared/ui/avatar";
import { Badge } from "@/shared/ui/badge";
import { Button } from "@/shared/ui/button";
import {
  ContextMenu,
  ContextMenuContent,
  ContextMenuItem,
  ContextMenuSeparator,
  ContextMenuTrigger,
} from "@/shared/ui/context-menu";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/shared/ui/dropdown-menu";
import { cn } from "@/shared/utils/cn";
import { matchesVnQuery } from "@/shared/utils/text";
import type { AppAction, AppState } from "@/app/store/app-reducer";
import {
  isThreadMuted,
  threadMemberCount,
  threadUnreadCount,
} from "@/features/threads/utils/threads";

interface ThreadsSidebarProps {
  state: AppState;
  dispatch: (action: AppAction) => void;
}

export function ThreadsSidebar({ state, dispatch }: ThreadsSidebarProps) {
  const listRef = useRef<HTMLDivElement>(null);
  const [unreadOnly, setUnreadOnly] = useState(false);
  const settings = state.settings;

  const hiddenSet = new Set(settings?.hidden_threads ?? []);
  const pinnedSet = new Set(settings?.pinned_threads ?? []);

  const visibleThreads = state.threads.filter((t) => {
    if (hiddenSet.has(t.id) || t.presentation.hidden) return false;
    if (state.threadFilter === "direct" && t.is_group) return false;
    if (state.threadFilter === "groups" && !t.is_group) return false;
    if (unreadOnly) {
      const unread = threadUnreadCount(t, state.unreadCounts);
      if (unread === 0 && !t.unread.marked_unread) return false;
    }
    if (!state.searchQuery.trim()) return true;
    return (
      matchesVnQuery(t.name, state.searchQuery) ||
      matchesVnQuery(t.preview, state.searchQuery) ||
      t.messages.some((m) => matchesVnQuery(m.text, state.searchQuery))
    );
  });

  const virtualizer = useVirtualizer({
    count: visibleThreads.length,
    getScrollElement: () => listRef.current,
    estimateSize: () => 70,
    overscan: 8,
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
  };

  const toggleHide = (thread: Thread) => {
    if (!settings) return;
    const nextHidden = [...settings.hidden_threads, thread.id];
    const nextSettings = { ...settings, hidden_threads: nextHidden };
    dispatch({ type: "UPDATE_SETTINGS_LOCAL", settings: nextSettings });
    void ipc.saveSettings(nextSettings);
    void ipc.hideThread(thread.id, thread.is_group, true);
  };

  return (
    <section className="flex h-full w-[345px] shrink-0 flex-col border-r border-border bg-sidebar text-sidebar-foreground">
      {/* Search row — 64 px tall */}
      <div className="flex h-16 shrink-0 items-center justify-between px-4">
        <div className="flex h-8 w-[240px] shrink-0 items-center gap-1.5 rounded-[5px] bg-muted px-2.5">
          <Search className="h-4 w-4 shrink-0 text-muted-foreground" />
          <input
            type="text"
            value={state.searchQuery}
            onChange={(e) => dispatch({ type: "SET_SEARCH_QUERY", query: e.target.value })}
            onKeyDown={(e) => handleSearchKeyDown(e.key)}
            placeholder="Tìm kiếm"
            className="w-full bg-transparent text-[13px] text-foreground placeholder:text-muted-foreground focus:outline-none"
          />
          {state.searchQuery ? (
            <button
              type="button"
              onClick={() => dispatch({ type: "SET_SEARCH_QUERY", query: "" })}
              title="Xóa tìm kiếm"
              className="inline-flex h-4 w-4 cursor-pointer items-center justify-center text-muted-foreground hover:text-foreground"
            >
              <X className="h-3 w-3" />
            </button>
          ) : null}
        </div>
        <div className="flex items-center gap-0.5">
          <Button
            variant="ghost"
            size="icon"
            title="Thêm bạn"
            className="h-8 w-8 shrink-0 text-muted-foreground hover:bg-foreground/5 hover:text-foreground"
            onClick={() => dispatch({ type: "SET_PHONE_MODAL", open: true })}
          >
            <UserPlus className="h-[18px] w-[18px]" />
          </Button>
          <Button
            variant="ghost"
            size="icon"
            title="Tạo nhóm"
            className="h-8 w-8 shrink-0 text-muted-foreground hover:bg-foreground/5 hover:text-foreground"
            onClick={() => dispatch({ type: "SET_PHONE_MODAL", open: true })}
          >
            <Users className="h-[18px] w-[18px]" />
          </Button>
        </div>
      </div>

      {/* Tabs + filter row — 32 px tall */}
      <div className="flex h-8 shrink-0 items-center justify-between border-b border-border/40 px-4">
        <div className="flex h-full items-center gap-4 text-[14px]">
          <button
            type="button"
            onClick={() => setUnreadOnly(false)}
            className={cn(
              "relative flex h-full cursor-pointer items-center transition-colors",
              !unreadOnly
                ? "font-medium text-accent-foreground after:absolute after:right-0 after:bottom-0 after:left-0 after:h-0.5 after:bg-accent-foreground"
                : "font-normal text-muted-foreground hover:text-foreground",
            )}
          >
            Tất cả
          </button>
          <button
            type="button"
            onClick={() => setUnreadOnly(true)}
            className={cn(
              "relative flex h-full cursor-pointer items-center transition-colors",
              unreadOnly
                ? "font-medium text-accent-foreground after:absolute after:right-0 after:bottom-0 after:left-0 after:h-0.5 after:bg-accent-foreground"
                : "font-normal text-muted-foreground hover:text-foreground",
            )}
          >
            Chưa đọc
          </button>
        </div>

        <div className="flex items-center gap-1">
          <DropdownMenu>
            <DropdownMenuTrigger asChild>
              <button
                type="button"
                className="inline-flex cursor-pointer items-center gap-0.5 text-[13px] text-muted-foreground transition hover:text-foreground"
              >
                <span>
                  {state.threadFilter === "direct"
                    ? "Cá nhân"
                    : state.threadFilter === "groups"
                      ? "Nhóm"
                      : "Phân loại"}
                </span>
                <ChevronDown className="h-3 w-3" />
              </button>
            </DropdownMenuTrigger>
            <DropdownMenuContent align="end">
              <DropdownMenuItem
                onClick={() => dispatch({ type: "SET_THREAD_FILTER", filter: "all" })}
              >
                Tất cả phân loại
              </DropdownMenuItem>
              <DropdownMenuItem
                onClick={() => dispatch({ type: "SET_THREAD_FILTER", filter: "direct" })}
              >
                Tin nhắn cá nhân
              </DropdownMenuItem>
              <DropdownMenuItem
                onClick={() => dispatch({ type: "SET_THREAD_FILTER", filter: "groups" })}
              >
                Tin nhắn nhóm
              </DropdownMenuItem>
            </DropdownMenuContent>
          </DropdownMenu>

          <DropdownMenu>
            <DropdownMenuTrigger asChild>
              <button
                type="button"
                title="Tùy chọn khác"
                className="inline-flex h-6 w-6 cursor-pointer items-center justify-center rounded text-muted-foreground hover:text-foreground"
              >
                <MoreHorizontal className="h-4 w-4" />
              </button>
            </DropdownMenuTrigger>
            <DropdownMenuContent align="end">
              <DropdownMenuItem
                onClick={() => {
                  for (const t of state.threads) {
                    const unread = threadUnreadCount(t, state.unreadCounts);
                    if (unread > 0) {
                      const last = t.messages.at(-1);
                      if (last?.id) void ipc.sendSeen(t.id, t.is_group, last.id);
                    }
                  }
                }}
              >
                <CheckCheck className="h-4 w-4" /> Đánh dấu đã đọc tất cả
              </DropdownMenuItem>
            </DropdownMenuContent>
          </DropdownMenu>
        </div>
      </div>

      {/* Conversation list — virtualised, items use inset-x-2 for 8 px side margins */}
      <div ref={listRef} className="flex-1 overflow-y-auto py-1">
        {visibleThreads.length === 0 ? (
          <div className="p-6 text-center text-[13px] text-muted-foreground">
            Không có hội thoại phù hợp
          </div>
        ) : (
          <div style={{ height: virtualizer.getTotalSize() }} className="relative w-full">
            {virtualizer.getVirtualItems().map((vRow) => {
              const t = visibleThreads.at(vRow.index);
              if (!t) return null;
              const selected = t.id === state.selectedThreadId;
              const muted = settings ? isThreadMuted(t, settings) : false;
              const pinned = t.presentation.pinned || pinnedSet.has(t.id);
              const unread = threadUnreadCount(t, state.unreadCounts);
              const hasDot = unread > 0 || t.unread.marked_unread;

              return (
                <ContextMenu key={t.id}>
                  <ContextMenuTrigger asChild>
                    <div
                      style={{
                        height: vRow.size,
                        transform: `translateY(${vRow.start}px)`,
                      }}
                      onClick={() => {
                        dispatch({ type: "SELECT_THREAD", threadId: t.id });
                        const last = t.messages.at(-1);
                        if (last && !last.is_outgoing && last.id) {
                          void ipc.sendSeen(t.id, t.is_group, last.id);
                        }
                      }}
                      className={cn(
                        "absolute inset-x-2 top-0 flex cursor-pointer items-center gap-3 rounded-[6px] px-2 transition-colors select-none",
                        selected
                          ? "bg-accent text-foreground"
                          : "hover:bg-foreground/5 dark:hover:bg-[rgba(255,255,255,0.05)]",
                      )}
                    >
                      <Avatar
                        id={t.id}
                        name={t.name}
                        avatarUrl={t.avatar}
                        isGroup={t.is_group}
                        memberCount={t.is_group ? threadMemberCount(t) : undefined}
                        size={48}
                        mediaCache={state.mediaCache}
                      />
                      <div className="flex min-w-0 flex-1 flex-col gap-0.5">
                        <div className="flex items-center justify-between gap-1">
                          <span
                            className={cn(
                              "truncate text-[16px] text-foreground",
                              hasDot ? "font-semibold" : "font-normal",
                            )}
                          >
                            {t.name || t.id}
                          </span>
                          <span className="shrink-0 text-[12px] text-muted-foreground">
                            {t.time}
                          </span>
                        </div>
                        <div className="flex items-center justify-between gap-1.5">
                          <span className="truncate text-[14px] text-muted-foreground">
                            {t.presentation.last_outgoing &&
                            !t.preview.startsWith("Bạn:") &&
                            !t.preview.startsWith("You:")
                              ? `You: ${t.preview || "Hình ảnh"}`
                              : t.is_group && t.presentation.last_sender && !t.preview.includes(":")
                                ? `${t.presentation.last_sender}: ${t.preview}`
                                : t.preview || "Chưa có tin nhắn"}
                          </span>
                          <div className="inline-flex shrink-0 items-center gap-1">
                            {pinned ? <Pin className="h-3 w-3 text-muted-foreground" /> : null}
                            {muted ? <BellOff className="h-3 w-3 text-muted-foreground" /> : null}
                            {unread > 0 ? (
                              <Badge
                                variant="destructive"
                                className={cn(
                                  "h-4 rounded-full text-[11px] leading-none font-bold",
                                  unread > 9 ? "min-w-4 px-1" : "w-4 p-0",
                                )}
                              >
                                {unread > 99 ? "99+" : unread}
                              </Badge>
                            ) : t.unread.marked_unread ? (
                              <span className="h-2 w-2 rounded-full bg-primary" />
                            ) : null}
                          </div>
                        </div>
                      </div>
                    </div>
                  </ContextMenuTrigger>

                  <ContextMenuContent>
                    <ContextMenuItem onSelect={() => togglePin(t)}>
                      {pinned ? (
                        <>
                          <PinOff className="h-4 w-4" /> Bỏ ghim hội thoại
                        </>
                      ) : (
                        <>
                          <Pin className="h-4 w-4" /> Ghim hội thoại
                        </>
                      )}
                    </ContextMenuItem>
                    <ContextMenuItem
                      onSelect={() => {
                        const markUnread = !t.unread.marked_unread;
                        void ipc.markThreadUnread(t.id, t.is_group, markUnread);
                      }}
                    >
                      {t.unread.marked_unread ? (
                        <>
                          <MailOpen className="h-4 w-4" /> Đánh dấu đã đọc
                        </>
                      ) : (
                        <>
                          <MailCheck className="h-4 w-4" /> Đánh dấu chưa đọc
                        </>
                      )}
                    </ContextMenuItem>
                    {muted ? (
                      <ContextMenuItem onSelect={() => toggleMute(t, 0, false)}>
                        <BellOff className="h-4 w-4" /> Bật lại thông báo
                      </ContextMenuItem>
                    ) : (
                      <>
                        <ContextMenuItem onSelect={() => toggleMute(t, 3600, true)}>
                          <BellOff className="h-4 w-4" /> Tắt thông báo 1 giờ
                        </ContextMenuItem>
                        <ContextMenuItem onSelect={() => toggleMute(t, -1, true)}>
                          <BellOff className="h-4 w-4" /> Tắt đến khi bật lại
                        </ContextMenuItem>
                      </>
                    )}
                    <ContextMenuItem onSelect={() => toggleHide(t)}>
                      <EyeOff className="h-4 w-4" /> Ẩn trò chuyện
                    </ContextMenuItem>
                    <ContextMenuSeparator />
                    <ContextMenuItem
                      destructive
                      onSelect={() => {
                        const last = t.messages.at(-1);
                        void ipc.deleteThread(
                          t.id,
                          t.is_group,
                          last?.client_message_id ?? last?.id ?? "",
                        );
                      }}
                    >
                      <Trash2 className="h-4 w-4" /> Xóa hội thoại
                    </ContextMenuItem>
                  </ContextMenuContent>
                </ContextMenu>
              );
            })}
          </div>
        )}
      </div>
    </section>
  );
}
