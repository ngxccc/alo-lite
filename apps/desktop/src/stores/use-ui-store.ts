import { create } from "zustand";
import { immer } from "zustand/middleware/immer";
import type { DevDiagnosticsSnapshot } from "@/shared/types/ipc";
import type { ImageViewerState, NavSection, SettingsTab } from "@/app/store/app-state";

export interface UIState {
  themeMode: "light" | "dark" | "system";
  activeSection: NavSection;
  devMode: boolean;
  appLogoDataUrl: string;
  activeDbPath: string;
  inlineVideoSupported: boolean;
  inlineVideoReason: string | null;
  devDiagnostics: DevDiagnosticsSnapshot | null;

  // Modals & Panels
  settingsOpen: boolean;
  activeSettingsTab: SettingsTab;
  devToolsOpen: boolean;
  phoneSyncOpen: boolean;
  phoneLookupOpen: boolean;
  wipeConfirmOpen: boolean;
  imageViewerState: ImageViewerState | null;

  // In-chat Search
  inChatSearchOpen: boolean;
  inChatSearchQuery: string;

  // Toast
  toast: { message: string; kind?: "info" | "error" | "success" } | null;

  // Actions
  setThemeMode: (mode: "light" | "dark" | "system") => void;
  setActiveSection: (section: NavSection) => void;
  setDevMode: (devMode: boolean) => void;
  setAppMeta: (meta: {
    appLogoDataUrl?: string;
    activeDbPath?: string;
    inlineVideoSupported?: boolean;
    inlineVideoReason?: string | null;
  }) => void;
  setDevDiagnostics: (diagnostics: DevDiagnosticsSnapshot | null) => void;

  setSettingsOpen: (open: boolean, tab?: SettingsTab) => void;
  setActiveSettingsTab: (tab: SettingsTab) => void;
  setDevToolsOpen: (open: boolean) => void;
  setPhoneSyncOpen: (open: boolean) => void;
  setPhoneLookupOpen: (open: boolean) => void;
  setWipeConfirmOpen: (open: boolean) => void;
  setImageViewerState: (viewer: ImageViewerState | null) => void;

  setInChatSearchOpen: (open: boolean) => void;
  setInChatSearchQuery: (query: string) => void;

  setToast: (toast: { message: string; kind?: "info" | "error" | "success" } | null) => void;
  clearToast: () => void;
}

export const useUIStore = create<UIState>()(
  immer((set) => ({
    themeMode: "dark",
    activeSection: "chat",
    devMode: false,
    appLogoDataUrl: "",
    activeDbPath: "",
    inlineVideoSupported: false,
    inlineVideoReason: null,
    devDiagnostics: null,

    settingsOpen: false,
    activeSettingsTab: "general",
    devToolsOpen: false,
    phoneSyncOpen: false,
    phoneLookupOpen: false,
    wipeConfirmOpen: false,
    imageViewerState: null,

    inChatSearchOpen: false,
    inChatSearchQuery: "",

    toast: null,

    setThemeMode: (mode) =>
      set((state) => {
        state.themeMode = mode;
      }),

    setActiveSection: (section) =>
      set((state) => {
        state.activeSection = section;
      }),

    setDevMode: (devMode) =>
      set((state) => {
        state.devMode = devMode;
      }),

    setAppMeta: (meta) =>
      set((state) => {
        if (meta.appLogoDataUrl !== undefined) state.appLogoDataUrl = meta.appLogoDataUrl;
        if (meta.activeDbPath !== undefined) state.activeDbPath = meta.activeDbPath;
        if (meta.inlineVideoSupported !== undefined)
          state.inlineVideoSupported = meta.inlineVideoSupported;
        if (meta.inlineVideoReason !== undefined) state.inlineVideoReason = meta.inlineVideoReason;
      }),

    setDevDiagnostics: (diagnostics) =>
      set((state) => {
        state.devDiagnostics = diagnostics;
      }),

    setSettingsOpen: (open, tab) =>
      set((state) => {
        state.settingsOpen = open;
        if (tab) state.activeSettingsTab = tab;
      }),

    setActiveSettingsTab: (tab) =>
      set((state) => {
        state.activeSettingsTab = tab;
      }),

    setDevToolsOpen: (open) =>
      set((state) => {
        state.devToolsOpen = open;
      }),

    setPhoneSyncOpen: (open) =>
      set((state) => {
        state.phoneSyncOpen = open;
      }),

    setPhoneLookupOpen: (open) =>
      set((state) => {
        state.phoneLookupOpen = open;
      }),

    setWipeConfirmOpen: (open) =>
      set((state) => {
        state.wipeConfirmOpen = open;
      }),

    setImageViewerState: (viewer) =>
      set((state) => {
        state.imageViewerState = viewer;
      }),

    setInChatSearchOpen: (open) =>
      set((state) => {
        state.inChatSearchOpen = open;
        if (!open) state.inChatSearchQuery = "";
      }),

    setInChatSearchQuery: (query) =>
      set((state) => {
        state.inChatSearchQuery = query;
      }),

    setToast: (toast) =>
      set((state) => {
        state.toast = toast;
      }),

    clearToast: () =>
      set((state) => {
        state.toast = null;
      }),
  })),
);
