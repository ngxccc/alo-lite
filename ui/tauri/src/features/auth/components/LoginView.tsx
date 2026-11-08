import { useEffect, useState, type FormEvent } from "react";
import { KeyRound, QrCode, RefreshCw, Shield } from "lucide-react";
import { ipc } from "../../../shared/api/ipc";
import type { AppAction, AppState } from "../../../app/store/appReducer";

interface LoginViewProps {
  state: AppState;
  dispatch: (action: AppAction) => void;
}

export function LoginView({ state, dispatch }: LoginViewProps) {
  const [tab, setTab] = useState<"qr" | "password">("qr");
  const [phone, setPhone] = useState("");
  const [pass, setPass] = useState("");
  const [proxyInput, setProxyInput] = useState(state.settings?.proxy_url ?? "");
  const [showProxy, setShowProxy] = useState(false);

  useEffect(() => {
    if (tab === "qr" && !state.qrDataUrl) {
      void ipc.requestQr();
    }
  }, [tab, state.qrDataUrl]);

  const submitPassword = (e: FormEvent) => {
    e.preventDefault();
    if (!phone.trim() || !pass) return;
    dispatch({ type: "SET_LOGIN_IN_PROGRESS", pending: true, status: "Đang đăng nhập…" });
    void ipc.loginPassword(phone.trim(), pass);
  };

  const applyProxy = () => {
    void ipc.setProxy(proxyInput.trim());
    if (state.settings) {
      const updated = { ...state.settings, proxy_url: proxyInput.trim() };
      dispatch({ type: "UPDATE_SETTINGS_LOCAL", settings: updated });
      void ipc.saveSettings(updated);
    }
  };

  return (
    <div className="login-shell">
      <div className="login-card">
        <div style={{ display: "flex", alignItems: "center", gap: 12 }}>
          {state.appLogoDataUrl ? (
            <img
              src={state.appLogoDataUrl}
              alt="AloLite"
              style={{ width: 42, height: 42, borderRadius: 10 }}
            />
          ) : null}
          <div>
            <div style={{ fontSize: 18, fontWeight: 700 }}>AloLite</div>
            <div style={{ fontSize: 12, color: "var(--text-secondary)" }}>
              Đăng nhập tài khoản Zalo (v{state.appVersion})
            </div>
          </div>
        </div>

        <div className="filter-tabs" style={{ background: "var(--bg-sidebar)", padding: 4, borderRadius: 10 }}>
          <button
            type="button"
            className={`filter-tab ${tab === "qr" ? "active" : ""}`}
            onClick={() => setTab("qr")}
          >
            <span style={{ display: "inline-flex", alignItems: "center", gap: 6 }}>
              <QrCode size={14} /> Quét mã QR
            </span>
          </button>
          <button
            type="button"
            className={`filter-tab ${tab === "password" ? "active" : ""}`}
            onClick={() => setTab("password")}
          >
            <span style={{ display: "inline-flex", alignItems: "center", gap: 6 }}>
              <KeyRound size={14} /> Mật khẩu
            </span>
          </button>
        </div>

        {tab === "qr" ? (
          <div style={{ display: "flex", flexDirection: "column", alignItems: "center", gap: 12 }}>
            <div
              style={{
                width: 220,
                height: 220,
                background: "#fff",
                borderRadius: 12,
                padding: 10,
                display: "grid",
                placeItems: "center",
                border: "1px solid var(--border)",
              }}
            >
              {state.qrDataUrl ? (
                <img
                  src={state.qrDataUrl}
                  alt="Zalo QR"
                  style={{ width: "100%", height: "100%", imageRendering: "pixelated" }}
                />
              ) : (
                <span style={{ color: "#475569", fontSize: 12, textAlign: "center" }}>
                  Đang tạo mã QR…
                </span>
              )}
            </div>

            {state.qrScannedName ? (
              <div
                style={{
                  padding: "8px 12px",
                  borderRadius: 8,
                  background: "color-mix(in srgb, var(--accent-online) 16%, transparent)",
                  color: "var(--accent-online)",
                  fontWeight: 600,
                  textAlign: "center",
                }}
              >
                Đã quét bởi {state.qrScannedName} — Hãy bấm Xác nhận trên điện thoại
              </div>
            ) : (
              <div style={{ fontSize: 12.4, color: "var(--text-secondary)", textAlign: "center" }}>
                {state.qrStatus}
              </div>
            )}

            <button
              type="button"
              className="btn"
              onClick={() => void ipc.requestQr()}
            >
              <RefreshCw size={14} /> Tạo mã QR mới
            </button>
          </div>
        ) : (
          <form onSubmit={submitPassword} style={{ display: "flex", flexDirection: "column", gap: 10 }}>
            <label style={{ display: "flex", flexDirection: "column", gap: 4 }}>
              <span style={{ fontSize: 12, color: "var(--text-secondary)" }}>Số điện thoại</span>
              <input
                className="field-input"
                type="text"
                value={phone}
                onChange={(e) => setPhone(e.target.value)}
                placeholder="09xxxxxxxx"
                autoFocus
              />
            </label>
            <label style={{ display: "flex", flexDirection: "column", gap: 4 }}>
              <span style={{ fontSize: 12, color: "var(--text-secondary)" }}>Mật khẩu</span>
              <input
                className="field-input"
                type="password"
                value={pass}
                onChange={(e) => setPass(e.target.value)}
                placeholder="••••••••"
              />
            </label>
            <button
              type="submit"
              className="btn btn-primary"
              disabled={state.loginInProgress || !phone.trim() || !pass}
            >
              {state.loginInProgress ? "Đang xác thực…" : "Đăng nhập"}
            </button>
          </form>
        )}

        {state.loginError ? (
          <div style={{ color: "var(--danger)", fontSize: 12.2, textAlign: "center" }}>
            {state.loginError}
          </div>
        ) : null}

        <div style={{ borderTop: "1px solid var(--border)", paddingTop: 10 }}>
          <button
            type="button"
            className="btn"
            style={{ width: "100%", justifyContent: "space-between" }}
            onClick={() => setShowProxy((v) => !v)}
          >
            <span style={{ display: "inline-flex", alignItems: "center", gap: 6 }}>
              <Shield size={14} /> Cấu hình Proxy HTTP / SOCKS5
            </span>
            <span style={{ fontSize: 11.5, color: "var(--text-muted)" }}>
              {showProxy ? "Ẩn" : "Mở"}
            </span>
          </button>
          {showProxy ? (
            <div style={{ display: "flex", gap: 6, marginTop: 8 }}>
              <input
                className="field-input"
                value={proxyInput}
                onChange={(e) => setProxyInput(e.target.value)}
                placeholder="http://127.0.0.1:8080 hoặc socks5://..."
              />
              <button type="button" className="btn btn-primary" onClick={applyProxy}>
                Áp dụng
              </button>
            </div>
          ) : null}
        </div>
        {state.savedAccounts.length > 0 ? (
          <div style={{ borderTop: "1px solid var(--border)", paddingTop: 10, display: "flex", flexDirection: "column", gap: 6 }}>
            <div style={{ fontSize: 12, color: "var(--text-secondary)", fontWeight: 600 }}>
              Đăng nhập nhanh tài khoản đã lưu
            </div>
            {state.savedAccounts.map((acc) => (
              <button
                key={acc.uin}
                type="button"
                className="btn"
                style={{ justifyContent: "space-between" }}
                onClick={() => void ipc.switchAccount(acc.uin)}
              >
                <span>{acc.display_name}</span>
                <span style={{ fontSize: 11.2, color: "var(--text-muted)" }}>UIN: {acc.uin}</span>
              </button>
            ))}
          </div>
        ) : null}
      </div>
    </div>
  );
}
