import { useEffect, useReducer, useState } from "react";
import { ipc, isTauriRuntime } from "../shared/api/ipc";
import { ImageViewerModal } from "../shared/ui/ImageViewerModal";
import { LoginView } from "../features/auth/components/LoginView";
import { NavRail } from "../features/navigation/components/NavRail";
import { ThreadsSidebar } from "../features/threads/components/ThreadsSidebar";
import { InfoDrawer } from "../features/threads/components/InfoDrawer";
import { ChatArea } from "../features/chat/components/ChatArea";
import { ForwardModal } from "../features/chat/components/ForwardModal";
import { ContactsView } from "../features/contacts/components/ContactsView";
import { PhoneLookupModal } from "../features/contacts/components/PhoneLookupModal";
import { PhoneSyncModal } from "../features/sync/components/PhoneSyncModal";
import { SettingsModal } from "../features/settings/components/SettingsModal";
import { WipeConfirmModal } from "../features/settings/components/WipeConfirmModal";
import { DevToolsModal } from "../features/devtools/components/DevToolsModal";
import { appReducer, initialAppState } from "./store/appReducer";

export function App() {
  const [state, dispatch] = useReducer(appReducer, initialAppState);
  const [bootError, setBootError] = useState<string | null>(null);

  useEffect(() => {
    const mode = state.settings?.theme_mode ?? "dark";
    const resolved =
      mode === "system" && window.matchMedia("(prefers-color-scheme: light)").matches
        ? "light"
        : mode === "light"
          ? "light"
          : "dark";
    document.documentElement.setAttribute("data-theme", resolved);
  }, [state.settings?.theme_mode]);

  useEffect(() => {
    if (!isTauriRuntime()) {
      setBootError("Giao diện cần chạy bên trong cửa sổ ứng dụng Tauri AloLite.");
      return;
    }

    let unlisten: (() => void) | undefined;
    let active = true;

    void ipc
      .onCoreEvent((event) => {
        if (active) dispatch({ type: "CORE_EVENT", event });
      })
      .then((fn) => {
        unlisten = fn;
      })
      .catch((err) => setBootError(String(err)));

    void ipc
      .bootstrapState()
      .then((snapshot) => {
        if (active) {
          dispatch({ type: "BOOTSTRAP", snapshot });
          if (snapshot.logged_in && snapshot.settings.auto_check_updates) {
            void ipc.checkForUpdates(true);
          }
        }
      })
      .catch((err) => setBootError(String(err)));

    return () => {
      active = false;
      unlisten?.();
    };
  }, []);

  useEffect(() => {
    if (!state.toast) return;
    const t = window.setTimeout(() => dispatch({ type: "CLEAR_TOAST" }), 2600);
    return () => window.clearTimeout(t);
  }, [state.toast]);

  useEffect(() => {
    const onKeyDown = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && e.shiftKey && e.key.toLowerCase() === "p") {
        e.preventDefault();
        dispatch({ type: "OPEN_SETTINGS", open: true, tab: "privacy" });
      }
      if (e.key === "Escape") {
        if (state.imageViewer) dispatch({ type: "SET_IMAGE_VIEWER", viewer: null });
        else if (state.settingsModalOpen) dispatch({ type: "OPEN_SETTINGS", open: false });
        else if (state.phoneModalOpen) dispatch({ type: "SET_PHONE_MODAL", open: false });
        else if (state.phoneSyncModalOpen) dispatch({ type: "SET_PHONE_SYNC_MODAL", open: false });
        else if (state.devModalOpen) dispatch({ type: "SET_DEV_MODAL", open: false });
        else if (state.forwardMsg) dispatch({ type: "SET_FORWARD_MSG", msg: null });
        else if (state.wipeConfirmOpen) dispatch({ type: "SET_WIPE_CONFIRM", open: false });
      }
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [
    state.imageViewer,
    state.settingsModalOpen,
    state.phoneModalOpen,
    state.phoneSyncModalOpen,
    state.devModalOpen,
    state.forwardMsg,
    state.wipeConfirmOpen,
  ]);

  if (bootError) {
    return (
      <div className="login-shell">
        <div className="login-card">
          <div style={{ fontWeight: 700, fontSize: 16 }}>AloLite UI</div>
          <div style={{ color: "var(--danger)", fontSize: 13 }}>{bootError}</div>
        </div>
      </div>
    );
  }

  if (!state.bootstrapped) {
    return (
      <div className="login-shell">
        <div style={{ color: "var(--text-secondary)", fontSize: 13.5 }}>Đang khởi tạo CoreEngine…</div>
      </div>
    );
  }

  return (
    <>
      {!state.loggedIn ? (
        <LoginView state={state} dispatch={dispatch} />
      ) : (
        <div className="app-shell">
          <NavRail state={state} dispatch={dispatch} />
          {state.activeSection === "contacts" ? (
            <ContactsView state={state} dispatch={dispatch} />
          ) : (
            <>
              <ThreadsSidebar state={state} dispatch={dispatch} />
              <ChatArea state={state} dispatch={dispatch} />
              <InfoDrawer state={state} dispatch={dispatch} />
            </>
          )}
        </div>
      )}

      <SettingsModal state={state} dispatch={dispatch} />
      <PhoneLookupModal state={state} dispatch={dispatch} />
      <PhoneSyncModal state={state} dispatch={dispatch} />
      <ForwardModal state={state} dispatch={dispatch} />
      <WipeConfirmModal state={state} dispatch={dispatch} />
      <DevToolsModal state={state} dispatch={dispatch} />
      <ImageViewerModal
        viewer={state.imageViewer}
        mediaCache={state.mediaCache}
        downloadDir={state.settings?.download_dir ?? ""}
        onClose={() => dispatch({ type: "SET_IMAGE_VIEWER", viewer: null })}
      />

      {state.toast ? (
        <div className="toast-pill">
          <span
            style={{
              width: 8,
              height: 8,
              borderRadius: "50%",
              background: "var(--accent-online)",
              display: "inline-block",
            }}
          />
          <span>{state.toast.message}</span>
        </div>
      ) : null}
    </>
  );
}
