import { describe, expect, it } from "bun:test";
import { useAuthStore } from "./use-auth-store";
import { useChatStore } from "./use-chat-store";
import { useUIStore } from "./use-ui-store";
import { useContactsStore } from "./use-contacts-store";
import type { ChatMessage, Thread } from "@/shared/types/ipc";

describe("Domain Stores Architecture", () => {
  it("useAuthStore handles login and logout lifecycle", () => {
    const auth = useAuthStore.getState();
    expect(auth.loggedIn).toBe(false);

    auth.setLoggedIn({
      uin: "12345678",
      displayName: "Nguyễn Văn A",
      avatar: "https://avatar.test/123.jpg",
    });

    const loggedInState = useAuthStore.getState();
    expect(loggedInState.loggedIn).toBe(true);
    expect(loggedInState.uin).toBe("12345678");
    expect(loggedInState.displayName).toBe("Nguyễn Văn A");

    loggedInState.setLoggedOut();
    const loggedOutState = useAuthStore.getState();
    expect(loggedOutState.loggedIn).toBe(false);
    expect(loggedOutState.uin).toBe("");
  });

  it("useUIStore toggles theme and modals", () => {
    const ui = useUIStore.getState();
    ui.setThemeMode("light");
    expect(useUIStore.getState().themeMode).toBe("light");

    ui.setSettingsOpen(true, "privacy");
    expect(useUIStore.getState().settingsOpen).toBe(true);
    expect(useUIStore.getState().activeSettingsTab).toBe("privacy");

    ui.setToast({ message: "Đã lưu cài đặt", kind: "success" });
    expect(useUIStore.getState().toast?.message).toBe("Đã lưu cài đặt");
  });

  it("useChatStore updates threads and messages with Immer", () => {
    const chat = useChatStore.getState();
    const mockThread: Thread = {
      id: "thread-1",
      name: "Team DevOps",
      preview: "Hello team",
      time: "10:30",
      avatar: null,
      is_group: true,
      messages: [],
      presentation: {
        members: [],
        last_message_ts: 0,
        last_message_id: "",
        last_sender: "",
        last_sender_id: "",
        last_outgoing: false,
        pinned: false,
        muted: false,
        hidden: false,
      },
      unread: {
        last_seen_id: null,
        unread_ids: ["msg-1"],
        marked_unread: false,
        muted: false,
      },
    };

    chat.setThreads([mockThread]);
    expect(useChatStore.getState().threads.length).toBe(1);

    const msg: ChatMessage = {
      id: "msg-2",
      client_message_id: "cli-2",
      sender: "Dương",
      text: "Đã triển khai kiến trúc mới!",
      time: "10:35",
      is_outgoing: true,
      ts: Date.now(),
    };

    chat.appendMessage("thread-1", msg);
    const updated = useChatStore.getState().threads[0];
    expect(updated?.messages.length).toBe(1);
    expect(updated?.preview).toBe("Đã triển khai kiến trúc mới!");

    chat.selectThread("thread-1");
    expect(useChatStore.getState().selectedThreadId).toBe("thread-1");
    expect(useChatStore.getState().unreadCounts["thread-1"]).toBe(0);
  });

  it("useContactsStore updates subtabs and phone lookup", () => {
    const contacts = useContactsStore.getState();
    contacts.setContactsSubTab("groups");
    expect(useContactsStore.getState().contactsSubTab).toBe("groups");

    contacts.setPhoneLookupLoading(true);
    expect(useContactsStore.getState().phoneLookupLoading).toBe(true);

    contacts.setPhoneLookupResult({
      user_id: "uid-888",
      display_name: "Trần B",
      zalo_name: "Trần B",
      phone: "0901234567",
      avatar: "",
      status: "online",
      friendship: "Stranger",
    });
    expect(useContactsStore.getState().phoneLookupResult?.display_name).toBe("Trần B");
    expect(useContactsStore.getState().phoneLookupLoading).toBe(false);
  });
});
