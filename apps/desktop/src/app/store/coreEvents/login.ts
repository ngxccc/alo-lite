import { sortThreads } from "@/features/threads/utils/threads";
import type { CoreEventDto } from "@/shared/types/ipc.generated";
import type { AppState } from "../app-state";
import { nextToast } from "../helpers";

export type LoginCoreEvent = Extract<
  CoreEventDto,
  | { type: "QrReady" }
  | { type: "QrScanned" }
  | { type: "QrExpired" }
  | { type: "LoginSuccess" }
  | { type: "LoginFailed" }
  | { type: "CaptchaRequired" }
  | { type: "LoggedOut" }
  | { type: "SessionExpired" }
  | { type: "AccountSwitched" }
  | { type: "SessionUnreachable" }
>;

export function handleLoginEvent(state: AppState, event: LoginCoreEvent): AppState {
  switch (event.type) {
    case "QrReady":
      return {
        ...state,
        qrDataUrl: event.payload.qr_data_url || null,
        qrStatus: "Mở Zalo trên điện thoại và quét mã QR để đăng nhập",
        qrScannedName: null,
        qrScannedAvatar: null,
        loginInProgress: false,
        loginError: "",
      };

    case "QrScanned":
      return {
        ...state,
        qrScannedName: event.payload.name,
        qrScannedAvatar: event.payload.avatar,
        qrStatus: `Đã quét (${event.payload.name}) — Vui lòng xác nhận đăng nhập trên điện thoại`,
      };

    case "QrExpired":
      return {
        ...state,
        qrDataUrl: null,
        qrScannedName: null,
        qrStatus: event.payload.message,
      };

    case "LoginSuccess":
      return {
        ...state,
        loggedIn: true,
        uin: event.payload.uin,
        displayName: event.payload.name,
        avatar: event.payload.avatar,
        loginInProgress: false,
        loginError: "",
        captchaToken: null,
        captchaImg: null,
        qrDataUrl: null,
        qrScannedName: null,
        statusMessage: "Đã đăng nhập",
        toast: nextToast(`Chào mừng ${event.payload.name}`),
      };

    case "LoginFailed":
      return {
        ...state,
        loginInProgress: false,
        loginError: event.payload.message,
        statusMessage: event.payload.message,
      };
    case "CaptchaRequired":
      return {
        ...state,
        loginInProgress: false,
        loginError: "",
        captchaToken: event.payload.token,
        captchaImg: event.payload.captcha_img,
      };

    case "LoggedOut":
    case "SessionExpired":
      return {
        ...state,
        loggedIn: false,
        uin: "",
        displayName: "",
        avatar: null,
        threads: [],
        selectedThreadId: null,
        unreadCounts: {},
        settingsModalOpen: false,
        qrDataUrl: null,
        qrScannedName: null,
        qrStatus: "Đang xin mã QR mới…",
        statusMessage: event.type === "SessionExpired" ? event.payload.message : "Đã đăng xuất",
      };

    case "AccountSwitched": {
      const sorted = state.settings
        ? sortThreads(event.payload.threads, state.settings)
        : event.payload.threads;
      return {
        ...state,
        loggedIn: true,
        uin: event.payload.uin,
        displayName: event.payload.name,
        avatar: event.payload.avatar,
        threads: sorted,
        selectedThreadId: sorted[0]?.id ?? null,
        unreadCounts: {},
        settingsModalOpen: false,
        statusMessage: `Đã chuyển sang tài khoản ${event.payload.name}`,
        toast: nextToast(`Đã chuyển sang ${event.payload.name}`),
      };
    }

    case "SessionUnreachable":
      return {
        ...state,
        statusMessage: `Mất kết nối tạm thời: ${event.payload.message}`,
      };
  }
}
