import { Trash2, X } from "lucide-react";
import { ipc } from "../../../shared/api/ipc";
import type { AppAction, AppState } from "../../../app/store/appReducer";

interface WipeConfirmModalProps {
  state: AppState;
  dispatch: (action: AppAction) => void;
}

export function WipeConfirmModal({ state, dispatch }: WipeConfirmModalProps) {
  if (!state.wipeConfirmOpen) return null;

  return (
    <div className="modal-backdrop" onClick={() => dispatch({ type: "SET_WIPE_CONFIRM", open: false })}>
      <div
        className="modal-card"
        style={{ width: 400, padding: 18, gap: 12 }}
        onClick={(e) => e.stopPropagation()}
      >
        <div className="settings-row">
          <div style={{ fontWeight: 700, color: "var(--danger)", display: "flex", alignItems: "center", gap: 6 }}>
            <Trash2 size={16} /> Xóa dữ liệu tin nhắn cục bộ
          </div>
          <button
            type="button"
            className="icon-btn"
            onClick={() => dispatch({ type: "SET_WIPE_CONFIRM", open: false })}
          >
            <X size={16} />
          </button>
        </div>

        <div style={{ fontSize: 12.6, color: "var(--text-secondary)" }}>
          Xóa <code>zalo_history.db</code> và <code>threads_cache.json</code> trên máy này. Không thể hoàn tác,
          nhưng phiên đăng nhập <code>session.json</code> vẫn được giữ nguyên.
        </div>

        <div style={{ display: "flex", justifyContent: "flex-end", gap: 8 }}>
          <button
            type="button"
            className="btn"
            onClick={() => dispatch({ type: "SET_WIPE_CONFIRM", open: false })}
          >
            Hủy
          </button>
          <button
            type="button"
            className="btn btn-danger"
            onClick={() => {
              dispatch({ type: "SET_WIPE_CONFIRM", open: false });
              void ipc.wipeLocalData();
            }}
          >
            Xóa dữ liệu
          </button>
        </div>
      </div>
    </div>
  );
}
