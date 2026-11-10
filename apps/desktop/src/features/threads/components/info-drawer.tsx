import { useState } from "react";
import {
  Bell,
  BellOff,
  ChevronDown,
  ChevronRight,
  Clock,
  FileText,
  Pencil,
  Pin,
  Shield,
  Trash2,
  UserCheck,
  UserPlus,
  Users,
  UserX,
} from "lucide-react";
import { ipc } from "@/shared/api/ipc";
import { Avatar } from "@/shared/ui/avatar";
import { Button } from "@/shared/ui/button";
import { cn } from "@/shared/utils/cn";
import { jxlToJpgUrl } from "@/shared/utils/text";
import type { AppAction, AppState } from "@/app/store/app-reducer";
import { useAppStore } from "@/app/store/use-app-store";
import { isPhotoMessage } from "@/features/chat/utils/messages";
import { isThreadMuted, threadMemberCount } from "@/features/threads/utils/threads";

interface InfoDrawerProps {
  state?: AppState;
  dispatch?: (action: AppAction) => void;
}

/** 8-px bg-canvas band that separates sections visually */
function SectionBand() {
  return <div className="h-2 shrink-0 bg-canvas" />;
}

/** Collapsible section header row: 16px/500, chevron on right */
function SectionHeader({
  label,
  expanded,
  onToggle,
  trailing,
}: {
  label: string;
  expanded: boolean;
  onToggle: () => void;
  trailing?: React.ReactNode;
}) {
  return (
    <button
      type="button"
      onClick={onToggle}
      className="flex w-full cursor-pointer items-center justify-between px-4 py-3 text-left hover:opacity-80"
    >
      <span className="text-base font-medium text-foreground">{label}</span>
      <div className="flex items-center gap-1.5 text-muted-foreground">
        {trailing}
        {expanded ? <ChevronDown className="h-4 w-4" /> : <ChevronRight className="h-4 w-4" />}
      </div>
    </button>
  );
}

/** Full-width "Xem tất cả" button: bg-secondary, 16px/500 */
function ViewAllButton({ onClick }: { onClick?: () => void }) {
  return (
    <button
      type="button"
      onClick={onClick}
      className="mx-4 mb-3 w-[calc(100%-2rem)] cursor-pointer rounded-md bg-secondary py-1.5 text-center text-base font-medium text-foreground transition-colors hover:bg-secondary/70"
    >
      Xem tất cả
    </button>
  );
}

export function InfoDrawer({ state: propState, dispatch: propDispatch }: InfoDrawerProps = {}) {
  const appStore = useAppStore();
  const state = propState ?? appStore;
  const dispatch = propDispatch ?? appStore.dispatch;
  const [membersExpanded, setMembersExpanded] = useState(true);
  const [boardExpanded, setBoardExpanded] = useState(true);
  const [photosExpanded, setPhotosExpanded] = useState(true);
  const [fileExpanded, setFileExpanded] = useState(true);
  const [linkExpanded, setLinkExpanded] = useState(true);

  const thread = state.threads.find((t) => t.id === state.selectedThreadId);
  if (!state.infoDrawerOpen || !thread) return null;

  const isCloud = thread.name.includes("Cloud của tôi");
  const friendStatus = state.friendStatusMap[thread.id] ?? "Unknown";
  const online = state.userOnlineMap[thread.id];
  const muted = state.settings ? isThreadMuted(thread, state.settings) : false;
  const pinned =
    thread.presentation.pinned || (state.settings?.pinned_threads.includes(thread.id) ?? false);
  const memberCount = threadMemberCount(thread);

  const sharedPhotos = thread.messages
    .filter((m) => isPhotoMessage(m) && Boolean(m.link_thumb ?? m.link_url))
    .slice(-8)
    .toReversed();

  const handleTogglePin = () => {
    if (state.settings) {
      const nextPinned = pinned
        ? state.settings.pinned_threads.filter((id) => id !== thread.id)
        : [...state.settings.pinned_threads, thread.id];
      const nextSettings = {
        ...state.settings,
        pinned_threads: nextPinned,
      };
      dispatch({
        type: "UPDATE_SETTINGS_LOCAL",
        settings: nextSettings,
      });
      void ipc.saveSettings(nextSettings);
    }
    void ipc.pinThread(thread.id, thread.is_group, !pinned);
  };

  const handleToggleMute = () => {
    if (state.settings) {
      const nextMuted = { ...state.settings.muted_threads };
      if (muted) delete nextMuted[thread.id];
      else nextMuted[thread.id] = -1;
      const nextSettings = {
        ...state.settings,
        muted_threads: nextMuted,
      };
      dispatch({
        type: "UPDATE_SETTINGS_LOCAL",
        settings: nextSettings,
      });
      void ipc.saveSettings(nextSettings);
    }
    void ipc.muteThread(thread.id, thread.is_group, muted ? 0 : -1, !muted);
  };

  /** Circular action button used in the quick-action row */
  const ActionBtn = ({
    icon,
    label,
    onClick,
    active = false,
  }: {
    icon: React.ReactNode;
    label: string;
    onClick?: () => void;
    active?: boolean;
  }) => (
    <button
      type="button"
      onClick={onClick}
      className="group flex cursor-pointer flex-col items-center gap-1.5"
    >
      <div
        className={cn(
          "flex h-10 w-10 items-center justify-center rounded-full transition-colors",
          active
            ? "bg-primary/20 text-primary"
            : "bg-secondary text-foreground/80 group-hover:bg-secondary/70",
        )}
      >
        {icon}
      </div>
      <span className="text-[12px] leading-tight text-muted-foreground group-hover:text-foreground">
        {label}
      </span>
    </button>
  );

  /* ------------------------------------------------------------------ */
  /* Action button sets per thread type                                   */
  /* ------------------------------------------------------------------ */
  const actionButtons = thread.is_group ? (
    // Group: 4 buttons
    <div className="grid grid-cols-4 gap-1 px-3 py-4">
      <ActionBtn
        icon={muted ? <BellOff className="h-5 w-5" /> : <Bell className="h-5 w-5" />}
        label={muted ? "Bật TB" : "Tắt thông báo"}
        onClick={handleToggleMute}
        active={muted}
      />
      <ActionBtn
        icon={<Pin className="h-5 w-5" />}
        label={pinned ? "Bỏ ghim" : "Ghim"}
        onClick={handleTogglePin}
        active={pinned}
      />
      <ActionBtn
        icon={<UserPlus className="h-5 w-5" />}
        label="Thêm thành viên"
        onClick={() => dispatch({ type: "SET_PHONE_MODAL", open: true })}
      />
      <ActionBtn
        icon={<Shield className="h-5 w-5" />}
        label="Quản lý nhóm"
        onClick={() => dispatch({ type: "OPEN_SETTINGS", open: true, tab: "general" })}
      />
    </div>
  ) : (
    // 1-on-1 (non-cloud): 3 buttons
    <div className="grid grid-cols-3 gap-1 px-3 py-4">
      <ActionBtn
        icon={muted ? <BellOff className="h-5 w-5" /> : <Bell className="h-5 w-5" />}
        label={muted ? "Bật TB" : "Tắt thông báo"}
        onClick={handleToggleMute}
        active={muted}
      />
      <ActionBtn
        icon={<Pin className="h-5 w-5" />}
        label={pinned ? "Bỏ ghim" : "Ghim"}
        onClick={handleTogglePin}
        active={pinned}
      />
      <ActionBtn
        icon={<Users className="h-5 w-5" />}
        label="Tạo nhóm trò chuyện"
        onClick={() => dispatch({ type: "SET_PHONE_MODAL", open: true })}
      />
    </div>
  );

  return (
    <aside className="flex h-full w-[343px] shrink-0 flex-col overflow-y-auto border-l border-border bg-background text-foreground">
      {/* Header: 68px, centered title 18px/500 */}
      <div className="flex h-[68px] shrink-0 items-center justify-center border-b border-border">
        <span className="text-[18px] font-medium text-foreground">
          {thread.is_group ? "Thông tin nhóm" : "Thông tin hội thoại"}
        </span>
      </div>

      {/* Avatar + Name row */}
      <div className="flex flex-col items-center gap-1 px-4 pt-5 pb-4 text-center">
        <Avatar
          id={thread.id}
          name={thread.name}
          avatarUrl={thread.avatar}
          isGroup={thread.is_group}
          memberCount={thread.is_group ? threadMemberCount(thread) : undefined}
          size={56}
          mediaCache={state.mediaCache}
        />
        <div className="mt-2 flex items-center gap-1.5">
          <span className="max-w-[220px] truncate text-[18px] font-medium text-foreground">
            {thread.name || thread.id}
          </span>
          <button
            type="button"
            title="Chỉnh sửa tên"
            className="inline-flex h-6 w-6 cursor-pointer items-center justify-center rounded text-muted-foreground hover:text-foreground"
          >
            <Pencil className="h-3.5 w-3.5" />
          </button>
        </div>

        {/* Subtitle */}
        <div className="text-[13px] text-muted-foreground">
          {thread.is_group
            ? `${memberCount} thành viên`
            : isCloud
              ? "Lưu trữ đám mây cá nhân"
              : friendStatus === "Friend"
                ? `Đã kết bạn${online?.[1] ? ` · ${online[1]}` : ""}`
                : friendStatus === "Stranger"
                  ? "Người lạ · Chưa kết bạn"
                  : "Đang kiểm tra…"}
        </div>

        {/* Friend action buttons (non-group, non-cloud, not Friend) */}
        {!thread.is_group && !isCloud && friendStatus !== "Friend" ? (
          <div className="mt-2 flex flex-wrap justify-center gap-1.5">
            {friendStatus === "Stranger" ? (
              <Button
                variant="default"
                size="sm"
                onClick={() => void ipc.addFriend(thread.id, thread.name)}
              >
                <UserPlus className="h-3.5 w-3.5" /> Kết bạn
              </Button>
            ) : null}
            {friendStatus === "IncomingPending" ? (
              <>
                <Button
                  variant="default"
                  size="sm"
                  onClick={() => void ipc.acceptFriend(thread.id, thread.name)}
                >
                  <UserCheck className="h-3.5 w-3.5" /> Đồng ý
                </Button>
                <Button
                  variant="outline"
                  size="sm"
                  onClick={() => void ipc.rejectFriend(thread.id, thread.name)}
                >
                  <UserX className="h-3.5 w-3.5" /> Từ chối
                </Button>
              </>
            ) : null}
            {friendStatus === "OutgoingPending" ? (
              <Button
                variant="outline"
                size="sm"
                onClick={() => void ipc.undoFriend(thread.id, thread.name)}
              >
                Thu hồi lời mời
              </Button>
            ) : null}
          </div>
        ) : null}
      </div>

      {/* Quick action buttons */}
      <div className="border-b border-border/60">{actionButtons}</div>

      {/* ---- Sections ---- */}

      {/* Group-specific sections: Thành viên nhóm + Bảng tin nhóm */}
      {thread.is_group ? (
        <>
          {/* Thành viên nhóm */}
          <SectionBand />
          <div className="bg-background">
            <SectionHeader
              label="Thành viên nhóm"
              expanded={membersExpanded}
              onToggle={() => setMembersExpanded((p) => !p)}
            />
            {membersExpanded ? (
              <div className="flex flex-col px-4 pb-2">
                <button
                  type="button"
                  className="flex cursor-pointer items-center gap-3 py-2.5 text-foreground hover:text-accent-foreground"
                >
                  <Users className="h-5 w-5 shrink-0 text-muted-foreground" />
                  <span className="text-[14px]">{memberCount} thành viên</span>
                </button>
                {thread.presentation.members.length > 0 ? (
                  <div className="flex max-h-48 flex-col gap-0.5 overflow-y-auto pt-1 pb-1">
                    {thread.presentation.members.map((m) => (
                      <div key={m.id} className="flex items-center gap-2.5 py-1">
                        <Avatar
                          id={m.id}
                          name={m.name}
                          avatarUrl={m.avatar}
                          size={32}
                          mediaCache={state.mediaCache}
                        />
                        <span className="truncate text-[14px] text-foreground">
                          {m.name || m.id}
                        </span>
                      </div>
                    ))}
                  </div>
                ) : null}
              </div>
            ) : null}
          </div>

          {/* Bảng tin nhóm */}
          <SectionBand />
          <div className="bg-background">
            <SectionHeader
              label="Bảng tin nhóm"
              expanded={boardExpanded}
              onToggle={() => setBoardExpanded((p) => !p)}
            />
            {boardExpanded ? (
              <div className="flex flex-col px-4 pb-3">
                <button
                  type="button"
                  className="flex cursor-pointer items-center gap-3 py-2.5 text-foreground hover:text-accent-foreground"
                >
                  <Clock className="h-5 w-5 shrink-0 text-muted-foreground" />
                  <span className="text-[14px]">Danh sách nhắc hẹn</span>
                </button>
                <button
                  type="button"
                  className="flex cursor-pointer items-center gap-3 py-2.5 text-foreground hover:text-accent-foreground"
                >
                  <FileText className="h-5 w-5 shrink-0 text-muted-foreground" />
                  <span className="text-[14px]">Ghi chú, ghim, bình chọn</span>
                </button>
              </div>
            ) : null}
          </div>
        </>
      ) : (
        <>
          {/* 1:1 chat reminders */}
          <SectionBand />
          <div className="flex flex-col bg-background px-4 py-1.5">
            <button
              type="button"
              className="flex cursor-pointer items-center gap-3 py-2.5 text-foreground hover:text-accent-foreground"
            >
              <Clock className="h-5 w-5 shrink-0 text-muted-foreground" />
              <span className="text-[14px]">Danh sách nhắc hẹn</span>
            </button>
          </div>
        </>
      )}
      {/* Ảnh / Video */}
      <SectionBand />
      <div className="bg-background">
        <SectionHeader
          label="Ảnh/Video"
          expanded={photosExpanded}
          onToggle={() => setPhotosExpanded((p) => !p)}
        />
        {photosExpanded ? (
          <>
            {sharedPhotos.length === 0 ? (
              <p className="px-4 pb-3 text-center text-[13px] text-muted-foreground">
                Chưa có hình ảnh nào
              </p>
            ) : (
              <>
                <div className="grid grid-cols-4 gap-1 px-4 pb-2">
                  {sharedPhotos.map((m) => {
                    const key = `img_${m.id || m.ts}`;
                    const fullUrl = m.link_url ?? m.link_thumb ?? "";
                    const thumbSrc =
                      state.mediaCache[key] ?? jxlToJpgUrl(m.link_thumb ?? m.link_url ?? "");
                    return (
                      <div
                        key={key}
                        onClick={() =>
                          dispatch({
                            type: "SET_IMAGE_VIEWER",
                            viewer: {
                              texKey: key,
                              fullUrl,
                              title: m.link_title || "Hình ảnh",
                              sender: m.sender,
                              time: m.time,
                            },
                          })
                        }
                        className="h-[60px] cursor-pointer overflow-hidden rounded bg-muted text-[10px] text-muted-foreground hover:opacity-90"
                      >
                        {thumbSrc ? (
                          <img
                            src={thumbSrc}
                            alt="Ảnh"
                            loading="lazy"
                            decoding="async"
                            className="h-full w-full object-cover"
                          />
                        ) : (
                          <div className="flex h-full items-center justify-center">Ảnh</div>
                        )}
                      </div>
                    );
                  })}
                </div>
                <ViewAllButton />
              </>
            )}
          </>
        ) : null}
      </div>

      {/* File */}
      <SectionBand />
      <div className="bg-background">
        <SectionHeader
          label="File"
          expanded={fileExpanded}
          onToggle={() => setFileExpanded((p) => !p)}
        />
        {fileExpanded ? (
          <>
            <p className="px-4 pb-2 text-[13px] text-muted-foreground">
              Chưa có file nào được chia sẻ
            </p>
            <ViewAllButton />
          </>
        ) : null}
      </div>

      {/* Liên kết */}
      <SectionBand />
      <div className="bg-background">
        <SectionHeader
          label="Liên kết"
          expanded={linkExpanded}
          onToggle={() => setLinkExpanded((p) => !p)}
        />
        {linkExpanded ? (
          <>
            <p className="px-4 pb-2 text-[13px] text-muted-foreground">
              Chưa có liên kết nào được chia sẻ
            </p>
            <ViewAllButton />
          </>
        ) : null}
      </div>

      {/* Delete conversation — quiet red text row */}
      <SectionBand />
      <button
        type="button"
        className="flex w-full cursor-pointer items-center justify-center gap-2 py-3.5 text-[14px] text-destructive hover:bg-destructive/5"
        onClick={() => {
          const last = thread.messages.at(-1);
          void ipc.deleteThread(
            thread.id,
            thread.is_group,
            last?.client_message_id ?? last?.id ?? "",
          );
        }}
      >
        <Trash2 className="h-4 w-4" />
        Xóa cuộc trò chuyện
      </button>
      <SectionBand />
    </aside>
  );
}
