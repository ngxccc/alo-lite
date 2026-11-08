import { useEffect, useState } from "react";
import {
  Bell,
  Cloud,
  Download,
  FolderOpen,
  Lock,
  MessageSquare,
  Palette,
  RefreshCw,
  Settings,
  Shield,
  Sparkles,
  Trash2,
  User,
  Wrench,
} from "lucide-react";
import { ipc, isTauriRuntime } from "../../../shared/api/ipc";
import type { AppSettings, AutoReplyRule } from "../../../shared/types/ipc";
import { Avatar } from "../../../shared/ui/Avatar";
import { Badge } from "../../../shared/ui/badge";
import { Button } from "../../../shared/ui/button";
import { Dialog, DialogContent, DialogTitle } from "../../../shared/ui/dialog";
import { Input } from "../../../shared/ui/input";
import { Switch } from "../../../shared/ui/switch";
import { cn } from "../../../shared/utils/cn";
import type { AppAction, AppState, SettingsTab } from "../../../app/store/appReducer";

interface SettingsModalProps {
  state: AppState;
  dispatch: (action: AppAction) => void;
}

const SETTINGS_TABS: Array<{ id: SettingsTab; label: string; icon: typeof Settings }> = [
  { id: "general", label: "Cài đặt chung", icon: Settings },
  { id: "account", label: "Tài khoản", icon: User },
  { id: "privacy", label: "Quyền riêng tư", icon: Lock },
  { id: "sync", label: "Đồng bộ & Dữ liệu", icon: Cloud },
  { id: "appearance", label: "Giao diện", icon: Palette },
  { id: "notifications", label: "Thông báo", icon: Bell },
  { id: "messages", label: "Tin nhắn", icon: MessageSquare },
  { id: "utilities", label: "Tiện ích & Proxy", icon: Wrench },
  { id: "update", label: "Cập nhật", icon: Sparkles },
];

function SettingSwitchRow({
  label,
  desc,
  checked,
  onChange,
}: {
  label: string;
  desc?: string;
  checked: boolean;
  onChange: (val: boolean) => void;
}) {
  return (
    <div className="flex items-center justify-between gap-4 py-1.5">
      <div className="min-w-0 flex-1">
        <div className="text-xs font-medium text-foreground">{label}</div>
        {desc ? <div className="text-[11px] text-muted-foreground">{desc}</div> : null}
      </div>
      <Switch checked={checked} onCheckedChange={onChange} />
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
        .then(setAutoReplyRules)
        .catch(() => {});
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
      <DialogContent className="flex h-[560px] max-w-3xl flex-row p-0 overflow-hidden">
        <nav className="flex w-52 shrink-0 flex-col gap-1 border-r border-border bg-sidebar p-3">
          <DialogTitle className="px-2 py-1.5 text-sm font-bold">Cài đặt</DialogTitle>
          {SETTINGS_TABS.map((item) => {
            const Icon = item.icon;
            const active = tab === item.id;
            return (
              <button
                key={item.id}
                type="button"
                onClick={() => dispatch({ type: "OPEN_SETTINGS", open: true, tab: item.id })}
                className={cn(
                  "flex items-center gap-2.5 rounded-lg px-2.5 py-2 text-left text-xs font-medium transition-colors cursor-pointer",
                  active
                    ? "bg-accent text-accent-foreground font-semibold"
                    : "text-muted-foreground hover:bg-muted hover:text-foreground",
                )}
              >
                <Icon className="h-4 w-4 shrink-0" />
                <span>{item.label}</span>
              </button>
            );
          })}
        </nav>

        <div className="flex flex-1 flex-col gap-3.5 overflow-y-auto p-5">
          <div className="text-base font-bold text-foreground">
            {SETTINGS_TABS.find((t) => t.id === tab)?.label}
          </div>

          {tab === "general" ? (
            <>
              <div className="flex flex-col gap-2 rounded-xl border border-border bg-card p-3.5">
                <div className="text-xs font-bold text-foreground">Danh bạ & Ngôn ngữ</div>
                <SettingSwitchRow
                  label="Hiển thị tất cả danh bạ"
                  checked={settings.show_all_contacts}
                  onChange={(v) => updateSettings({ show_all_contacts: v })}
                />
                <div className="flex items-center justify-between py-1">
                  <span className="text-xs font-medium">Ngôn ngữ</span>
                  <select
                    value={settings.language}
                    onChange={(e) => updateSettings({ language: e.target.value })}
                    className="h-8 rounded-md border border-input bg-background px-2.5 text-xs text-foreground"
                  >
                    <option value="Tiếng Việt">Tiếng Việt</option>
                    <option value="English">English</option>
                  </select>
                </div>
              </div>

              <div className="flex flex-col gap-2.5 rounded-xl border border-border bg-card p-3.5">
                <div className="text-xs font-bold text-foreground">
                  Khởi động & Thư mục tải xuống
                </div>
                <SettingSwitchRow
                  label="Khởi động AloLite cùng hệ thống"
                  checked={settings.autostart_on_boot}
                  onChange={(v) => updateSettings({ autostart_on_boot: v })}
                />
                <SettingSwitchRow
                  label="Ghi nhớ tài khoản đăng nhập lần trước"
                  checked={settings.remember_login}
                  onChange={(v) => updateSettings({ remember_login: v })}
                />
                <div className="text-xs text-muted-foreground">
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
            </>
          ) : null}

          {tab === "account" ? (
            <>
              <div className="flex items-center justify-between rounded-xl border border-border bg-card p-3.5">
                <div className="flex items-center gap-3">
                  <Avatar
                    id={state.uin || "self"}
                    name={state.displayName || "Me"}
                    avatarUrl={state.avatar}
                    size={44}
                    mediaCache={state.mediaCache}
                  />
                  <div>
                    <div className="text-sm font-bold text-foreground">
                      {state.displayName || "Người dùng Zalo"}
                    </div>
                    <div className="text-xs font-medium text-online">
                      Đang hoạt động {state.uin ? `· UIN: ${state.uin}` : ""}
                    </div>
                  </div>
                </div>
                <Button variant="destructive" size="sm" onClick={() => void ipc.logout()}>
                  Đăng xuất
                </Button>
              </div>

              <div className="flex flex-col gap-2.5 rounded-xl border border-border bg-card p-3.5">
                <div className="flex items-center justify-between">
                  <span className="text-xs font-bold text-foreground">Đa tài khoản đã lưu</span>
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
                  <div className="text-xs text-muted-foreground">
                    Chưa có tài khoản phụ nào được lưu trong thư mục accounts/.
                  </div>
                ) : (
                  state.savedAccounts.map((acc) => (
                    <div
                      key={acc.uin}
                      className="flex items-center justify-between border-t border-border pt-2.5"
                    >
                      <div className="flex items-center gap-2.5">
                        <Avatar
                          id={acc.uin}
                          name={acc.display_name}
                          avatarUrl={acc.avatar}
                          size={32}
                          mediaCache={state.mediaCache}
                        />
                        <div>
                          <div className="text-xs font-semibold text-foreground">
                            {acc.display_name}
                          </div>
                          <div className="text-[11px] text-muted-foreground">UIN: {acc.uin}</div>
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
                  ))
                )}
              </div>
            </>
          ) : null}

          {tab === "privacy" ? (
            <>
              <div className="flex flex-col gap-2 rounded-xl border border-border bg-card p-3.5">
                <div className="flex items-center gap-1.5 text-xs font-bold text-foreground">
                  <Shield className="h-4 w-4 text-primary" /> Chặn gửi trạng thái lên máy chủ
                  (Ctrl+Shift+P)
                </div>
                <SettingSwitchRow
                  label='Ẩn trạng thái "Đang soạn tin" (Block Typing)'
                  desc="Chặn gửi gói /api/message/typing & /api/group/typing."
                  checked={settings.block_typing}
                  onChange={(v) => updateSettings({ block_typing: v })}
                />
                <SettingSwitchRow
                  label='Ẩn trạng thái "Đã nhận" (Block Delivered)'
                  desc="Chặn gửi xác nhận đã nhận tin."
                  checked={settings.block_delivered}
                  onChange={(v) => updateSettings({ block_delivered: v })}
                />
                <SettingSwitchRow
                  label='Ẩn trạng thái "Đã xem" (Block Seen)'
                  desc="Đọc tin nhắn mà không gửi /api/message/seenv2."
                  checked={settings.block_seen}
                  onChange={(v) => updateSettings({ block_seen: v, show_seen_status: !v })}
                />
              </div>

              <div className="flex flex-col gap-2 rounded-xl border border-border bg-card p-3.5">
                <div className="text-xs font-bold text-foreground">Quyền riêng tư hiển thị</div>
                <SettingSwitchRow
                  label="Hiển thị trạng thái truy cập"
                  checked={settings.show_online_status}
                  onChange={(v) => updateSettings({ show_online_status: v })}
                />
                <SettingSwitchRow
                  label="Cho phép tìm kiếm qua số điện thoại"
                  checked={settings.allow_search_by_phone}
                  onChange={(v) => updateSettings({ allow_search_by_phone: v })}
                />
              </div>
            </>
          ) : null}

          {tab === "sync" ? (
            <>
              <div className="flex flex-col gap-2.5 rounded-xl border border-border bg-card p-3.5">
                <div className="text-xs font-bold text-foreground">
                  Đồng bộ từ điện thoại & Đám mây
                </div>
                <div className="text-xs text-muted-foreground">
                  Cơ sở dữ liệu đang hoạt động:{" "}
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
                      <RefreshCw className="h-3.5 w-3.5" /> Đồng bộ Nhóm & Cloud
                    </Button>
                  )}
                </div>
              </div>

              <div className="flex flex-col gap-2.5 rounded-xl border border-border bg-card p-3.5">
                <div className="text-xs font-bold text-foreground">
                  Nhập lịch sử từ tệp sao lưu (.db / .sqlite / .json)
                </div>
                <div className="flex gap-1.5">
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

              <div className="flex items-center justify-between rounded-xl border border-border bg-card p-3.5">
                <div>
                  <div className="text-xs font-bold text-destructive">
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
            </>
          ) : null}

          {tab === "appearance" ? (
            <div className="flex flex-col gap-3 rounded-xl border border-border bg-card p-3.5">
              <div className="text-xs font-bold text-foreground">Chủ đề giao diện</div>
              <div className="grid grid-cols-3 gap-2">
                {[
                  { id: "light" as const, label: "Sáng (Zalo Slate)" },
                  { id: "dark" as const, label: "Tối (Zinc Dark)" },
                  { id: "system" as const, label: "Theo hệ thống" },
                ].map((opt) => (
                  <Button
                    key={opt.id}
                    variant={settings.theme_mode === opt.id ? "default" : "outline"}
                    onClick={() => updateSettings({ theme_mode: opt.id })}
                  >
                    {opt.label}
                  </Button>
                ))}
              </div>
              <SettingSwitchRow
                label="Sử dụng ảnh đại diện làm nền mờ"
                checked={settings.use_avatar_as_wallpaper}
                onChange={(v) => updateSettings({ use_avatar_as_wallpaper: v })}
              />
            </div>
          ) : null}

          {tab === "notifications" ? (
            <div className="flex flex-col gap-2 rounded-xl border border-border bg-card p-3.5">
              <SettingSwitchRow
                label="Bật thông báo tin nhắn mới"
                checked={settings.enable_notifications}
                onChange={(v) => updateSettings({ enable_notifications: v })}
              />
              <SettingSwitchRow
                label="Phát âm thanh thông báo"
                checked={settings.play_notification_sound}
                onChange={(v) => updateSettings({ play_notification_sound: v })}
              />
            </div>
          ) : null}

          {tab === "messages" ? (
            <div className="flex flex-col gap-2 rounded-xl border border-border bg-card p-3.5">
              <SettingSwitchRow
                label="Nhấn đúp vào tin nhắn để trả lời nhanh"
                checked={settings.double_click_to_reply}
                onChange={(v) => updateSettings({ double_click_to_reply: v })}
              />
              <SettingSwitchRow
                label="Hiển thị trạng thái đang soạn tin"
                checked={settings.show_typing_indicator}
                onChange={(v) => updateSettings({ show_typing_indicator: v })}
              />
              <div className="flex items-center justify-between py-1">
                <span className="text-xs font-medium">Phím gửi tin nhắn</span>
                <select
                  value={settings.send_message_key}
                  onChange={(e) =>
                    updateSettings({
                      send_message_key: e.target.value as "enter" | "cmd_enter",
                    })
                  }
                  className="h-8 rounded-md border border-input bg-background px-2.5 text-xs text-foreground"
                >
                  <option value="enter">Enter để gửi</option>
                  <option value="cmd_enter">Ctrl/Cmd + Enter để gửi</option>
                </select>
              </div>
              <SettingSwitchRow
                label="Bật tin nhắn nhanh"
                checked={settings.quick_messages_enabled}
                onChange={(v) => updateSettings({ quick_messages_enabled: v })}
              />
              <SettingSwitchRow
                label="Phân tách hộp thư Ưu tiên / Khác"
                checked={settings.split_priority_inbox}
                onChange={(v) => updateSettings({ split_priority_inbox: v })}
              />
            </div>
          ) : null}

          {tab === "utilities" ? (
            <>
              <div className="flex flex-col gap-2 rounded-xl border border-border bg-card p-3.5">
                <div className="text-xs font-bold text-foreground">Gợi ý & Sticker</div>
                <SettingSwitchRow
                  label="Gợi ý nhãn dán theo nội dung đang soạn"
                  checked={settings.suggest_stickers_while_typing}
                  onChange={(v) => updateSettings({ suggest_stickers_while_typing: v })}
                />
                <SettingSwitchRow
                  label="Gợi ý nhắc tên (@mention) trong nhóm"
                  checked={settings.suggest_mentions}
                  onChange={(v) => updateSettings({ suggest_mentions: v })}
                />
                <SettingSwitchRow
                  label="Bật tìm kiếm nhãn dán theo từ khóa"
                  checked={settings.enable_sticker_keyword_search}
                  onChange={(v) => updateSettings({ enable_sticker_keyword_search: v })}
                />
                <Button
                  variant="outline"
                  size="sm"
                  className="w-fit"
                  onClick={async () => {
                    const p = await ipc.exportCustomStickersTemplate();
                    dispatch({ type: "SHOW_TOAST", message: `Đã xuất mẫu ${p}` });
                  }}
                >
                  Xuất mẫu custom_stickers.json
                </Button>
              </div>

              <div className="flex flex-col gap-2.5 rounded-xl border border-border bg-card p-3.5">
                <div className="text-xs font-bold text-foreground">Proxy & Đa phiên bản</div>
                <div className="flex gap-1.5">
                  <Input
                    value={settings.proxy_url}
                    onChange={(e) => updateSettings({ proxy_url: e.target.value })}
                    placeholder="http://127.0.0.1:8080 hoặc socks5://..."
                  />
                  <Button variant="default" onClick={() => void ipc.setProxy(settings.proxy_url)}>
                    Áp dụng Proxy
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
                      void ipc.spawnMultiProfile(settings.multi_profile_name, settings.proxy_url)
                    }
                  >
                    Mở cửa sổ Profile mới
                  </Button>
                </div>
              </div>

              <div className="flex flex-col gap-2 rounded-xl border border-border bg-card p-3.5">
                <div className="text-xs font-bold text-foreground">
                  Tự động trả lời (Auto-Reply Rules)
                </div>
                {autoReplyRules.map((rule) => (
                  <div
                    key={rule.id}
                    className="flex items-center justify-between border-t border-border pt-2 text-xs"
                  >
                    <span>
                      <code className="rounded bg-muted px-1.5 py-0.5">{rule.keyword}</code> →{" "}
                      {rule.reply_text}
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
                <div className="flex gap-1.5">
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
            </>
          ) : null}

          {tab === "update" ? (
            <div className="flex flex-col gap-2.5 rounded-xl border border-border bg-card p-3.5">
              <div className="flex items-center justify-between">
                <div>
                  <div className="text-xs font-bold text-foreground">
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
              ) : null}
              {state.updateDownloadMessage ? (
                <div className="text-xs text-muted-foreground">{state.updateDownloadMessage}</div>
              ) : null}
            </div>
          ) : null}
        </div>
      </DialogContent>
    </Dialog>
  );
}
