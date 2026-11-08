import { useEffect, useRef, useState, type ClipboardEvent, type DragEvent, type KeyboardEvent } from "react";
import { CornerUpLeft, Image, Paperclip, Send, Smile, X } from "lucide-react";
import { ipc, isTauriRuntime } from "../../../shared/api/ipc";
import type { StickerItem, Thread } from "../../../shared/types/ipc";
import { extractFirstUrl } from "../../../shared/utils/text";
import type { AppAction, AppState } from "../../../app/store/appReducer";
import { stickerThumbnailUrl } from "../utils/messages";

interface ComposerProps {
  state: AppState;
  dispatch: (action: AppAction) => void;
  thread: Thread;
}

type StickerTabId = "popular" | "onion" | "search" | "store" | number;

function StickerThumb({
  item,
  onPick,
}: {
  item: StickerItem;
  onPick: () => void;
}) {
  return (
    <button type="button" className="sticker-cell" title={item.label} onClick={onPick}>
      <img
        src={stickerThumbnailUrl(item)}
        alt={item.label}
        loading="lazy"
        decoding="async"
        style={{ width: 58, height: 58, objectFit: "contain" }}
      />
    </button>
  );
}

export function Composer({ state, dispatch, thread }: ComposerProps) {
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
    if (state.settings?.show_typing_indicator && !state.settings?.block_typing && isTauriRuntime()) {
      const now = Date.now();
      if (now - lastTypingAt.current > 3500 && val.trim().length > 0) {
        lastTypingAt.current = now;
        void ipc.sendTyping(thread.id, thread.is_group);
      }
    }
  };

  const handlePaste = async (e: ClipboardEvent<HTMLTextAreaElement>) => {
    const items = e.clipboardData?.items;
    if (!items) return;
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
    const file = e.dataTransfer.files?.[0];
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

  return (
    <div
      className="composer-shell"
      onDragOver={(e) => e.preventDefault()}
      onDrop={(e) => void handleDrop(e)}
    >
      {stickerOpen ? (
        <div className="popover-panel sticker-picker">
          <div style={{ display: "flex", gap: 4, padding: "8px 8px 6px", borderBottom: "1px solid var(--border)", overflowX: "auto" }}>
            <button
              type="button"
              className={`filter-tab ${stickerTab === "popular" ? "active" : ""}`}
              onClick={() => setStickerTab("popular")}
            >
              Phổ biến
            </button>
            <button
              type="button"
              className={`filter-tab ${stickerTab === "onion" ? "active" : ""}`}
              onClick={() => setStickerTab("onion")}
            >
              Củ hành
            </button>
            {state.pinnedPacks.map((p) => (
              <button
                key={p.cate_id}
                type="button"
                className={`filter-tab ${stickerTab === p.cate_id ? "active" : ""}`}
                onClick={() => setStickerTab(p.cate_id)}
              >
                {p.name}
              </button>
            ))}
            <button
              type="button"
              className={`filter-tab ${stickerTab === "search" ? "active" : ""}`}
              onClick={() => setStickerTab("search")}
            >
              Tìm kiếm
            </button>
            <button
              type="button"
              className={`filter-tab ${stickerTab === "store" ? "active" : ""}`}
              onClick={() => setStickerTab("store")}
            >
              Kho
            </button>
          </div>

          {stickerTab === "search" ? (
            <div style={{ padding: "8px 10px", display: "flex", gap: 6 }}>
              <input
                className="field-input"
                value={stickerQuery}
                onChange={(e) => setStickerQuery(e.target.value)}
                onKeyDown={(e) => {
                  if (e.key === "Enter" && stickerQuery.trim()) {
                    dispatch({ type: "SET_STICKER_SEARCH_LOADING", loading: true });
                    void ipc.searchStickers(stickerQuery.trim());
                  }
                }}
                placeholder="Tìm nhãn dán theo từ khóa…"
              />
              <button
                type="button"
                className="btn btn-primary"
                onClick={() => {
                  if (!stickerQuery.trim()) return;
                  dispatch({ type: "SET_STICKER_SEARCH_LOADING", loading: true });
                  void ipc.searchStickers(stickerQuery.trim());
                }}
              >
                Tìm
              </button>
            </div>
          ) : null}

          {stickerTab === "store" ? (
            <div style={{ flex: 1, overflowY: "auto", padding: 10, display: "flex", flexDirection: "column", gap: 6 }}>
              {state.stickerStoreLoading ? (
                <div style={{ color: "var(--text-muted)", textAlign: "center", padding: 20 }}>
                  Đang tải kho nhãn dán…
                </div>
              ) : (
                state.stickerStorePacks.map((pack) => (
                  <button
                    key={pack.id}
                    type="button"
                    className="btn"
                    style={{ justifyContent: "space-between" }}
                    onClick={() => {
                      setStickerTab(pack.id);
                      if (!state.stickerPackItems[pack.id]) {
                        void ipc.fetchStickerPack(pack.id, pack.name);
                      }
                    }}
                  >
                    <span>{pack.name}</span>
                    <span style={{ fontSize: 11.5, color: "var(--text-muted)" }}>{pack.total_image} nhãn</span>
                  </button>
                ))
              )}
            </div>
          ) : (
            <div className="sticker-grid">
              {displayedStickers.map((item) => (
                <StickerThumb
                  key={`${item.cate_id}_${item.sticker_id}`}
                  item={item}
                  onPick={() => {
                    void ipc.sendSticker(thread.id, item.sticker_id, item.cate_id, item.sticker_type, thread.is_group);
                    setStickerOpen(false);
                  }}
                />
              ))}
            </div>
          )}
        </div>
      ) : null}

      {state.composerStickerSuggestions.length > 0 && text.trim().length >= 2 ? (
        <div style={{ display: "flex", gap: 6, overflowX: "auto", paddingBottom: 2 }}>
          {state.composerStickerSuggestions.slice(0, 8).map((item) => (
            <div key={`sug_${item.cate_id}_${item.sticker_id}`} style={{ width: 64, flexShrink: 0 }}>
              <StickerThumb
                item={item}
                onPick={() => {
                  void ipc.sendSticker(thread.id, item.sticker_id, item.cate_id, item.sticker_type, thread.is_group);
                  setText("");
                }}
              />
            </div>
          ))}
        </div>
      ) : null}

      {firstUrl && firstUrl !== dismissedLinkFor ? (
        <div
          style={{
            display: "flex",
            alignItems: "center",
            justifyContent: "space-between",
            padding: "6px 10px",
            borderRadius: 8,
            background: "var(--bg-elevated)",
            border: "1px solid var(--border)",
            fontSize: 12.2,
          }}
        >
          <div style={{ minWidth: 0 }}>
            <div style={{ fontWeight: 700, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
              {linkPreview?.title || "Đang tải xem trước liên kết…"}
            </div>
            <div style={{ color: "var(--text-muted)", fontSize: 11.5, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
              {linkPreview?.desc || firstUrl}
            </div>
          </div>
          <button type="button" className="icon-btn" onClick={() => setDismissedLinkFor(firstUrl)}>
            <X size={14} />
          </button>
        </div>
      ) : null}

      {state.replyingTo ? (
        <div
          style={{
            display: "flex",
            alignItems: "center",
            justifyContent: "space-between",
            padding: "6px 10px",
            borderLeft: "3px solid var(--primary)",
            background: "var(--bg-elevated)",
            borderRadius: 6,
            fontSize: 12.2,
          }}
        >
          <div style={{ display: "flex", alignItems: "center", gap: 8, minWidth: 0 }}>
            <CornerUpLeft size={14} color="var(--primary)" />
            <div style={{ minWidth: 0 }}>
              <div style={{ fontWeight: 700 }}>Trả lời {state.replyingTo.sender}</div>
              <div style={{ color: "var(--text-secondary)", overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
                {state.replyingTo.text}
              </div>
            </div>
          </div>
          <button
            type="button"
            className="icon-btn"
            onClick={() => dispatch({ type: "SET_REPLYING_TO", msg: null })}
          >
            <X size={14} />
          </button>
        </div>
      ) : null}

      {pendingUpload ? (
        <div
          style={{
            display: "flex",
            alignItems: "center",
            justifyContent: "space-between",
            padding: "6px 10px",
            background: "var(--bg-elevated)",
            borderRadius: 8,
            border: "1px solid var(--border)",
          }}
        >
          <div style={{ display: "flex", alignItems: "center", gap: 10 }}>
            {pendingUpload.previewUrl ? (
              <img
                src={pendingUpload.previewUrl}
                alt={pendingUpload.fileName}
                style={{ width: 42, height: 42, objectFit: "cover", borderRadius: 6 }}
              />
            ) : null}
            <span style={{ fontSize: 12.5, fontWeight: 600 }}>{pendingUpload.fileName}</span>
          </div>
          <button type="button" className="icon-btn" onClick={() => setPendingUpload(null)}>
            <X size={14} />
          </button>
        </div>
      ) : null}

      <div className="composer-toolbar">
        <button
          type="button"
          className="icon-btn"
          title="Gửi nhãn dán"
          onClick={() => setStickerOpen((v) => !v)}
        >
          <Smile size={18} />
        </button>
        <button
          type="button"
          className="icon-btn"
          title="Gửi hình ảnh"
          onClick={() => void pickAndUpload(true)}
        >
          <Image size={18} />
        </button>
        <button
          type="button"
          className="icon-btn"
          title="Đính kèm tập tin"
          onClick={() => void pickAndUpload(false)}
        >
          <Paperclip size={18} />
        </button>
      </div>

      <div className="composer-row">
        <textarea
          className="composer-input"
          rows={1}
          value={text}
          onChange={(e) => handleTextChange(e.target.value)}
          onKeyDown={handleKeyDown}
          onPaste={(e) => void handlePaste(e)}
          placeholder={`Nhập tin nhắn tới ${thread.name || thread.id}…`}
        />
        <button
          type="button"
          className="btn btn-primary"
          disabled={!text.trim() && !pendingUpload}
          onClick={() => void sendNow()}
          title="Gửi tin nhắn"
        >
          <Send size={15} /> Gửi
        </button>
      </div>
    </div>
  );
}
