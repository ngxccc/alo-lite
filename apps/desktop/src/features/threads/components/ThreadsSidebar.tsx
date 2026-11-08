import { useRef } from "react";
import { useVirtualizer } from "@tanstack/react-virtual";
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
import { Badge } from "../../../shared/ui/badge";
import { Button } from "../../../shared/ui/button";
import {
  ContextMenu,
  ContextMenuContent,
  ContextMenuItem,
  ContextMenuSeparator,
  ContextMenuTrigger,
} from "../../../shared/ui/context-menu";
import { Tabs, TabsList, TabsTrigger } from "../../../shared/ui/tabs";
import { cn } from "../../../shared/utils/cn";
import { matchesVnQuery } from "../../../shared/utils/text";
import type { AppAction, AppState } from "../../../app/store/appReducer";
import { isThreadMuted, threadUnreadCount } from "../utils/threads";

interface ThreadsSidebarProps {
  state: AppState;
  dispatch: (action: AppAction) => void;
}

export function ThreadsSidebar({ state, dispatch }: ThreadsSidebarProps) {
  const listRef = useRef<HTMLDivElement>(null);
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

  const virtualizer = useVirtualizer({
    count: visibleThreads.length,
    getScrollElement: () => listRef.current,
    estimateSize: () => 62,
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
      nextMuted[thread.id] =
        durationSec > 0 ? Math.floor(Date.now() / 1000) + durationSec : -1;
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
    <section className="flex h-full w-[310px] shrink-0 flex-col border-r border-border bg-sidebar text-sidebar-foreground">
      <div className="flex flex-col gap-2 border-b border-border p-2.5">
        <div className="flex items-center gap-1.5">
          <div className="flex h-8 flex-1 items-center gap-1.5 rounded-md border border-border bg-muted/60 px-2.5 focus-within:border-ring focus-within:bg-background">
            <Search className="h-3.5 w-3.5 shrink-0 text-muted-foreground" />
            <input
              type="text"
              value={state.searchQuery}
              onChange={(e) =>
                dispatch({ type: "SET_SEARCH_QUERY", query: e.target.value })
              }
              onKeyDown={(e) => handleSearchKeyDown(e.key)}
              placeholder="Tìm hội thoại, tin nhắn, SĐT…"
              className="w-full bg-transparent text-xs text-foreground placeholder:text-muted-foreground focus:outline-none"
            />
            {state.searchQuery ? (
              <button
                type="button"
                onClick={() =>
                  dispatch({ type: "SET_SEARCH_QUERY", query: "" })
                }
                title="Xóa tìm kiếm"
                className="inline-flex h-4 w-4 cursor-pointer items-center justify-center rounded text-muted-foreground hover:text-foreground"
              >
                <X className="h-3 w-3" />
              </button>
            ) : null}
          </div>
          <Button
            variant="ghost"
            size="icon"
            title="Tra cứu SĐT & Kết bạn"
            onClick={() => dispatch({ type: "SET_PHONE_MODAL", open: true })}
          >
            <UserPlus className="h-4 w-4" />
          </Button>
        </div>

        <Tabs
          value={state.threadFilter}
          onValueChange={(v) =>
            dispatch({
              type: "SET_THREAD_FILTER",
              filter: v as "all" | "direct" | "groups",
            })
          }
        >
          <TabsList className="grid w-full grid-cols-3">
            <TabsTrigger value="all">Tất cả</TabsTrigger>
            <TabsTrigger value="direct">Cá nhân</TabsTrigger>
            <TabsTrigger value="groups">Nhóm</TabsTrigger>
          </TabsList>
        </Tabs>
      </div>

      <div ref={listRef} className="flex-1 overflow-y-auto px-1.5 py-1">
        {visibleThreads.length === 0 ? (
          <div className="p-6 text-center text-xs text-muted-foreground">
            Không có hội thoại phù hợp
          </div>
        ) : (
          <div
            style={{ height: virtualizer.getTotalSize() }}
            className="relative w-full"
          >
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
                        "absolute top-0 right-0 left-0 flex cursor-pointer items-center gap-2.5 rounded-lg px-2.5 py-2 transition-colors",
                        selected
                          ? "bg-accent text-accent-foreground"
                          : "hover:bg-muted/70",
                      )}
                    >
                      <Avatar
                        id={t.id}
                        name={t.name}
                        avatarUrl={t.avatar}
                        isGroup={t.is_group}
                        size={40}
                        mediaCache={state.mediaCache}
                      />
                      <div className="flex min-w-0 flex-1 flex-col gap-0.5">
                        <div className="flex items-center justify-between gap-1.5">
                          <span
                            className={cn(
                              "truncate text-[13.2px] text-foreground",
                              hasDot ? "font-bold" : "font-semibold",
                            )}
                          >
                            {t.name || t.id}
                          </span>
                          <span className="shrink-0 text-[11px] text-muted-foreground">
                            {t.time}
                          </span>
                        </div>
                        <div className="flex items-center justify-between gap-1.5">
                          <span className="truncate text-xs text-muted-foreground">
                            {t.preview || "Chưa có tin nhắn"}
                          </span>
                          <div className="inline-flex shrink-0 items-center gap-1">
                            {pinned ? (
                              <Pin className="h-3 w-3 text-primary" />
                            ) : null}
                            {muted ? (
                              <BellOff className="h-3 w-3 text-muted-foreground" />
                            ) : null}
                            {unread > 0 ? (
                              <Badge
                                variant="destructive"
                                className="h-4 min-w-4 px-1 text-[10px]"
                              >
                                {unread > 99 ? "99+" : unread}
                              </Badge>
                            ) : t.unread.marked_unread ? (
                              <span className="h-2.5 w-2.5 rounded-full bg-primary" />
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
                          <PinOff className="h-3.5 w-3.5" /> Bỏ ghim hội thoại
                        </>
                      ) : (
                        <>
                          <Pin className="h-3.5 w-3.5" /> Ghim hội thoại
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
                          <MailOpen className="h-3.5 w-3.5" /> Đánh dấu đã đọc
                        </>
                      ) : (
                        <>
                          <MailCheck className="h-3.5 w-3.5" /> Đánh dấu chưa
                          đọc
                        </>
                      )}
                    </ContextMenuItem>
                    {muted ? (
                      <ContextMenuItem onSelect={() => toggleMute(t, 0, false)}>
                        <BellOff className="h-3.5 w-3.5" /> Bật lại thông báo
                      </ContextMenuItem>
                    ) : (
                      <>
                        <ContextMenuItem
                          onSelect={() => toggleMute(t, 3600, true)}
                        >
                          <BellOff className="h-3.5 w-3.5" /> Tắt thông báo 1
                          giờ
                        </ContextMenuItem>
                        <ContextMenuItem
                          onSelect={() => toggleMute(t, -1, true)}
                        >
                          <BellOff className="h-3.5 w-3.5" /> Tắt đến khi bật
                          lại
                        </ContextMenuItem>
                      </>
                    )}
                    <ContextMenuItem onSelect={() => toggleHide(t)}>
                      <EyeOff className="h-3.5 w-3.5" /> Ẩn trò chuyện
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
                      <Trash2 className="h-3.5 w-3.5" /> Xóa hội thoại
                    </ContextMenuItem>
                  </ContextMenuContent>
                </ContextMenu>
              );
            })}
          </div>
        )}
      </div>

      <div className="flex items-center justify-between border-t border-border px-3 py-1.5 text-[11px] text-muted-foreground">
        <span className="truncate">{state.statusMessage}</span>
        <Button
          variant="ghost"
          size="icon-sm"
          title="Làm mới"
          onClick={() => void ipc.refreshThreads(true)}
        >
          <RefreshCw className="h-3 w-3" />
        </Button>
      </div>
    </section>
  );
}
