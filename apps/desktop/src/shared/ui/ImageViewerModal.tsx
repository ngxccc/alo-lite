import { useEffect, useState } from "react";
import { Download, RotateCcw, X, ZoomIn, ZoomOut } from "lucide-react";
import { ipc } from "../api/ipc";
import { jxlToJpgUrl } from "../utils/text";
import { Button } from "./button";
import { Dialog, DialogContent } from "./dialog";
import type { ImageViewerState } from "../../app/store/appReducer";

interface ImageViewerModalProps {
  viewer: ImageViewerState | null;
  mediaCache: Record<string, string>;
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
      <DialogContent hideClose className="max-w-[88vw] w-[88vw] h-[86vh] max-h-[86vh] bg-background">
        <div className="flex items-center justify-between h-14 px-4 border-b border-border bg-sidebar shrink-0">
          <div className="min-w-0">
            <div className="font-semibold text-sm truncate">{viewer.title || "Hình ảnh"}</div>
            <div className="text-xs text-muted-foreground truncate">
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
            <span className="min-w-12 text-center text-xs text-muted-foreground font-mono">
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
                  void ipc.downloadFile(viewer.fullUrl, viewer.title || "zalo_image.jpg", downloadDir)
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

        <div className="flex-1 overflow-auto flex items-center justify-center p-6 bg-black/80">
          {src ? (
            <img
              src={src}
              alt={viewer.title}
              style={{ transform: `scale(${zoom})` }}
              className="max-w-full max-h-full object-contain rounded-md transition-transform duration-150 origin-center"
            />
          ) : (
            <span className="text-muted-foreground text-xs">Đang tải ảnh độ phân giải cao…</span>
          )}
        </div>
      </DialogContent>
    </Dialog>
  );
}
