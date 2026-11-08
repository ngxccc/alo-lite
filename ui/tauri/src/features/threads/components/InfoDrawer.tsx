import { BellOff, Pin, RefreshCw, Star, Trash2, UserCheck, UserPlus, UserX } from "lucide-react";
import { ipc } from "../../../shared/api/ipc";
import { Avatar } from "../../../shared/ui/Avatar";
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
    .filter((m) => isPhotoMessage(m) && (m.link_thumb || m.link_url))
    .slice(-9)
    .reverse();

  return (
    <aside className="info-drawer">
      <div style={{ display: "flex", flexDirection: "column", alignItems: "center", textAlign: "center", gap: 8 }}>
        <Avatar
          id={thread.id}
          name={thread.name}
          avatarUrl={thread.avatar}
          isGroup={thread.is_group}
          size={64}
          mediaCache={state.mediaCache}
        />
        <div style={{ fontWeight: 700, fontSize: 15 }}>{thread.name || thread.id}</div>
        <div style={{ fontSize: 12, color: "var(--text-secondary)" }}>
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
          <div style={{ display: "flex", gap: 6, flexWrap: "wrap", justifyContent: "center" }}>
            {friendStatus === "Stranger" ? (
              <button
                type="button"
                className="btn btn-primary"
                onClick={() => void ipc.addFriend(thread.id, thread.name)}
              >
                <UserPlus size={14} /> Kết bạn
              </button>
            ) : null}
            {friendStatus === "IncomingPending" ? (
              <>
                <button
                  type="button"
                  className="btn btn-primary"
                  onClick={() => void ipc.acceptFriend(thread.id, thread.name)}
                >
                  <UserCheck size={14} /> Đồng ý
                </button>
                <button
                  type="button"
                  className="btn"
                  onClick={() => void ipc.rejectFriend(thread.id, thread.name)}
                >
                  <UserX size={14} /> Từ chối
                </button>
              </>
            ) : null}
            {friendStatus === "OutgoingPending" ? (
              <button
                type="button"
                className="btn"
                onClick={() => void ipc.undoFriend(thread.id, thread.name)}
              >
                Thu hồi lời mời
              </button>
            ) : null}
          </div>
        ) : null}
      </div>

      <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 8 }}>
        <button
          type="button"
          className="btn"
          onClick={() => {
            dispatch({ type: "START_CLOUD_SYNC" });
            void ipc.syncHistory([[thread.id, thread.name, thread.is_group]]);
          }}
        >
          <RefreshCw size={14} /> Đồng bộ
        </button>
        <button
          type="button"
          className="btn"
          onClick={() => dispatch({ type: "TOGGLE_STARRED_FILTER" })}
        >
          <Star size={14} color={state.filterStarredOnly ? "var(--warning)" : undefined} /> Tin gắn sao
        </button>
        <button
          type="button"
          className="btn"
          onClick={() => {
            if (state.settings) {
              const nextPinned = pinned
                ? state.settings.pinned_threads.filter((id) => id !== thread.id)
                : [...state.settings.pinned_threads, thread.id];
              const nextSettings = { ...state.settings, pinned_threads: nextPinned };
              dispatch({ type: "UPDATE_SETTINGS_LOCAL", settings: nextSettings });
              void ipc.saveSettings(nextSettings);
            }
            void ipc.pinThread(thread.id, thread.is_group, !pinned);
          }}
        >
          <Pin size={14} /> {pinned ? "Bỏ ghim" : "Ghim"}
        </button>
        <button
          type="button"
          className="btn"
          onClick={() => {
            if (state.settings) {
              const nextMuted = { ...state.settings.muted_threads };
              if (muted) delete nextMuted[thread.id];
              else nextMuted[thread.id] = -1;
              const nextSettings = { ...state.settings, muted_threads: nextMuted };
              dispatch({ type: "UPDATE_SETTINGS_LOCAL", settings: nextSettings });
              void ipc.saveSettings(nextSettings);
            }
            void ipc.muteThread(thread.id, thread.is_group, muted ? 0 : -1, !muted);
          }}
        >
          <BellOff size={14} /> {muted ? "Bật TB" : "Tắt TB"}
        </button>
      </div>

      {thread.is_group && thread.presentation.members.length > 0 ? (
        <div>
          <div style={{ fontWeight: 700, fontSize: 12.6, marginBottom: 8 }}>
            Thành viên ({thread.presentation.member_count ?? thread.presentation.members.length})
          </div>
          <div style={{ display: "flex", flexDirection: "column", gap: 6, maxHeight: 160, overflowY: "auto" }}>
            {thread.presentation.members.map((m) => (
              <div key={m.id} style={{ display: "flex", alignItems: "center", gap: 8, fontSize: 12.5 }}>
                <Avatar id={m.id} name={m.name} avatarUrl={m.avatar} size={26} mediaCache={state.mediaCache} />
                <span style={{ overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
                  {m.name || m.id}
                </span>
              </div>
            ))}
          </div>
        </div>
      ) : null}

      <div>
        <div style={{ fontWeight: 700, fontSize: 12.6, marginBottom: 8 }}>
          Ảnh đã chia sẻ ({sharedPhotos.length})
        </div>
        {sharedPhotos.length === 0 ? (
          <div style={{ fontSize: 12, color: "var(--text-muted)" }}>Chưa có hình ảnh nào</div>
        ) : (
          <div style={{ display: "grid", gridTemplateColumns: "repeat(3, 1fr)", gap: 6 }}>
            {sharedPhotos.map((m) => {
              const key = `img_${m.id || m.ts}`;
              const dataUrl = state.mediaCache[key];
              const fullUrl = m.link_url || m.link_thumb || "";
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
                  style={{
                    aspectRatio: "1",
                    borderRadius: 6,
                    overflow: "hidden",
                    background: "var(--bg-elevated)",
                    border: "1px solid var(--border)",
                    cursor: "pointer",
                    display: "grid",
                    placeItems: "center",
                    fontSize: 11,
                    color: "var(--text-muted)",
                  }}
                >
                  {dataUrl ? (
                    <img src={dataUrl} alt="Ảnh" style={{ width: "100%", height: "100%", objectFit: "cover" }} />
                  ) : (
                    "Ảnh"
                  )}
                </div>
              );
            })}
          </div>
        )}
      </div>

      <button
        type="button"
        className="btn btn-danger"
        style={{ marginTop: "auto" }}
        onClick={() => {
          const last = thread.messages[thread.messages.length - 1];
          void ipc.deleteThread(thread.id, thread.is_group, last?.client_message_id || last?.id || "");
        }}
      >
        <Trash2 size={14} /> Xóa cuộc trò chuyện
      </button>
    </aside>
  );
}
