import { useState } from "react";
import { Forward, Search } from "lucide-react";
import { ipc } from "@/shared/api/ipc";
import { Avatar } from "@/shared/ui/Avatar";
import { Dialog, DialogContent, DialogHeader, DialogTitle } from "@/shared/ui/dialog";
import { Input } from "@/shared/ui/input";
import { matchesVnQuery } from "@/shared/utils/text";
import type { AppAction, AppState } from "@/app/store/appReducer";

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
    <Dialog
      open={Boolean(state.forwardMsg)}
      onOpenChange={(open) => !open && dispatch({ type: "SET_FORWARD_MSG", msg: null })}
    >
      <DialogContent className="max-w-sm">
        <DialogHeader>
          <DialogTitle>
            <Forward className="h-4 w-4 text-primary" /> Chuyển tiếp tin nhắn
          </DialogTitle>
        </DialogHeader>

        <div className="flex flex-col gap-3 p-4">
          <div className="truncate rounded-md bg-muted px-2.5 py-1.5 text-xs text-muted-foreground">
            Nội dung: {preview}
          </div>

          <div className="relative flex items-center">
            <Search className="pointer-events-none absolute left-2.5 h-3.5 w-3.5 text-muted-foreground" />
            <Input
              value={filter}
              onChange={(e) => setFilter(e.target.value)}
              placeholder="Tìm hội thoại…"
              className="pl-8"
              autoFocus
            />
          </div>

          <div className="flex max-h-72 flex-col gap-1 overflow-y-auto">
            {candidates.map((t) => (
              <button
                key={t.id}
                type="button"
                onClick={() => {
                  void ipc.forwardMessage(t.id, t.is_group, msg);
                  dispatch({ type: "SET_FORWARD_MSG", msg: null });
                  dispatch({
                    type: "SHOW_TOAST",
                    message: `Đang chuyển tiếp tới ${t.name}`,
                  });
                }}
                className="flex cursor-pointer items-center gap-2.5 rounded-md px-2.5 py-1.5 text-left text-xs font-semibold hover:bg-accent hover:text-accent-foreground"
              >
                <Avatar
                  id={t.id}
                  name={t.name}
                  avatarUrl={t.avatar}
                  isGroup={t.is_group}
                  size={30}
                  mediaCache={state.mediaCache}
                />
                <span className="truncate">{t.is_group ? `[Nhóm] ${t.name}` : t.name}</span>
              </button>
            ))}
          </div>
        </div>
      </DialogContent>
    </Dialog>
  );
}
