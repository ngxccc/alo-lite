import { Search, UserCheck, UserPlus, Users } from "lucide-react";
import { ipc } from "../../../shared/api/ipc";
import { Avatar } from "../../../shared/ui/Avatar";
import { foldVnSearch, matchesVnQuery } from "../../../shared/utils/text";
import type { AppAction, AppState, ContactsSubTab } from "../../../app/store/appReducer";

interface ContactsViewProps {
  state: AppState;
  dispatch: (action: AppAction) => void;
}

const CONTACT_TABS: Array<{ id: ContactsSubTab; label: string; kind: "user" | "group" }> = [
  { id: "friends", label: "Danh sách bạn bè", kind: "user" },
  { id: "groups", label: "Danh sách nhóm và cộng đồng", kind: "group" },
  { id: "friend_requests", label: "Lời mời kết bạn", kind: "user" },
  { id: "group_invites", label: "Lời mời vào nhóm và cộng đồng", kind: "group" },
];

export function ContactsView({ state, dispatch }: ContactsViewProps) {
  const sub = state.contactsSubTab;
  const wantGroup = sub === "groups";

  const items = state.threads
    .filter((t) => {
      if (sub === "friend_requests") {
        return !t.is_group && state.friendStatusMap[t.id] === "IncomingPending";
      }
      if (sub === "group_invites") return false;
      return (
        t.is_group === wantGroup &&
        !t.name.includes("Cloud của tôi") &&
        matchesVnQuery(t.name, state.contactsSearchQuery)
      );
    })
    .sort((a, b) => foldVnSearch(a.name).localeCompare(foldVnSearch(b.name)));

  return (
    <div style={{ flex: 1, display: "flex", minWidth: 0, minHeight: 0 }}>
      <aside className="sidebar" style={{ padding: 12, gap: 10 }}>
        <div className="search-row">
          <div className="search-box">
            <Search size={14} color="var(--text-muted)" />
            <input
              value={state.contactsSearchQuery}
              onChange={(e) => dispatch({ type: "SET_CONTACTS_SEARCH", query: e.target.value })}
              placeholder="Tìm kiếm danh bạ…"
            />
          </div>
          <button
            type="button"
            className="icon-btn"
            title="Thêm bạn qua số điện thoại"
            onClick={() => dispatch({ type: "SET_PHONE_MODAL", open: true })}
          >
            <UserPlus size={16} />
          </button>
        </div>

        <div style={{ display: "flex", flexDirection: "column", gap: 4 }}>
          {CONTACT_TABS.map((tab) => (
            <button
              key={tab.id}
              type="button"
              className={`settings-nav-item ${sub === tab.id ? "active" : ""}`}
              onClick={() => dispatch({ type: "SET_CONTACTS_SUBTAB", tab: tab.id })}
            >
              {tab.kind === "group" ? <Users size={16} /> : <UserCheck size={16} />}
              <span>{tab.label}</span>
            </button>
          ))}
        </div>
      </aside>

      <main className="chat-main">
        <header className="chat-header">
          <div style={{ fontWeight: 700, fontSize: 15 }}>
            {CONTACT_TABS.find((t) => t.id === sub)?.label} ({items.length})
          </div>
          <button
            type="button"
            className="btn"
            onClick={() => dispatch({ type: "SET_PHONE_MODAL", open: true })}
          >
            <UserPlus size={14} /> Thêm bạn qua SĐT
          </button>
        </header>

        <div style={{ flex: 1, overflowY: "auto", padding: 20 }}>
          <div className="settings-box" style={{ maxWidth: 760, margin: "0 auto" }}>
            {items.length === 0 ? (
              <div style={{ padding: 28, textAlign: "center", color: "var(--text-muted)" }}>
                Không có mục nào phù hợp
              </div>
            ) : (
              items.map((t) => (
                <div
                  key={t.id}
                  className="thread-row"
                  onClick={() => dispatch({ type: "SELECT_THREAD", threadId: t.id })}
                >
                  <Avatar
                    id={t.id}
                    name={t.name}
                    avatarUrl={t.avatar}
                    isGroup={t.is_group}
                    size={36}
                    mediaCache={state.mediaCache}
                  />
                  <div style={{ flex: 1, minWidth: 0 }}>
                    <div style={{ fontWeight: 600 }}>{t.name || t.id}</div>
                    <div style={{ fontSize: 11.8, color: "var(--text-muted)" }}>
                      {t.is_group ? "Nhóm trò chuyện" : "Bạn bè / Hội thoại"}
                    </div>
                  </div>
                  {sub === "friend_requests" ? (
                    <div style={{ display: "flex", gap: 6 }} onClick={(e) => e.stopPropagation()}>
                      <button
                        type="button"
                        className="btn btn-primary"
                        onClick={() => void ipc.acceptFriend(t.id, t.name)}
                      >
                        Đồng ý
                      </button>
                      <button
                        type="button"
                        className="btn"
                        onClick={() => void ipc.rejectFriend(t.id, t.name)}
                      >
                        Từ chối
                      </button>
                    </div>
                  ) : null}
                </div>
              ))
            )}
          </div>
        </div>
      </main>
    </div>
  );
}
