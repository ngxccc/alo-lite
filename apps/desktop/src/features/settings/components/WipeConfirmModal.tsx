import { Trash2 } from "lucide-react";
import { ipc } from "../../../shared/api/ipc";
import { Button } from "../../../shared/ui/button";
import {
  Dialog,
  DialogContent,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "../../../shared/ui/dialog";
import type { AppAction, AppState } from "../../../app/store/appReducer";

interface WipeConfirmModalProps {
  state: AppState;
  dispatch: (action: AppAction) => void;
}

export function WipeConfirmModal({ state, dispatch }: WipeConfirmModalProps) {
  if (!state.wipeConfirmOpen) return null;

  return (
    <Dialog
      open={state.wipeConfirmOpen}
      onOpenChange={(open) => !open && dispatch({ type: "SET_WIPE_CONFIRM", open: false })}
    >
      <DialogContent className="max-w-sm">
        <DialogHeader>
          <DialogTitle className="text-destructive">
            <Trash2 className="h-4 w-4" /> Xóa dữ liệu tin nhắn cục bộ
          </DialogTitle>
        </DialogHeader>

        <div className="p-4 text-xs leading-relaxed text-muted-foreground">
          Xóa <code className="rounded bg-muted px-1 py-0.5 text-foreground">zalo_history.db</code>{" "}
          và{" "}
          <code className="rounded bg-muted px-1 py-0.5 text-foreground">threads_cache.json</code>{" "}
          trên máy này. Không thể hoàn tác, nhưng phiên đăng nhập{" "}
          <code className="rounded bg-muted px-1 py-0.5 text-foreground">session.json</code> vẫn
          được giữ nguyên.
        </div>

        <DialogFooter>
          <Button
            variant="outline"
            onClick={() => dispatch({ type: "SET_WIPE_CONFIRM", open: false })}
          >
            Hủy
          </Button>
          <Button
            variant="destructive"
            onClick={() => {
              dispatch({ type: "SET_WIPE_CONFIRM", open: false });
              void ipc.wipeLocalData();
            }}
          >
            Xóa dữ liệu
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
