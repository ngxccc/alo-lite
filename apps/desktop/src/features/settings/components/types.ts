import type { AppAction, AppState } from "@/app/store/app-reducer";
import type { AppSettings } from "@/shared/types/ipc";

export interface SettingsModalProps {
  state?: AppState;
  dispatch?: (action: AppAction) => void;
}

export interface SettingsTabProps {
  state: AppState;
  settings: AppSettings;
  dispatch: (action: AppAction) => void;
  updateSettings: (patch: Partial<AppSettings>) => void;
}
