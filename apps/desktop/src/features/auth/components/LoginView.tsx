import { useEffect, useState, type FormEvent } from "react";
import { KeyRound, QrCode, RefreshCw, Shield } from "lucide-react";
import { ipc } from "../../../shared/api/ipc";
import { Button } from "../../../shared/ui/button";
import { Input } from "../../../shared/ui/input";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "../../../shared/ui/tabs";
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
    <div className="flex h-full w-full items-center justify-center bg-canvas p-6">
      <div className="flex w-full max-w-[400px] flex-col gap-4 rounded-xl border border-border bg-card p-6 text-card-foreground shadow-2xl">
        <div className="flex items-center gap-3">
          {state.appLogoDataUrl ? (
            <img
              src={state.appLogoDataUrl}
              alt="AloLite"
              className="h-10 w-10 rounded-xl object-contain"
            />
          ) : null}
          <div>
            <div className="text-lg font-bold tracking-tight">AloLite</div>
            <div className="text-xs text-muted-foreground">
              Đăng nhập tài khoản Zalo (v{state.appVersion})
            </div>
          </div>
        </div>

        <Tabs
          value={tab}
          onValueChange={(v) => setTab(v as "qr" | "password")}
          className="flex flex-col gap-4"
        >
          <TabsList className="grid w-full grid-cols-2">
            <TabsTrigger value="qr" className="gap-1.5">
              <QrCode className="h-3.5 w-3.5" /> Quét mã QR
            </TabsTrigger>
            <TabsTrigger value="password" className="gap-1.5">
              <KeyRound className="h-3.5 w-3.5" /> Mật khẩu
            </TabsTrigger>
          </TabsList>

          <TabsContent value="qr" className="flex flex-col items-center gap-3">
            <div className="grid h-[220px] w-[220px] place-items-center rounded-xl border border-border bg-white p-2.5 shadow-xs">
              {state.qrDataUrl ? (
                <img
                  src={state.qrDataUrl}
                  alt="Zalo QR"
                  className="h-full w-full [image-rendering:pixelated]"
                />
              ) : (
                <span className="text-center text-xs text-slate-600">Đang tạo mã QR…</span>
              )}
            </div>

            {state.qrScannedName ? (
              <div className="w-full rounded-md bg-online/15 px-3 py-2 text-center text-xs font-semibold text-online">
                Đã quét bởi {state.qrScannedName} — Hãy bấm Xác nhận trên điện thoại
              </div>
            ) : (
              <div className="text-center text-xs text-muted-foreground">{state.qrStatus}</div>
            )}

            <Button variant="outline" size="sm" onClick={() => void ipc.requestQr()}>
              <RefreshCw className="h-3.5 w-3.5" /> Tạo mã QR mới
            </Button>
          </TabsContent>

          <TabsContent value="password">
            <form onSubmit={submitPassword} className="flex flex-col gap-3">
              <label className="flex flex-col gap-1">
                <span className="text-xs font-medium text-muted-foreground">Số điện thoại</span>
                <Input
                  type="text"
                  value={phone}
                  onChange={(e) => setPhone(e.target.value)}
                  placeholder="09xxxxxxxx"
                  autoFocus
                />
              </label>
              <label className="flex flex-col gap-1">
                <span className="text-xs font-medium text-muted-foreground">Mật khẩu</span>
                <Input
                  type="password"
                  value={pass}
                  onChange={(e) => setPass(e.target.value)}
                  placeholder="••••••••"
                />
              </label>
              <Button
                type="submit"
                variant="default"
                className="w-full"
                disabled={state.loginInProgress || !phone.trim() || !pass}
              >
                {state.loginInProgress ? "Đang xác thực…" : "Đăng nhập"}
              </Button>
            </form>
          </TabsContent>
        </Tabs>

        {state.loginError ? (
          <div className="rounded-md bg-destructive/10 px-3 py-2 text-center text-xs text-destructive">
            {state.loginError}
          </div>
        ) : null}

        <div className="border-t border-border pt-2.5">
          <Button
            variant="ghost"
            size="sm"
            className="w-full justify-between px-2"
            onClick={() => setShowProxy((v) => !v)}
          >
            <span className="inline-flex items-center gap-1.5">
              <Shield className="h-3.5 w-3.5" /> Cấu hình Proxy HTTP / SOCKS5
            </span>
            <span className="text-[11px] text-muted-foreground">{showProxy ? "Ẩn" : "Mở"}</span>
          </Button>
          {showProxy ? (
            <div className="mt-2 flex gap-1.5">
              <Input
                value={proxyInput}
                onChange={(e) => setProxyInput(e.target.value)}
                placeholder="http://127.0.0.1:8080 hoặc socks5://..."
              />
              <Button variant="default" onClick={applyProxy}>
                Áp dụng
              </Button>
            </div>
          ) : null}
        </div>

        {state.savedAccounts.length > 0 ? (
          <div className="flex flex-col gap-1.5 border-t border-border pt-2.5">
            <div className="text-xs font-semibold text-muted-foreground">
              Đăng nhập nhanh tài khoản đã lưu
            </div>
            {state.savedAccounts.map((acc) => (
              <Button
                key={acc.uin}
                variant="outline"
                size="sm"
                className="w-full justify-between"
                onClick={() => void ipc.switchAccount(acc.uin)}
              >
                <span className="truncate font-medium">{acc.display_name}</span>
                <span className="text-[11px] text-muted-foreground">UIN: {acc.uin}</span>
              </Button>
            ))}
          </div>
        ) : null}
      </div>
    </div>
  );
}
