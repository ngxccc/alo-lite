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
  UserX,
} from "lucide-react";
import { ipc } from "@/shared/api/ipc";
import { Avatar } from "@/shared/ui/Avatar";
import { Button } from "@/shared/ui/button";
import { cn } from "@/shared/utils/cn";
import { jxlToJpgUrl } from "@/shared/utils/text";
import type { AppAction, AppState } from "@/app/store/appReducer";
import { isPhotoMessage } from "@/features/chat/utils/messages";
import { isThreadMuted } from "@/features/threads/utils/threads";
interface InfoDrawerProps {
  state: AppState;
  dispatch: (action: AppAction) => void;
}

export function InfoDrawer({ state, dispatch }: InfoDrawerProps) {
  const [membersExpanded, setMembersExpanded] = useState(true);
  const thread = state.threads.find((t) => t.id === state.selectedThreadId);
  if (!state.infoDrawerOpen || !thread) return null;

  const isCloud = thread.name.includes("Cloud của tôi");
  const friendStatus = state.friendStatusMap[thread.id] ?? "Unknown";
  const online = state.userOnlineMap[thread.id];
  const muted = state.settings ? isThreadMuted(thread, state.settings) : false;
  const pinned =
    thread.presentation.pinned || (state.settings?.pinned_threads.includes(thread.id) ?? false);
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

  return (
    <aside className="flex h-full w-[320px] shrink-0 flex-col overflow-y-auto border-l border-border bg-sidebar text-sidebar-foreground">
      <div className="flex h-15 shrink-0 items-center justify-between border-b border-border px-4">
        <span className="text-[15px] font-bold text-foreground">
          {thread.is_group ? "Thông tin nhóm" : "Thông tin hội thoại"}
        </span>
      </div>

      <div className="flex flex-col items-center gap-1.5 px-4 pt-4 pb-3 text-center">
        <Avatar
          id={thread.id}
          name={thread.name}
          avatarUrl={thread.avatar}
          isGroup={thread.is_group}
          memberCount={
            thread.presentation.member_count ||
            (thread.is_group ? thread.presentation.members.length : undefined)
          }
          size={68}
          mediaCache={state.mediaCache}
        />
        <div className="flex items-center gap-1.5 mt-1">
          <span className="text-[15.5px] font-bold text-foreground truncate max-w-[220px]">
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
        <div className="text-xs text-muted-foreground">
          {thread.is_group
            ? `${thread.presentation.member_count || thread.presentation.members.length || 0} thành viên`
            : isCloud
              ? "Lưu trữ đám mây cá nhân"
              : friendStatus === "Friend"
                ? `Đã kết bạn${online?.[1] ? ` · ${online[1]}` : ""}`
                : friendStatus === "Stranger"
                  ? "Người lạ · Chưa kết bạn"
                  : "Đang kiểm tra…"}
        </div>

        {!thread.is_group && !isCloud && friendStatus !== "Friend" ? (
          <div className="flex flex-wrap justify-center gap-1.5 mt-2">
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

      {/* Quick Action Icons Row */}
      <div className="grid grid-cols-4 border-b border-border/70 px-2 py-3">
        <button
          type="button"
          onClick={handleToggleMute}
          className="flex flex-col items-center gap-1.5 cursor-pointer text-muted-foreground hover:text-foreground group"
        >
          <div
            className={cn(
              "flex h-9 w-9 items-center justify-center rounded-full transition-colors",
              muted ? "bg-primary/20 text-primary" : "bg-muted/70 group-hover:bg-muted text-foreground/80",
            )}
          >
            {muted ? <BellOff className="h-4.5 w-4.5" /> : <Bell className="h-4.5 w-4.5" />}
          </div>
          <span className="text-[11px] font-medium">{muted ? "Bật TB" : "Tắt TB"}</span>
        </button>

        <button
          type="button"
          onClick={handleTogglePin}
          className="flex flex-col items-center gap-1.5 cursor-pointer text-muted-foreground hover:text-foreground group"
        >
          <div
            className={cn(
              "flex h-9 w-9 items-center justify-center rounded-full transition-colors",
              pinned ? "bg-primary/20 text-primary" : "bg-muted/70 group-hover:bg-muted text-foreground/80",
            )}
          >
            <Pin className="h-4.5 w-4.5" />
          </div>
          <span className="text-[11px] font-medium">{pinned ? "Bỏ ghim" : "Ghim"}</span>
        </button>

        <button
          type="button"
          onClick={() => dispatch({ type: "SET_PHONE_MODAL", open: true })}
          className="flex flex-col items-center gap-1.5 cursor-pointer text-muted-foreground hover:text-foreground group"
        >
          <div className="flex h-9 w-9 items-center justify-center rounded-full bg-muted/70 text-foreground/80 transition-colors group-hover:bg-muted">
            <UserPlus className="h-4.5 w-4.5" />
          </div>
          <span className="text-[11px] font-medium">Thêm TV</span>
        </button>

        <button
          type="button"
          onClick={() => dispatch({ type: "OPEN_SETTINGS", open: true, tab: "general" })}
          className="flex flex-col items-center gap-1.5 cursor-pointer text-muted-foreground hover:text-foreground group"
        >
          <div className="flex h-9 w-9 items-center justify-center rounded-full bg-muted/70 text-foreground/80 transition-colors group-hover:bg-muted">
            <Shield className="h-4.5 w-4.5" />
          </div>
          <span className="text-[11px] font-medium">Quản lý</span>
        </button>
      </div>

      <div className="flex flex-col divide-y divide-border/60">
        {/* Group members section */}
        {thread.is_group ? (
          <div className="flex flex-col py-2.5 px-3.5">
            <button
              type="button"
              onClick={() => setMembersExpanded((prev) => !prev)}
              className="flex w-full cursor-pointer items-center justify-between text-xs font-semibold text-foreground hover:opacity-80"
            >
              <span>Thành viên nhóm</span>
              <div className="flex items-center gap-1 text-muted-foreground">
                <span>
                  {thread.presentation.member_count || thread.presentation.members.length || 0} thành viên
                </span>
                {membersExpanded ? (
                  <ChevronDown className="h-3.5 w-3.5" />
                ) : (
                  <ChevronRight className="h-3.5 w-3.5" />
                )}
              </div>
            </button>
            {membersExpanded && thread.presentation.members.length > 0 ? (
              <div className="mt-2.5 flex max-h-44 flex-col gap-1.5 overflow-y-auto pr-1">
                {thread.presentation.members.map((m) => (
                  <div key={m.id} className="flex items-center gap-2 text-xs py-0.5">
                    <Avatar
                      id={m.id}
                      name={m.name}
                      avatarUrl={m.avatar}
                      size={28}
                      mediaCache={state.mediaCache}
                    />
                    <span className="truncate text-foreground/90">{m.name || m.id}</span>
                  </div>
                ))}
              </div>
            ) : null}
          </div>
        ) : null}

        {/* Group board section */}
        <div className="flex flex-col py-2.5 px-3.5 gap-2">
          <span className="text-xs font-semibold text-foreground">Bảng tin nhóm</span>
          <div className="flex flex-col gap-1.5">
            <button
              type="button"
              className="flex items-center gap-2.5 py-1 text-xs text-muted-foreground hover:text-foreground cursor-pointer"
            >
              <Clock className="h-4 w-4 text-muted-foreground" />
              <span>Danh sách nhắc hẹn</span>
            </button>
            <button
              type="button"
              className="flex items-center gap-2.5 py-1 text-xs text-muted-foreground hover:text-foreground cursor-pointer"
            >
              <FileText className="h-4 w-4 text-muted-foreground" />
              <span>Ghi chú, ghim, bình chọn</span>
            </button>
          </div>
        </div>

        {/* Photos & Videos */}
        <div className="flex flex-col py-2.5 px-3.5 gap-2">
          <div className="flex items-center justify-between text-xs font-semibold text-foreground">
            <span>Ảnh / Video</span>
            <span className="text-muted-foreground font-normal">({sharedPhotos.length})</span>
          </div>
          {sharedPhotos.length === 0 ? (
            <div className="text-xs text-muted-foreground py-2 text-center">Chưa có hình ảnh nào</div>
          ) : (
            <>
              <div className="grid grid-cols-4 gap-1.5">
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
                      className="grid aspect-square cursor-pointer place-items-center overflow-hidden rounded-md border border-border/80 bg-muted text-[10px] text-muted-foreground hover:opacity-90"
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
                        "Ảnh"
                      )}
                    </div>
                  );
                })}
              </div>
              <button
                type="button"
                className="mt-1 w-full rounded-md bg-muted/60 py-1.5 text-center text-xs font-medium text-foreground hover:bg-muted transition cursor-pointer"
              >
                Xem tất cả
              </button>
            </>
          )}
        </div>

        {/* File and Link notices */}
        <div className="flex flex-col py-2.5 px-3.5 gap-2">
          <div className="flex flex-col gap-1">
            <span className="text-xs font-semibold text-foreground">Tập tin</span>
            <span className="text-[11px] text-muted-foreground">
              Sử dụng ứng dụng Zalo trên máy tính để xem tập tin lưu trữ
            </span>
          </div>
          <div className="flex flex-col gap-1 mt-1">
            <span className="text-xs font-semibold text-foreground">Liên kết</span>
            <span className="text-[11px] text-muted-foreground">
              Sử dụng ứng dụng Zalo trên máy tính để xem liên kết đã gửi
            </span>
          </div>
        </div>
      </div>

      <div className="mt-auto p-3.5">
        <Button
          variant="destructive"
          className="w-full justify-center text-xs h-9 cursor-pointer"
          onClick={() => {
            const last = thread.messages.at(-1);
            void ipc.deleteThread(
              thread.id,
              thread.is_group,
              last?.client_message_id ?? last?.id ?? "",
            );
          }}
        >
          <Trash2 className="h-3.5 w-3.5" /> Xóa cuộc trò chuyện
        </Button>
      </div>
    </aside>
  );
}
