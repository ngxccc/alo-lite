import {
  useEffect,
  useRef,
  useState,
  type ClipboardEvent,
  type DragEvent,
  type KeyboardEvent,
} from "react";
import {
  Clock,
  Contact,
  CornerUpLeft,
  Crop,
  Image,
  Info,
  MoreHorizontal,
  Paperclip,
  Send,
  Smile,
  ThumbsUp,
  Type,
  Vote,
  X,
} from "lucide-react";
import { ipc, isTauriRuntime } from "@/shared/api/ipc";
import type { StickerItem, Thread } from "@/shared/types/ipc";
import { Button } from "@/shared/ui/button";
import { Input } from "@/shared/ui/input";
import { Popover, PopoverContent, PopoverTrigger } from "@/shared/ui/popover";
import { cn } from "@/shared/utils/cn";
import { extractFirstUrl } from "@/shared/utils/text";
import type { AppAction, AppState } from "@/app/store/app-reducer";
import { useAppStore } from "@/app/store/use-app-store";
import { stickerThumbnailUrl } from "@/features/chat/utils/messages";

interface ComposerProps {
  state?: AppState;
  dispatch?: (action: AppAction) => void;
  thread: Thread;
}
type StickerTabId = "popular" | "onion" | "search" | "store" | number;

function StickerThumb({ item, onPick }: { item: StickerItem; onPick: () => void }) {
  return (
    <button
      type="button"
      title={item.label}
      onClick={onPick}
      className="flex aspect-square cursor-pointer items-center justify-center rounded-lg border border-transparent bg-muted/40 p-1 transition hover:border-border hover:bg-accent"
    >
      <img
        src={stickerThumbnailUrl({
          sticker_id: item.sticker_id,
          cate_id: item.cate_id,
        })}
        alt={item.label}
        loading="lazy"
        decoding="async"
        className="h-14 w-14 object-contain transition-transform hover:scale-110"
      />
    </button>
  );
}

export function Composer({ state: propState, dispatch: propDispatch, thread }: ComposerProps) {
  const appStore = useAppStore();
  const state = propState ?? appStore;
  const dispatch = propDispatch ?? appStore.dispatch;
  const [text, setText] = useState("");
  const [stickerOpen, setStickerOpen] = useState(false);
  const [stickerTab, setStickerTab] = useState<StickerTabId>("popular");
  const [stickerQuery, setStickerQuery] = useState("");
  const [dismissedLinkFor, setDismissedLinkFor] = useState("");
  const [pendingUpload, setPendingUpload] = useState<{
    fileName: string;
    bytes: number[];
    previewUrl: string;
  } | null>(null);
  const lastTypingAt = useRef(0);

  // Check if thread is restricted to owner/admin only
  const isSendLocked =
    thread.is_group &&
    (thread.preview === "Only owner/admins can send messages" ||
      thread.preview === "Chỉ trưởng/phó nhóm mới được gửi tin nhắn" ||
      ("is_locked" in thread && Boolean(thread.is_locked)));
  const firstUrl = extractFirstUrl(text);
  const linkPreview =
    firstUrl && firstUrl !== dismissedLinkFor ? state.linkMetadataCache[firstUrl] : undefined;

  useEffect(() => {
    if (!firstUrl || state.linkMetadataCache[firstUrl] || !isTauriRuntime()) return;
    const t = window.setTimeout(() => {
      void ipc.parseLinkPreview(firstUrl);
    }, 220);
    return () => window.clearTimeout(t);
  }, [firstUrl, state.linkMetadataCache]);

  useEffect(() => {
    if (!state.settings?.suggest_stickers_while_typing || !isTauriRuntime()) return;
    const trimmed = text.trim();
    if (trimmed.length < 2 || trimmed.length > 24 || trimmed.includes("http")) return;
    const t = window.setTimeout(() => {
      void ipc.suggestComposerStickers(trimmed);
    }, 260);
    return () => window.clearTimeout(t);
  }, [text, state.settings?.suggest_stickers_while_typing]);

  useEffect(() => {
    if (stickerTab === "store" && state.stickerStorePacks.length === 0 && isTauriRuntime()) {
      dispatch({ type: "SET_STICKER_STORE_LOADING", loading: true });
      void ipc.fetchStickerStore();
    }
  }, [stickerTab, state.stickerStorePacks.length, dispatch]);

  const sendNow = async () => {
    if (pendingUpload) {
      const { fileName, bytes } = pendingUpload;
      setPendingUpload(null);
      await ipc.sendAttachmentBytes(thread.id, thread.is_group, fileName, bytes, text.trim());
      setText("");
      return;
    }
    const clean = text.trim();
    if (!clean) return;
    setText("");
    if (state.replyingTo) {
      const quoted = state.replyingTo;
      dispatch({ type: "SET_REPLYING_TO", msg: null });
      await ipc.sendQuote(thread.id, clean, quoted, thread.is_group);
    } else {
      await ipc.sendMessage(thread.id, clean, thread.is_group);
    }
  };

  const handleKeyDown = (e: KeyboardEvent<HTMLTextAreaElement>) => {
    const cmdEnterOnly = state.settings?.send_message_key === "cmd_enter";
    if (e.key === "Enter") {
      if (cmdEnterOnly && (e.ctrlKey || e.metaKey)) {
        e.preventDefault();
        void sendNow();
      } else if (!cmdEnterOnly && !e.shiftKey) {
        e.preventDefault();
        void sendNow();
      }
    }
  };

  const handleTextChange = (val: string) => {
    setText(val);
    if (state.settings?.show_typing_indicator && !state.settings.block_typing && isTauriRuntime()) {
      const now = Date.now();
      if (now - lastTypingAt.current > 3500 && val.trim().length > 0) {
        lastTypingAt.current = now;
        void ipc.sendTyping(thread.id, thread.is_group);
      }
    }
  };

  const handlePaste = async (e: ClipboardEvent<HTMLTextAreaElement>) => {
    const items = e.clipboardData.items;
    for (const item of Array.from(items)) {
      if (item.kind === "file") {
        const file = item.getAsFile();
        if (file) {
          e.preventDefault();
          const buf = new Uint8Array(await file.arrayBuffer());
          setPendingUpload({
            fileName: file.name || `clipboard_${Date.now()}.png`,
            bytes: Array.from(buf),
            previewUrl: URL.createObjectURL(file),
          });
          return;
        }
      }
    }
  };

  const handleDrop = async (e: DragEvent<HTMLDivElement>) => {
    e.preventDefault();
    const file = e.dataTransfer.files.item(0);
    if (!file) return;
    const buf = new Uint8Array(await file.arrayBuffer());
    setPendingUpload({
      fileName: file.name || `drop_${Date.now()}`,
      bytes: Array.from(buf),
      previewUrl: file.type.startsWith("image/") ? URL.createObjectURL(file) : "",
    });
  };

  const pickAndUpload = async (imagesOnly: boolean) => {
    const picked = await ipc.pickFile({
      imagesOnly,
      title: imagesOnly ? "Chọn hình ảnh để gửi" : "Chọn tập tin để gửi",
    });
    if (picked) {
      await ipc.sendAttachmentFile(thread.id, thread.is_group, picked, text.trim());
      setText("");
    }
  };

  const displayedStickers: StickerItem[] =
    stickerTab === "popular"
      ? state.popularStickers
      : stickerTab === "onion"
        ? state.onionStickers
        : stickerTab === "search"
          ? state.stickerSearchResults
          : typeof stickerTab === "number"
            ? (state.stickerPackItems[stickerTab] ?? [])
            : [];

  // Locked-group notice bar at the bottom
  if (isSendLocked) {
    return (
      <div className="flex h-14 shrink-0 items-center justify-center gap-2 border-t border-border bg-sidebar px-4 text-[13px] text-muted-foreground dark:border-[rgba(255,255,255,0.1)]">
        <Info className="h-4.5 w-4.5 shrink-0 text-muted-foreground" />
        <span>
          Chỉ <span className="font-semibold text-primary">trưởng/phó nhóm</span> mới được gửi tin
          nhắn vào nhóm này.{" "}
          <span className="cursor-pointer text-primary hover:underline">Tìm hiểu thêm</span>
        </span>
      </div>
    );
  }

  return (
    <div
      onDragOver={(e) => e.preventDefault()}
      onDrop={(e) => void handleDrop(e)}
      className="relative flex shrink-0 flex-col border-t border-border bg-sidebar px-4 py-1.5 dark:border-[rgba(255,255,255,0.1)]"
    >
      {/* Sticker typing suggestions */}
      {state.composerStickerSuggestions.length > 0 && text.trim().length >= 2 ? (
        <div className="flex gap-1.5 overflow-x-auto pb-1">
          {state.composerStickerSuggestions.slice(0, 8).map((item) => (
            <div key={`sug_${item.cate_id}_${item.sticker_id}`} className="w-16 shrink-0">
              <StickerThumb
                item={item}
                onPick={() => {
                  void ipc.sendSticker(
                    thread.id,
                    item.sticker_id,
                    item.cate_id,
                    item.sticker_type,
                    thread.is_group,
                  );
                  setText("");
                }}
              />
            </div>
          ))}
        </div>
      ) : null}

      {/* Link preview bar */}
      {firstUrl && firstUrl !== dismissedLinkFor ? (
        <div className="mb-1 flex items-center justify-between rounded-lg border border-border bg-muted/60 px-2.5 py-1.5 text-xs">
          <div className="min-w-0">
            <div className="truncate font-bold text-foreground">
              {linkPreview?.title ?? "Đang tải xem trước liên kết…"}
            </div>
            <div className="truncate text-[11px] text-muted-foreground">
              {linkPreview?.desc ?? firstUrl}
            </div>
          </div>
          <Button variant="ghost" size="icon-sm" onClick={() => setDismissedLinkFor(firstUrl)}>
            <X className="h-3.5 w-3.5" />
          </Button>
        </div>
      ) : null}

      {/* Replying-to bar */}
      {state.replyingTo ? (
        <div className="mb-1 flex items-center justify-between rounded-md border-l-[3px] border-primary bg-muted/60 px-2.5 py-1.5 text-xs">
          <div className="flex min-w-0 items-center gap-2">
            <CornerUpLeft className="h-3.5 w-3.5 shrink-0 text-primary" />
            <div className="min-w-0">
              <div className="font-bold text-foreground">Trả lời {state.replyingTo.sender}</div>
              <div className="truncate text-muted-foreground">{state.replyingTo.text}</div>
            </div>
          </div>
          <Button
            variant="ghost"
            size="icon-sm"
            onClick={() => dispatch({ type: "SET_REPLYING_TO", msg: null })}
          >
            <X className="h-3.5 w-3.5" />
          </Button>
        </div>
      ) : null}

      {/* Pending upload preview */}
      {pendingUpload ? (
        <div className="mb-1 flex items-center justify-between rounded-lg border border-border bg-muted/60 px-2.5 py-1.5">
          <div className="flex items-center gap-2.5">
            {pendingUpload.previewUrl ? (
              <img
                src={pendingUpload.previewUrl}
                alt={pendingUpload.fileName}
                className="h-10 w-10 rounded-md object-cover"
              />
            ) : null}
            <span className="text-xs font-semibold text-foreground">{pendingUpload.fileName}</span>
          </div>
          <Button variant="ghost" size="icon-sm" onClick={() => setPendingUpload(null)}>
            <X className="h-3.5 w-3.5" />
          </Button>
        </div>
      ) : null}

      {/* ── Toolbar row: 24px icons at 40px pitch ── */}
      <div className="flex items-center text-muted-foreground">
        <Popover open={stickerOpen} onOpenChange={setStickerOpen}>
          <PopoverTrigger asChild>
            <button
              type="button"
              className={cn(
                "flex h-10 w-10 cursor-pointer items-center justify-center rounded-md transition hover:bg-black/5 hover:text-foreground dark:hover:bg-white/5",
                stickerOpen ? "text-primary" : "text-muted-foreground",
              )}
              title="Gửi nhãn dán"
            >
              <Smile className="h-6 w-6" strokeWidth={1.65} />
            </button>
          </PopoverTrigger>
          <PopoverContent
            side="top"
            align="start"
            className="flex h-[360px] w-[350px] flex-col overflow-hidden p-0"
          >
            <div className="flex gap-1 overflow-x-auto border-b border-border p-1.5">
              {[
                { id: "popular" as const, label: "Phổ biến" },
                { id: "onion" as const, label: "Củ hành" },
                ...state.pinnedPacks.map((p) => ({
                  id: p.cate_id,
                  label: p.name,
                })),
                { id: "search" as const, label: "Tìm kiếm" },
                { id: "store" as const, label: "Kho" },
              ].map((t) => (
                <button
                  key={String(t.id)}
                  type="button"
                  onClick={() => setStickerTab(t.id)}
                  className={cn(
                    "shrink-0 cursor-pointer rounded-md px-2.5 py-1 text-xs font-medium transition",
                    stickerTab === t.id
                      ? "bg-accent font-semibold text-accent-foreground"
                      : "text-muted-foreground hover:bg-muted hover:text-foreground",
                  )}
                >
                  {t.label}
                </button>
              ))}
            </div>

            {stickerTab === "search" ? (
              <div className="flex gap-1.5 border-b border-border p-2">
                <Input
                  value={stickerQuery}
                  onChange={(e) => setStickerQuery(e.target.value)}
                  onKeyDown={(e) => {
                    if (e.key === "Enter" && stickerQuery.trim()) {
                      dispatch({
                        type: "SET_STICKER_SEARCH_LOADING",
                        loading: true,
                      });
                      void ipc.searchStickers(stickerQuery.trim());
                    }
                  }}
                  placeholder="Tìm nhãn dán theo từ khóa…"
                />
                <Button
                  variant="default"
                  onClick={() => {
                    if (!stickerQuery.trim()) return;
                    dispatch({
                      type: "SET_STICKER_SEARCH_LOADING",
                      loading: true,
                    });
                    void ipc.searchStickers(stickerQuery.trim());
                  }}
                >
                  Tìm
                </Button>
              </div>
            ) : null}

            {stickerTab === "store" ? (
              <div className="flex flex-1 flex-col gap-1.5 overflow-y-auto p-2.5">
                {state.stickerStoreLoading ? (
                  <div className="p-5 text-center text-xs text-muted-foreground">
                    Đang tải kho nhãn dán…
                  </div>
                ) : (
                  state.stickerStorePacks.map((pack) => (
                    <Button
                      key={pack.id}
                      variant="outline"
                      className="w-full justify-between"
                      onClick={() => {
                        setStickerTab(pack.id);
                        if (!state.stickerPackItems[pack.id]) {
                          void ipc.fetchStickerPack(pack.id, pack.name);
                        }
                      }}
                    >
                      <span className="truncate">{pack.name}</span>
                      <span className="text-[11px] text-muted-foreground">
                        {pack.total_image} nhãn
                      </span>
                    </Button>
                  ))
                )}
              </div>
            ) : (
              <div className="grid flex-1 grid-cols-4 gap-1.5 overflow-y-auto p-2.5">
                {displayedStickers.map((item) => (
                  <StickerThumb
                    key={`${item.cate_id}_${item.sticker_id}`}
                    item={item}
                    onPick={() => {
                      void ipc.sendSticker(
                        thread.id,
                        item.sticker_id,
                        item.cate_id,
                        item.sticker_type,
                        thread.is_group,
                      );
                      setStickerOpen(false);
                    }}
                  />
                ))}
              </div>
            )}
          </PopoverContent>
        </Popover>

        <button
          type="button"
          className="flex h-10 w-10 cursor-pointer items-center justify-center rounded-md text-muted-foreground transition hover:bg-black/5 hover:text-foreground dark:hover:bg-white/5"
          title="Gửi hình ảnh"
          onClick={() => void pickAndUpload(true)}
        >
          <Image className="h-6 w-6" strokeWidth={1.65} />
        </button>

        <button
          type="button"
          className="flex h-10 w-10 cursor-pointer items-center justify-center rounded-md text-muted-foreground transition hover:bg-black/5 hover:text-foreground dark:hover:bg-white/5"
          title="Đính kèm tập tin"
          onClick={() => void pickAndUpload(false)}
        >
          <Paperclip className="h-6 w-6" strokeWidth={1.65} />
        </button>

        <button
          type="button"
          className="flex h-10 w-10 cursor-pointer items-center justify-center rounded-md text-muted-foreground transition hover:bg-black/5 hover:text-foreground dark:hover:bg-white/5"
          title="Gửi danh thiếp"
        >
          <Contact className="h-6 w-6" strokeWidth={1.65} />
        </button>

        <button
          type="button"
          className="flex h-10 w-10 cursor-pointer items-center justify-center rounded-md text-muted-foreground transition hover:bg-black/5 hover:text-foreground dark:hover:bg-white/5"
          title="Chụp ảnh màn hình"
          onClick={() => void pickAndUpload(true)}
        >
          <Crop className="h-6 w-6" strokeWidth={1.65} />
        </button>

        <button
          type="button"
          className="flex h-10 w-10 cursor-pointer items-center justify-center rounded-md text-muted-foreground transition hover:bg-black/5 hover:text-foreground dark:hover:bg-white/5"
          title="Định dạng văn bản"
        >
          <Type className="h-6 w-6" strokeWidth={1.65} />
        </button>

        <button
          type="button"
          className="flex h-10 w-10 cursor-pointer items-center justify-center rounded-md text-muted-foreground transition hover:bg-black/5 hover:text-foreground dark:hover:bg-white/5"
          title="Tạo nhắc hẹn"
        >
          <Clock className="h-6 w-6" strokeWidth={1.65} />
        </button>

        {thread.is_group ? (
          <button
            type="button"
            className="flex h-10 w-10 cursor-pointer items-center justify-center rounded-md text-muted-foreground transition hover:bg-black/5 hover:text-foreground dark:hover:bg-white/5"
            title="Tạo bình chọn"
          >
            <Vote className="h-6 w-6" strokeWidth={1.65} />
          </button>
        ) : null}

        <button
          type="button"
          className="flex h-10 w-10 cursor-pointer items-center justify-center rounded-md text-muted-foreground transition hover:bg-black/5 hover:text-foreground dark:hover:bg-white/5"
          title="Tùy chọn khác"
        >
          <MoreHorizontal className="h-6 w-6" strokeWidth={1.65} />
        </button>
      </div>

      {/* ── Input row ── */}
      <div className="flex items-center gap-2 pt-0.5">
        <textarea
          rows={1}
          value={text}
          onChange={(e) => handleTextChange(e.target.value)}
          onKeyDown={handleKeyDown}
          onPaste={(e) => void handlePaste(e)}
          placeholder={`@, Nhập tin nhắn tới ${thread.name || thread.id}`}
          className="max-h-28 min-h-[38px] flex-1 resize-none bg-transparent px-2 py-1.5 text-[15px] text-foreground select-text placeholder:text-muted-foreground/60 focus:outline-none"
        />
        <div className="flex shrink-0 items-center gap-1 pr-1">
          <button
            type="button"
            className="flex h-9 w-9 cursor-pointer items-center justify-center rounded-md text-muted-foreground transition hover:bg-black/5 hover:text-foreground dark:hover:bg-white/5"
            title="Biểu tượng cảm xúc"
            onClick={() => setStickerOpen((prev) => !prev)}
          >
            <Smile className="h-6 w-6" strokeWidth={1.65} />
          </button>

          {text.trim() || pendingUpload ? (
            <Button
              variant="default"
              size="icon"
              onClick={() => void sendNow()}
              title="Gửi tin nhắn"
              className="h-9 w-9 cursor-pointer rounded-full shadow-xs"
            >
              <Send className="h-4.5 w-4.5" />
            </Button>
          ) : (
            <button
              type="button"
              onClick={() => void ipc.sendMessage(thread.id, "👍", thread.is_group)}
              title="Gửi Like nhanh"
              className="flex h-9 w-9 cursor-pointer items-center justify-center rounded-full text-yellow-500 transition hover:scale-110 active:scale-95"
            >
              <ThumbsUp className="h-5.5 w-5.5 fill-yellow-500" />
            </button>
          )}
        </div>
      </div>
    </div>
  );
}
