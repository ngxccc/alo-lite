import { useEffect, useState } from "react";
import { Download, RotateCcw, X, ZoomIn, ZoomOut } from "lucide-react";
import { ipc } from "../api/ipc";
import { jxlToJpgUrl } from "../utils/text";
import { Button } from "./button";
import { Dialog, DialogContent } from "./dialog";
import type { ImageViewerState } from "../../app/store/appReducer";

interface ImageViewerModalProps {
  viewer: ImageViewerState | null;
  mediaCache: Record<string, string | undefined>;
  downloadDir: string;
  onClose: () => void;
}

export function ImageViewerModal({
  viewer,
  mediaCache,
  downloadDir,
  onClose,
}: ImageViewerModalProps) {
  const [zoom, setZoom] = useState(1);

  useEffect(() => {
    setZoom(1);
  }, [viewer?.texKey, viewer?.fullUrl]);

  if (!viewer) return null;

  const hdKey = `hd_${viewer.texKey}`;
  const src = mediaCache[hdKey] ?? mediaCache[viewer.texKey] ?? jxlToJpgUrl(viewer.fullUrl);

  return (
    <Dialog open={Boolean(viewer)} onOpenChange={(open) => !open && onClose()}>
      <DialogContent
        hideClose
        className="h-[86vh] max-h-[86vh] w-[88vw] max-w-[88vw] bg-background"
      >
        <div className="flex h-14 shrink-0 items-center justify-between border-b border-border bg-sidebar px-4">
          <div className="min-w-0">
            <div className="truncate text-sm font-semibold">{viewer.title || "Hình ảnh"}</div>
            <div className="truncate text-xs text-muted-foreground">
              {viewer.sender} · {viewer.time}
            </div>
          </div>

          <div className="flex items-center gap-1.5">
            <Button
              variant="outline"
              size="icon-sm"
              title="Thu nhỏ"
              onClick={() => setZoom((z) => Math.max(0.25, Number((z - 0.25).toFixed(2))))}
            >
              <ZoomOut className="h-3.5 w-3.5" />
            </Button>
            <span className="min-w-12 text-center font-mono text-xs text-muted-foreground">
              {Math.round(zoom * 100)}%
            </span>
            <Button
              variant="outline"
              size="icon-sm"
              title="Phóng to"
              onClick={() => setZoom((z) => Math.min(4, Number((z + 0.25).toFixed(2))))}
            >
              <ZoomIn className="h-3.5 w-3.5" />
            </Button>
            <Button variant="outline" size="icon-sm" title="Đặt lại" onClick={() => setZoom(1)}>
              <RotateCcw className="h-3.5 w-3.5" />
            </Button>
            {viewer.fullUrl ? (
              <Button
                variant="default"
                size="sm"
                onClick={() =>
                  void ipc.downloadFile(
                    viewer.fullUrl,
                    viewer.title || "zalo_image.jpg",
                    downloadDir,
                  )
                }
              >
                <Download className="h-3.5 w-3.5" /> Tải xuống
              </Button>
            ) : null}
            <Button variant="ghost" size="icon-sm" onClick={onClose} title="Đóng (Esc)">
              <X className="h-4 w-4" />
            </Button>
          </div>
        </div>

        <div className="flex flex-1 items-center justify-center overflow-auto bg-black/80 p-6">
          {src ? (
            <img
              src={src}
              alt={viewer.title}
              style={{ transform: `scale(${zoom})` }}
              className="max-h-full max-w-full origin-center rounded-md object-contain transition-transform duration-150"
            />
          ) : (
            <span className="text-xs text-muted-foreground">Đang tải ảnh độ phân giải cao…</span>
          )}
        </div>
      </DialogContent>
    </Dialog>
  );
}
