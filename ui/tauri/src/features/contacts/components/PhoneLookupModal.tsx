import { useState, type FormEvent } from "react";
import { MessageSquare, Search, UserPlus, X } from "lucide-react";
import { ipc } from "../../../shared/api/ipc";
import { Avatar } from "../../../shared/ui/Avatar";
import type { AppAction, AppState } from "../../../app/store/appReducer";

interface PhoneLookupModalProps {
  state: AppState;
  dispatch: (action: AppAction) => void;
}

export function PhoneLookupModal({ state, dispatch }: PhoneLookupModalProps) {
  const [phone, setPhone] = useState("");
  if (!state.phoneModalOpen) return null;

  const submit = (e: FormEvent) => {
    e.preventDefault();
    if (!phone.trim()) return;
    dispatch({ type: "START_PHONE_LOOKUP" });
    void ipc.lookupPhone(phone.trim());
  };

  const p = state.phoneLookupResult;

  return (
    <div className="modal-backdrop" onClick={() => dispatch({ type: "SET_PHONE_MODAL", open: false })}>
      <div
        className="modal-card"
        style={{ width: 420, padding: 18, gap: 12 }}
        onClick={(e) => e.stopPropagation()}
      >
        <div className="settings-row">
          <div style={{ fontWeight: 700, display: "flex", alignItems: "center", gap: 6 }}>
            <UserPlus size={16} color="var(--primary)" /> Thêm bạn & Tra cứu SĐT
          </div>
          <button
            type="button"
            className="icon-btn"
            onClick={() => dispatch({ type: "SET_PHONE_MODAL", open: false })}
          >
            <X size={16} />
          </button>
        </div>

        <form onSubmit={submit} style={{ display: "flex", gap: 8 }}>
          <input
            className="field-input"
            value={phone}
            onChange={(e) => setPhone(e.target.value)}
            placeholder="Nhập số điện thoại (ví dụ 0912345678)…"
            autoFocus
          />
          <button type="submit" className="btn btn-primary" disabled={state.phoneLookupLoading}>
            <Search size={14} /> {state.phoneLookupLoading ? "Đang tìm…" : "Tìm"}
          </button>
        </form>

        {state.phoneLookupError ? (
          <div style={{ color: "var(--danger)", fontSize: 12.5 }}>{state.phoneLookupError}</div>
        ) : null}

        {p ? (
          <div className="settings-box">
            <div style={{ display: "flex", alignItems: "center", gap: 12 }}>
              <Avatar
                id={p.user_id}
                name={p.display_name || p.zalo_name}
                avatarUrl={p.avatar}
                size={46}
                mediaCache={state.mediaCache}
              />
              <div style={{ flex: 1, minWidth: 0 }}>
                <div style={{ fontWeight: 700 }}>{p.display_name || p.zalo_name}</div>
                <div style={{ fontSize: 12, color: "var(--text-secondary)" }}>
                  {p.phone} · UID: {p.user_id}
                </div>
                {p.status ? (
                  <div style={{ fontSize: 11.8, color: "var(--text-muted)" }}>{p.status}</div>
                ) : null}
              </div>
            </div>
            <div style={{ display: "flex", gap: 8, justifyContent: "flex-end" }}>
              <button
                type="button"
                className="btn"
                onClick={() => {
                  dispatch({ type: "SELECT_THREAD", threadId: p.user_id });
                  dispatch({ type: "SET_PHONE_MODAL", open: false });
                }}
              >
                <MessageSquare size={14} /> Nhắn tin
              </button>
              <button
                type="button"
                className="btn btn-primary"
                onClick={() => void ipc.addFriend(p.user_id, p.display_name || p.zalo_name)}
              >
                <UserPlus size={14} /> Kết bạn
              </button>
            </div>
          </div>
        ) : null}
      </div>
    </div>
  );
}
