import { useState, type SyntheticEvent } from "react";
import { MessageSquare, Search, UserPlus } from "lucide-react";
import { ipc } from "@/shared/api/ipc";
import { Avatar } from "@/shared/ui/Avatar";
import { Dialog, DialogContent, DialogHeader, DialogTitle } from "@/shared/ui/dialog";
import type { AppAction, AppState } from "@/app/store/appReducer";

interface PhoneLookupModalProps {
  state: AppState;
  dispatch: (action: AppAction) => void;
}

export function PhoneLookupModal({ state, dispatch }: PhoneLookupModalProps) {
  const [phone, setPhone] = useState("");
  if (!state.phoneModalOpen) return null;

  const submit = (e: SyntheticEvent<HTMLFormElement>) => {
    e.preventDefault();
    if (!phone.trim()) return;
    dispatch({ type: "START_PHONE_LOOKUP" });
    void ipc.lookupPhone(phone.trim());
  };

  const p = state.phoneLookupResult;

  return (
    <Dialog
      open={state.phoneModalOpen}
      onOpenChange={(open) => !open && dispatch({ type: "SET_PHONE_MODAL", open: false })}
    >
      {/* Zalo-style: rounded-lg (8px), dark overlay, bg-card body */}
      <DialogContent className="max-w-[460px] rounded-lg border border-border bg-card p-0 shadow-2xl">
        {/* Dialog header: bg-background, 52px */}
        <DialogHeader className="flex h-[52px] flex-row items-center gap-2 rounded-t-lg border-b border-border bg-background px-4">
          <UserPlus className="h-5 w-5 text-muted-foreground" />
          <DialogTitle className="text-[18px] font-medium text-foreground">Thêm bạn</DialogTitle>
        </DialogHeader>

        {/* Body */}
        <div className="flex flex-col gap-4 p-5">
          {/* Phone input row with (+84) prefix */}
          <form id="phone-lookup-form" onSubmit={submit} className="flex gap-2">
            {/* Country prefix badge */}
            <div className="flex h-10 items-center justify-center rounded-[6px] bg-canvas px-3 text-[14px] text-foreground">
              (+84)
            </div>
            {/* Phone number input */}
            <div className="flex h-10 flex-1 items-center rounded-[6px] bg-canvas px-3">
              <input
                value={phone}
                onChange={(e) => setPhone(e.target.value)}
                placeholder="Số điện thoại"
                autoFocus
                className="min-w-0 flex-1 bg-transparent text-[14px] text-foreground placeholder:text-muted-foreground focus:outline-none"
              />
            </div>
          </form>

          {/* Error message */}
          {state.phoneLookupError ? (
            <div className="rounded-md bg-destructive/10 px-3 py-2 text-[13px] text-destructive">
              {state.phoneLookupError}
            </div>
          ) : null}

          {/* Lookup result card */}
          {p ? (
            <div className="flex flex-col gap-3 rounded-lg border border-border bg-background p-3">
              <div className="flex items-center gap-3">
                <Avatar
                  id={p.user_id}
                  name={p.display_name || p.zalo_name}
                  avatarUrl={p.avatar}
                  size={46}
                  mediaCache={state.mediaCache}
                />
                <div className="min-w-0 flex-1">
                  <div className="truncate text-[15px] font-medium text-foreground">
                    {p.display_name || p.zalo_name}
                  </div>
                  <div className="text-[13px] text-muted-foreground">
                    {p.phone} · UID: {p.user_id}
                  </div>
                  {p.status ? (
                    <div className="truncate text-[12px] text-muted-foreground">{p.status}</div>
                  ) : null}
                </div>
              </div>
              <div className="flex justify-end gap-2">
                <button
                  type="button"
                  onClick={() => {
                    dispatch({ type: "SELECT_THREAD", threadId: p.user_id });
                    dispatch({ type: "SET_PHONE_MODAL", open: false });
                  }}
                  className="flex h-8 items-center gap-1.5 rounded-[6px] bg-secondary px-4 text-[14px] font-medium text-foreground transition-colors hover:bg-secondary/80"
                >
                  <MessageSquare className="h-3.5 w-3.5" /> Nhắn tin
                </button>
                <button
                  type="button"
                  onClick={() => void ipc.addFriend(p.user_id, p.display_name || p.zalo_name)}
                  className="flex h-8 items-center gap-1.5 rounded-[6px] bg-primary px-4 text-[14px] font-medium text-white transition-colors hover:bg-primary/90"
                >
                  <UserPlus className="h-3.5 w-3.5" /> Kết bạn
                </button>
              </div>
            </div>
          ) : null}

          {/* Footer buttons */}
          <div className="flex justify-end gap-2 pt-1">
            <button
              type="button"
              onClick={() => dispatch({ type: "SET_PHONE_MODAL", open: false })}
              className="flex h-9 items-center rounded-[6px] bg-secondary px-6 text-[14px] font-medium text-foreground transition-colors hover:bg-secondary/80"
            >
              Hủy
            </button>
            <button
              type="submit"
              form="phone-lookup-form"
              onClick={() => {
                if (!phone.trim()) return;
                dispatch({ type: "START_PHONE_LOOKUP" });
                void ipc.lookupPhone(phone.trim());
              }}
              disabled={state.phoneLookupLoading}
              className="flex h-9 items-center gap-1.5 rounded-[6px] bg-primary px-6 text-[14px] font-medium text-white transition-colors hover:bg-primary/90 disabled:opacity-60"
            >
              <Search className="h-3.5 w-3.5" />
              {state.phoneLookupLoading ? "Đang tìm…" : "Tìm kiếm"}
            </button>
          </div>
        </div>
      </DialogContent>
    </Dialog>
  );
}
