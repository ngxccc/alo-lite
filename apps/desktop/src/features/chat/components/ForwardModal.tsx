import { useState } from "react";
import { Forward, Search, X } from "lucide-react";
import { ipc } from "../../../shared/api/ipc";
import { Avatar } from "../../../shared/ui/Avatar";
import { matchesVnQuery } from "../../../shared/utils/text";
import type { AppAction, AppState } from "../../../app/store/appReducer";

interface ForwardModalProps {
  state: AppState;
  dispatch: (action: AppAction) => void;
}

export function ForwardModal({ state, dispatch }: ForwardModalProps) {
  const [filter, setFilter] = useState("");
  if (!state.forwardMsg) return null;

  const msg = state.forwardMsg;
  const preview = msg.sticker_id ? "[Nhãn dán]" : msg.text.slice(0, 80);
  const candidates = state.threads.filter((t) => matchesVnQuery(t.name, filter));

  return (
    <div className="modal-backdrop" onClick={() => dispatch({ type: "SET_FORWARD_MSG", msg: null })}>
      <div
        className="modal-card"
        style={{ width: 380, maxHeight: 480, padding: 16, gap: 10 }}
        onClick={(e) => e.stopPropagation()}
      >
        <div className="settings-row">
          <div style={{ fontWeight: 700, display: "flex", alignItems: "center", gap: 6 }}>
            <Forward size={16} color="var(--primary)" /> Chuyển tiếp tin nhắn
          </div>
          <button
            type="button"
            className="icon-btn"
            onClick={() => dispatch({ type: "SET_FORWARD_MSG", msg: null })}
          >
            <X size={16} />
          </button>
        </div>

        <div style={{ fontSize: 12.2, color: "var(--text-secondary)" }}>Nội dung: {preview}</div>

        <div className="search-box">
          <Search size={14} color="var(--text-muted)" />
          <input
            value={filter}
            onChange={(e) => setFilter(e.target.value)}
            placeholder="Tìm hội thoại…"
            autoFocus
          />
        </div>

        <div style={{ flex: 1, overflowY: "auto", display: "flex", flexDirection: "column", gap: 4 }}>
          {candidates.map((t) => (
            <div
              key={t.id}
              className="thread-row"
              onClick={() => {
                void ipc.forwardMessage(t.id, t.is_group, msg);
                dispatch({ type: "SET_FORWARD_MSG", msg: null });
                dispatch({ type: "SHOW_TOAST", message: `Đang chuyển tiếp tới ${t.name}` });
              }}
            >
              <Avatar
                id={t.id}
                name={t.name}
                avatarUrl={t.avatar}
                isGroup={t.is_group}
                size={32}
                mediaCache={state.mediaCache}
              />
              <span style={{ fontWeight: 600 }}>{t.is_group ? `[Nhóm] ${t.name}` : t.name}</span>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}
