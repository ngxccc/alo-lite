import { FolderOpen, RefreshCw, Square } from "lucide-react";
import { ipc } from "@/shared/api/ipc";
import { Button } from "@/shared/ui/button";
import { Dialog, DialogContent, DialogTitle } from "@/shared/ui/dialog";
import type { AppAction, AppState } from "@/app/store/app-reducer";
import { useAppStore } from "@/app/store/use-app-store";

interface PhoneSyncModalProps {
  state?: AppState;
  dispatch?: (action: AppAction) => void;
}

export function PhoneSyncModal({
  state: propState,
  dispatch: propDispatch,
}: PhoneSyncModalProps = {}) {
  const appStore = useAppStore();
  const state = propState ?? appStore;
  const dispatch = propDispatch ?? appStore.dispatch;
  if (!state.phoneSyncModalOpen) return null;

  return (
    <Dialog
      open={state.phoneSyncModalOpen}
      onOpenChange={(open) => !open && dispatch({ type: "SET_PHONE_SYNC_MODAL", open: false })}
    >
      <DialogContent className="w-[480px] max-w-[480px] rounded-lg p-0">
        {/* Header */}
        <div className="border-b border-border px-5 py-4">
          <DialogTitle className="text-base font-medium text-foreground">
            Đồng bộ tin nhắn từ điện thoại
          </DialogTitle>
        </div>

        {/* Body */}
        <div className="flex flex-col gap-3 px-5 py-4">
          <div className="rounded-lg bg-card px-4 py-3 text-sm">
            <div className="mb-2 font-medium text-foreground">
              Các bước thực hiện trên điện thoại:
            </div>
            <div className="text-muted-foreground">1. Mở ứng dụng Zalo trên điện thoại.</div>
            <div className="mt-1 font-medium text-primary">
              2. Khi màn hình hiện "Đồng bộ tin nhắn?", bấm "Đồng bộ ngay".
            </div>
            <div className="mt-1 font-medium text-[hsl(var(--online,142_71%_45%))]">
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

        {/* Footer */}
        <div className="flex items-center justify-between border-t border-border px-5 py-3">
          <Button
            variant="secondary"
            className="h-8 rounded-md px-4 text-sm"
            onClick={() => {
              dispatch({ type: "SET_PHONE_SYNC_MODAL", open: false });
              dispatch({ type: "OPEN_SETTINGS", open: true, tab: "sync" });
            }}
          >
            <FolderOpen className="h-3.5 w-3.5" /> Nhập từ file cục bộ
          </Button>

          {state.phoneSyncRunning ? (
            <Button
              variant="destructive"
              className="h-8 rounded-md px-4 text-sm"
              onClick={() => void ipc.cancelPhoneSync()}
            >
              <Square className="h-3.5 w-3.5" /> Dừng đồng bộ
            </Button>
          ) : (
            <Button
              variant="default"
              className="h-8 rounded-md px-4 text-sm"
              onClick={() => {
                dispatch({ type: "START_PHONE_SYNC" });
                void ipc.syncPhoneMessages(true);
              }}
            >
              <RefreshCw className="h-3.5 w-3.5" /> Bắt đầu đồng bộ ngay
            </Button>
          )}
        </div>
      </DialogContent>
    </Dialog>
  );
}
