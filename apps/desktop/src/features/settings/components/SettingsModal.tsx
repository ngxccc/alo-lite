import { useEffect, useState } from "react";
import {
  Bell,
  ChevronRight,
  Cloud,
  Download,
  FolderOpen,
  Lock,
  MessageSquare,
  Palette,
  RefreshCw,
  Settings,
  Sparkles,
  Trash2,
  User,
  Wrench,
  X,
} from "lucide-react";
import { ipc, isTauriRuntime } from "@/shared/api/ipc";
import type { AppSettings, AutoReplyRule } from "@/shared/types/ipc";
import { Avatar } from "@/shared/ui/Avatar";
import { Badge } from "@/shared/ui/badge";
import { Button } from "@/shared/ui/button";
import { Dialog, DialogContent, DialogTitle } from "@/shared/ui/dialog";
import { Input } from "@/shared/ui/input";
import { Switch } from "@/shared/ui/switch";
import { cn } from "@/shared/utils/cn";
import type { AppAction, AppState, SettingsTab } from "@/app/store/appReducer";

interface SettingsModalProps {
  state: AppState;
  dispatch: (action: AppAction) => void;
}

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

/** Zalo-style row: label+optional desc on left, toggle on right */
function SwitchRow({
  label,
  desc,
  checked,
  onChange,
}: {
  label: string;
  desc?: string;
  checked: boolean;
  onChange: (v: boolean) => void;
}) {
  return (
    <div className="flex min-h-[44px] items-center justify-between gap-4 px-4 py-2">
      <div className="min-w-0 flex-1">
        <div className="text-sm text-foreground">{label}</div>
        {desc ? <div className="mt-0.5 text-xs text-muted-foreground">{desc}</div> : null}
      </div>
      {/* 40x22 toggle */}
      <Switch
        checked={checked}
        onCheckedChange={onChange}
        className="h-[22px] w-10 data-[state=checked]:bg-primary data-[state=unchecked]:bg-[#3c4149] [&>span]:h-[18px] [&>span]:w-[18px] [&>span]:data-[state=checked]:translate-x-[18px]"
      />
    </div>
  );
}

/** Select row */
function SelectRow({
  label,
  value,
  onChange,
  options,
}: {
  label: string;
  value: string;
  onChange: (v: string) => void;
  options: { value: string; label: string }[];
}) {
  return (
    <div className="flex min-h-[44px] items-center justify-between gap-4 px-4 py-2">
      <span className="text-sm text-foreground">{label}</span>
      <select
        value={value}
        onChange={(e) => onChange(e.target.value)}
        className="h-[38px] min-w-[160px] rounded-[6px] border border-border bg-canvas px-3 text-sm text-foreground focus:outline-none"
      >
        {options.map((o) => (
          <option key={o.value} value={o.value}>
            {o.label}
          </option>
        ))}
      </select>
    </div>
  );
}

/** Chevron-only nav row (block messages, manage quick messages, etc.) */
function ChevronRow({ label, desc }: { label: string; desc?: string }) {
  return (
    <div className="flex min-h-[44px] cursor-pointer items-center justify-between gap-4 px-4 py-2 hover:bg-white/[0.03]">
      <div className="min-w-0 flex-1">
        <div className="text-sm text-foreground">{label}</div>
        {desc ? <div className="mt-0.5 text-xs text-muted-foreground">{desc}</div> : null}
      </div>
      <ChevronRight className="h-4 w-4 shrink-0 text-muted-foreground" />
    </div>
  );
}

/** Section heading (not inside a card) */
function SectionHeading({ children }: { children: React.ReactNode }) {
  return (
    <div className="mt-4 mb-2 text-base font-medium text-foreground first:mt-0">{children}</div>
  );
}

/** Card wrapper */
function SettingCard({ children, className }: { children: React.ReactNode; className?: string }) {
  return <div className={cn("overflow-hidden rounded-lg bg-card", className)}>{children}</div>;
}

/** Divider between rows inside card */
function RowDivider() {
  return <div className="mx-4 h-px bg-border" />;
}

/** CSS mini-mockup for theme picker */
function ThemeThumbnail({ mode }: { mode: "light" | "dark" | "system" }) {
  if (mode === "light") {
    return (
      <div className="relative h-[72px] w-full overflow-hidden rounded bg-[#ebecf0]">
        {/* sidebar strip */}
        <div className="absolute inset-y-0 left-0 w-[30%] bg-white" />
        {/* chat bubble out */}
        <div className="absolute right-3 bottom-3 h-[12px] w-[45%] rounded-sm bg-[#e5f1ff]" />
        {/* chat bubble in */}
        <div className="absolute bottom-7 left-[32%] h-[12px] w-[40%] rounded-sm bg-white" />
        {/* header bar */}
        <div className="absolute inset-x-0 top-0 h-[14px] bg-white" />
      </div>
    );
  }
  if (mode === "dark") {
    return (
      <div className="relative h-[72px] w-full overflow-hidden rounded bg-[#16191d]">
        <div className="absolute inset-y-0 left-0 w-[30%] bg-[#22262b]" />
        <div className="absolute right-3 bottom-3 h-[12px] w-[45%] rounded-sm bg-[#1f344d]" />
        <div className="absolute bottom-7 left-[32%] h-[12px] w-[40%] rounded-sm bg-[#282d33]" />
        <div className="absolute inset-x-0 top-0 h-[14px] bg-[#22262b]" />
      </div>
    );
  }
  // system: half-and-half
  return (
    <div className="relative h-[72px] w-full overflow-hidden rounded">
      {/* left half dark */}
      <div className="absolute inset-y-0 left-0 w-1/2 bg-[#16191d]">
        <div className="absolute inset-y-0 left-0 w-[35%] bg-[#22262b]" />
        <div className="absolute inset-x-0 top-0 h-[14px] bg-[#22262b]" />
      </div>
      {/* right half light */}
      <div className="absolute inset-y-0 right-0 w-1/2 bg-[#ebecf0]">
        <div className="absolute inset-x-0 top-0 h-[14px] bg-white" />
        <div className="absolute right-2 bottom-3 h-[10px] w-[55%] rounded-sm bg-[#e5f1ff]" />
      </div>
    </div>
  );
}

/** CSS mini-mockup for notification toggle (laptop outline) */
function NotifThumbnail({ active }: { active: boolean }) {
  return (
    <div
      className={cn(
        "relative mx-auto flex h-[60px] w-[80px] flex-col items-center justify-center rounded border-2",
        active ? "border-primary" : "border-border",
      )}
    >
      {/* laptop screen */}
      <div
        className={cn(
          "h-[32px] w-[56px] rounded-t border-2",
          active ? "border-primary" : "border-muted-foreground",
        )}
      >
        {active && <div className="mt-1 ml-1 h-2 w-6 rounded-sm bg-primary/40" />}
      </div>
      {/* laptop base */}
      <div
        className={cn(
          "h-[6px] w-[64px] rounded-b border-x-2 border-b-2",
          active ? "border-primary" : "border-muted-foreground",
        )}
      />
    </div>
  );
}

export function SettingsModal({ state, dispatch }: SettingsModalProps) {
  const [migratePath, setMigratePath] = useState("");
  const [autoReplyRules, setAutoReplyRules] = useState<AutoReplyRule[]>([]);
  const [newRuleKw, setNewRuleKw] = useState("");
  const [newRuleReply, setNewRuleReply] = useState("");

  useEffect(() => {
    if (!state.settingsModalOpen || !isTauriRuntime()) return;
    if (state.settingsTab === "account") {
      void ipc
        .listSavedAccounts()
        .then((accounts) => dispatch({ type: "SET_SAVED_ACCOUNTS", accounts }));
    } else if (state.settingsTab === "utilities") {
      void ipc
        .getAutoreplyRules()
        .then((r) => setAutoReplyRules(Array.isArray(r) ? r : []))
        .catch(() => undefined);
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
            {/* ── General Settings ── */}
            {tab === "general" ? (
              <>
                <SectionHeading>Ngôn ngữ</SectionHeading>
                <SettingCard>
                  <SelectRow
                    label="Thay đổi ngôn ngữ"
                    value={settings.language}
                    onChange={(v) => updateSettings({ language: v })}
                    options={[
                      { value: "Tiếng Việt", label: "Tiếng Việt" },
                      { value: "English", label: "English" },
                    ]}
                  />
                </SettingCard>

                <SectionHeading>Khởi động</SectionHeading>
                <SettingCard>
                  <SwitchRow
                    label="Khởi động AloLite cùng hệ thống"
                    checked={settings.autostart_on_boot}
                    onChange={(v) => updateSettings({ autostart_on_boot: v })}
                  />
                  <RowDivider />
                  <SwitchRow
                    label="Ghi nhớ tài khoản đăng nhập"
                    checked={settings.remember_login}
                    onChange={(v) => updateSettings({ remember_login: v })}
                  />
                  <RowDivider />
                  <SwitchRow
                    label="Hiển thị tất cả danh bạ"
                    checked={settings.show_all_contacts}
                    onChange={(v) => updateSettings({ show_all_contacts: v })}
                  />
                </SettingCard>

                <SectionHeading>Thư mục tải xuống</SectionHeading>
                <SettingCard>
                  <div className="px-4 py-3">
                    <div className="mb-2 text-xs text-muted-foreground">
                      Lưu tại:{" "}
                      <code className="rounded bg-muted px-1.5 py-0.5 text-foreground">
                        {settings.download_dir}
                      </code>
                    </div>
                    <div className="flex gap-2">
                      <Button
                        variant="outline"
                        size="sm"
                        onClick={async () => {
                          const dir = await ipc.pickFile({
                            directory: true,
                            title: "Chọn thư mục lưu file",
                          });
                          if (dir) updateSettings({ download_dir: dir });
                        }}
                      >
                        <FolderOpen className="h-3.5 w-3.5" /> Thay đổi thư mục
                      </Button>
                      <Button
                        variant="outline"
                        size="sm"
                        onClick={() => void ipc.openPathInOs(settings.download_dir)}
                      >
                        Mở thư mục
                      </Button>
                    </div>
                  </div>
                </SettingCard>
              </>
            ) : null}

            {/* ── Account ── */}
            {tab === "account" ? (
              <>
                <SectionHeading>Tài khoản hiện tại</SectionHeading>
                <SettingCard>
                  <div className="flex items-center justify-between px-4 py-3">
                    <div className="flex items-center gap-3">
                      <Avatar
                        id={state.uin || "self"}
                        name={state.displayName || "Me"}
                        avatarUrl={state.avatar}
                        size={44}
                        mediaCache={state.mediaCache}
                      />
                      <div>
                        <div className="text-sm font-medium text-foreground">
                          {state.displayName || "Người dùng Zalo"}
                        </div>
                        <div className="text-xs text-muted-foreground">
                          {state.uin ? `UIN: ${state.uin}` : "Đang hoạt động"}
                        </div>
                      </div>
                    </div>
                    <Button variant="destructive" size="sm" onClick={() => void ipc.logout()}>
                      Đăng xuất
                    </Button>
                  </div>
                </SettingCard>

                <SectionHeading>Đa tài khoản</SectionHeading>
                <SettingCard>
                  <div className="flex items-center justify-between px-4 py-3">
                    <span className="text-sm text-foreground">Tài khoản đã lưu</span>
                    <Button
                      variant="default"
                      size="sm"
                      onClick={() => {
                        dispatch({ type: "OPEN_SETTINGS", open: false });
                        void ipc.addNewAccount();
                      }}
                    >
                      + Thêm tài khoản (QR)
                    </Button>
                  </div>
                  {state.savedAccounts.length === 0 ? (
                    <>
                      <RowDivider />
                      <div className="px-4 py-3 text-xs text-muted-foreground">
                        Chưa có tài khoản phụ nào được lưu trong thư mục accounts/.
                      </div>
                    </>
                  ) : (
                    state.savedAccounts.map((acc) => (
                      <div key={acc.uin}>
                        <RowDivider />
                        <div className="flex items-center justify-between px-4 py-2.5">
                          <div className="flex items-center gap-2.5">
                            <Avatar
                              id={acc.uin}
                              name={acc.display_name}
                              avatarUrl={acc.avatar}
                              size={32}
                              mediaCache={state.mediaCache}
                            />
                            <div>
                              <div className="text-sm font-medium text-foreground">
                                {acc.display_name}
                              </div>
                              <div className="text-xs text-muted-foreground">UIN: {acc.uin}</div>
                            </div>
                          </div>
                          <div className="flex items-center gap-1.5">
                            {acc.is_current ? (
                              <Badge variant="online">Đang dùng</Badge>
                            ) : (
                              <>
                                <Button
                                  variant="default"
                                  size="sm"
                                  onClick={() => void ipc.switchAccount(acc.uin)}
                                >
                                  Chuyển đổi
                                </Button>
                                <Button
                                  variant="destructive"
                                  size="icon-sm"
                                  onClick={async () => {
                                    await ipc.deleteSavedAccount(acc.uin);
                                    const next = await ipc.listSavedAccounts();
                                    dispatch({ type: "SET_SAVED_ACCOUNTS", accounts: next });
                                  }}
                                >
                                  <Trash2 className="h-3.5 w-3.5" />
                                </Button>
                              </>
                            )}
                          </div>
                        </div>
                      </div>
                    ))
                  )}
                </SettingCard>
              </>
            ) : null}

            {/* ── Privacy ── */}
            {tab === "privacy" ? (
              <>
                <SectionHeading>Thông tin cá nhân</SectionHeading>
                <SettingCard>
                  <SelectRow
                    label="Hiển thị ngày sinh"
                    value={settings.show_birthday}
                    onChange={(v) => updateSettings({ show_birthday: v })}
                    options={[
                      { value: "hide", label: "Không hiển thị" },
                      { value: "friends", label: "Bạn bè" },
                      { value: "everyone", label: "Tất cả mọi người" },
                    ]}
                  />
                  <RowDivider />
                  <SwitchRow
                    label="Hiển thị trạng thái truy cập"
                    checked={settings.show_online_status}
                    onChange={(v) => updateSettings({ show_online_status: v })}
                  />
                </SettingCard>

                <SectionHeading>Tin nhắn và cuộc gọi</SectionHeading>
                <SettingCard>
                  <SwitchRow
                    label='Hiển thị trạng thái "Đã xem"'
                    checked={settings.show_seen_status}
                    onChange={(v) => updateSettings({ show_seen_status: v, block_seen: !v })}
                  />
                  <RowDivider />
                  <SelectRow
                    label="Cho phép nhắn tin"
                    value={settings.allow_message_from}
                    onChange={(v) => updateSettings({ allow_message_from: v })}
                    options={[
                      { value: "everyone", label: "Tất cả mọi người" },
                      { value: "friends", label: "Bạn bè" },
                      { value: "none", label: "Không ai" },
                    ]}
                  />
                  <RowDivider />
                  <SelectRow
                    label="Cho phép gọi điện"
                    value={settings.allow_call_from}
                    onChange={(v) => updateSettings({ allow_call_from: v })}
                    options={[
                      { value: "everyone", label: "Tất cả mọi người" },
                      { value: "friends_strangers", label: "Bạn bè và người lạ bạn biết" },
                      { value: "friends", label: "Bạn bè" },
                    ]}
                  />
                  <RowDivider />
                  <SwitchRow
                    label='Ẩn trạng thái "Đang soạn tin"'
                    desc="Chặn gửi gói typing đến người khác."
                    checked={settings.block_typing}
                    onChange={(v) => updateSettings({ block_typing: v })}
                  />
                  <RowDivider />
                  <SwitchRow
                    label='Ẩn trạng thái "Đã nhận"'
                    desc="Chặn gửi xác nhận đã nhận tin."
                    checked={settings.block_delivered}
                    onChange={(v) => updateSettings({ block_delivered: v })}
                  />
                </SettingCard>

                <SectionHeading>Chặn tin nhắn</SectionHeading>
                <SettingCard>
                  <ChevronRow label="Chặn tin nhắn" desc="Quản lý danh sách chặn" />
                </SettingCard>

                <SectionHeading>Nguồn tìm kiếm</SectionHeading>
                <SettingCard>
                  <SwitchRow
                    label="Cho phép tìm kiếm qua số điện thoại"
                    checked={settings.allow_search_by_phone}
                    onChange={(v) => updateSettings({ allow_search_by_phone: v })}
                  />
                </SettingCard>

                <SectionHeading>Cho phép người lạ kết bạn</SectionHeading>
                <SettingCard>
                  <div className="flex flex-col gap-0">
                    {[
                      { key: "friend_src_qr" as const, label: "Qua mã QR của tôi" },
                      { key: "friend_src_group" as const, label: "Từ nhóm chung" },
                      { key: "friend_src_card" as const, label: "Qua thẻ danh thiếp Zalo" },
                    ].map((item, i) => (
                      <div key={item.key}>
                        {i > 0 && <RowDivider />}
                        <label className="flex min-h-[44px] cursor-pointer items-center gap-3 px-4 py-2">
                          <input
                            type="checkbox"
                            checked={settings[item.key]}
                            onChange={(e) => updateSettings({ [item.key]: e.target.checked })}
                            className="h-4 w-4 rounded border-border accent-primary"
                          />
                          <span className="text-sm text-foreground">{item.label}</span>
                        </label>
                      </div>
                    ))}
                  </div>
                </SettingCard>
              </>
            ) : null}

            {/* ── Sync messages ── */}
            {tab === "sync" ? (
              <>
                {/* Illustration card matching set_sync.png */}
                <div className="mb-4 flex flex-col items-center justify-center rounded-lg bg-card py-8">
                  <div className="mb-3 flex items-center gap-3">
                    {/* phone icon */}
                    <div className="flex h-10 w-10 items-center justify-center rounded-full border-2 border-border">
                      <svg
                        width="20"
                        height="20"
                        viewBox="0 0 24 24"
                        fill="none"
                        stroke="currentColor"
                        strokeWidth="1.5"
                        className="text-primary"
                      >
                        <rect x="5" y="2" width="14" height="20" rx="2" />
                        <line
                          x1="12"
                          y1="18"
                          x2="12"
                          y2="18.01"
                          strokeWidth="2"
                          strokeLinecap="round"
                        />
                      </svg>
                    </div>
                    <div className="flex gap-1">
                      <div className="h-1.5 w-1.5 rounded-full bg-primary/40" />
                      <div className="h-1.5 w-1.5 rounded-full bg-primary/60" />
                      <div className="h-1.5 w-1.5 rounded-full bg-primary" />
                    </div>
                    {/* PC icon */}
                    <div className="flex h-10 w-10 items-center justify-center rounded-full border-2 border-primary bg-primary/10">
                      <svg
                        width="20"
                        height="20"
                        viewBox="0 0 24 24"
                        fill="none"
                        stroke="currentColor"
                        strokeWidth="1.5"
                        className="text-primary"
                      >
                        <rect x="2" y="3" width="20" height="14" rx="2" />
                        <line
                          x1="8"
                          y1="21"
                          x2="16"
                          y2="21"
                          strokeWidth="2"
                          strokeLinecap="round"
                        />
                        <line
                          x1="12"
                          y1="17"
                          x2="12"
                          y2="21"
                          strokeWidth="2"
                          strokeLinecap="round"
                        />
                      </svg>
                    </div>
                  </div>
                  <div className="text-sm font-medium text-foreground">Đồng bộ tin nhắn</div>
                  <div className="mt-1 text-xs text-muted-foreground">
                    Đồng bộ tin nhắn từ điện thoại sang Zalo PC
                  </div>
                </div>

                <SettingCard>
                  <div className="px-4 py-3">
                    <div className="mb-2 text-xs text-muted-foreground">
                      CSDL đang dùng:{" "}
                      <code className="rounded bg-muted px-1.5 py-0.5 text-foreground">
                        {state.activeDbPath}
                      </code>
                    </div>
                    <div className="flex flex-wrap gap-2">
                      <Button
                        variant="default"
                        size="sm"
                        onClick={() => dispatch({ type: "SET_PHONE_SYNC_MODAL", open: true })}
                      >
                        Đồng bộ từ điện thoại
                      </Button>
                      {state.syncRunning ? (
                        <Button
                          variant="destructive"
                          size="sm"
                          onClick={() => void ipc.cancelCloudSync()}
                        >
                          Dừng đồng bộ Nhóm/Cloud
                        </Button>
                      ) : (
                        <Button
                          variant="outline"
                          size="sm"
                          onClick={() => {
                            const targets = state.threads
                              .filter((t) => t.is_group || t.name.includes("Cloud của tôi"))
                              .map<[string, string, boolean]>((t) => [t.id, t.name, t.is_group]);
                            dispatch({ type: "START_CLOUD_SYNC" });
                            void ipc.syncHistory(targets);
                          }}
                        >
                          <RefreshCw className="h-3.5 w-3.5" /> Đồng bộ Nhóm &amp; Cloud
                        </Button>
                      )}
                    </div>
                  </div>
                </SettingCard>

                <SectionHeading>Nhập lịch sử từ tệp sao lưu</SectionHeading>
                <SettingCard>
                  <div className="px-4 py-3">
                    <div className="mb-2 flex gap-1.5">
                      <Input
                        value={migratePath}
                        onChange={(e) => setMigratePath(e.target.value)}
                        placeholder="Đường dẫn file hoặc thư mục Zalo…"
                      />
                      <Button
                        variant="outline"
                        onClick={async () => {
                          const p = await ipc.pickFile({
                            title: "Chọn file hoặc thư mục dữ liệu Zalo",
                          });
                          if (p) setMigratePath(p);
                        }}
                      >
                        Duyệt…
                      </Button>
                      <Button
                        variant="default"
                        onClick={async () => {
                          if (!migratePath.trim()) return;
                          try {
                            await ipc.importLocalHistory(migratePath.trim());
                          } catch (e) {
                            dispatch({ type: "SHOW_TOAST", message: String(e) });
                          }
                        }}
                      >
                        Nhập ngay
                      </Button>
                    </div>
                    <Button
                      variant="outline"
                      size="sm"
                      className="w-fit"
                      onClick={async () => {
                        try {
                          await ipc.autoDetectImportHistory();
                        } catch (e) {
                          dispatch({ type: "SHOW_TOAST", message: String(e) });
                        }
                      }}
                    >
                      Tự động quét dữ liệu Zalo trên máy
                    </Button>
                  </div>
                </SettingCard>

                <SectionHeading>Xóa dữ liệu cục bộ</SectionHeading>
                <SettingCard>
                  <div className="flex items-center justify-between px-4 py-3">
                    <div>
                      <div className="text-sm font-medium text-destructive">
                        Xóa dữ liệu tin nhắn cục bộ
                      </div>
                      <div className="text-xs text-muted-foreground">
                        Giữ nguyên phiên đăng nhập session.json.
                      </div>
                    </div>
                    <Button
                      variant="destructive"
                      size="sm"
                      onClick={() => dispatch({ type: "SET_WIPE_CONFIRM", open: true })}
                    >
                      <Trash2 className="h-3.5 w-3.5" /> Xóa dữ liệu
                    </Button>
                  </div>
                </SettingCard>
              </>
            ) : null}

            {/* ── Appearance / Theme ── */}
            {tab === "appearance" ? (
              <>
                <SectionHeading>Chủ đề giao diện</SectionHeading>
                <SettingCard>
                  <div className="px-4 py-4">
                    <div className="grid grid-cols-3 gap-3">
                      {(
                        [
                          { id: "light" as const, label: "Sáng" },
                          { id: "dark" as const, label: "Tối" },
                          { id: "system" as const, label: "Hệ thống" },
                        ] as const
                      ).map((opt) => {
                        const selected = settings.theme_mode === opt.id;
                        return (
                          <button
                            key={opt.id}
                            type="button"
                            onClick={() => updateSettings({ theme_mode: opt.id })}
                            className={cn(
                              "flex flex-col gap-2 rounded-lg border-2 p-2 transition-colors",
                              selected
                                ? "border-primary"
                                : "border-border hover:border-muted-foreground",
                            )}
                          >
                            <ThemeThumbnail mode={opt.id} />
                            <div className="flex items-center gap-1.5">
                              <div
                                className={cn(
                                  "h-3.5 w-3.5 rounded-full border-2",
                                  selected
                                    ? "border-primary bg-primary"
                                    : "border-muted-foreground",
                                )}
                              />
                              <span className="text-xs text-foreground">{opt.label}</span>
                            </div>
                          </button>
                        );
                      })}
                    </div>
                  </div>
                </SettingCard>

                <SectionHeading>Nền trò chuyện</SectionHeading>
                <SettingCard>
                  <SwitchRow
                    label="Sử dụng ảnh đại diện làm nền mờ"
                    checked={settings.use_avatar_as_wallpaper}
                    onChange={(v) => updateSettings({ use_avatar_as_wallpaper: v })}
                  />
                </SettingCard>
              </>
            ) : null}

            {/* ── Notifications ── */}
            {tab === "notifications" ? (
              <>
                <SectionHeading>Bật thông báo</SectionHeading>
                <div className="mb-1 text-xs text-muted-foreground">
                  Hiển thị thông báo khi có tin nhắn mới
                </div>
                <SettingCard>
                  <div className="px-4 py-4">
                    <div className="grid grid-cols-2 gap-4">
                      {[
                        { active: true, label: "Bật" },
                        { active: false, label: "Tắt" },
                      ].map((opt) => {
                        const selected = settings.enable_notifications === opt.active;
                        return (
                          <button
                            key={opt.label}
                            type="button"
                            onClick={() => updateSettings({ enable_notifications: opt.active })}
                            className={cn(
                              "flex flex-col items-center gap-3 rounded-lg border-2 py-4 transition-colors",
                              selected
                                ? "border-primary"
                                : "border-border hover:border-muted-foreground",
                            )}
                          >
                            <NotifThumbnail active={opt.active} />
                            <div className="flex items-center gap-1.5">
                              <div
                                className={cn(
                                  "h-3.5 w-3.5 rounded-full border-2",
                                  selected
                                    ? "border-primary bg-primary"
                                    : "border-muted-foreground",
                                )}
                              />
                              <span className="text-xs text-foreground">{opt.label}</span>
                            </div>
                          </button>
                        );
                      })}
                    </div>
                  </div>
                </SettingCard>

                <SectionHeading>Âm thanh thông báo</SectionHeading>
                <SettingCard>
                  <SwitchRow
                    label="Bật âm thanh cho tin nhắn &amp; thông báo mới"
                    checked={settings.play_notification_sound}
                    onChange={(v) => updateSettings({ play_notification_sound: v })}
                  />
                </SettingCard>
              </>
            ) : null}

            {/* ── Messages ── */}
            {tab === "messages" ? (
              <>
                <SectionHeading>Tin nhắn nhanh</SectionHeading>
                <div className="mb-1 text-xs text-muted-foreground">
                  Tạo, chỉnh sửa và quản lý phím tắt cho tin nhắn thường dùng
                </div>
                <SettingCard>
                  <SwitchRow
                    label="Tin nhắn nhanh"
                    checked={settings.quick_messages_enabled}
                    onChange={(v) => updateSettings({ quick_messages_enabled: v })}
                  />
                  <RowDivider />
                  <ChevronRow label="Quản lý tin nhắn nhanh" />
                </SettingCard>

                <SectionHeading>Ẩn trò chuyện</SectionHeading>
                <div className="mb-1 text-xs text-muted-foreground">
                  Đặt mã PIN để ẩn trò chuyện riêng tư
                </div>
                <SettingCard>
                  <div className="flex min-h-[44px] items-center justify-between px-4 py-2">
                    <span className="text-sm text-foreground">PIN</span>
                    <span className="flex items-center gap-1 text-sm text-muted-foreground">
                      {settings.hidden_chat_pin_enabled ? "Đã bật" : "Chưa bật"}
                      <ChevronRight className="h-4 w-4" />
                    </span>
                  </div>
                </SettingCard>

                <SectionHeading>Tách Quan trọng và Khác</SectionHeading>
                <div className="mb-1 text-xs text-muted-foreground">
                  Tách các cuộc trò chuyện phân tâm sang tab Khác để tập trung hơn
                </div>
                <SettingCard>
                  <SwitchRow
                    label="Phân tách hộp thư Ưu tiên / Khác"
                    checked={settings.split_priority_inbox}
                    onChange={(v) => updateSettings({ split_priority_inbox: v })}
                  />
                </SettingCard>

                <SectionHeading>Hiển thị nội dung file Excel</SectionHeading>
                <div className="mb-1 text-xs text-muted-foreground">
                  Sao chép định dạng Excel sau khi dán vào Zalo
                </div>
                <SettingCard>
                  <div className="flex flex-col">
                    {[
                      { value: "photo", label: "Hình ảnh" },
                      { value: "text", label: "Văn bản" },
                      { value: "ask", label: "Luôn hỏi" },
                    ].map((opt, i) => (
                      <div key={opt.value}>
                        {i > 0 && <RowDivider />}
                        <label className="flex min-h-[44px] cursor-pointer items-center gap-3 px-4 py-2">
                          <input
                            type="radio"
                            name="excel_paste_mode"
                            value={opt.value}
                            checked={settings.excel_paste_mode === opt.value}
                            onChange={() => updateSettings({ excel_paste_mode: opt.value })}
                            className="h-4 w-4 accent-primary"
                          />
                          <span className="text-sm text-foreground">{opt.label}</span>
                        </label>
                      </div>
                    ))}
                  </div>
                </SettingCard>

                <SectionHeading>Cài đặt khác</SectionHeading>
                <SettingCard>
                  <SwitchRow
                    label="Nhấp đúp vào tin nhắn để trả lời"
                    checked={settings.double_click_to_reply}
                    onChange={(v) => updateSettings({ double_click_to_reply: v })}
                  />
                  <RowDivider />
                  <SwitchRow
                    label="Hiển thị trạng thái đang soạn tin"
                    checked={settings.show_typing_indicator}
                    onChange={(v) => updateSettings({ show_typing_indicator: v })}
                  />
                  <RowDivider />
                  <SelectRow
                    label="Phím gửi tin nhắn"
                    value={settings.send_message_key}
                    onChange={(v) =>
                      updateSettings({
                        send_message_key: v as "enter" | "cmd_enter",
                      })
                    }
                    options={[
                      { value: "enter", label: "Enter để gửi" },
                      { value: "cmd_enter", label: "Ctrl/Cmd + Enter để gửi" },
                    ]}
                  />
                </SettingCard>
              </>
            ) : null}

            {/* ── Utilities ── */}
            {tab === "utilities" ? (
              <>
                <SectionHeading>Gợi ý nhãn dán</SectionHeading>
                <SettingCard>
                  <SwitchRow
                    label="Hiển thị gợi ý sticker theo từ khóa đang soạn"
                    checked={settings.suggest_stickers_while_typing}
                    onChange={(v) => updateSettings({ suggest_stickers_while_typing: v })}
                  />
                </SettingCard>

                <SectionHeading>Phím tắt @</SectionHeading>
                <SettingCard>
                  {[
                    {
                      key: "suggest_mentions" as const,
                      label: "Gợi ý nhắc tên (@mention)",
                      desc: "Gợi ý mention khi soạn tin",
                      color: "bg-[#e94949]",
                      abbr: "@",
                    },
                    {
                      key: "enable_gif_search" as const,
                      label: "Tìm GIF",
                      desc: "Nhập từ khóa để tìm GIF",
                      color: "bg-[#2d7dd2]",
                      abbr: "GIF",
                    },
                    {
                      key: "enable_sticker_keyword_search" as const,
                      label: "Tìm nhãn dán",
                      desc: "Nhập từ khóa để tìm nhãn dán",
                      color: "bg-[#8b5cf6]",
                      abbr: ":)",
                    },
                  ].map((item, i) => (
                    <div key={item.key}>
                      {i > 0 && <RowDivider />}
                      <div className="flex min-h-[52px] items-center justify-between gap-3 px-4 py-2">
                        <div
                          className={cn(
                            "flex h-8 w-8 shrink-0 items-center justify-center rounded-full text-[10px] font-bold text-white",
                            item.color,
                          )}
                        >
                          {item.abbr}
                        </div>
                        <div className="min-w-0 flex-1">
                          <div className="text-sm text-foreground">{item.label}</div>
                          <div className="text-xs text-muted-foreground">{item.desc}</div>
                        </div>
                        <Switch
                          checked={settings[item.key]}
                          onCheckedChange={(v) => updateSettings({ [item.key]: v })}
                          className="h-[22px] w-10 data-[state=checked]:bg-primary data-[state=unchecked]:bg-[#3c4149] [&>span]:h-[18px] [&>span]:w-[18px] [&>span]:data-[state=checked]:translate-x-[18px]"
                        />
                      </div>
                    </div>
                  ))}
                </SettingCard>

                <SectionHeading>Proxy &amp; Đa phiên bản</SectionHeading>
                <SettingCard>
                  <div className="px-4 py-3">
                    <div className="mb-2 flex gap-1.5">
                      <Input
                        value={settings.proxy_url}
                        onChange={(e) => updateSettings({ proxy_url: e.target.value })}
                        placeholder="http://127.0.0.1:8080 hoặc socks5://..."
                      />
                      <Button
                        variant="default"
                        onClick={() => void ipc.setProxy(settings.proxy_url)}
                      >
                        Áp dụng
                      </Button>
                    </div>
                    <div className="flex gap-1.5">
                      <Input
                        value={settings.multi_profile_name}
                        onChange={(e) => updateSettings({ multi_profile_name: e.target.value })}
                        placeholder="Tên profile phụ (vd. acc2)"
                      />
                      <Button
                        variant="outline"
                        onClick={() =>
                          void ipc.spawnMultiProfile(
                            settings.multi_profile_name,
                            settings.proxy_url,
                          )
                        }
                      >
                        Mở Profile mới
                      </Button>
                    </div>
                  </div>
                </SettingCard>

                <SectionHeading>Tự động trả lời</SectionHeading>
                <SettingCard>
                  <div className="px-4 py-3">
                    {autoReplyRules.map((rule) => (
                      <div
                        key={rule.id}
                        className="flex items-center justify-between border-b border-border py-2 last:border-0"
                      >
                        <span className="text-sm">
                          <code className="rounded bg-muted px-1.5 py-0.5 text-xs">
                            {rule.keyword}
                          </code>{" "}
                          → {rule.reply_text}
                        </span>
                        <Button
                          variant="destructive"
                          size="sm"
                          onClick={async () => {
                            const next = autoReplyRules.filter((r) => r.id !== rule.id);
                            setAutoReplyRules(next);
                            await ipc.saveAutoreplyRules(next);
                          }}
                        >
                          Xóa
                        </Button>
                      </div>
                    ))}
                    <div className="mt-2 flex gap-1.5">
                      <Input
                        value={newRuleKw}
                        onChange={(e) => setNewRuleKw(e.target.value)}
                        placeholder="Từ khóa (vd. /ping)"
                      />
                      <Input
                        value={newRuleReply}
                        onChange={(e) => setNewRuleReply(e.target.value)}
                        placeholder="Nội dung trả lời…"
                      />
                      <Button
                        variant="default"
                        onClick={async () => {
                          if (!newRuleKw.trim() || !newRuleReply.trim()) return;
                          const next: AutoReplyRule[] = [
                            ...autoReplyRules,
                            {
                              id: `rule_${Date.now()}`,
                              keyword: newRuleKw.trim(),
                              match_mode: "contains",
                              reply_text: newRuleReply.trim(),
                              enabled: true,
                              scope: "direct",
                            },
                          ];
                          setAutoReplyRules(next);
                          setNewRuleKw("");
                          setNewRuleReply("");
                          await ipc.saveAutoreplyRules(next);
                        }}
                      >
                        Thêm
                      </Button>
                    </div>
                  </div>
                </SettingCard>

                <SectionHeading>Nhãn dán tùy chỉnh</SectionHeading>
                <SettingCard>
                  <div className="px-4 py-3">
                    <Button
                      variant="outline"
                      size="sm"
                      onClick={async () => {
                        const p = await ipc.exportCustomStickersTemplate();
                        dispatch({
                          type: "SHOW_TOAST",
                          message: `Đã xuất mẫu ${p}`,
                        });
                      }}
                    >
                      Xuất mẫu custom_stickers.json
                    </Button>
                  </div>
                </SettingCard>
              </>
            ) : null}

            {/* ── Update ── */}
            {tab === "update" ? (
              <>
                <SectionHeading>Cập nhật ứng dụng</SectionHeading>
                <SettingCard>
                  <div className="flex items-center justify-between px-4 py-3">
                    <div>
                      <div className="text-sm font-medium text-foreground">
                        AloLite v{state.appVersion}
                      </div>
                      <div className="text-xs text-muted-foreground">
                        {state.updateChecking
                          ? "Đang kiểm tra bản phát hành mới…"
                          : state.updateInfo?.has_update
                            ? `Có bản mới v${state.updateInfo.latest_version}`
                            : "Kiểm tra cập nhật trực tiếp từ GitHub Releases"}
                      </div>
                    </div>
                    <Button
                      variant="default"
                      size="sm"
                      disabled={state.updateChecking}
                      onClick={() => {
                        dispatch({ type: "SET_UPDATE_CHECKING", checking: true });
                        void ipc.checkForUpdates(false);
                      }}
                    >
                      <RefreshCw className="h-3.5 w-3.5" /> Kiểm tra ngay
                    </Button>
                  </div>
                  {state.updateInfo?.has_update && state.updateInfo.download_url ? (
                    <>
                      <RowDivider />
                      <div className="px-4 py-3">
                        <Button
                          variant="default"
                          disabled={state.updateDownloading}
                          onClick={() =>
                            void ipc.downloadAndApplyUpdate(
                              state.updateInfo!.download_url!,
                              state.updateInfo!.asset_name ?? "alolite",
                              state.updateInfo!.latest_version,
                              state.updateInfo!.expected_sha256,
                            )
                          }
                        >
                          <Download className="h-3.5 w-3.5" /> Tải và cài đặt v
                          {state.updateInfo.latest_version}
                        </Button>
                      </div>
                    </>
                  ) : null}
                  {state.updateDownloadMessage ? (
                    <>
                      <RowDivider />
                      <div className="px-4 py-2 text-xs text-muted-foreground">
                        {state.updateDownloadMessage}
                      </div>
                    </>
                  ) : null}
                </SettingCard>

                <SectionHeading>Tự động cập nhật</SectionHeading>
                <SettingCard>
                  <SwitchRow
                    label="Tự động kiểm tra cập nhật"
                    checked={settings.auto_check_updates}
                    onChange={(v) => updateSettings({ auto_check_updates: v })}
                  />
                </SettingCard>
              </>
            ) : null}
          </div>
        </div>
      </DialogContent>
    </Dialog>
  );
}
