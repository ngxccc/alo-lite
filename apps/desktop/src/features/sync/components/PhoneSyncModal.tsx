import { FolderOpen, RefreshCw, Smartphone, Square } from "lucide-react";
import { ipc } from "@/shared/api/ipc";
import { Button } from "@/shared/ui/button";
import {
  Dialog,
  DialogContent,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/shared/ui/dialog";
import type { AppAction, AppState } from "@/app/store/appReducer";

interface PhoneSyncModalProps {
  state: AppState;
  dispatch: (action: AppAction) => void;
}

export function PhoneSyncModal({ state, dispatch }: PhoneSyncModalProps) {
  if (!state.phoneSyncModalOpen) return null;

  return (
    <Dialog
      open={state.phoneSyncModalOpen}
      onOpenChange={(open) => !open && dispatch({ type: "SET_PHONE_SYNC_MODAL", open: false })}
    >
      <DialogContent className="max-w-md">
        <DialogHeader>
          <DialogTitle>
            <Smartphone className="h-4 w-4 text-primary" /> Đồng bộ tin nhắn từ điện thoại
          </DialogTitle>
        </DialogHeader>

        <div className="flex flex-col gap-3 p-4">
          <div className="flex flex-col gap-1.5 rounded-lg border border-border bg-muted/50 p-3 text-xs">
            <div className="font-bold text-foreground">Các bước cần thực hiện trên điện thoại:</div>
            <div>1. Mở ứng dụng Zalo trên điện thoại.</div>
            <div className="font-semibold text-primary">
              2. Khi màn hình hiện “Đồng bộ tin nhắn?”, bấm “Đồng bộ ngay”.
            </div>
            <div className="font-semibold text-online">
              3. Xác thực bằng FaceID / Vân tay / Mã PIN trên điện thoại.
            </div>
          </div>

          {state.phoneSyncRunning ? (
            <div className="flex flex-col gap-2">
              <div className="h-2 overflow-hidden rounded-full bg-muted">
                <div
                  style={{
                    width: `${Math.round(Math.max(0.05, Math.min(1, state.phoneSyncProgress)) * 100)}%`,
                  }}
                  className="h-full bg-primary transition-all duration-200"
                />
              </div>
              <div className="text-xs text-muted-foreground">{state.phoneSyncStatus}</div>
            </div>
          ) : state.phoneSyncStatus ? (
            <div className="text-xs text-muted-foreground">{state.phoneSyncStatus}</div>
          ) : null}
        </div>

        <DialogFooter className="justify-between">
          <Button
            variant="outline"
            onClick={() => {
              dispatch({ type: "SET_PHONE_SYNC_MODAL", open: false });
              dispatch({ type: "OPEN_SETTINGS", open: true, tab: "sync" });
            }}
          >
            <FolderOpen className="h-3.5 w-3.5" /> Nhập từ file cục bộ
          </Button>

          {state.phoneSyncRunning ? (
            <Button variant="destructive" onClick={() => void ipc.cancelPhoneSync()}>
              <Square className="h-3.5 w-3.5" /> Dừng đồng bộ
            </Button>
          ) : (
            <Button
              variant="default"
              onClick={() => {
                dispatch({ type: "START_PHONE_SYNC" });
                void ipc.syncPhoneMessages(state.threads, true);
              }}
            >
              <RefreshCw className="h-3.5 w-3.5" /> Bắt đầu đồng bộ ngay
            </Button>
          )}
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
