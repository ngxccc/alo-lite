import type { ChatMessage, StickerItem } from "../../../shared/types/ipc";

export function stickerCacheKey(stickerId: number, cateId = 0): string {
  return `stk_${cateId}_${stickerId}`;
}

export function stickerThumbnailUrl(
  item: Pick<StickerItem, "sticker_id" | "cate_id">,
): string {
  if (item.cate_id === 0 && item.sticker_id >= 1 && item.sticker_id <= 30) {
    return `https://stc-chat.zdn.vn/images/stickers/default/thumb/${item.sticker_id}.png`;
  }
  return `https://api.zaloapp.com/api/emoticon/sticker/webpc?eid=${item.sticker_id}&size=130`;
}

export function uniqueReferenceIndex(
  messages: ChatMessage[],
  reference: string,
): number | null {
  const ref = reference.trim();
  if (!ref) return null;
  const byId = messages.reduce<number[]>((acc, m, idx) => {
    if (m.id === ref || m.client_message_id === ref) acc.push(idx);
    return acc;
  }, []);
  if (byId.length === 1) return byId[0];
  if (byId.length > 1) return null;
  const byText = messages.reduce<number[]>((acc, m, idx) => {
    if (m.text === ref) acc.push(idx);
    return acc;
  }, []);
  return byText.length === 1 ? byText[0] : null;
}

export interface CallCardMeta {
  title: string;
  subtitle: string;
  isMissedOrFailed: boolean;
  isVideo: boolean;
}

export function parseCallCard(msg: ChatMessage): CallCardMeta | null {
  const txt = msg.text.trim();
  const isCall =
    txt.startsWith("[Cuộc gọi") ||
    txt.startsWith("Cuộc gọi") ||
    (msg.kind === "action" && txt.toLowerCase().includes("cuộc gọi"));
  if (!isCall) return null;
  const isVideo = txt.toLowerCase().includes("video");
  const missed =
    txt.toLowerCase().includes("nhỡ") ||
    txt.toLowerCase().includes("từ chối") ||
    txt.toLowerCase().includes("hủy");
  return {
    title: isVideo ? "Cuộc gọi video" : "Cuộc gọi thoại",
    subtitle: txt.replace(/^\[|\]$/g, ""),
    isMissedOrFailed: missed,
    isVideo,
  };
}

export function isPhotoMessage(msg: ChatMessage): boolean {
  if (msg.sticker_id) return false;
  if (msg.kind === "photo" || msg.kind === "gif") return true;
  if (msg.link_src === "Hình ảnh Zalo") return true;
  const t = msg.text.trim();
  return t === "[Hình ảnh]" || t.startsWith("[Hình ảnh] ");
}

export function isFileMessage(msg: ChatMessage): boolean {
  if (msg.kind === "file") return true;
  return Boolean(msg.link_url && msg.link_src?.includes("Tập tin"));
}

export function isVideoMessage(msg: ChatMessage): boolean {
  if (msg.kind === "video") return true;
  return msg.link_src === "Video Zalo" || msg.text.trim() === "[Video]";
}
