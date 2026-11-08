import { Bug, Cloud, MessageSquare, Moon, Settings, Smartphone, Sun, Users } from "lucide-react";
import { ipc } from "@/shared/api/ipc";
import { Avatar } from "@/shared/ui/Avatar";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/shared/ui/tooltip";
import { cn } from "@/shared/utils/cn";
import type { AppAction, AppState } from "@/app/store/appReducer";
import { threadUnreadCount } from "@/features/threads/utils/threads";

interface NavRailProps {
  state: AppState;
  dispatch: (action: AppAction) => void;
}

function RailButton({
  active = false,
  label,
  onClick,
  children,
}: {
  active?: boolean;
  label: string;
  onClick: () => void;
  children: React.ReactNode;
}) {
  return (
    <Tooltip>
      <TooltipTrigger asChild>
        <button
          type="button"
          onClick={onClick}
          aria-label={label}
          className={cn(
            "relative flex h-11 w-11 cursor-pointer items-center justify-center rounded-xl transition-all duration-150 select-none",
            active
              ? "bg-white/15 text-white shadow-xs"
              : "text-rail-foreground/75 hover:bg-white/8 hover:text-white",
          )}
        >
          {children}
        </button>
      </TooltipTrigger>
      <TooltipContent side="right">{label}</TooltipContent>
    </Tooltip>
  );
}

export function NavRail({ state, dispatch }: NavRailProps) {
  const totalUnread = state.threads.reduce(
    (sum, t) => sum + threadUnreadCount(t, state.unreadCounts),
    0,
  );
  const isDark = (state.settings?.theme_mode ?? "dark") !== "light";

  const toggleTheme = () => {
    if (!state.settings) return;
    const next = {
      ...state.settings,
      theme_mode: isDark ? ("light" as const) : ("dark" as const),
    };
    dispatch({ type: "UPDATE_SETTINGS_LOCAL", settings: next });
    void ipc.saveSettings(next);
  };

  const openCloudThread = () => {
    const cloud = state.threads.find((t) => !t.is_group && t.name.includes("Cloud của tôi"));
    if (cloud) {
      dispatch({ type: "SELECT_THREAD", threadId: cloud.id });
    } else {
      dispatch({
        type: "SHOW_TOAST",
        message: "Chưa tìm thấy hội thoại Cloud của tôi",
      });
    }
  };

  return (
    <aside
      aria-label="Thanh điều hướng chính"
      className="flex h-full w-15 shrink-0 flex-col items-center justify-between border-r border-border bg-rail py-3.5 text-rail-foreground"
    >
      <div className="flex flex-col items-center gap-2">
        <Tooltip>
          <TooltipTrigger asChild>
            <button
              type="button"
              onClick={() => dispatch({ type: "OPEN_SETTINGS", open: true, tab: "account" })}
              className="relative mb-1 cursor-pointer rounded-full ring-2 ring-white/25 transition hover:ring-white/60"
            >
              <Avatar
                id={state.uin || "self"}
                name={state.displayName || "Me"}
                avatarUrl={state.avatar}
                size={38}
                mediaCache={state.mediaCache}
              />
              <span className="absolute right-0 bottom-0 h-2.5 w-2.5 rounded-full border-2 border-rail bg-online" />
            </button>
          </TooltipTrigger>
          <TooltipContent side="right">
            {state.displayName || "Tài khoản"} — Cài đặt tài khoản
          </TooltipContent>
        </Tooltip>

        <RailButton
          active={state.activeSection === "chat"}
          label="Tin nhắn"
          onClick={() => dispatch({ type: "SET_NAV_SECTION", section: "chat" })}
        >
          <MessageSquare className="h-5 w-5" />
          {totalUnread > 0 ? (
            <span className="absolute -top-0.5 -right-0.5 flex h-4.5 min-w-4.5 items-center justify-center rounded-full bg-destructive px-1 text-[10.5px] font-bold text-destructive-foreground">
              {totalUnread > 99 ? "99+" : totalUnread}
            </span>
          ) : null}
        </RailButton>

        <RailButton
          active={state.activeSection === "contacts"}
          label="Danh bạ"
          onClick={() => dispatch({ type: "SET_NAV_SECTION", section: "contacts" })}
        >
          <Users className="h-5 w-5" />
        </RailButton>

        <RailButton label="Cloud của tôi" onClick={openCloudThread}>
          <Cloud className="h-5 w-5" />
        </RailButton>

        <RailButton
          label="Đồng bộ tin nhắn từ điện thoại"
          onClick={() => dispatch({ type: "SET_PHONE_SYNC_MODAL", open: true })}
        >
          <Smartphone className="h-5 w-5" />
        </RailButton>
      </div>

      <div className="flex flex-col items-center gap-2">
        <RailButton
          label={isDark ? "Chuyển sang giao diện Sáng" : "Chuyển sang giao diện Tối"}
          onClick={toggleTheme}
        >
          {isDark ? <Sun className="h-[19px] w-[19px]" /> : <Moon className="h-[19px] w-[19px]" />}
        </RailButton>

        <RailButton
          label="Kiểm tra nhật ký & giao thức (DevTools)"
          onClick={() => dispatch({ type: "SET_DEV_MODAL", open: true })}
        >
          <Bug className="h-[19px] w-[19px]" />
        </RailButton>

        <RailButton
          label="Cài đặt (Ctrl+Shift+P mở Quyền riêng tư)"
          onClick={() => dispatch({ type: "OPEN_SETTINGS", open: true, tab: "general" })}
        >
          <Settings className="h-5 w-5" />
        </RailButton>
      </div>
    </aside>
  );
}
