import { useState } from "react";
import {
  ChevronDown,
  ClipboardList,
  MoreHorizontal,
  Search,
  UserPlus,
  Users,
  UserCheck,
  ArrowUpDown,
  Filter,
} from "lucide-react";
import { ipc } from "@/shared/api/ipc";
import { Avatar } from "@/shared/ui/Avatar";
import { cn } from "@/shared/utils/cn";
import { foldVnSearch, matchesVnQuery } from "@/shared/utils/text";
import type { AppAction, AppState, ContactsSubTab } from "@/app/store/appReducer";
import type { Thread } from "@/shared/types/ipc";

interface ContactsViewProps {
  state: AppState;
  dispatch: (action: AppAction) => void;
}

// Nav items in the left column
const CONTACT_TABS: { id: ContactsSubTab; label: string; icon: React.ComponentType<{ className?: string }> }[] = [
  { id: "friends",         label: "Danh sách bạn bè",               icon: UserCheck },
  { id: "groups",          label: "Danh sách nhóm và cộng đồng",    icon: Users },
  { id: "friend_requests", label: "Lời mời kết bạn",                icon: UserPlus },
  { id: "group_invites",   label: "Lời mời vào nhóm và cộng đồng", icon: ClipboardList },
];

// Header display label for each sub-tab
const TAB_HEADER: Record<ContactsSubTab, string> = {
  friends:         "Danh sách bạn bè",
  groups:          "Danh sách nhóm và cộng đồng",
  friend_requests: "Lời mời kết bạn",
  group_invites:   "Lời mời vào nhóm và cộng đồng",
};

// Caption prefix for each sub-tab
const TAB_CAPTION: Record<ContactsSubTab, string> = {
  friends:         "Bạn bè",
  groups:          "Nhóm và cộng đồng",
  friend_requests: "Yêu cầu nhận được",
  group_invites:   "Lời mời nhóm",
};

type SortKey = "name_asc" | "name_desc" | "recent";

/** Strip Vietnamese diacritics and return uppercase first letter for grouping */
function groupLetter(name: string): string {
  const folded = foldVnSearch(name.trim());
  const firstChar = folded.charAt(0).toUpperCase();
  if (!firstChar) return "#";
  return /[A-Z]/.test(firstChar) ? firstChar : "#";
}

export function ContactsView({ state, dispatch }: ContactsViewProps) {
  const sub = state.contactsSubTab;

  // Per-tab search query (local state)
  const [localSearch, setLocalSearch] = useState("");
  const [userSortKey, setUserSortKey] = useState<SortKey | null>(null);
  const [filterVal, setFilterVal] = useState("all");

  const sortKey: SortKey = userSortKey ?? (sub === "groups" ? "recent" : "name_asc");

  // Build items list based on the active sub-tab
  const rawItems = state.threads.filter((t) => {
    if (sub === "friend_requests") {
      return !t.is_group && state.friendStatusMap[t.id] === "IncomingPending";
    }
    if (sub === "group_invites") {
      // group_invites come from a separate IPC source; always empty here
      return false;
    }
    if (sub === "groups") {
      return t.is_group && !t.name.includes("Cloud của tôi");
    }
    // friends: non-group, not incoming pending, not Cloud của tôi
    return (
      !t.is_group &&
      !t.name.includes("Cloud của tôi") &&
      state.friendStatusMap[t.id] !== "IncomingPending"
    );
  });

  // Apply search filter (from local search input or left sidebar search input)
  const searchQuery = (localSearch || state.contactsSearchQuery).trim();
  const searched = searchQuery
    ? rawItems.filter((t) => matchesVnQuery(t.name, searchQuery))
    : rawItems;
  // Sort
  const sorted = [...searched].sort((a, b) => {
    if (sortKey === "name_asc") return foldVnSearch(a.name).localeCompare(foldVnSearch(b.name));
    if (sortKey === "name_desc") return foldVnSearch(b.name).localeCompare(foldVnSearch(a.name));
    // recent: last_message_ts descending
    return (b.presentation.last_message_ts ?? 0) - (a.presentation.last_message_ts ?? 0);
  });

  const headerIcon = (() => {
    const tab = CONTACT_TABS.find((t) => t.id === sub);
    if (!tab) return null;
    const Icon = tab.icon;
    return <Icon className="h-[18px] w-[18px] text-muted-foreground" />;
  })();

  return (
    <div className="flex min-h-0 min-w-0 flex-1">
      {/* Left nav column — 345px */}
      <aside className="flex h-full w-[345px] shrink-0 flex-col border-r border-border bg-background">
        {/* Top row: 64px, search + add-friend + create-group */}
        <div className="flex h-16 shrink-0 items-center gap-1.5 px-3">
          {/* Search input 240×32 */}
          <div className="flex h-8 w-[240px] shrink-0 items-center gap-1.5 rounded-[5px] bg-canvas px-2.5">
            <Search className="h-3.5 w-3.5 shrink-0 text-muted-foreground" />
            <input
              value={state.contactsSearchQuery}
              onChange={(e) => dispatch({ type: "SET_CONTACTS_SEARCH", query: e.target.value })}
              placeholder="Tìm kiếm"
              className="min-w-0 flex-1 bg-transparent text-[13px] text-foreground placeholder:text-muted-foreground focus:outline-none"
            />
          </div>
          {/* Add friend button (opens PhoneLookupModal) */}
          <button
            type="button"
            title="Thêm bạn"
            onClick={() => dispatch({ type: "SET_PHONE_MODAL", open: true })}
            className="flex h-8 w-8 items-center justify-center rounded-lg text-muted-foreground transition-colors hover:bg-foreground/5 hover:text-foreground"
          >
            <UserPlus className="h-5 w-5" />
          </button>
          {/* Create group button */}
          <button
            type="button"
            title="Tạo nhóm"
            className="flex h-8 w-8 items-center justify-center rounded-lg text-muted-foreground transition-colors hover:bg-foreground/5 hover:text-foreground"
          >
            <Users className="h-5 w-5" />
          </button>
        </div>

        {/* Nav items */}
        <nav className="flex flex-col gap-0.5 px-2 py-1">
          {CONTACT_TABS.map((tab) => {
            const Icon = tab.icon;
            const active = sub === tab.id;
            return (
              <button
                key={tab.id}
                type="button"
                onClick={() => {
                  dispatch({ type: "SET_CONTACTS_SUBTAB", tab: tab.id });
                  setLocalSearch("");
                  setUserSortKey(null);
                }}
                className={cn(
                  "flex h-12 w-full cursor-pointer items-center gap-3 rounded-[6px] px-3 text-left text-sm font-normal transition-colors",
                  active
                    ? "bg-accent text-foreground"
                    : "text-foreground hover:bg-foreground/5",
                )}
              >
                <Icon className={cn("h-5 w-5 shrink-0", active ? "text-foreground" : "text-muted-foreground")} />
                <span className="truncate">{tab.label}</span>
              </button>
            );
          })}
        </nav>
      </aside>

      {/* Right main area */}
      <main className="flex min-w-0 flex-1 flex-col bg-canvas">
        {/* Header: 52px */}
        <header className="flex h-[52px] shrink-0 items-center gap-2.5 border-b border-border bg-background px-4">
          {headerIcon}
          <span className="text-base font-medium text-foreground">{TAB_HEADER[sub]}</span>
        </header>

        {/* Body (scrollable) */}
        <div className="flex-1 overflow-y-auto px-4 py-3">
          {sub === "group_invites" ? (
            <GroupInvitesEmpty />
          ) : sub === "friend_requests" ? (
            <FriendRequestsPanel items={sorted} state={state} dispatch={dispatch} />
          ) : (
            <ContactListPanel
              sub={sub}
              items={sorted}
              state={state}
              localSearch={localSearch}
              onSearch={setLocalSearch}
              sortKey={sortKey}
              onSort={setUserSortKey}
              filterVal={filterVal}
              onFilter={setFilterVal}
              dispatch={dispatch}
            />
          )}
        </div>
      </main>
    </div>
  );
}

// ─── Friends / Groups list panel ─────────────────────────────────────────────

interface ContactListPanelProps {
  sub: ContactsSubTab;
  items: Thread[];
  state: AppState;
  localSearch: string;
  onSearch: (q: string) => void;
  sortKey: SortKey;
  onSort: (k: SortKey) => void;
  filterVal: string;
  onFilter: (v: string) => void;
  dispatch: (action: AppAction) => void;
}

function ContactListPanel({
  sub,
  items,
  state,
  localSearch,
  onSearch,
  sortKey,
  onSort,
  dispatch,
}: ContactListPanelProps) {
  const isGroups = sub === "groups";
  const captionLabel = TAB_CAPTION[sub];

  // Build grouped sections (friends only; groups are flat)
  const sections: { letter: string; rows: Thread[] }[] = [];
  if (!isGroups) {
    const map = new Map<string, Thread[]>();
    for (const item of items) {
      const letter = groupLetter(item.name);
      let list = map.get(letter);
      if (!list) {
        list = [];
        map.set(letter, list);
      }
      list.push(item);
    }
    for (const [letter, rows] of [...map.entries()].sort((a, b) => a[0].localeCompare(b[0]))) {
      sections.push({ letter, rows });
    }
  }

  return (
    <>
      {/* Caption */}
      <div className="mb-3 text-[14px] font-medium text-foreground">
        {captionLabel} ({items.length})
      </div>

      {/* Card */}
      <div className="rounded-lg bg-card">
        {/* Toolbar row */}
        <div className="flex items-center gap-2 px-3 py-2.5">
          {/* Search */}
          <div className="flex h-8 flex-1 items-center gap-1.5 rounded-[5px] bg-canvas px-2.5">
            <Search className="h-3.5 w-3.5 shrink-0 text-muted-foreground" />
            <input
              value={localSearch}
              onChange={(e) => onSearch(e.target.value)}
              placeholder={isGroups ? "Tìm kiếm..." : "Tìm bạn"}
              className="min-w-0 flex-1 bg-transparent text-[13px] text-foreground placeholder:text-muted-foreground focus:outline-none"
            />
          </div>
          {/* Sort select */}
          <div className="relative flex h-8 items-center gap-1.5 rounded-[5px] bg-canvas px-2.5">
            <ArrowUpDown className="h-3.5 w-3.5 shrink-0 text-muted-foreground" />
            <select
              value={sortKey}
              onChange={(e) => onSort(e.target.value as SortKey)}
              className="cursor-pointer appearance-none bg-transparent pr-5 text-[13px] text-foreground focus:outline-none"
            >
              {isGroups ? (
                <>
                  <option value="recent">Cập nhật gần nhất</option>
                  <option value="name_asc">Tên (A-Z)</option>
                  <option value="name_desc">Tên (Z-A)</option>
                </>
              ) : (
                <>
                  <option value="name_asc">Tên (A-Z)</option>
                  <option value="name_desc">Tên (Z-A)</option>
                </>
              )}
            </select>
            <ChevronDown className="pointer-events-none absolute right-1.5 h-3.5 w-3.5 text-muted-foreground" />
          </div>
          {/* Filter select */}
          <div className="relative flex h-8 items-center gap-1.5 rounded-[5px] bg-canvas px-2.5">
            <Filter className="h-3.5 w-3.5 shrink-0 text-muted-foreground" />
            <select
              className="cursor-pointer appearance-none bg-transparent pr-5 text-[13px] text-foreground focus:outline-none"
              defaultValue="all"
            >
              <option value="all">Tất cả</option>
            </select>
            <ChevronDown className="pointer-events-none absolute right-1.5 h-3.5 w-3.5 text-muted-foreground" />
          </div>
        </div>

        {/* Rows */}
        {items.length === 0 ? (
          <div className="px-4 py-8 text-center text-[13px] text-muted-foreground">
            Không có mục nào phù hợp
          </div>
        ) : isGroups ? (
          // Groups: flat list, no letter headings
          items.map((t) => (
            <GroupRow
              key={t.id}
              thread={t}
              state={state}
              dispatch={dispatch}
            />
          ))
        ) : (
          // Friends: alphabetical sections
          sections.map(({ letter, rows }) => (
            <div key={letter}>
              <div className="px-4 pb-1 pt-3 text-base font-medium text-foreground">{letter}</div>
              {rows.map((t) => (
                <FriendRow key={t.id} thread={t} state={state} dispatch={dispatch} />
              ))}
            </div>
          ))
        )}
      </div>
    </>
  );
}

// ─── Individual row components ────────────────────────────────────────────────

interface RowProps {
  thread: Thread;
  state: AppState;
  dispatch: (action: AppAction) => void;
}

function FriendRow({ thread, state, dispatch }: RowProps) {
  return (
    <div
      onClick={() => dispatch({ type: "SELECT_THREAD", threadId: thread.id })}
      className="group flex h-14 cursor-pointer items-center gap-3 px-4 hover:bg-foreground/5"
    >
      <Avatar
        id={thread.id}
        name={thread.name}
        avatarUrl={thread.avatar}
        isGroup={false}
        size={40}
        mediaCache={state.mediaCache}
      />
      <span className="min-w-0 flex-1 truncate text-[16px] font-medium text-foreground">
        {thread.name}
      </span>
      <button
        type="button"
        onClick={(e) => e.stopPropagation()}
        className="invisible flex h-7 w-7 items-center justify-center rounded-full text-muted-foreground transition-colors hover:bg-foreground/10 group-hover:visible"
        title="Tùy chọn"
      >
        <MoreHorizontal className="h-4 w-4" />
      </button>
    </div>
  );
}

function GroupRow({ thread, state, dispatch }: RowProps) {
  const memberCount = thread.presentation.member_count;
  return (
    <div
      onClick={() => dispatch({ type: "SELECT_THREAD", threadId: thread.id })}
      className="group flex h-[60px] cursor-pointer items-center gap-3 px-4 hover:bg-foreground/5"
    >
      <Avatar
        id={thread.id}
        name={thread.name}
        avatarUrl={thread.avatar}
        isGroup
        memberCount={memberCount ?? undefined}
        size={40}
        mediaCache={state.mediaCache}
      />
      <div className="min-w-0 flex-1">
        <div className="truncate text-[16px] font-medium text-foreground">{thread.name}</div>
        {memberCount != null && (
          <div className="text-[12px] text-muted-foreground">{memberCount} thành viên</div>
        )}
      </div>
      <button
        type="button"
        onClick={(e) => e.stopPropagation()}
        className="invisible flex h-7 w-7 items-center justify-center rounded-full text-muted-foreground transition-colors hover:bg-foreground/10 group-hover:visible"
        title="Tùy chọn"
      >
        <MoreHorizontal className="h-4 w-4" />
      </button>
    </div>
  );
}
// ─── Friend Requests panel ────────────────────────────────────────────────────

interface FriendRequestsPanelProps {
  items: Thread[];
  state: AppState;
  dispatch: (action: AppAction) => void;
}

function FriendRequestsPanel({ items, state }: FriendRequestsPanelProps) {
  return (
    <>
      <div className="mb-3 text-[14px] font-medium text-foreground">
        Yêu cầu nhận được ({items.length})
      </div>
      {items.length === 0 ? (
        <div className="rounded-lg bg-card px-4 py-8 text-center text-[13px] text-muted-foreground">
          Không có lời mời kết bạn nào
        </div>
      ) : (
        <div className="flex flex-col gap-3">
          {items.map((t) => (
            <FriendRequestCard key={t.id} thread={t} state={state} />
          ))}
        </div>
      )}
    </>
  );
}

interface FriendRequestCardProps {
  thread: Thread;
  state: AppState;
}

function FriendRequestCard({ thread, state }: FriendRequestCardProps) {
  return (
    <div className="w-[397px] rounded-lg bg-card p-3">
      {/* Top row: avatar + name + time + icon */}
      <div className="mb-2 flex items-start gap-3">
        <Avatar
          id={thread.id}
          name={thread.name}
          avatarUrl={thread.avatar}
          isGroup={false}
          size={40}
          mediaCache={state.mediaCache}
        />
        <div className="min-w-0 flex-1">
          <div className="text-[16px] font-medium text-foreground">{thread.name}</div>
          <div className="text-[13px] text-muted-foreground">
            {thread.time} - Từ số điện thoại
          </div>
        </div>
        <button type="button" className="mt-0.5 text-muted-foreground hover:text-foreground">
          <ClipboardList className="h-5 w-5" />
        </button>
      </div>

      {/* Message box (read-only textarea appearance) */}
      <div className="mb-3 min-h-[60px] rounded-md border border-border bg-canvas px-3 py-2 text-[13px] text-muted-foreground">
        Xin chào, mình là {thread.name}. Mình biết bạn qua số điện thoại.
      </div>

      {/* Action buttons */}
      <div className="flex gap-2">
        <button
          type="button"
          onClick={() => void ipc.rejectFriend(thread.id, thread.name)}
          className="flex h-8 flex-1 cursor-pointer items-center justify-center rounded-[6px] bg-secondary text-[14px] font-medium text-secondary-foreground transition-colors hover:bg-secondary/80"
        >
          Từ chối
        </button>
        <button
          type="button"
          onClick={() => void ipc.acceptFriend(thread.id, thread.name)}
          className="flex h-8 flex-1 cursor-pointer items-center justify-center rounded-[6px] bg-primary text-[14px] font-medium text-white transition-colors hover:bg-primary/90 dark:bg-[#00378a] dark:text-[#66a6ff] dark:hover:bg-primary dark:hover:text-white"
        >
          Đồng ý
        </button>
      </div>
    </div>
  );
}

// ─── Group Invites empty state ────────────────────────────────────────────────

function GroupInvitesEmpty() {
  return (
    <div className="flex flex-col items-center justify-center py-20">
      {/* 104px circle icon */}
      <div className="mb-5 flex h-[104px] w-[104px] items-center justify-center rounded-full bg-secondary">
        <ClipboardList className="h-12 w-12 text-muted-foreground" />
      </div>
      <div className="mb-1 text-[14px] font-medium text-foreground">
        Chưa có lời mời vào nhóm và cộng đồng nào
      </div>
      <div className="text-[13px] text-muted-foreground">
        Khi nào bạn được mời,{" "}
        <span className="cursor-pointer text-primary hover:underline">Tìm hiểu thêm</span>
      </div>
    </div>
  );
}
