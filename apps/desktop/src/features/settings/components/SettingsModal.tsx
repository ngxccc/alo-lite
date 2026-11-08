import type React from "react";
import { useEffect } from "react";
import {
  Bell,
  Cloud,
  Lock,
  MessageSquare,
  Palette,
  Settings,
  Sparkles,
  User,
  Wrench,
  X,
} from "lucide-react";
import { ipc, isTauriRuntime } from "@/shared/api/ipc";
import type { AppSettings } from "@/shared/types/ipc";
import { Dialog, DialogContent, DialogTitle } from "@/shared/ui/dialog";
import { cn } from "@/shared/utils/cn";
import type { SettingsTab } from "@/app/store/appReducer";
import {
  AccountTab,
  AppearanceTab,
  GeneralTab,
  MessagesTab,
  NotificationsTab,
  PrivacyTab,
  SyncTab,
  UpdateTab,
  UtilitiesTab,
} from "./tabs";
import type { SettingsModalProps, SettingsTabProps } from "./types";

export type { SettingsModalProps };

const SETTINGS_TABS: {
  id: SettingsTab;
  label: string;
  icon: typeof Settings;
}[] = [
  { id: "general", label: "Cài đặt chung", icon: Settings },
  { id: "account", label: "Tài khoản", icon: User },
  { id: "privacy", label: "Quyền riêng tư", icon: Lock },
  { id: "sync", label: "Đồng bộ tin nhắn", icon: Cloud },
  { id: "appearance", label: "Giao diện", icon: Palette },
  { id: "notifications", label: "Thông báo", icon: Bell },
  { id: "messages", label: "Tin nhắn", icon: MessageSquare },
  { id: "utilities", label: "Tiện ích", icon: Wrench },
  { id: "update", label: "Cập nhật", icon: Sparkles },
];

const TAB_COMPONENTS: Record<SettingsTab, React.ComponentType<SettingsTabProps>> = {
  general: GeneralTab,
  account: AccountTab,
  privacy: PrivacyTab,
  sync: SyncTab,
  appearance: AppearanceTab,
  notifications: NotificationsTab,
  messages: MessagesTab,
  utilities: UtilitiesTab,
  update: UpdateTab,
};

export function SettingsModal({ state, dispatch }: SettingsModalProps) {
  useEffect(() => {
    if (!state.settingsModalOpen || !isTauriRuntime()) return;
    if (state.settingsTab === "account") {
      void ipc
        .listSavedAccounts()
        .then((accounts) => dispatch({ type: "SET_SAVED_ACCOUNTS", accounts }));
    }
  }, [state.settingsModalOpen, state.settingsTab, dispatch]);

  if (!state.settingsModalOpen || !state.settings) return null;

  const settings = state.settings;
  const tab = state.settingsTab;

  const updateSettings = (patch: Partial<AppSettings>) => {
    const next: AppSettings = { ...settings, ...patch };
    dispatch({ type: "UPDATE_SETTINGS_LOCAL", settings: next });
    void ipc.saveSettings(next);
  };

  const ActiveTabContent = TAB_COMPONENTS[tab];

  return (
    <Dialog
      open={state.settingsModalOpen}
      onOpenChange={(open) => !open && dispatch({ type: "OPEN_SETTINGS", open: false })}
    >
      {/* 653x652 dialog, radius-lg (8px), no default padding */}
      <DialogContent
        className="flex h-[652px] w-[653px] max-w-[653px] flex-row overflow-hidden rounded-lg p-0"
        hideClose
      >
        {/* ── Left nav 186px ── */}
        <nav className="flex w-[186px] shrink-0 flex-col gap-0.5 overflow-y-auto bg-card px-2 py-4">
          <DialogTitle className="mb-2 px-2 text-lg font-medium text-foreground">
            Cài đặt
          </DialogTitle>

          {SETTINGS_TABS.map((item) => {
            const Icon = item.icon;
            const active = tab === item.id;
            return (
              <button
                key={item.id}
                type="button"
                onClick={() => dispatch({ type: "OPEN_SETTINGS", open: true, tab: item.id })}
                className={cn(
                  "flex h-9 cursor-pointer items-center gap-2.5 rounded-md px-2.5 text-left text-sm font-normal transition-colors",
                  active ? "bg-accent text-foreground" : "text-foreground hover:bg-white/[0.05]",
                )}
              >
                <Icon className="h-[18px] w-[18px] shrink-0 text-muted-foreground" />
                <span className="truncate">{item.label}</span>
              </button>
            );
          })}
        </nav>

        {/* ── Right body bg-canvas ── */}
        <div className="relative flex flex-1 flex-col overflow-hidden bg-canvas">
          {/* Close X */}
          <button
            type="button"
            onClick={() => dispatch({ type: "OPEN_SETTINGS", open: false })}
            className="absolute top-3 right-3 z-10 inline-flex h-6 w-6 cursor-pointer items-center justify-center rounded-md text-muted-foreground hover:bg-white/[0.08] hover:text-foreground"
          >
            <X className="h-[18px] w-[18px]" />
          </button>

          <div className="flex-1 overflow-y-auto px-6 py-4">
            <ActiveTabContent
              state={state}
              settings={settings}
              dispatch={dispatch}
              updateSettings={updateSettings}
            />
          </div>
        </div>
      </DialogContent>
    </Dialog>
  );
}
