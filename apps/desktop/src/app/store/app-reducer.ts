import type { AppAction, AppState } from "./app-state";
import { handleCoreEvent } from "./coreEvents";
import { handleUiAction } from "./ui-actions";

export type {
  AppAction,
  AppState,
  ContactsSubTab,
  ImageViewerState,
  NavSection,
  SettingsTab,
} from "./app-state";
export { initialAppState } from "./app-state";
export { handleCoreEvent } from "./coreEvents";

export function appReducer(state: AppState, action: AppAction): AppState {
  switch (action.type) {
    case "CORE_EVENT":
      return handleCoreEvent(state, action.event);
    default:
      return handleUiAction(state, action);
  }
}
