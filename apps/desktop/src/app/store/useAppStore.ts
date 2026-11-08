import { create } from "zustand";
import { appReducer, initialAppState, type AppAction, type AppState } from "./appReducer";

export interface AppStore extends AppState {
  dispatch: (action: AppAction) => void;
}

export const useAppStore = create<AppStore>((set) => ({
  ...initialAppState,
  dispatch: (action: AppAction) =>
    set((state) => {
      const next = appReducer(state, action);
      return next;
    }),
}));
