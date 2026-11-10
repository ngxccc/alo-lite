import { PanelRight, Star, Tag, UserPlus, Users } from "lucide-react";
import type { Thread } from "@/shared/types/ipc.generated";
import { Avatar } from "@/shared/ui/avatar";
import { Button } from "@/shared/ui/button";
import { cn } from "@/shared/utils/cn";
import type { AppAction, AppState } from "@/app/store/app-reducer";
import { threadMemberCount } from "@/features/threads/utils/threads";
import { InChatSearch } from "./in-chat-search";

interface ChatHeaderProps {
  thread: Thread;
  state: AppState;
  dispatch: (action: AppAction) => void;
}

export function ChatHeader({ thread, state, dispatch }: ChatHeaderProps) {
  const friendStatus = state.friendStatusMap[thread.id];
  const onlineInfo = state.userOnlineMap[thread.id];

  return (
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
          <div className="truncate text-lg font-semibold text-foreground">
            {thread.name || thread.id}
          </div>
          <div
            className={cn(
              "flex items-center gap-1 text-sm",
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

        <InChatSearch
          isOpen={state.inChatSearchOpen}
          query={state.inChatSearchQuery}
          onOpen={() => dispatch({ type: "TOGGLE_IN_CHAT_SEARCH", open: true })}
          onClose={() => dispatch({ type: "TOGGLE_IN_CHAT_SEARCH", open: false })}
          onQueryChange={(query) =>
            dispatch({
              type: "SET_IN_CHAT_SEARCH_QUERY",
              query,
            })
          }
        />

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
  );
}
