import type { AppAction, AppState } from "./appState";
import { handleCoreEvent } from "./coreEvents";
import { handleUiAction } from "./uiActions";

export type {
  AppAction,
  AppState,
  ContactsSubTab,
  ImageViewerState,
  NavSection,
  SettingsTab,
} from "./appState";
export { initialAppState } from "./appState";
export { handleCoreEvent } from "./coreEvents";

export function appReducer(state: AppState, action: AppAction): AppState {
  switch (action.type) {
    case "CORE_EVENT":
      return handleCoreEvent(state, action.event);
    default:
      return handleUiAction(state, action);
  }
}
