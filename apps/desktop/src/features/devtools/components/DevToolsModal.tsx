import { useEffect, useState } from "react";
import { Bug, RefreshCw, ShieldAlert, Terminal, X } from "lucide-react";
import { ipc, isTauriRuntime } from "../../../shared/api/ipc";
import type { DevDiagnosticsSnapshot, LogEntry } from "../../../shared/types/ipc";
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
    <div className="modal-backdrop" onClick={() => dispatch({ type: "SET_DEV_MODAL", open: false })}>
      <div
        className="modal-card"
        style={{ width: 820, height: 560, padding: 16, gap: 12 }}
        onClick={(e) => e.stopPropagation()}
      >
        <div className="settings-row">
          <div style={{ fontWeight: 700, display: "flex", alignItems: "center", gap: 8 }}>
            <Bug size={17} color="var(--primary)" /> Công cụ kiểm tra & Chẩn đoán giao thức
          </div>
          <div style={{ display: "flex", gap: 6 }}>
            <button type="button" className="btn" onClick={() => void load()}>
              <RefreshCw size={14} /> Làm mới
            </button>
            <button
              type="button"
              className="icon-btn"
              onClick={() => dispatch({ type: "SET_DEV_MODAL", open: false })}
            >
              <X size={16} />
            </button>
          </div>
        </div>

        <div className="filter-tabs" style={{ maxWidth: 420 }}>
          <button
            type="button"
            className={`filter-tab ${tab === "logs" ? "active" : ""}`}
            onClick={() => setTab("logs")}
          >
            <Terminal size={13} /> Nhật ký HTTP ({diag?.logs.length ?? 0})
          </button>
          <button
            type="button"
            className={`filter-tab ${tab === "scanner" ? "active" : ""}`}
            onClick={() => setTab("scanner")}
          >
            <ShieldAlert size={13} /> Quét chẩn đoán ({diag?.findings.length ?? 0})
          </button>
          <button
            type="button"
            className={`filter-tab ${tab === "crypto" ? "active" : ""}`}
            onClick={() => setTab("crypto")}
          >
            Tham số giao thức
          </button>
        </div>

        <div style={{ flex: 1, minHeight: 0, overflow: "hidden", display: "flex", gap: 12 }}>
          {tab === "logs" ? (
            <>
              <div style={{ width: 320, overflowY: "auto", borderRight: "1px solid var(--border)", paddingRight: 8 }}>
                {(diag?.logs ?? []).slice().reverse().map((e) => (
                  <div
                    key={e.id}
                    className={`thread-row ${selectedLog?.id === e.id ? "selected" : ""}`}
                    style={{ fontSize: 12 }}
                    onClick={() => setSelectedLog(e)}
                  >
                    <span style={{ color: e.is_error ? "var(--danger)" : "var(--accent-online)", fontWeight: 700 }}>
                      {e.status ?? "ERR"}
                    </span>
                    <span style={{ flex: 1, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
                      {e.method} {e.url}
                    </span>
                    <span style={{ color: "var(--text-muted)" }}>{e.latency_ms}ms</span>
                  </div>
                ))}
              </div>
              <div style={{ flex: 1, overflowY: "auto", fontSize: 12.2 }}>
                {selectedLog ? (
                  <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
                    <div style={{ fontWeight: 700 }}>{selectedLog.method} {selectedLog.url}</div>
                    <div style={{ color: "var(--text-muted)" }}>
                      {selectedLog.timestamp} · {selectedLog.latency_ms}ms · HTTP {selectedLog.status ?? "N/A"}
                    </div>
                    {selectedLog.error_msg ? (
                      <div style={{ color: "var(--danger)" }}>Lỗi: {selectedLog.error_msg}</div>
                    ) : null}
                    <pre style={{ whiteSpace: "pre-wrap", background: "var(--bg-input)", padding: 10, borderRadius: 8 }}>
                      {selectedLog.decrypted_preview ?? selectedLog.payload_preview}
                    </pre>
                  </div>
                ) : (
                  <div style={{ color: "var(--text-muted)", padding: 20 }}>
                    {diag?.dev_mode
                      ? "Chọn một request bên trái để xem chi tiết."
                      : "Chế độ ghi nhật ký HTTP chỉ bật khi chạy với cờ --dev."}
                  </div>
                )}
              </div>
            </>
          ) : tab === "scanner" ? (
            <div style={{ flex: 1, overflowY: "auto", display: "flex", flexDirection: "column", gap: 8 }}>
              {(diag?.findings ?? []).map((f, idx) => (
                <div key={idx} className="settings-box">
                  <div style={{ fontWeight: 700 }}>
                    [{f.level}] {f.title}
                  </div>
                  <div style={{ fontSize: 12.5 }}>{f.detail}</div>
                  <div style={{ fontSize: 12, color: "var(--primary)" }}>Khắc phục: {f.suggestion}</div>
                </div>
              ))}
            </div>
          ) : (
            <div style={{ flex: 1, overflowY: "auto" }}>
              <div className="settings-box">
                <div className="settings-row">
                  <span>API Version (zpw_ver)</span>
                  <code>{diag?.crypto.zpw_ver}</code>
                </div>
                <div className="settings-row">
                  <span>API Type (zpw_type)</span>
                  <code>{diag?.crypto.zpw_type}</code>
                </div>
                <div className="settings-row">
                  <span>Thuật toán</span>
                  <code>{diag?.crypto.cipher_algo}</code>
                </div>
                <div className="settings-row">
                  <span>SignKey</span>
                  <code>{diag?.crypto.sign_key_formula}</code>
                </div>
                <div className="settings-row">
                  <span>ParamCipher key bytes</span>
                  <code>{diag?.crypto.param_cipher_key_len} bytes</code>
                </div>
              </div>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
