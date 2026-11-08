import { BellOff, Pin, RefreshCw, Star, Trash2, UserCheck, UserPlus, UserX } from "lucide-react";
import { ipc } from "../../../shared/api/ipc";
import { Avatar } from "../../../shared/ui/Avatar";
import { Button } from "../../../shared/ui/button";
import { cn } from "../../../shared/utils/cn";
import { jxlToJpgUrl } from "../../../shared/utils/text";
import type { AppAction, AppState } from "../../../app/store/appReducer";
import { isPhotoMessage } from "../../chat/utils/messages";
import { isThreadMuted } from "../utils/threads";

interface InfoDrawerProps {
  state: AppState;
  dispatch: (action: AppAction) => void;
}

export function InfoDrawer({ state, dispatch }: InfoDrawerProps) {
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
    .slice(-9)
    .toReversed();

  return (
    <aside className="flex h-full w-[290px] shrink-0 flex-col gap-4 overflow-y-auto border-l border-border bg-sidebar p-3.5 text-sidebar-foreground">
      <div className="flex flex-col items-center gap-2 text-center">
        <Avatar
          id={thread.id}
          name={thread.name}
          avatarUrl={thread.avatar}
          isGroup={thread.is_group}
          size={64}
          mediaCache={state.mediaCache}
        />
        <div className="text-sm font-bold text-foreground">{thread.name || thread.id}</div>
        <div className="text-xs text-muted-foreground">
          {thread.is_group
            ? "Nhóm đang hoạt động"
            : isCloud
              ? "Lưu trữ đám mây cá nhân"
              : friendStatus === "Friend"
                ? `Đã kết bạn${online?.[1] ? ` · ${online[1]}` : ""}`
                : friendStatus === "IncomingPending"
                  ? "Đã gửi lời mời kết bạn cho bạn"
                  : friendStatus === "OutgoingPending"
                    ? "Đã gửi lời mời kết bạn"
                    : friendStatus === "Stranger"
                      ? "Người lạ · Chưa kết bạn"
                      : "Đang kiểm tra trạng thái…"}
        </div>

        {!thread.is_group && !isCloud ? (
          <div className="flex flex-wrap justify-center gap-1.5">
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

      <div className="grid grid-cols-2 gap-1.5">
        <Button
          variant="outline"
          size="sm"
          onClick={() => {
            dispatch({ type: "START_CLOUD_SYNC" });
            void ipc.syncHistory([[thread.id, thread.name, thread.is_group]]);
          }}
        >
          <RefreshCw className="h-3.5 w-3.5" /> Đồng bộ
        </Button>
        <Button
          variant={state.filterStarredOnly ? "accent" : "outline"}
          size="sm"
          onClick={() => dispatch({ type: "TOGGLE_STARRED_FILTER" })}
        >
          <Star className={cn("h-3.5 w-3.5", state.filterStarredOnly && "text-warning")} /> Tin gắn
          sao
        </Button>
        <Button
          variant={pinned ? "accent" : "outline"}
          size="sm"
          onClick={() => {
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
          }}
        >
          <Pin className="h-3.5 w-3.5" /> {pinned ? "Bỏ ghim" : "Ghim"}
        </Button>
        <Button
          variant={muted ? "accent" : "outline"}
          size="sm"
          onClick={() => {
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
          }}
        >
          <BellOff className="h-3.5 w-3.5" /> {muted ? "Bật TB" : "Tắt TB"}
        </Button>
      </div>

      {thread.is_group && thread.presentation.members.length > 0 ? (
        <div className="flex flex-col gap-2">
          <div className="text-xs font-bold text-foreground">
            Thành viên ({thread.presentation.member_count ?? thread.presentation.members.length})
          </div>
          <div className="flex max-h-40 flex-col gap-1.5 overflow-y-auto pr-1">
            {thread.presentation.members.map((m) => (
              <div key={m.id} className="flex items-center gap-2 text-xs">
                <Avatar
                  id={m.id}
                  name={m.name}
                  avatarUrl={m.avatar}
                  size={26}
                  mediaCache={state.mediaCache}
                />
                <span className="truncate">{m.name || m.id}</span>
              </div>
            ))}
          </div>
        </div>
      ) : null}

      <div className="flex flex-col gap-2">
        <div className="text-xs font-bold text-foreground">
          Ảnh đã chia sẻ ({sharedPhotos.length})
        </div>
        {sharedPhotos.length === 0 ? (
          <div className="text-xs text-muted-foreground">Chưa có hình ảnh nào</div>
        ) : (
          <div className="grid grid-cols-3 gap-1.5">
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
                  className="grid aspect-square cursor-pointer place-items-center overflow-hidden rounded-md border border-border bg-muted text-[11px] text-muted-foreground hover:opacity-90"
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
        )}
      </div>

      <Button
        variant="destructive"
        className="mt-auto w-full"
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
    </aside>
  );
}
