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
  X,
} from "lucide-react";
import { ipc, isTauriRuntime } from "../../../shared/api/ipc";
import type { AppSettings, AutoReplyRule } from "../../../shared/types/ipc";
import { Avatar } from "../../../shared/ui/Avatar";
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

export function SettingsModal({ state, dispatch }: SettingsModalProps) {
  const [migratePath, setMigratePath] = useState("");
  const [autoReplyRules, setAutoReplyRules] = useState<AutoReplyRule[]>([]);
  const [newRuleKw, setNewRuleKw] = useState("");
  const [newRuleReply, setNewRuleReply] = useState("");

  useEffect(() => {
    if (!state.settingsModalOpen || !isTauriRuntime()) return;
    if (state.settingsTab === "account") {
      void ipc.listSavedAccounts().then((accounts) =>
        dispatch({ type: "SET_SAVED_ACCOUNTS", accounts }),
      );
    } else if (state.settingsTab === "utilities") {
      void ipc.getAutoreplyRules().then(setAutoReplyRules).catch(() => {});
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

  const renderToggle = (label: string, checked: boolean, onChange: (val: boolean) => void, desc?: string) => (
    <label className="settings-row" style={{ cursor: "pointer" }}>
      <div style={{ flex: 1 }}>
        <div style={{ fontWeight: 500 }}>{label}</div>
        {desc ? <div style={{ fontSize: 11.5, color: "var(--text-secondary)" }}>{desc}</div> : null}
      </div>
      <input type="checkbox" checked={checked} onChange={(e) => onChange(e.target.checked)} />
    </label>
  );

  return (
    <div className="modal-backdrop" onClick={() => dispatch({ type: "OPEN_SETTINGS", open: false })}>
      <div className="modal-card settings-modal" onClick={(e) => e.stopPropagation()}>
        <nav className="settings-nav">
          <div style={{ fontWeight: 700, fontSize: 14.5, padding: "4px 8px 10px" }}>Cài đặt</div>
          {SETTINGS_TABS.map((item) => {
            const Icon = item.icon;
            return (
              <button
                key={item.id}
                type="button"
                className={`settings-nav-item ${tab === item.id ? "active" : ""}`}
                onClick={() => dispatch({ type: "OPEN_SETTINGS", open: true, tab: item.id })}
              >
                <Icon size={15} />
                <span>{item.label}</span>
              </button>
            );
          })}
        </nav>

        <div className="settings-body">
          <div className="settings-row">
            <div style={{ fontWeight: 700, fontSize: 15.5 }}>
              {SETTINGS_TABS.find((t) => t.id === tab)?.label}
            </div>
            <button
              type="button"
              className="icon-btn"
              onClick={() => dispatch({ type: "OPEN_SETTINGS", open: false })}
            >
              <X size={17} />
            </button>
          </div>

          {tab === "general" ? (
            <>
              <div className="settings-box">
                <div style={{ fontWeight: 700 }}>Danh bạ & Ngôn ngữ</div>
                {renderToggle("Hiển thị tất cả danh bạ", settings.show_all_contacts, (v) =>
                  updateSettings({ show_all_contacts: v }),
                )}
                <div className="settings-row">
                  <span>Ngôn ngữ</span>
                  <select
                    className="field-input"
                    style={{ width: 150 }}
                    value={settings.language}
                    onChange={(e) => updateSettings({ language: e.target.value })}
                  >
                    <option value="Tiếng Việt">Tiếng Việt</option>
                    <option value="English">English</option>
                  </select>
                </div>
              </div>

              <div className="settings-box">
                <div style={{ fontWeight: 700 }}>Khởi động & Thư mục tải xuống</div>
                {renderToggle("Khởi động AloLite cùng hệ thống", settings.autostart_on_boot, (v) =>
                  updateSettings({ autostart_on_boot: v }),
                )}
                {renderToggle("Ghi nhớ tài khoản đăng nhập lần trước", settings.remember_login, (v) =>
                  updateSettings({ remember_login: v }),
                )}
                <div style={{ fontSize: 12, color: "var(--text-secondary)" }}>
                  Lưu tại: <code>{settings.download_dir}</code>
                </div>
                <div style={{ display: "flex", gap: 8 }}>
                  <button
                    type="button"
                    className="btn"
                    onClick={async () => {
                      const dir = await ipc.pickFile({ directory: true, title: "Chọn thư mục lưu file" });
                      if (dir) updateSettings({ download_dir: dir });
                    }}
                  >
                    <FolderOpen size={14} /> Thay đổi thư mục
                  </button>
                  <button
                    type="button"
                    className="btn"
                    onClick={() => void ipc.openPathInOs(settings.download_dir)}
                  >
                    Mở thư mục
                  </button>
                </div>
              </div>
            </>
          ) : null}

          {tab === "account" ? (
            <>
              <div className="settings-box">
                <div className="settings-row">
                  <div style={{ display: "flex", alignItems: "center", gap: 12 }}>
                    <Avatar
                      id={state.uin || "self"}
                      name={state.displayName || "Me"}
                      avatarUrl={state.avatar}
                      size={44}
                      mediaCache={state.mediaCache}
                    />
                    <div>
                      <div style={{ fontWeight: 700, fontSize: 14.5 }}>{state.displayName || "Người dùng Zalo"}</div>
                      <div style={{ fontSize: 12, color: "var(--accent-online)" }}>
                        Đang hoạt động {state.uin ? `· UIN: ${state.uin}` : ""}
                      </div>
                    </div>
                  </div>
                  <button type="button" className="btn btn-danger" onClick={() => void ipc.logout()}>
                    Đăng xuất
                  </button>
                </div>
              </div>

              <div className="settings-box">
                <div className="settings-row">
                  <span style={{ fontWeight: 700 }}>Đa tài khoản đã lưu</span>
                  <button
                    type="button"
                    className="btn btn-primary"
                    onClick={() => {
                      dispatch({ type: "OPEN_SETTINGS", open: false });
                      void ipc.addNewAccount();
                    }}
                  >
                    + Thêm tài khoản (QR)
                  </button>
                </div>
                {state.savedAccounts.length === 0 ? (
                  <div style={{ fontSize: 12, color: "var(--text-muted)" }}>
                    Chưa có tài khoản phụ nào được lưu trong thư mục accounts/.
                  </div>
                ) : (
                  state.savedAccounts.map((acc) => (
                    <div key={acc.uin} className="settings-row" style={{ paddingTop: 6, borderTop: "1px solid var(--border)" }}>
                      <div style={{ display: "flex", alignItems: "center", gap: 10 }}>
                        <Avatar
                          id={acc.uin}
                          name={acc.display_name}
                          avatarUrl={acc.avatar}
                          size={32}
                          mediaCache={state.mediaCache}
                        />
                        <div>
                          <div style={{ fontWeight: 600 }}>{acc.display_name}</div>
                          <div style={{ fontSize: 11.5, color: "var(--text-muted)" }}>UIN: {acc.uin}</div>
                        </div>
                      </div>
                      <div style={{ display: "flex", gap: 6 }}>
                        {acc.is_current ? (
                          <span style={{ fontSize: 12, color: "var(--accent-online)", fontWeight: 600 }}>
                            Đang dùng
                          </span>
                        ) : (
                          <>
                            <button
                              type="button"
                              className="btn btn-primary"
                              onClick={() => void ipc.switchAccount(acc.uin)}
                            >
                              Chuyển đổi
                            </button>
                            <button
                              type="button"
                              className="btn btn-danger"
                              onClick={async () => {
                                await ipc.deleteSavedAccount(acc.uin);
                                const next = await ipc.listSavedAccounts();
                                dispatch({ type: "SET_SAVED_ACCOUNTS", accounts: next });
                              }}
                            >
                              <Trash2 size={13} />
                            </button>
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
              <div className="settings-box">
                <div style={{ fontWeight: 700, display: "flex", alignItems: "center", gap: 6 }}>
                  <Shield size={15} color="var(--primary)" /> Chặn gửi trạng thái lên máy chủ (Ctrl+Shift+P)
                </div>
                {renderToggle(
                  "Ẩn trạng thái \"Đang soạn tin\" (Block Typing)",
                  settings.block_typing,
                  (v) => updateSettings({ block_typing: v }),
                  "Chặn gửi gói /api/message/typing & /api/group/typing.",
                )}
                {renderToggle(
                  "Ẩn trạng thái \"Đã nhận\" (Block Delivered)",
                  settings.block_delivered,
                  (v) => updateSettings({ block_delivered: v }),
                  "Chặn gửi xác nhận đã nhận tin.",
                )}
                {renderToggle(
                  "Ẩn trạng thái \"Đã xem\" (Block Seen)",
                  settings.block_seen,
                  (v) => updateSettings({ block_seen: v, show_seen_status: !v }),
                  "Đọc tin nhắn mà không gửi /api/message/seenv2.",
                )}
              </div>

              <div className="settings-box">
                <div style={{ fontWeight: 700 }}>Quyền riêng tư hiển thị</div>
                {renderToggle("Hiển thị trạng thái truy cập", settings.show_online_status, (v) =>
                  updateSettings({ show_online_status: v }),
                )}
                {renderToggle("Cho phép tìm kiếm qua số điện thoại", settings.allow_search_by_phone, (v) =>
                  updateSettings({ allow_search_by_phone: v }),
                )}
              </div>
            </>
          ) : null}

          {tab === "sync" ? (
            <>
              <div className="settings-box">
                <div style={{ fontWeight: 700 }}>Đồng bộ từ điện thoại & Đám mây</div>
                <div style={{ fontSize: 12.2, color: "var(--text-secondary)" }}>
                  Cơ sở dữ liệu đang hoạt động: <code>{state.activeDbPath}</code>
                </div>
                <div style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
                  <button
                    type="button"
                    className="btn btn-primary"
                    onClick={() => dispatch({ type: "SET_PHONE_SYNC_MODAL", open: true })}
                  >
                    Đồng bộ từ điện thoại
                  </button>
                  {state.syncRunning ? (
                    <button type="button" className="btn btn-danger" onClick={() => void ipc.cancelCloudSync()}>
                      Dừng đồng bộ Nhóm/Cloud
                    </button>
                  ) : (
                    <button
                      type="button"
                      className="btn"
                      onClick={() => {
                        const targets = state.threads
                          .filter((t) => t.is_group || t.name.includes("Cloud của tôi"))
                          .map<[string, string, boolean]>((t) => [t.id, t.name, t.is_group]);
                        dispatch({ type: "START_CLOUD_SYNC" });
                        void ipc.syncHistory(targets);
                      }}
                    >
                      <RefreshCw size={14} /> Đồng bộ Nhóm & Cloud
                    </button>
                  )}
                </div>
              </div>

              <div className="settings-box">
                <div style={{ fontWeight: 700 }}>Nhập lịch sử từ tệp sao lưu (.db / .sqlite / .json)</div>
                <div style={{ display: "flex", gap: 6 }}>
                  <input
                    className="field-input"
                    value={migratePath}
                    onChange={(e) => setMigratePath(e.target.value)}
                    placeholder="Đường dẫn file hoặc thư mục Zalo…"
                  />
                  <button
                    type="button"
                    className="btn"
                    onClick={async () => {
                      const p = await ipc.pickFile({ title: "Chọn file hoặc thư mục dữ liệu Zalo" });
                      if (p) setMigratePath(p);
                    }}
                  >
                    Duyệt…
                  </button>
                  <button
                    type="button"
                    className="btn btn-primary"
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
                  </button>
                </div>
                <button
                  type="button"
                  className="btn"
                  onClick={async () => {
                    try {
                      await ipc.autoDetectImportHistory();
                    } catch (e) {
                      dispatch({ type: "SHOW_TOAST", message: String(e) });
                    }
                  }}
                >
                  Tự động quét dữ liệu Zalo trên máy
                </button>
              </div>

              <div className="settings-box">
                <div className="settings-row">
                  <div>
                    <div style={{ fontWeight: 700, color: "var(--danger)" }}>Xóa dữ liệu tin nhắn cục bộ</div>
                    <div style={{ fontSize: 12, color: "var(--text-secondary)" }}>
                      Giữ nguyên phiên đăng nhập session.json.
                    </div>
                  </div>
                  <button
                    type="button"
                    className="btn btn-danger"
                    onClick={() => dispatch({ type: "SET_WIPE_CONFIRM", open: true })}
                  >
                    <Trash2 size={14} /> Xóa dữ liệu
                  </button>
                </div>
              </div>
            </>
          ) : null}

          {tab === "appearance" ? (
            <div className="settings-box">
              <div style={{ fontWeight: 700 }}>Chủ đề giao diện</div>
              <div style={{ display: "grid", gridTemplateColumns: "repeat(3, 1fr)", gap: 10 }}>
                {[
                  { id: "light" as const, label: "Sáng (Zalo Light)" },
                  { id: "dark" as const, label: "Tối (Atom One Dark)" },
                  { id: "system" as const, label: "Theo hệ thống" },
                ].map((opt) => (
                  <button
                    key={opt.id}
                    type="button"
                    className={`btn ${settings.theme_mode === opt.id ? "btn-primary" : ""}`}
                    onClick={() => updateSettings({ theme_mode: opt.id })}
                  >
                    {opt.label}
                  </button>
                ))}
              </div>
              {renderToggle("Sử dụng ảnh đại diện làm nền mờ", settings.use_avatar_as_wallpaper, (v) =>
                updateSettings({ use_avatar_as_wallpaper: v }),
              )}
            </div>
          ) : null}

          {tab === "notifications" ? (
            <div className="settings-box">
              {renderToggle("Bật thông báo tin nhắn mới", settings.enable_notifications, (v) =>
                updateSettings({ enable_notifications: v }),
              )}
              {renderToggle("Phát âm thanh thông báo", settings.play_notification_sound, (v) =>
                updateSettings({ play_notification_sound: v }),
              )}
            </div>
          ) : null}

          {tab === "messages" ? (
            <div className="settings-box">
              {renderToggle("Nhấn đúp vào tin nhắn để trả lời nhanh", settings.double_click_to_reply, (v) =>
                updateSettings({ double_click_to_reply: v }),
              )}
              {renderToggle("Hiển thị trạng thái đang soạn tin", settings.show_typing_indicator, (v) =>
                updateSettings({ show_typing_indicator: v }),
              )}
              <div className="settings-row">
                <span>Phím gửi tin nhắn</span>
                <select
                  className="field-input"
                  style={{ width: 170 }}
                  value={settings.send_message_key}
                  onChange={(e) =>
                    updateSettings({ send_message_key: e.target.value as "enter" | "cmd_enter" })
                  }
                >
                  <option value="enter">Enter để gửi</option>
                  <option value="cmd_enter">Ctrl/Cmd + Enter để gửi</option>
                </select>
              </div>
              {renderToggle("Bật tin nhắn nhanh", settings.quick_messages_enabled, (v) =>
                updateSettings({ quick_messages_enabled: v }),
              )}
              {renderToggle("Phân tách hộp thư Ưu tiên / Khác", settings.split_priority_inbox, (v) =>
                updateSettings({ split_priority_inbox: v }),
              )}
            </div>
          ) : null}

          {tab === "utilities" ? (
            <>
              <div className="settings-box">
                <div style={{ fontWeight: 700 }}>Gợi ý & Sticker</div>
                {renderToggle("Gợi ý nhãn dán theo nội dung đang soạn", settings.suggest_stickers_while_typing, (v) =>
                  updateSettings({ suggest_stickers_while_typing: v }),
                )}
                {renderToggle("Gợi ý nhắc tên (@mention) trong nhóm", settings.suggest_mentions, (v) =>
                  updateSettings({ suggest_mentions: v }),
                )}
                {renderToggle("Bật tìm kiếm nhãn dán theo từ khóa", settings.enable_sticker_keyword_search, (v) =>
                  updateSettings({ enable_sticker_keyword_search: v }),
                )}
                <button
                  type="button"
                  className="btn"
                  onClick={async () => {
                    const p = await ipc.exportCustomStickersTemplate();
                    dispatch({ type: "SHOW_TOAST", message: `Đã xuất mẫu ${p}` });
                  }}
                >
                  Xuất mẫu custom_stickers.json
                </button>
              </div>

              <div className="settings-box">
                <div style={{ fontWeight: 700 }}>Proxy & Đa phiên bản</div>
                <div style={{ display: "flex", gap: 6 }}>
                  <input
                    className="field-input"
                    value={settings.proxy_url}
                    onChange={(e) => updateSettings({ proxy_url: e.target.value })}
                    placeholder="http://127.0.0.1:8080 hoặc socks5://..."
                  />
                  <button type="button" className="btn btn-primary" onClick={() => void ipc.setProxy(settings.proxy_url)}>
                    Áp dụng Proxy
                  </button>
                </div>
                <div style={{ display: "flex", gap: 6 }}>
                  <input
                    className="field-input"
                    value={settings.multi_profile_name}
                    onChange={(e) => updateSettings({ multi_profile_name: e.target.value })}
                    placeholder="Tên profile phụ (vd. acc2)"
                  />
                  <button
                    type="button"
                    className="btn"
                    onClick={() => void ipc.spawnMultiProfile(settings.multi_profile_name, settings.proxy_url)}
                  >
                    Mở cửa sổ Profile mới
                  </button>
                </div>
              </div>

              <div className="settings-box">
                <div style={{ fontWeight: 700 }}>Tự động trả lời (Auto-Reply Rules)</div>
                {autoReplyRules.map((rule) => (
                  <div key={rule.id} className="settings-row" style={{ fontSize: 12.2 }}>
                    <span>
                      <code>{rule.keyword}</code> → {rule.reply_text}
                    </span>
                    <button
                      type="button"
                      className="btn btn-danger"
                      onClick={async () => {
                        const next = autoReplyRules.filter((r) => r.id !== rule.id);
                        setAutoReplyRules(next);
                        await ipc.saveAutoreplyRules(next);
                      }}
                    >
                      Xóa
                    </button>
                  </div>
                ))}
                <div style={{ display: "flex", gap: 6 }}>
                  <input
                    className="field-input"
                    value={newRuleKw}
                    onChange={(e) => setNewRuleKw(e.target.value)}
                    placeholder="Từ khóa (vd. /ping)"
                  />
                  <input
                    className="field-input"
                    value={newRuleReply}
                    onChange={(e) => setNewRuleReply(e.target.value)}
                    placeholder="Nội dung trả lời…"
                  />
                  <button
                    type="button"
                    className="btn btn-primary"
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
                  </button>
                </div>
              </div>
            </>
          ) : null}

          {tab === "update" ? (
            <div className="settings-box">
              <div className="settings-row">
                <div>
                  <div style={{ fontWeight: 700 }}>AloLite v{state.appVersion}</div>
                  <div style={{ fontSize: 12, color: "var(--text-secondary)" }}>
                    {state.updateChecking
                      ? "Đang kiểm tra bản phát hành mới…"
                      : state.updateInfo?.has_update
                        ? `Có bản mới v${state.updateInfo.latest_version}`
                        : "Kiểm tra cập nhật trực tiếp từ GitHub Releases"}
                  </div>
                </div>
                <button
                  type="button"
                  className="btn btn-primary"
                  disabled={state.updateChecking}
                  onClick={() => {
                    dispatch({ type: "SET_UPDATE_CHECKING", checking: true });
                    void ipc.checkForUpdates(false);
                  }}
                >
                  <RefreshCw size={14} /> Kiểm tra ngay
                </button>
              </div>
              {state.updateInfo?.has_update && state.updateInfo.download_url ? (
                <button
                  type="button"
                  className="btn btn-primary"
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
                  <Download size={14} /> Tải và cài đặt v{state.updateInfo.latest_version}
                </button>
              ) : null}
              {state.updateDownloadMessage ? (
                <div style={{ fontSize: 12 }}>{state.updateDownloadMessage}</div>
              ) : null}
            </div>
          ) : null}
        </div>
      </div>
    </div>
  );
}
