import type { CoreEventDto } from "@/shared/types/ipc";
import type { AppState } from "../app-state";

export type ContactsCoreEvent = Extract<
  CoreEventDto,
  { type: "PhoneLookupSuccess" } | { type: "PhoneLookupFailed" } | { type: "FriendStatusChecked" }
>;

export function handleContactsEvent(state: AppState, event: ContactsCoreEvent): AppState {
  switch (event.type) {
    case "PhoneLookupSuccess":
      return {
        ...state,
        phoneLookupLoading: false,
        phoneLookupResult: event.payload.profile,
        phoneLookupError: null,
        friendStatusMap: {
          ...state.friendStatusMap,
          [event.payload.profile.user_id]: event.payload.profile.friendship,
        },
      };

    case "PhoneLookupFailed":
      return {
        ...state,
        phoneLookupLoading: false,
        phoneLookupResult: null,
        phoneLookupError: event.payload.error,
      };

    case "FriendStatusChecked":
      return {
        ...state,
        friendStatusMap: {
          ...state.friendStatusMap,
          [event.payload.user_id]: event.payload.status,
        },
        userOnlineMap: event.payload.online
          ? {
              ...state.userOnlineMap,
              [event.payload.user_id]: event.payload.online,
            }
          : state.userOnlineMap,
      };
  }
}
