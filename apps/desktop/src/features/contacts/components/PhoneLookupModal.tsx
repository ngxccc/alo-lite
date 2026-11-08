import { useState, type SyntheticEvent } from "react";
import { MessageSquare, Search, UserPlus } from "lucide-react";
import { ipc } from "../../../shared/api/ipc";
import { Avatar } from "../../../shared/ui/Avatar";
import { Button } from "../../../shared/ui/button";
import { Dialog, DialogContent, DialogHeader, DialogTitle } from "../../../shared/ui/dialog";
import { Input } from "../../../shared/ui/input";
import type { AppAction, AppState } from "../../../app/store/appReducer";

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
      <DialogContent className="max-w-md">
        <DialogHeader>
          <DialogTitle>
            <UserPlus className="h-4 w-4 text-primary" /> Thêm bạn & Tra cứu SĐT
          </DialogTitle>
        </DialogHeader>

        <div className="flex flex-col gap-3 p-4">
          <form onSubmit={submit} className="flex gap-2">
            <Input
              value={phone}
              onChange={(e) => setPhone(e.target.value)}
              placeholder="Nhập số điện thoại (ví dụ 0912345678)…"
              autoFocus
            />
            <Button type="submit" variant="default" disabled={state.phoneLookupLoading}>
              <Search className="h-3.5 w-3.5" /> {state.phoneLookupLoading ? "Đang tìm…" : "Tìm"}
            </Button>
          </form>

          {state.phoneLookupError ? (
            <div className="rounded-md bg-destructive/10 px-3 py-2 text-xs text-destructive">
              {state.phoneLookupError}
            </div>
          ) : null}

          {p ? (
            <div className="flex flex-col gap-3 rounded-lg border border-border bg-card p-3">
              <div className="flex items-center gap-3">
                <Avatar
                  id={p.user_id}
                  name={p.display_name || p.zalo_name}
                  avatarUrl={p.avatar}
                  size={46}
                  mediaCache={state.mediaCache}
                />
                <div className="min-w-0 flex-1">
                  <div className="truncate font-bold text-foreground">
                    {p.display_name || p.zalo_name}
                  </div>
                  <div className="text-xs text-muted-foreground">
                    {p.phone} · UID: {p.user_id}
                  </div>
                  {p.status ? (
                    <div className="truncate text-[11px] text-muted-foreground">{p.status}</div>
                  ) : null}
                </div>
              </div>
              <div className="flex justify-end gap-2">
                <Button
                  variant="outline"
                  size="sm"
                  onClick={() => {
                    dispatch({ type: "SELECT_THREAD", threadId: p.user_id });
                    dispatch({ type: "SET_PHONE_MODAL", open: false });
                  }}
                >
                  <MessageSquare className="h-3.5 w-3.5" /> Nhắn tin
                </Button>
                <Button
                  variant="default"
                  size="sm"
                  onClick={() => void ipc.addFriend(p.user_id, p.display_name || p.zalo_name)}
                >
                  <UserPlus className="h-3.5 w-3.5" /> Kết bạn
                </Button>
              </div>
            </div>
          ) : null}
        </div>
      </DialogContent>
    </Dialog>
  );
}
