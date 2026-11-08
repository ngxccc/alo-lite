import {
  Bug,
  Cloud,
  MessageSquare,
  Moon,
  RefreshCw,
  Settings,
  Smartphone,
  Sun,
  Users,
} from "lucide-react";
import { ipc } from "../../../shared/api/ipc";
import { Avatar } from "../../../shared/ui/Avatar";
import type { AppAction, AppState } from "../../../app/store/appReducer";
import { threadUnreadCount } from "../../threads/utils/threads";

interface NavRailProps {
  state: AppState;
  dispatch: (action: AppAction) => void;
}

export function NavRail({ state, dispatch }: NavRailProps) {
  const totalUnread = state.threads.reduce(
    (sum, t) => sum + threadUnreadCount(t, state.unreadCounts),
    0,
  );
  const isDark = (state.settings?.theme_mode ?? "dark") !== "light";

  const toggleTheme = () => {
    if (!state.settings) return;
    const next = { ...state.settings, theme_mode: isDark ? ("light" as const) : ("dark" as const) };
    dispatch({ type: "UPDATE_SETTINGS_LOCAL", settings: next });
    void ipc.saveSettings(next);
  };

  const openCloudThread = () => {
    const cloud = state.threads.find((t) => !t.is_group && t.name.includes("Cloud của tôi"));
    if (cloud) {
      dispatch({ type: "SELECT_THREAD", threadId: cloud.id });
    } else {
      dispatch({ type: "SHOW_TOAST", message: "Chưa tìm thấy hội thoại Cloud của tôi" });
    }
  };

  return (
    <aside className="nav-rail" aria-label="Thanh điều hướng chính">
      <div className="nav-group">
        <button
          type="button"
          className="nav-btn"
          title={`${state.displayName || "Tài khoản"} — Cài đặt tài khoản`}
          onClick={() => dispatch({ type: "OPEN_SETTINGS", open: true, tab: "account" })}
        >
          <Avatar
            id={state.uin || "self"}
            name={state.displayName || "Me"}
            avatarUrl={state.avatar}
            size={36}
            mediaCache={state.mediaCache}
          />
        </button>

        <button
          type="button"
          className={`nav-btn ${state.activeSection === "chat" ? "active" : ""}`}
          title="Tin nhắn"
          onClick={() => dispatch({ type: "SET_NAV_SECTION", section: "chat" })}
        >
          <MessageSquare size={20} />
          {totalUnread > 0 ? (
            <span className="nav-badge">{totalUnread > 99 ? "99+" : totalUnread}</span>
          ) : null}
        </button>

        <button
          type="button"
          className={`nav-btn ${state.activeSection === "contacts" ? "active" : ""}`}
          title="Danh bạ"
          onClick={() => dispatch({ type: "SET_NAV_SECTION", section: "contacts" })}
        >
          <Users size={20} />
        </button>

        <button
          type="button"
          className="nav-btn"
          title="Cloud của tôi"
          onClick={openCloudThread}
        >
          <Cloud size={20} />
        </button>

        <button
          type="button"
          className="nav-btn"
          title="Đồng bộ tin nhắn từ điện thoại"
          onClick={() => dispatch({ type: "SET_PHONE_SYNC_MODAL", open: true })}
        >
          <Smartphone size={20} />
        </button>
      </div>

      <div className="nav-group">
        <button
          type="button"
          className="nav-btn"
          title="Làm mới hội thoại từ máy chủ"
          onClick={() => void ipc.refreshThreads(true)}
        >
          <RefreshCw size={18} />
        </button>

        <button
          type="button"
          className="nav-btn"
          title={isDark ? "Chuyển sang giao diện Sáng" : "Chuyển sang giao diện Tối"}
          onClick={toggleTheme}
        >
          {isDark ? <Sun size={18} /> : <Moon size={18} />}
        </button>

        <button
          type="button"
          className="nav-btn"
          title="Kiểm tra nhật ký & giao thức (DevTools)"
          onClick={() => dispatch({ type: "SET_DEV_MODAL", open: true })}
        >
          <Bug size={18} />
        </button>

        <button
          type="button"
          className="nav-btn"
          title="Cài đặt (Ctrl+Shift+P mở Quyền riêng tư)"
          onClick={() => dispatch({ type: "OPEN_SETTINGS", open: true, tab: "general" })}
        >
          <Settings size={19} />
        </button>
      </div>
    </aside>
  );
}
