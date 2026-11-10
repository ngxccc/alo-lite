import { create } from "zustand";
import { immer } from "zustand/middleware/immer";
import type { FriendshipStatus, PhoneLookupResult } from "@/shared/types/ipc";
import type { ContactsSubTab } from "@/app/store/app-state";

export interface ContactsStoreState {
  contactsSubTab: ContactsSubTab;
  contactsSearchQuery: string;
  phoneLookupQuery: string;
  phoneLookupLoading: boolean;
  phoneLookupResult: PhoneLookupResult | null;
  phoneLookupError: string | null;
  friendStatusByUid: Record<string, FriendshipStatus | undefined>;

  // Actions
  setContactsSubTab: (tab: ContactsSubTab) => void;
  setContactsSearchQuery: (query: string) => void;
  setPhoneLookupQuery: (query: string) => void;
  setPhoneLookupLoading: (loading: boolean) => void;
  setPhoneLookupResult: (result: PhoneLookupResult | null) => void;
  setPhoneLookupError: (error: string | null) => void;
  setFriendStatus: (uid: string, status: FriendshipStatus) => void;
  resetPhoneLookup: () => void;
}

export const useContactsStore = create<ContactsStoreState>()(
  immer((set) => ({
    contactsSubTab: "friends",
    contactsSearchQuery: "",
    phoneLookupQuery: "",
    phoneLookupLoading: false,
    phoneLookupResult: null,
    phoneLookupError: null,
    friendStatusByUid: {},

    setContactsSubTab: (tab) =>
      set((state) => {
        state.contactsSubTab = tab;
      }),

    setContactsSearchQuery: (query) =>
      set((state) => {
        state.contactsSearchQuery = query;
      }),

    setPhoneLookupQuery: (query) =>
      set((state) => {
        state.phoneLookupQuery = query;
      }),

    setPhoneLookupLoading: (loading) =>
      set((state) => {
        state.phoneLookupLoading = loading;
      }),

    setPhoneLookupResult: (result) =>
      set((state) => {
        state.phoneLookupResult = result;
        state.phoneLookupLoading = false;
        state.phoneLookupError = null;
      }),

    setPhoneLookupError: (error) =>
      set((state) => {
        state.phoneLookupError = error;
        state.phoneLookupLoading = false;
      }),

    setFriendStatus: (uid, status) =>
      set((state) => {
        state.friendStatusByUid[uid] = status;
      }),

    resetPhoneLookup: () =>
      set((state) => {
        state.phoneLookupQuery = "";
        state.phoneLookupLoading = false;
        state.phoneLookupResult = null;
        state.phoneLookupError = null;
      }),
  })),
);
