import { useEffect, useState } from "react";
import { Download, ExternalLink, FileText, Phone, Play, Video } from "lucide-react";
import { ipc } from "@/shared/api/ipc";
import type { ChatMessage } from "@/shared/types/ipc.generated";
import { Button } from "@/shared/ui/button";
import { cn } from "@/shared/utils/cn";
import { jxlToJpgUrl } from "@/shared/utils/text";
import {
  type CallCardMeta,
  isGifUrl,
  shouldHideMediaCaption,
  stickerThumbnailUrl,
} from "@/features/chat/utils/messages";

export function StickerMessageMedia({ msg }: { msg: ChatMessage }) {
  const sid = msg.sticker_id ?? 0;
  const cid = msg.sticker_cate_id ?? 0;
  if (!sid) return null;
  return (
    <img
      src={stickerThumbnailUrl({ sticker_id: sid, cate_id: cid })}
      alt={msg.text}
      loading="lazy"
      decoding="async"
      className="h-32 w-32 object-contain transition-transform select-none hover:scale-105"
    />
  );
}

export function PhotoMessageMedia({
  msg,
  onOpenViewer,
}: {
  msg: ChatMessage;
  onOpenViewer: (texKey: string, fullUrl: string) => void;
}) {
  const rawUrl = msg.link_url ?? (isGifUrl(msg.text) ? msg.text.trim() : "");
  const isGif = isGifUrl(rawUrl) || isGifUrl(msg.link_title);
  const initialSrc = isGif
    ? rawUrl || jxlToJpgUrl(msg.link_thumb ?? "")
    : jxlToJpgUrl(msg.link_thumb ?? rawUrl);
  const fullUrl = rawUrl || initialSrc;
  const [src, setSrc] = useState(initialSrc);
  const key = `img_${msg.id || msg.ts}`;

  useEffect(() => {
    setSrc(initialSrc);
  }, [initialSrc]);

  return (
    <div className="flex flex-col gap-1.5">
      {src ? (
        <div className="relative">
          {/* HD badge as in Zalo */}
          <span className="absolute top-2 left-2 z-10 rounded bg-black/60 px-1.5 py-0.5 text-xs font-bold text-white">
            HD
          </span>
          <img
            src={src}
            alt={msg.link_title ?? "Hình ảnh"}
            loading="lazy"
            decoding="async"
            onError={() => {
              const fallback = jxlToJpgUrl(rawUrl);
              if (fallback && fallback !== src) {
                setSrc(fallback);
              }
            }}
            onClick={() => onOpenViewer(key, fullUrl)}
            className="max-h-[320px] max-w-[480px] min-w-[80px] cursor-zoom-in rounded-[6px] object-contain"
          />
        </div>
      ) : (
        <Button variant="outline" size="sm" onClick={() => onOpenViewer(key, fullUrl)}>
          Xem hình ảnh
        </Button>
      )}
      {!shouldHideMediaCaption(msg) ? <span className="select-text">{msg.text}</span> : null}
    </div>
  );
}

export function VideoMessageMedia({
  msg,
  downloadDir,
  inlineVideoSupported,
  inlineVideoReason,
  onNotify,
}: {
  msg: ChatMessage;
  downloadDir: string;
  inlineVideoSupported: boolean;
  inlineVideoReason: string | null;
  onNotify: (message: string) => void;
}) {
  const [playing, setPlaying] = useState(false);
  const [playbackError, setPlaybackError] = useState(false);
  const [blockedClick, setBlockedClick] = useState(false);
  const videoUrl = msg.link_url ?? "";
  const posterUrl = msg.link_thumb ? jxlToJpgUrl(msg.link_thumb) : undefined;
  const fileName =
    msg.link_title && !msg.link_title.startsWith("[")
      ? msg.link_title.endsWith(".mp4")
        ? msg.link_title
        : `${msg.link_title}.mp4`
      : "video.mp4";

  useEffect(() => {
    setPlaying(false);
    setPlaybackError(false);
    setBlockedClick(false);
  }, [videoUrl]);

  const unsupportedMessage =
    inlineVideoReason ??
    "Thiếu GStreamer (gst-plugins-good / gst-libav) nên không thể phát video trực tiếp.";

  return (
    <div className="flex flex-col gap-1.5">
      {videoUrl && inlineVideoSupported && playing && !playbackError ? (
        <video
          src={videoUrl}
          poster={posterUrl}
          controls
          autoPlay
          preload="auto"
          onError={(e) => {
            const err = e.currentTarget.error;
            console.error("Video playback error:", err?.code, err?.message, videoUrl);
            setPlaybackError(true);
            onNotify("Không thể phát trực tiếp video này — hãy thử mở ngoài hoặc tải về.");
          }}
          className="max-h-[300px] max-w-[340px] rounded-[6px] bg-black/90 object-contain"
        />
      ) : videoUrl ? (
        <button
          type="button"
          onClick={() => {
            if (!inlineVideoSupported) {
              if (blockedClick) {
                void ipc.openExternalUrl(videoUrl);
              } else {
                setBlockedClick(true);
                onNotify(unsupportedMessage);
              }
            } else if (playbackError) {
              void ipc.openExternalUrl(videoUrl);
            } else {
              setPlaying(true);
            }
          }}
          className="group/vid relative flex min-h-[150px] max-w-[320px] min-w-[220px] cursor-pointer items-center justify-center overflow-hidden rounded-[6px] border border-border bg-black/80"
        >
          {/* HD badge */}
          <span className="absolute top-2 left-2 z-10 rounded bg-black/60 px-1.5 py-0.5 text-xs font-bold text-white">
            HD
          </span>
          {posterUrl ? (
            <img
              src={posterUrl}
              alt={fileName}
              loading="lazy"
              decoding="async"
              className="max-h-[260px] w-full object-cover opacity-85 transition group-hover/vid:opacity-100"
            />
          ) : null}
          <div className="absolute inset-0 flex items-center justify-center">
            <div className="flex h-10 w-10 items-center justify-center rounded-full bg-black/55 shadow-md backdrop-blur-sm transition group-hover/vid:bg-black/70">
              <Play className="h-5 w-5 fill-white text-white" />
            </div>
          </div>
          {!inlineVideoSupported || playbackError ? (
            <div className="absolute right-0 bottom-2 left-0 flex justify-center">
              <span className="rounded bg-black/60 px-2 py-0.5 text-xs text-white/80">
                {playbackError ? "Nhấn để mở ngoài" : "Nhấn để xem"}
              </span>
            </div>
          ) : null}
        </button>
      ) : null}
      {!shouldHideMediaCaption(msg) ? (
        <span className="text-base select-text">{msg.text}</span>
      ) : null}
      {/* Download button for videos */}
      {videoUrl ? (
        <button
          type="button"
          className="flex w-fit items-center gap-1 text-xs text-muted-foreground hover:text-foreground"
          onClick={() => void ipc.downloadFile(videoUrl, fileName, downloadDir)}
        >
          <Download className="h-3.5 w-3.5" />
          Tải xuống
        </button>
      ) : null}
    </div>
  );
}

/** Derive file extension color for the file icon badge */
export function fileIconColor(fileName?: string): string {
  const ext = (fileName ?? "").split(".").pop()?.toLowerCase() ?? "";
  if (["pdf"].includes(ext)) return "bg-red-500";
  if (["doc", "docx"].includes(ext)) return "bg-blue-600";
  if (["xls", "xlsx"].includes(ext)) return "bg-green-600";
  if (["ppt", "pptx"].includes(ext)) return "bg-orange-500";
  if (["zip", "rar", "7z"].includes(ext)) return "bg-yellow-600";
  if (["mp3", "ogg", "wav", "aac"].includes(ext)) return "bg-purple-500";
  return "bg-primary";
}

/** Format file size from bytes string */
export function formatFileSize(desc?: string): string {
  if (!desc) return "";
  // If desc already has KB/MB, just return it
  if (/[KMG]B/i.test(desc)) return desc;
  const n = parseFloat(desc);
  if (isNaN(n)) return desc;
  if (n < 1024) return `${n} B`;
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KB`;
  return `${(n / 1024 / 1024).toFixed(1)} MB`;
}

export function FileMessageMedia({ msg, downloadDir }: { msg: ChatMessage; downloadDir: string }) {
  return (
    <div className="flex max-w-[340px] min-w-[220px] items-center gap-3">
      <div
        className={cn(
          "flex h-10 w-10 shrink-0 items-center justify-center rounded-lg",
          fileIconColor(msg.link_title ?? msg.text),
        )}
      >
        <FileText className="h-5 w-5 text-white" />
      </div>
      <div className="min-w-0 flex-1">
        <div className="truncate text-sm font-medium">{msg.link_title ?? msg.text}</div>
        {msg.link_desc ? (
          <div className="text-xs text-muted-foreground">{formatFileSize(msg.link_desc)}</div>
        ) : null}
      </div>
      <Button
        variant="ghost"
        size="icon-sm"
        title="Tải xuống"
        className="h-8 w-8 shrink-0"
        onClick={() =>
          void ipc.downloadFile(msg.link_url!, msg.link_title ?? "attachment", downloadDir)
        }
      >
        <Download className="h-4 w-4" />
      </Button>
    </div>
  );
}

export function LinkPreviewCard({ msg }: { msg: ChatMessage }) {
  if (!msg.link_url) return null;
  return (
    <div
      onClick={() => void ipc.openExternalUrl(msg.link_url!)}
      className="mt-2 max-w-[360px] cursor-pointer rounded-[6px] border border-border/60 bg-background/50 p-2.5 transition-colors hover:bg-background/80"
    >
      {msg.link_thumb ? (
        <img
          src={msg.link_thumb}
          alt={msg.link_title ?? ""}
          loading="lazy"
          className="mb-2 w-full rounded-[4px] object-cover"
          style={{ maxHeight: 140 }}
        />
      ) : null}
      <div className="flex items-start gap-1.5">
        {!msg.link_thumb ? (
          <ExternalLink className="mt-0.5 h-3.5 w-3.5 shrink-0 text-primary" />
        ) : null}
        <div className="min-w-0">
          <div className="truncate text-sm font-semibold text-foreground">
            {msg.link_title ?? msg.link_url}
          </div>
          {msg.link_desc ? (
            <div className="mt-0.5 line-clamp-2 text-xs text-muted-foreground">{msg.link_desc}</div>
          ) : null}
          {msg.link_src ? (
            <div className="mt-1 truncate text-xs text-primary">{msg.link_src}</div>
          ) : null}
        </div>
      </div>
    </div>
  );
}

export function CallCardMedia({ callCard }: { callCard: CallCardMeta }) {
  return (
    <div className="flex min-w-[200px] items-center gap-2.5">
      <div
        className={cn(
          "grid h-9 w-9 shrink-0 place-items-center rounded-full",
          callCard.isMissedOrFailed
            ? "bg-destructive/15 text-destructive"
            : "bg-online/15 text-online",
        )}
      >
        {callCard.isVideo ? <Video className="h-4.5 w-4.5" /> : <Phone className="h-4.5 w-4.5" />}
      </div>
      <div>
        <div className="text-sm font-bold">{callCard.title}</div>
        <div className="text-xs text-muted-foreground">{callCard.subtitle}</div>
      </div>
    </div>
  );
}
