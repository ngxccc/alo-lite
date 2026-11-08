import { FolderOpen, RefreshCw, Smartphone, Square, X } from "lucide-react";
import { ipc } from "../../../shared/api/ipc";
import type { AppAction, AppState } from "../../../app/store/appReducer";

interface PhoneSyncModalProps {
  state: AppState;
  dispatch: (action: AppAction) => void;
}

export function PhoneSyncModal({ state, dispatch }: PhoneSyncModalProps) {
  if (!state.phoneSyncModalOpen) return null;

  return (
    <div
      className="modal-backdrop"
      onClick={() => dispatch({ type: "SET_PHONE_SYNC_MODAL", open: false })}
    >
      <div
        className="modal-card"
        style={{ width: 460, padding: 20, gap: 14 }}
        onClick={(e) => e.stopPropagation()}
      >
        <div className="settings-row">
          <div style={{ fontWeight: 700, display: "flex", alignItems: "center", gap: 8 }}>
            <Smartphone size={18} color="var(--primary)" /> Đồng bộ tin nhắn từ điện thoại
          </div>
          <button
            type="button"
            className="icon-btn"
            onClick={() => dispatch({ type: "SET_PHONE_SYNC_MODAL", open: false })}
          >
            <X size={16} />
          </button>
        </div>

        <div className="settings-box" style={{ fontSize: 12.6 }}>
          <div style={{ fontWeight: 700 }}>Các bước cần thực hiện trên điện thoại:</div>
          <div>1. Mở ứng dụng Zalo trên điện thoại.</div>
          <div style={{ color: "var(--primary)", fontWeight: 600 }}>
            2. Khi màn hình hiện “Đồng bộ tin nhắn?”, bấm “Đồng bộ ngay”.
          </div>
          <div style={{ color: "var(--accent-online)", fontWeight: 600 }}>
            3. Xác thực bằng FaceID / Vân tay / Mã PIN trên điện thoại.
          </div>
        </div>

        {state.phoneSyncRunning ? (
          <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
            <div style={{ height: 8, background: "var(--bg-input)", borderRadius: 999, overflow: "hidden" }}>
              <div
                style={{
                  height: "100%",
                  width: `${Math.round(Math.max(0.05, Math.min(1, state.phoneSyncProgress)) * 100)}%`,
                  background: "var(--primary)",
                  transition: "width 0.2s",
                }}
              />
            </div>
            <div style={{ fontSize: 12, color: "var(--text-secondary)" }}>{state.phoneSyncStatus}</div>
          </div>
        ) : state.phoneSyncStatus ? (
          <div style={{ fontSize: 12.2, color: "var(--text-secondary)" }}>{state.phoneSyncStatus}</div>
        ) : null}

        <div style={{ display: "flex", justifyContent: "space-between", gap: 8 }}>
          <button
            type="button"
            className="btn"
            onClick={() => {
              dispatch({ type: "SET_PHONE_SYNC_MODAL", open: false });
              dispatch({ type: "OPEN_SETTINGS", open: true, tab: "sync" });
            }}
          >
            <FolderOpen size={14} /> Nhập từ file cục bộ
          </button>

          {state.phoneSyncRunning ? (
            <button
              type="button"
              className="btn btn-danger"
              onClick={() => void ipc.cancelPhoneSync()}
            >
              <Square size={14} /> Dừng đồng bộ
            </button>
          ) : (
            <button
              type="button"
              className="btn btn-primary"
              onClick={() => {
                dispatch({ type: "START_PHONE_SYNC" });
                void ipc.syncPhoneMessages(state.threads, true);
              }}
            >
              <RefreshCw size={14} /> Bắt đầu đồng bộ ngay
            </button>
          )}
        </div>
      </div>
    </div>
  );
}
