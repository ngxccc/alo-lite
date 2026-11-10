import { useState } from "react";
import * as DropdownMenuPrimitive from "@radix-ui/react-dropdown-menu";
import {
  Briefcase,
  Bug,
  ChevronRight,
  CircleHelp,
  Cloud,
  Database,
  FolderOpen,
  Globe,
  LogOut,
  Megaphone,
  MessageSquare,
  Moon,
  Package,
  Settings,
  Smartphone,
  Star,
  Sun,
  User,
  Users,
  Zap,
} from "lucide-react";
import { ipc } from "@/shared/api/ipc";
import { Avatar } from "@/shared/ui/avatar";
import { Popover, PopoverContent, PopoverTrigger } from "@/shared/ui/popover";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/shared/ui/tooltip";
import { cn } from "@/shared/utils/cn";
import type { AppAction, AppState } from "@/app/store/app-reducer";
import type { AppSettings } from "@/shared/types/ipc";
import { threadUnreadCount } from "@/features/threads/utils/threads";

interface NavRailProps {
  state: AppState;
  dispatch: (action: AppAction) => void;
}

// ─── Shared rail button ───────────────────────────────────────────────────────

function RailBtn({
  active = false,
  isDark,
  label,
  onClick,
  children,
  badge,
}: {
  active?: boolean;
  isDark: boolean;
  label: string;
  onClick?: () => void;
  children: React.ReactNode;
  badge?: number;
}) {
  return (
    <Tooltip>
      <TooltipTrigger asChild>
        <button
          type="button"
          onClick={onClick}
          aria-label={label}
          className={cn(
            "relative flex h-12 w-12 cursor-pointer items-center justify-center rounded-xl transition-all duration-150 select-none",
            active
              ? isDark
                ? "bg-white/[0.15] text-white"
                : "bg-black/[0.25] text-white"
              : "text-white/70 hover:bg-white/[0.1] hover:text-white",
          )}
        >
          {children}
          {badge != null && badge > 0 && (
            <span className="absolute -top-0.5 -right-0.5 flex h-4 min-w-[16px] items-center justify-center rounded-full bg-destructive px-[3px] text-[11px] leading-none font-bold text-white">
              {badge > 99 ? "99+" : badge}
            </span>
          )}
        </button>
      </TooltipTrigger>
      <TooltipContent side="right">{label}</TooltipContent>
    </Tooltip>
  );
}

// ─── Shared Radix DropdownMenu styling ───────────────────────────────────────

const dmContentCls =
  "z-50 min-w-[196px] overflow-hidden rounded-lg border border-border bg-popover p-1 text-popover-foreground shadow-xl";

const dmItemCls =
  "relative flex w-full cursor-pointer select-none items-center gap-2.5 rounded-md px-3 py-2 text-sm text-popover-foreground outline-none transition-colors hover:bg-white/[0.05] focus:bg-white/[0.05] data-[disabled]:pointer-events-none data-[disabled]:opacity-40";

const dmSubContentCls =
  "z-50 min-w-[176px] overflow-hidden rounded-lg border border-border bg-popover p-1 text-popover-foreground shadow-xl";

// ─── Settings popup menu ──────────────────────────────────────────────────────

function SettingsMenu({
  state,
  dispatch,
  isDark,
}: {
  state: AppState;
  dispatch: (action: AppAction) => void;
  isDark: boolean;
}) {
  const [open, setOpen] = useState(false);

  const toggleTheme = () => {
    if (!state.settings) return;
    const next: AppSettings = {
      ...state.settings,
      theme_mode: isDark ? "light" : "dark",
    };
    dispatch({ type: "UPDATE_SETTINGS_LOCAL", settings: next });
    void ipc.saveSettings(next);
  };

  return (
    <DropdownMenuPrimitive.Root open={open} onOpenChange={setOpen}>
      <Tooltip>
        <TooltipTrigger asChild>
          <DropdownMenuPrimitive.Trigger asChild>
            <button
              type="button"
              aria-label="Cài đặt"
              className={cn(
                "relative flex h-12 w-12 cursor-pointer items-center justify-center rounded-xl transition-all duration-150 select-none focus-visible:outline-none",
                open
                  ? "bg-white/[0.15] text-white"
                  : "text-white/70 hover:bg-white/[0.1] hover:text-white",
              )}
            >
              <Settings className="h-6 w-6" />
            </button>
          </DropdownMenuPrimitive.Trigger>
        </TooltipTrigger>
        <TooltipContent side="right">Cài đặt</TooltipContent>
      </Tooltip>
      <DropdownMenuPrimitive.Portal>
        <DropdownMenuPrimitive.Content
          side="right"
          align="end"
          sideOffset={8}
          className={dmContentCls}
        >
          {/* Account information */}
          <DropdownMenuPrimitive.Item
            className={dmItemCls}
            onSelect={() => dispatch({ type: "OPEN_SETTINGS", open: true, tab: "account" })}
          >
            <User className="h-4 w-4 shrink-0 opacity-70" />
            Thông tin tài khoản
          </DropdownMenuPrimitive.Item>

          {/* General settings */}
          <DropdownMenuPrimitive.Item
            className={dmItemCls}
            onSelect={() => dispatch({ type: "OPEN_SETTINGS", open: true, tab: "general" })}
          >
            <Settings className="h-4 w-4 shrink-0 opacity-70" />
            Cài đặt
          </DropdownMenuPrimitive.Item>
          <DropdownMenuPrimitive.Separator className="-mx-1 my-1 h-px bg-border" />

          {/* Data sub-menu */}
          <DropdownMenuPrimitive.Sub>
            <DropdownMenuPrimitive.SubTrigger className={cn(dmItemCls, "justify-between")}>
              <span className="flex items-center gap-2.5">
                <Database className="h-4 w-4 shrink-0 opacity-70" />
                Dữ liệu
              </span>
              <ChevronRight className="ml-auto h-3.5 w-3.5 shrink-0 opacity-50" />
            </DropdownMenuPrimitive.SubTrigger>
            <DropdownMenuPrimitive.Portal>
              <DropdownMenuPrimitive.SubContent sideOffset={4} className={dmSubContentCls}>
                <DropdownMenuPrimitive.Item
                  className={dmItemCls}
                  onSelect={() => dispatch({ type: "START_CLOUD_SYNC" })}
                >
                  <Cloud className="h-4 w-4 shrink-0 opacity-60" />
                  Đồng bộ zCloud
                </DropdownMenuPrimitive.Item>
                <DropdownMenuPrimitive.Item
                  className={dmItemCls}
                  onSelect={() => dispatch({ type: "SET_PHONE_SYNC_MODAL", open: true })}
                >
                  <Smartphone className="h-4 w-4 shrink-0 opacity-60" />
                  Đồng bộ tin nhắn từ điện thoại
                </DropdownMenuPrimitive.Item>
              </DropdownMenuPrimitive.SubContent>
            </DropdownMenuPrimitive.Portal>
          </DropdownMenuPrimitive.Sub>

          {/* Language sub-menu */}
          <DropdownMenuPrimitive.Sub>
            <DropdownMenuPrimitive.SubTrigger className={cn(dmItemCls, "justify-between")}>
              <span className="flex items-center gap-2.5">
                <Globe className="h-4 w-4 shrink-0 opacity-70" />
                Ngôn ngữ
              </span>
              <ChevronRight className="ml-auto h-3.5 w-3.5 shrink-0 opacity-50" />
            </DropdownMenuPrimitive.SubTrigger>
            <DropdownMenuPrimitive.Portal>
              <DropdownMenuPrimitive.SubContent sideOffset={4} className={dmSubContentCls}>
                <DropdownMenuPrimitive.Item className={dmItemCls} disabled>
                  Tiếng Việt (mặc định)
                </DropdownMenuPrimitive.Item>
              </DropdownMenuPrimitive.SubContent>
            </DropdownMenuPrimitive.Portal>
          </DropdownMenuPrimitive.Sub>

          {/* Help sub-menu */}
          <DropdownMenuPrimitive.Sub>
            <DropdownMenuPrimitive.SubTrigger className={cn(dmItemCls, "justify-between")}>
              <span className="flex items-center gap-2.5">
                <CircleHelp className="h-4 w-4 shrink-0 opacity-70" />
                Hỗ trợ
              </span>
              <ChevronRight className="ml-auto h-3.5 w-3.5 shrink-0 opacity-50" />
            </DropdownMenuPrimitive.SubTrigger>
            <DropdownMenuPrimitive.Portal>
              <DropdownMenuPrimitive.SubContent sideOffset={4} className={dmSubContentCls}>
                <DropdownMenuPrimitive.Item className={dmItemCls} disabled>
                  Trung tâm hỗ trợ
                </DropdownMenuPrimitive.Item>
              </DropdownMenuPrimitive.SubContent>
            </DropdownMenuPrimitive.Portal>
          </DropdownMenuPrimitive.Sub>
          <DropdownMenuPrimitive.Separator className="-mx-1 my-1 h-px bg-border" />

          {/* Theme toggle */}
          <DropdownMenuPrimitive.Item className={dmItemCls} onSelect={toggleTheme}>
            {isDark ? (
              <Sun className="h-4 w-4 shrink-0 opacity-60" />
            ) : (
              <Moon className="h-4 w-4 shrink-0 opacity-60" />
            )}
            {isDark ? "Giao diện sáng" : "Giao diện tối"}
          </DropdownMenuPrimitive.Item>

          {/* Dev tools – only in devMode */}
          {state.devMode && (
            <DropdownMenuPrimitive.Item
              className={dmItemCls}
              onSelect={() => dispatch({ type: "SET_DEV_MODAL", open: true })}
            >
              <Bug className="h-4 w-4 shrink-0 opacity-60" />
              Công cụ nhà phát triển
            </DropdownMenuPrimitive.Item>
          )}

          <DropdownMenuPrimitive.Separator className="-mx-1 my-1 h-px bg-border" />

          {/* Logout */}
          <DropdownMenuPrimitive.Item
            className={cn(
              dmItemCls,
              "text-destructive hover:bg-destructive/10 focus:bg-destructive/10 focus:text-destructive",
            )}
            onSelect={() => void ipc.logout()}
          >
            <LogOut className="h-4 w-4 shrink-0 opacity-80" />
            Đăng xuất
          </DropdownMenuPrimitive.Item>
        </DropdownMenuPrimitive.Content>
      </DropdownMenuPrimitive.Portal>
    </DropdownMenuPrimitive.Root>
  );
}

// ─── zBusiness Tools popover ──────────────────────────────────────────────────

interface ToolItemDef {
  icon: React.ComponentType<{ className?: string }>;
  iconBg: string;
  label: string;
  badge?: "BASIC" | "PRO";
  disabled?: boolean;
  onSelect?: () => void;
}

function ToolItemBtn({ item }: { item: ToolItemDef }) {
  const Icon = item.icon;
  return (
    <button
      type="button"
      disabled={item.disabled}
      onClick={item.onSelect}
      className={cn(
        "flex flex-col items-center gap-1.5 rounded-lg p-2.5 text-center transition-colors",
        item.disabled ? "cursor-not-allowed opacity-50" : "cursor-pointer hover:bg-white/[0.05]",
      )}
    >
      <div
        className={cn(
          "relative flex h-10 w-10 items-center justify-center rounded-full",
          item.iconBg,
        )}
      >
        <Icon className="h-5 w-5 text-white" />
        {item.badge && (
          <span
            className={cn(
              "absolute -top-1 -right-1 rounded-full px-[5px] text-[9px] leading-[14px] font-bold text-white",
              item.badge === "BASIC" ? "bg-sky-500" : "bg-orange-500",
            )}
          >
            {item.badge}
          </span>
        )}
      </div>
      <span className="w-full text-[11px] leading-tight text-popover-foreground">{item.label}</span>
    </button>
  );
}

function ToolsPopover({ dispatch }: { dispatch: (action: AppAction) => void; isDark: boolean }) {
  const [open, setOpen] = useState(false);

  const tools: ToolItemDef[] = [
    {
      icon: Zap,
      iconBg: "bg-orange-500",
      label: "Tin nhắn nhanh",
      onSelect: () => {
        setOpen(false);
        dispatch({ type: "OPEN_SETTINGS", open: true, tab: "messages" });
      },
    },
    {
      icon: Package,
      iconBg: "bg-blue-500",
      label: "Danh mục sản phẩm",
      badge: "BASIC",
      disabled: true,
    },
    {
      icon: Zap,
      iconBg: "bg-purple-500",
      label: "Tin nhắn tự động",
      badge: "BASIC",
      disabled: true,
    },
    {
      icon: Star,
      iconBg: "bg-yellow-500",
      label: "Tin nhắn gắn sao",
      onSelect: () => {
        setOpen(false);
        dispatch({ type: "SET_NAV_SECTION", section: "chat" });
        dispatch({ type: "TOGGLE_STARRED_FILTER" });
      },
    },
    {
      icon: Megaphone,
      iconBg: "bg-sky-600",
      label: "Tin nhắn hàng loạt",
      badge: "PRO",
      disabled: true,
    },
  ];

  return (
    <Popover open={open} onOpenChange={setOpen}>
      <Tooltip>
        <TooltipTrigger asChild>
          <PopoverTrigger asChild>
            <button
              type="button"
              aria-label="Công cụ zBusiness"
              className={cn(
                "relative flex h-12 w-12 cursor-pointer items-center justify-center rounded-xl transition-all duration-150 select-none",
                open
                  ? "bg-white/[0.15] text-white"
                  : "text-white/70 hover:bg-white/[0.1] hover:text-white",
              )}
            >
              <Briefcase className="h-6 w-6" />
            </button>
          </PopoverTrigger>
        </TooltipTrigger>
        <TooltipContent side="right">Công cụ zBusiness</TooltipContent>
      </Tooltip>
      <PopoverContent side="right" align="end" sideOffset={8} className="w-[272px] p-0">
        {/* Popover header */}
        <div className="border-b border-border px-3 py-2.5">
          <span className="text-[13px] font-semibold text-popover-foreground">
            Công cụ zBusiness
          </span>
        </div>
        {/* 3-column tool grid */}
        <div className="grid grid-cols-3 gap-0.5 p-2">
          {tools.map((item) => (
            <ToolItemBtn key={item.label} item={item} />
          ))}
        </div>
      </PopoverContent>
    </Popover>
  );
}

// ─── Main NavRail ─────────────────────────────────────────────────────────────

export function NavRail({ state, dispatch }: NavRailProps) {
  const totalUnread = state.threads.reduce(
    (sum, t) => sum + threadUnreadCount(t, state.unreadCounts),
    0,
  );
  const isDark = (state.settings?.theme_mode ?? "dark") !== "light";

  const openCloudThread = () => {
    const cloud = state.threads.find((t) => !t.is_group && t.name.includes("Cloud của tôi"));
    if (cloud) {
      dispatch({ type: "SELECT_THREAD", threadId: cloud.id });
    } else {
      dispatch({ type: "SHOW_TOAST", message: "Chưa tìm thấy hội thoại Cloud của tôi" });
    }
  };

  return (
    <aside
      aria-label="Thanh điều hướng chính"
      className="flex h-full w-16 shrink-0 flex-col items-center justify-between border-r border-border bg-rail py-3 text-rail-foreground"
    >
      {/* ── Top group: avatar + main navigation tabs ── */}
      <div className="flex flex-col items-center gap-1">
        {/* User avatar – opens account settings */}
        <Tooltip>
          <TooltipTrigger asChild>
            <button
              type="button"
              onClick={() => dispatch({ type: "OPEN_SETTINGS", open: true, tab: "account" })}
              className="relative mb-2 cursor-pointer rounded-full ring-2 ring-white/25 transition hover:ring-white/50 focus-visible:outline-none"
            >
              <Avatar
                id={state.uin || "self"}
                name={state.displayName || "Me"}
                avatarUrl={state.avatar}
                size={40}
                mediaCache={state.mediaCache}
              />
              {/* Online status dot */}
              <span className="absolute right-0 bottom-0 h-2.5 w-2.5 rounded-full border-2 border-rail bg-online" />
            </button>
          </TooltipTrigger>
          <TooltipContent side="right">{state.displayName || "Tài khoản"}</TooltipContent>
        </Tooltip>

        {/* Messages tab */}
        <RailBtn
          active={state.activeSection === "chat"}
          isDark={isDark}
          label="Tin nhắn"
          onClick={() => dispatch({ type: "SET_NAV_SECTION", section: "chat" })}
          badge={totalUnread}
        >
          <MessageSquare
            className="h-6 w-6"
            strokeWidth={state.activeSection === "chat" ? 2.5 : 1.75}
          />
        </RailBtn>

        {/* Contacts tab */}
        <RailBtn
          active={state.activeSection === "contacts"}
          isDark={isDark}
          label="Danh bạ"
          onClick={() => dispatch({ type: "SET_NAV_SECTION", section: "contacts" })}
        >
          <Users
            className="h-6 w-6"
            strokeWidth={state.activeSection === "contacts" ? 2.5 : 1.75}
          />
        </RailBtn>
      </div>

      {/* ── Bottom group: utility / settings buttons ── */}
      <div className="flex flex-col items-center gap-1 pb-0.5">
        {/* zCloud – trigger cloud sync */}
        <RailBtn
          isDark={isDark}
          label="zCloud"
          onClick={() => dispatch({ type: "START_CLOUD_SYNC" })}
        >
          <Cloud className="h-6 w-6" />
        </RailBtn>

        {/* My Documents – opens Cloud của tôi thread */}
        <RailBtn isDark={isDark} label="Cloud của tôi" onClick={openCloudThread}>
          <FolderOpen className="h-6 w-6" />
        </RailBtn>

        {/* zBusiness Tools popover */}
        <ToolsPopover dispatch={dispatch} isDark={isDark} />

        {/* Settings dropdown */}
        <SettingsMenu state={state} dispatch={dispatch} isDark={isDark} />
      </div>
    </aside>
  );
}
