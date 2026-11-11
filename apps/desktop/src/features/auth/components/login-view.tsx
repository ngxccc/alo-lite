import { useState, type SyntheticEvent } from "react";
import {
  AlertCircle,
  KeyRound,
  Loader2,
  QrCode,
  RefreshCw,
  Smartphone,
  UserCheck,
} from "lucide-react";
import { ipc } from "@/shared/api/ipc";
import { Avatar } from "@/shared/ui/avatar";
import { Badge } from "@/shared/ui/badge";
import { Button } from "@/shared/ui/button";
import { Input } from "@/shared/ui/input";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/shared/ui/tabs";
import { TooltipProvider } from "@/shared/ui/tooltip";
import type { AppAction, AppState } from "@/app/store/app-reducer";
import { useAppStore } from "@/app/store/use-app-store";

interface LoginViewProps {
  state?: AppState;
  dispatch?: (action: AppAction) => void;
}

function formatAuthErrorMessage(error: string): { title: string; detail: string } {
  const raw = error.trim();
  if (!raw) return { title: "Đã xảy ra lỗi", detail: "Vui lòng thử lại sau." };

  if (raw.includes("chk_wait_scan") || raw.includes("reqqr")) {
    return {
      title: "Không thể lấy mã QR",
      detail:
        "Máy chủ Zalo chưa phản hồi mã quét hợp lệ. Vui lòng bấm thử lại hoặc kiểm tra kết nối mạng.",
    };
  }
  if (raw.includes("hết hạn") || raw.includes("expired") || raw.includes("QrExpired")) {
    return {
      title: "Mã QR đã hết hạn",
      detail: "Phiên chờ quét đã kết thúc. Vui lòng bấm tạo mã mới để tiếp tục.",
    };
  }
  if (raw.includes("mạng") || raw.includes("timeout") || raw.includes("connect")) {
    return {
      title: "Lỗi kết nối mạng",
      detail: "Không thể kết nối đến máy chủ Zalo. Vui lòng kiểm tra đường truyền Internet.",
    };
  }
  if (raw.includes("sai") || raw.includes("mật khẩu") || raw.includes("tài khoản")) {
    return {
      title: "Thông tin không chính xác",
      detail: raw,
    };
  }
  if (raw.includes("18029") || raw.includes("Captcha") || raw.includes("mã xác thực")) {
    return {
      title: "Yêu cầu mã xác thực",
      detail: "Vui lòng nhập các ký tự từ hình ảnh bên dưới để tiếp tục.",
    };
  }

  return {
    title: "Đăng nhập không thành công",
    detail: raw,
  };
}

export function LoginView({ state: propState, dispatch: propDispatch }: LoginViewProps = {}) {
  const appStore = useAppStore();
  const state = propState ?? appStore;
  const dispatch = propDispatch ?? appStore.dispatch;

  const [tab, setTab] = useState<"qr" | "password">("qr");
  const [phone, setPhone] = useState("");
  const [pass, setPass] = useState("");
  const [captchaCode, setCaptchaCode] = useState("");
  const [qrRequested, setQrRequested] = useState(false);

  // Starting a login is an explicit user action: mounting the view must not touch the network.
  const startQr = () => {
    if (state.loginInProgress) return;
    setQrRequested(true);
    void ipc.requestQr();
  };

  const handlePasswordLogin = (e: SyntheticEvent<HTMLFormElement>) => {
    e.preventDefault();
    if (!phone.trim() || !pass) return;
    if (state.captchaToken && !captchaCode.trim()) return;
    dispatch({
      type: "SET_LOGIN_IN_PROGRESS",
      pending: true,
      status: "Đang đăng nhập…",
    });
    void ipc.loginPassword(
      phone.trim(),
      pass,
      state.captchaToken ?? undefined,
      captchaCode.trim() ? captchaCode.trim() : undefined,
    );
  };

  const hasQrError = Boolean(state.loginError && tab === "qr" && !state.qrDataUrl);
  const formattedError = state.loginError ? formatAuthErrorMessage(state.loginError) : null;

  return (
    <TooltipProvider delayDuration={250}>
      <div className="flex h-full min-h-screen w-full items-center justify-center bg-canvas p-6 selection:bg-primary/20">
        <div className="flex w-full max-w-[440px] flex-col gap-5 rounded-2xl border border-border/80 bg-card/95 p-7 text-card-foreground shadow-2xl backdrop-blur-md">
          {/* ── Header ── */}
          <div className="flex items-center justify-between border-b border-border/60 pb-4">
            <div className="flex items-center gap-3">
              {state.appLogoDataUrl ? (
                <img
                  src={state.appLogoDataUrl}
                  alt="AloLite"
                  className="h-10 w-10 rounded-xl object-contain shadow-xs"
                />
              ) : (
                <div className="flex h-10 w-10 items-center justify-center rounded-xl bg-primary text-base font-bold text-primary-foreground shadow-xs">
                  A
                </div>
              )}
              <div>
                <div className="flex items-center gap-1.5 leading-none">
                  <span className="text-lg font-bold tracking-tight text-foreground">AloLite</span>
                  <Badge variant="secondary" className="px-1.5 py-0.5 font-mono text-xs">
                    v{state.appVersion}
                  </Badge>
                </div>
                <div className="mt-1 text-sm text-muted-foreground">Đăng nhập tài khoản Zalo</div>
              </div>
            </div>
          </div>

          {/* ── Auth Tabs ── */}
          <Tabs
            value={tab}
            onValueChange={(v) => {
              setTab(v as "qr" | "password");
              // Switching tabs must stay offline: the QR ticket is created only by the
              // explicit buttons below, because the webview focuses the first trigger on
              // mount and would otherwise fire `onValueChange` without any user action.
              if (state.loginError) {
                dispatch({
                  type: "CORE_EVENT",
                  event: { type: "LoginFailed", payload: { message: "" } },
                });
              }
            }}
            className="flex flex-col gap-4"
          >
            <TabsList className="grid h-10 w-full grid-cols-2 rounded-lg bg-muted p-1">
              <TabsTrigger value="qr" className="h-8 gap-1.5 text-sm font-medium">
                <QrCode className="h-4 w-4" /> Quét mã QR
              </TabsTrigger>
              <TabsTrigger value="password" className="h-8 gap-1.5 text-sm font-medium">
                <KeyRound className="h-4 w-4" /> Mật khẩu
              </TabsTrigger>
            </TabsList>

            {/* ── QR Login Tab ── */}
            <TabsContent value="qr" className="m-0 flex flex-col items-center gap-3">
              {/* Soft Elevated White Plate */}
              <div className="relative flex h-[240px] w-[240px] items-center justify-center overflow-hidden rounded-2xl border border-black/10 bg-white p-3 shadow-md">
                {/* State A: Phone scanned, awaiting mobile confirmation */}
                {state.qrScannedName ? (
                  <div className="animate-in fade-in zoom-in-95 flex flex-col items-center justify-center gap-2.5 p-2 text-center duration-200">
                    <div className="relative">
                      {state.qrScannedAvatar ? (
                        <img
                          src={state.qrScannedAvatar}
                          alt={state.qrScannedName}
                          className="h-16 w-16 animate-pulse rounded-full border-2 border-white object-cover shadow-md ring-4 ring-online/30"
                        />
                      ) : (
                        <div className="flex h-16 w-16 animate-pulse items-center justify-center rounded-full bg-primary/10 text-primary ring-4 ring-online/30">
                          <UserCheck className="h-8 w-8" />
                        </div>
                      )}
                      <span className="absolute right-0 bottom-0 flex h-4 w-4 items-center justify-center rounded-full bg-online text-white shadow-xs">
                        <span className="h-2 w-2 rounded-full bg-white" />
                      </span>
                    </div>
                    <div>
                      <div className="max-w-[170px] truncate text-xs font-bold text-slate-800">
                        {state.qrScannedName}
                      </div>
                      <div className="mt-0.5 text-xs font-medium text-online">
                        Đã quét mã thành công
                      </div>
                    </div>
                  </div>
                ) : hasQrError && formattedError ? (
                  /* State B: Error / Timeout directly on QR plate */
                  <div className="animate-in fade-in flex flex-col items-center justify-center gap-2 p-3 text-center duration-150">
                    <div className="flex h-11 w-11 items-center justify-center rounded-full bg-destructive/15 text-destructive">
                      <AlertCircle className="h-6 w-6" />
                    </div>
                    <div className="text-xs font-bold text-slate-800">{formattedError.title}</div>
                    <div className="line-clamp-3 text-xs leading-relaxed text-slate-500">
                      {formattedError.detail}
                    </div>
                    <Button
                      variant="default"
                      size="sm"
                      className="mt-1 h-7 gap-1.5 rounded-md px-3 text-xs"
                      onClick={startQr}
                    >
                      <RefreshCw className="h-3 w-3" /> Thử lại
                    </Button>
                  </div>
                ) : state.qrDataUrl ? (
                  /* State C: QR ready to be scanned */
                  <img
                    src={state.qrDataUrl}
                    alt="Zalo QR"
                    className="h-full w-full object-contain select-none [image-rendering:pixelated]"
                  />
                ) : qrRequested ? (
                  /* State D: Loading QR skeleton */
                  <div className="flex flex-col items-center justify-center gap-2.5 text-center">
                    <div className="relative flex h-14 w-14 items-center justify-center rounded-xl border-2 border-dashed border-primary/40 bg-primary/10">
                      <Loader2 className="h-6 w-6 animate-spin text-primary" />
                    </div>
                    <div className="text-xs font-medium text-slate-600">
                      Đang tạo mã QR bảo mật…
                    </div>
                  </div>
                ) : (
                  /* State E: Idle — nothing requested until the user asks for a code */
                  <div className="flex flex-col items-center justify-center gap-2.5 p-3 text-center">
                    <div className="flex h-14 w-14 items-center justify-center rounded-xl border-2 border-dashed border-primary/40 bg-primary/10 text-primary">
                      <QrCode className="h-6 w-6" />
                    </div>
                    <div className="text-xs font-medium text-slate-600">
                      Bấm để tạo mã QR đăng nhập
                    </div>
                    <Button
                      variant="default"
                      size="sm"
                      className="mt-0.5 h-7 gap-1.5 rounded-md px-3 text-xs"
                      onClick={startQr}
                    >
                      <QrCode className="h-3 w-3" /> Tạo mã QR
                    </Button>
                  </div>
                )}
              </div>

              {/* Status explanation below plate */}
              {state.qrScannedName ? (
                <div className="flex items-center gap-1.5 text-center text-xs font-medium text-foreground">
                  <Smartphone className="h-3.5 w-3.5 animate-bounce text-primary" />
                  <span>Hãy bấm &quot;Đăng nhập&quot; trên điện thoại của bạn</span>
                </div>
              ) : !hasQrError ? (
                <div className="flex flex-col items-center gap-1 text-center">
                  <div className="text-xs text-muted-foreground">
                    {state.qrDataUrl
                      ? "Mở ứng dụng Zalo trên điện thoại để quét mã"
                      : "Mã QR chỉ được tạo khi bạn yêu cầu"}
                  </div>
                  {state.qrDataUrl ? (
                    <Button
                      variant="ghost"
                      size="sm"
                      className="h-6 gap-1 px-2 text-xs text-muted-foreground hover:text-foreground"
                      onClick={startQr}
                    >
                      <RefreshCw className="h-3 w-3" /> Làm mới mã
                    </Button>
                  ) : null}
                </div>
              ) : null}
            </TabsContent>

            {/* ── Password Login Tab ── */}
            <TabsContent value="password" className="m-0">
              <form onSubmit={handlePasswordLogin} className="flex flex-col gap-3.5">
                <div className="flex flex-col gap-1.5">
                  <label htmlFor="phone-input" className="text-sm font-medium text-foreground">
                    Số điện thoại
                  </label>
                  <Input
                    id="phone-input"
                    type="tel"
                    value={phone}
                    onChange={(e) => setPhone(e.target.value)}
                    placeholder="09xxxxxxxx"
                    autoFocus
                    disabled={state.loginInProgress}
                    className="h-10 text-sm"
                  />
                </div>

                <div className="flex flex-col gap-1.5">
                  <label htmlFor="password-input" className="text-sm font-medium text-foreground">
                    Mật khẩu
                  </label>
                  <Input
                    id="password-input"
                    type="password"
                    value={pass}
                    onChange={(e) => setPass(e.target.value)}
                    placeholder="••••••••"
                    disabled={state.loginInProgress}
                    className="h-10 text-sm"
                  />
                </div>
                {state.captchaImg ? (
                  <div className="flex flex-col gap-2 rounded-lg border border-border/80 bg-muted/30 p-3">
                    <div className="flex items-center justify-between">
                      <label
                        htmlFor="captcha-input"
                        className="text-sm font-medium text-foreground"
                      >
                        Mã xác thực
                      </label>
                      <Button
                        type="button"
                        variant="ghost"
                        size="sm"
                        className="h-7 gap-1 px-2 text-xs text-muted-foreground hover:text-foreground"
                        onClick={() => void ipc.refreshCaptcha()}
                        disabled={state.loginInProgress}
                        title="Đổi mã khác"
                      >
                        <RefreshCw className="h-3.5 w-3.5" />
                        <span>Đổi mã</span>
                      </Button>
                    </div>

                    <div className="flex items-center gap-2.5">
                      <div className="flex h-11 w-40 shrink-0 items-center justify-center overflow-hidden rounded border border-border bg-white p-0.5 shadow-xs">
                        <img
                          src={`data:image/jpeg;base64,${state.captchaImg}`}
                          alt="Mã xác thực"
                          className="h-full w-full object-contain select-none"
                        />
                      </div>
                      <Input
                        id="captcha-input"
                        type="text"
                        value={captchaCode}
                        onChange={(e) => setCaptchaCode(e.target.value)}
                        placeholder="Nhập mã"
                        autoFocus
                        autoComplete="off"
                        autoCorrect="off"
                        autoCapitalize="off"
                        spellCheck={false}
                        disabled={state.loginInProgress}
                        maxLength={8}
                        className="h-11 text-center text-base font-medium tracking-wider"
                      />
                    </div>
                  </div>
                ) : null}

                {state.loginError && tab === "password" && formattedError ? (
                  <div className="flex items-start gap-2.5 rounded-lg border border-destructive/30 bg-destructive/10 p-3 text-sm text-destructive">
                    <AlertCircle className="mt-0.5 h-4.5 w-4.5 shrink-0" />
                    <div>
                      <div className="font-semibold">{formattedError.title}</div>
                      <div className="mt-0.5 text-xs opacity-90">{formattedError.detail}</div>
                    </div>
                  </div>
                ) : null}

                <Button
                  type="submit"
                  variant="default"
                  className="mt-1 h-10 w-full gap-2 text-sm font-semibold"
                  disabled={
                    state.loginInProgress ||
                    !phone.trim() ||
                    !pass ||
                    (Boolean(state.captchaToken) && !captchaCode.trim())
                  }
                >
                  {state.loginInProgress ? (
                    <>
                      <Loader2 className="h-4 w-4 animate-spin" />
                      <span>Đang xác thực tài khoản…</span>
                    </>
                  ) : (
                    "Đăng nhập"
                  )}
                </Button>
              </form>
            </TabsContent>
          </Tabs>

          {/* ── Saved accounts switcher (macOS Switch User Style) ── */}
          {state.savedAccounts.length > 0 ? (
            <div className="border-t border-border/60 pt-3.5">
              <div className="mb-2 text-xs font-semibold tracking-wider text-muted-foreground uppercase">
                Tài khoản đã lưu trên máy
              </div>
              <div className="flex max-h-[150px] flex-col gap-2 overflow-y-auto pr-0.5">
                {state.savedAccounts.map((acc) => (
                  <button
                    key={acc.uin}
                    type="button"
                    onClick={() => void ipc.switchAccount(acc.uin)}
                    className="group flex w-full cursor-pointer items-center justify-between rounded-xl border border-border/50 bg-secondary/40 p-2.5 text-left transition hover:border-primary/50 hover:bg-accent/40 focus:ring-2 focus:ring-ring focus:outline-none"
                  >
                    <div className="flex min-w-0 items-center gap-3">
                      <Avatar
                        id={acc.uin}
                        name={acc.display_name}
                        avatarUrl={acc.avatar}
                        size={32}
                        className="border border-border/60"
                      />
                      <div className="min-w-0 truncate">
                        <div className="truncate text-sm font-semibold text-foreground group-hover:text-primary">
                          {acc.display_name || "Tài khoản Zalo"}
                        </div>
                        <div className="font-mono text-xs text-muted-foreground">
                          UIN: {acc.uin}
                        </div>
                      </div>
                    </div>
                    <Badge variant="outline" className="text-xs opacity-70 group-hover:opacity-100">
                      Chuyển
                    </Badge>
                  </button>
                ))}
              </div>
            </div>
          ) : null}
        </div>
      </div>
    </TooltipProvider>
  );
}
