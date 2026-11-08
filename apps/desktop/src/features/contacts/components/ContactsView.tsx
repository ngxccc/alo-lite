import { Search, UserCheck, UserPlus, Users } from "lucide-react";
import { ipc } from "@/shared/api/ipc";
import { Avatar } from "@/shared/ui/Avatar";
import { Button } from "@/shared/ui/button";
import { cn } from "@/shared/utils/cn";
import { foldVnSearch, matchesVnQuery } from "@/shared/utils/text";
import type { AppAction, AppState, ContactsSubTab } from "@/app/store/appReducer";

interface ContactsViewProps {
  state: AppState;
  dispatch: (action: AppAction) => void;
}

const CONTACT_TABS: {
  id: ContactsSubTab;
  label: string;
  kind: "user" | "group";
}[] = [
  { id: "friends", label: "Danh sách bạn bè", kind: "user" },
  { id: "groups", label: "Danh sách nhóm và cộng đồng", kind: "group" },
  { id: "friend_requests", label: "Lời mời kết bạn", kind: "user" },
  {
    id: "group_invites",
    label: "Lời mời vào nhóm và cộng đồng",
    kind: "group",
  },
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
    <div className="flex min-h-0 min-w-0 flex-1">
      <aside className="flex h-full w-[310px] shrink-0 flex-col gap-2.5 border-r border-border bg-sidebar p-3 text-sidebar-foreground">
        <div className="flex items-center gap-1.5">
          <div className="flex h-8 flex-1 items-center gap-1.5 rounded-md border border-border bg-muted/60 px-2.5 focus-within:border-ring focus-within:bg-background">
            <Search className="h-3.5 w-3.5 shrink-0 text-muted-foreground" />
            <input
              value={state.contactsSearchQuery}
              onChange={(e) => dispatch({ type: "SET_CONTACTS_SEARCH", query: e.target.value })}
              placeholder="Tìm kiếm danh bạ…"
              className="w-full bg-transparent text-xs text-foreground placeholder:text-muted-foreground focus:outline-none"
            />
          </div>
          <Button
            variant="ghost"
            size="icon"
            title="Thêm bạn qua số điện thoại"
            onClick={() => dispatch({ type: "SET_PHONE_MODAL", open: true })}
          >
            <UserPlus className="h-4 w-4" />
          </Button>
        </div>

        <div className="flex flex-col gap-1">
          {CONTACT_TABS.map((tab) => (
            <button
              key={tab.id}
              type="button"
              onClick={() => dispatch({ type: "SET_CONTACTS_SUBTAB", tab: tab.id })}
              className={cn(
                "flex cursor-pointer items-center gap-2.5 rounded-lg px-3 py-2 text-left text-xs font-medium transition-colors",
                sub === tab.id
                  ? "bg-accent font-semibold text-accent-foreground"
                  : "text-muted-foreground hover:bg-muted hover:text-foreground",
              )}
            >
              {tab.kind === "group" ? (
                <Users className="h-4 w-4 shrink-0" />
              ) : (
                <UserCheck className="h-4 w-4 shrink-0" />
              )}
              <span>{tab.label}</span>
            </button>
          ))}
        </div>
      </aside>

      <main className="flex min-w-0 flex-1 flex-col bg-canvas">
        <header className="flex h-14 shrink-0 items-center justify-between border-b border-border bg-sidebar px-4">
          <div className="text-sm font-bold text-foreground">
            {CONTACT_TABS.find((t) => t.id === sub)?.label} ({items.length})
          </div>
          <Button
            variant="outline"
            size="sm"
            onClick={() => dispatch({ type: "SET_PHONE_MODAL", open: true })}
          >
            <UserPlus className="h-3.5 w-3.5" /> Thêm bạn qua SĐT
          </Button>
        </header>

        <div className="flex-1 overflow-y-auto p-5">
          <div className="mx-auto flex max-w-3xl flex-col divide-y divide-border rounded-xl border border-border bg-card shadow-xs">
            {items.length === 0 ? (
              <div className="p-8 text-center text-xs text-muted-foreground">
                Không có mục nào phù hợp
              </div>
            ) : (
              items.map((t) => (
                <div
                  key={t.id}
                  onClick={() => dispatch({ type: "SELECT_THREAD", threadId: t.id })}
                  className="flex cursor-pointer items-center gap-3 px-4 py-2.5 hover:bg-muted/60"
                >
                  <Avatar
                    id={t.id}
                    name={t.name}
                    avatarUrl={t.avatar}
                    isGroup={t.is_group}
                    size={36}
                    mediaCache={state.mediaCache}
                  />
                  <div className="min-w-0 flex-1">
                    <div className="truncate text-xs font-semibold text-foreground">
                      {t.name || t.id}
                    </div>
                    <div className="text-[11px] text-muted-foreground">
                      {t.is_group ? "Nhóm trò chuyện" : "Bạn bè / Hội thoại"}
                    </div>
                  </div>
                  {sub === "friend_requests" ? (
                    <div className="flex gap-1.5" onClick={(e) => e.stopPropagation()}>
                      <Button
                        variant="default"
                        size="sm"
                        onClick={() => void ipc.acceptFriend(t.id, t.name)}
                      >
                        Đồng ý
                      </Button>
                      <Button
                        variant="outline"
                        size="sm"
                        onClick={() => void ipc.rejectFriend(t.id, t.name)}
                      >
                        Từ chối
                      </Button>
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
