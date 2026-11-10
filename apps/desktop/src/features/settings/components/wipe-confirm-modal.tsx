import { Trash2 } from "lucide-react";
import { ipc } from "@/shared/api/ipc";
import { Button } from "@/shared/ui/button";
import { Dialog, DialogContent, DialogTitle } from "@/shared/ui/dialog";
import type { AppAction, AppState } from "@/app/store/app-reducer";
import { useAppStore } from "@/app/store/use-app-store";

interface WipeConfirmModalProps {
  state?: AppState;
  dispatch?: (action: AppAction) => void;
}

export function WipeConfirmModal({
  state: propState,
  dispatch: propDispatch,
}: WipeConfirmModalProps = {}) {
  const appStore = useAppStore();
  const state = propState ?? appStore;
  const dispatch = propDispatch ?? appStore.dispatch;
  if (!state.wipeConfirmOpen) return null;

  return (
    <Dialog
      open={state.wipeConfirmOpen}
      onOpenChange={(open) => !open && dispatch({ type: "SET_WIPE_CONFIRM", open: false })}
    >
      <DialogContent className="w-[400px] max-w-[400px] rounded-lg p-0">
        {/* Header */}
        <div className="flex items-center gap-2.5 border-b border-border px-5 py-4">
          <Trash2 className="h-5 w-5 shrink-0 text-destructive" />
          <DialogTitle className="text-base font-medium text-foreground">
            Xóa dữ liệu tin nhắn cục bộ
          </DialogTitle>
        </div>

        {/* Body */}
        <div className="px-5 py-4 text-sm leading-relaxed text-muted-foreground">
          Xóa{" "}
          <code className="rounded bg-muted px-1.5 py-0.5 text-xs text-foreground">
            zalo_history.db
          </code>{" "}
          và{" "}
          <code className="rounded bg-muted px-1.5 py-0.5 text-xs text-foreground">
            threads_cache.json
          </code>{" "}
          trên máy này. Không thể hoàn tác, nhưng phiên đăng nhập{" "}
          <code className="rounded bg-muted px-1.5 py-0.5 text-xs text-foreground">
            session.json
          </code>{" "}
          vẫn được giữ nguyên.
        </div>

        {/* Footer */}
        <div className="flex items-center justify-end gap-2 border-t border-border px-5 py-3">
          <Button
            variant="secondary"
            className="h-8 rounded-md px-4 text-sm"
            onClick={() => dispatch({ type: "SET_WIPE_CONFIRM", open: false })}
          >
            Hủy
          </Button>
          <Button
            variant="destructive"
            className="h-8 rounded-md px-4 text-sm"
            onClick={() => {
              dispatch({ type: "SET_WIPE_CONFIRM", open: false });
              void ipc.wipeLocalData();
            }}
          >
            Xóa dữ liệu
          </Button>
        </div>
      </DialogContent>
    </Dialog>
  );
}
