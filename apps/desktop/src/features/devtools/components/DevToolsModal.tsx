import { useEffect, useState } from "react";
import { Bug, RefreshCw, ShieldAlert, Terminal } from "lucide-react";
import { ipc, isTauriRuntime } from "../../../shared/api/ipc";
import type { DevDiagnosticsSnapshot, LogEntry } from "../../../shared/types/ipc";
import { Badge } from "../../../shared/ui/badge";
import { Button } from "../../../shared/ui/button";
import { Dialog, DialogContent, DialogHeader, DialogTitle } from "../../../shared/ui/dialog";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "../../../shared/ui/tabs";
import { cn } from "../../../shared/utils/cn";
import type { AppAction, AppState } from "../../../app/store/appReducer";

interface DevToolsModalProps {
  state: AppState;
  dispatch: (action: AppAction) => void;
}

export function DevToolsModal({ state, dispatch }: DevToolsModalProps) {
  const [tab, setTab] = useState<"logs" | "scanner" | "crypto">("logs");
  const [diag, setDiag] = useState<DevDiagnosticsSnapshot | null>(null);
  const [selectedLog, setSelectedLog] = useState<LogEntry | null>(null);

  const load = async () => {
    if (!isTauriRuntime()) return;
    const snap = await ipc.getDevDiagnostics();
    setDiag(snap);
  };

  useEffect(() => {
    if (state.devModalOpen) {
      void load();
    }
  }, [state.devModalOpen]);

  if (!state.devModalOpen) return null;

  return (
    <Dialog
      open={state.devModalOpen}
      onOpenChange={(open) => !open && dispatch({ type: "SET_DEV_MODAL", open: false })}
    >
      <DialogContent className="max-w-4xl h-[560px]">
        <DialogHeader className="pr-10">
          <DialogTitle>
            <Bug className="h-4 w-4 text-primary" /> Công cụ kiểm tra & Chẩn đoán giao thức
          </DialogTitle>
          <Button variant="outline" size="sm" onClick={() => void load()}>
            <RefreshCw className="h-3.5 w-3.5" /> Làm mới
          </Button>
        </DialogHeader>

        <Tabs
          value={tab}
          onValueChange={(v) => setTab(v as "logs" | "scanner" | "crypto")}
          className="flex min-h-0 flex-1 flex-col p-4 gap-3"
        >
          <TabsList className="w-fit">
            <TabsTrigger value="logs" className="gap-1.5">
              <Terminal className="h-3.5 w-3.5" /> Nhật ký HTTP ({diag?.logs.length ?? 0})
            </TabsTrigger>
            <TabsTrigger value="scanner" className="gap-1.5">
              <ShieldAlert className="h-3.5 w-3.5" /> Quét chẩn đoán ({diag?.findings.length ?? 0})
            </TabsTrigger>
            <TabsTrigger value="crypto">Tham số giao thức</TabsTrigger>
          </TabsList>

          <TabsContent value="logs" className="flex min-h-0 flex-1 gap-3 overflow-hidden">
            <div className="flex w-80 flex-col gap-1 overflow-y-auto border-r border-border pr-2">
              {(diag?.logs ?? [])
                .slice()
                .reverse()
                .map((e) => (
                  <button
                    key={e.id}
                    type="button"
                    onClick={() => setSelectedLog(e)}
                    className={cn(
                      "flex items-center gap-2 rounded-md px-2.5 py-1.5 text-left text-xs cursor-pointer",
                      selectedLog?.id === e.id ? "bg-accent text-accent-foreground" : "hover:bg-muted",
                    )}
                  >
                    <Badge variant={e.is_error ? "destructive" : "online"}>
                      {e.status ?? "ERR"}
                    </Badge>
                    <span className="flex-1 truncate font-mono text-[11px]">
                      {e.method} {e.url}
                    </span>
                    <span className="text-[11px] text-muted-foreground">{e.latency_ms}ms</span>
                  </button>
                ))}
            </div>

            <div className="flex-1 overflow-y-auto text-xs">
              {selectedLog ? (
                <div className="flex flex-col gap-2">
                  <div className="font-mono font-bold text-foreground">
                    {selectedLog.method} {selectedLog.url}
                  </div>
                  <div className="text-muted-foreground">
                    {selectedLog.timestamp} · {selectedLog.latency_ms}ms · HTTP{" "}
                    {selectedLog.status ?? "N/A"}
                  </div>
                  {selectedLog.error_msg ? (
                    <div className="text-destructive">Lỗi: {selectedLog.error_msg}</div>
                  ) : null}
                  <pre className="overflow-x-auto whitespace-pre-wrap rounded-lg border border-border bg-muted p-3 font-mono text-[11.5px]">
                    {selectedLog.decrypted_preview ?? selectedLog.payload_preview}
                  </pre>
                </div>
              ) : (
                <div className="p-6 text-muted-foreground">
                  {diag?.dev_mode
                    ? "Chọn một request bên trái để xem chi tiết."
                    : "Chế độ ghi nhật ký HTTP chỉ bật khi chạy với cờ --dev."}
                </div>
              )}
            </div>
          </TabsContent>

          <TabsContent value="scanner" className="flex min-h-0 flex-1 flex-col gap-2 overflow-y-auto">
            {(diag?.findings ?? []).map((f, idx) => (
              <div
                key={idx}
                className="flex flex-col gap-1 rounded-lg border border-border bg-card p-3 text-xs"
              >
                <div className="font-bold text-foreground">
                  [{f.level}] {f.title}
                </div>
                <div className="text-muted-foreground">{f.detail}</div>
                <div className="text-primary">Khắc phục: {f.suggestion}</div>
              </div>
            ))}
          </TabsContent>

          <TabsContent value="crypto" className="min-h-0 flex-1 overflow-y-auto">
            <div className="flex flex-col divide-y divide-border rounded-lg border border-border bg-card px-4 text-xs">
              {[
                ["API Version (zpw_ver)", diag?.crypto.zpw_ver],
                ["API Type (zpw_type)", diag?.crypto.zpw_type],
                ["Thuật toán", diag?.crypto.cipher_algo],
                ["SignKey", diag?.crypto.sign_key_formula],
                ["ParamCipher key bytes", `${diag?.crypto.param_cipher_key_len ?? 0} bytes`],
              ].map(([label, val]) => (
                <div key={String(label)} className="flex items-center justify-between py-2.5">
                  <span className="text-muted-foreground">{label}</span>
                  <code className="rounded bg-muted px-2 py-0.5 font-mono text-foreground">
                    {val}
                  </code>
                </div>
              ))}
            </div>
          </TabsContent>
        </Tabs>
      </DialogContent>
    </Dialog>
  );
}
