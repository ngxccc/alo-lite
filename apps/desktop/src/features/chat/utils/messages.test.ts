import { describe, expect, it } from "bun:test";
import type { ChatMessage } from "../../../shared/types/ipc";
import { isFileMessage, isPhotoMessage, parseCallCard, stickerThumbnailUrl } from "./messages";

function msg(patch: Partial<ChatMessage>): ChatMessage {
  return {
    id: "m1",
    sender: "A",
    text: "",
    time: "10:00",
    is_outgoing: false,
    ts: 1000,
    ...patch,
  };
}

describe("chat message utils", () => {
  it("parses voice and missed video call cards", () => {
    const voice = parseCallCard(msg({ text: "[Cuộc gọi thoại 02:15]" }));
    expect(voice?.isVideo).toBe(false);
    expect(voice?.isMissedOrFailed).toBe(false);

    const missed = parseCallCard(msg({ text: "[Cuộc gọi video nhỡ]" }));
    expect(missed?.isVideo).toBe(true);
    expect(missed?.isMissedOrFailed).toBe(true);
  });

  it("classifies photo vs sticker vs file attachments cleanly", () => {
    expect(isPhotoMessage(msg({ text: "[Hình ảnh]", sticker_id: 123 }))).toBe(false);
    expect(isPhotoMessage(msg({ text: "[Hình ảnh]" }))).toBe(true);
    expect(
      isFileMessage(
        msg({
          link_url: "https://cdn.example/a.pdf",
          link_src: "Tập tin Zalo",
        }),
      ),
    ).toBe(true);
  });

  it("builds Onion and standard sticker CDN URLs", () => {
    expect(stickerThumbnailUrl({ sticker_id: 5, cate_id: 0 })).toContain("/default/thumb/5.png");
    expect(stickerThumbnailUrl({ sticker_id: 98930, cate_id: 61900 })).toContain("eid=98930");
  });
});
