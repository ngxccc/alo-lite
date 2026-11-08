import { useEffect, useState } from "react";
import { Download, RotateCcw, X, ZoomIn, ZoomOut } from "lucide-react";
import { ipc } from "../api/ipc";
import { jxlToJpgUrl } from "../utils/text";
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
    <div className="modal-backdrop" onClick={onClose}>
      <div
        className="modal-card"
        style={{ width: "86vw", height: "86vh", background: "var(--bg-app)" }}
        onClick={(e) => e.stopPropagation()}
      >
        <div
          className="chat-header"
          style={{ justifyContent: "space-between", background: "var(--bg-sidebar)" }}
        >
          <div style={{ minWidth: 0 }}>
            <div style={{ fontWeight: 700, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
              {viewer.title || "Xem ảnh"}
            </div>
            <div style={{ fontSize: 11.5, color: "var(--text-muted)" }}>
              {viewer.sender} {viewer.time ? `· ${viewer.time}` : ""}
            </div>
          </div>
          <div style={{ display: "flex", alignItems: "center", gap: 6 }}>
            <button
              type="button"
              className="icon-btn"
              title="Thu nhỏ"
              onClick={() => setZoom((z) => Math.max(0.25, +(z - 0.25).toFixed(2)))}
            >
              <ZoomOut size={16} />
            </button>
            <span style={{ fontSize: 12, minWidth: 44, textAlign: "center" }}>
              {Math.round(zoom * 100)}%
            </span>
            <button
              type="button"
              className="icon-btn"
              title="Phóng to"
              onClick={() => setZoom((z) => Math.min(4, +(z + 0.25).toFixed(2)))}
            >
              <ZoomIn size={16} />
            </button>
            <button
              type="button"
              className="icon-btn"
              title="Đặt lại tỷ lệ"
              onClick={() => setZoom(1)}
            >
              <RotateCcw size={15} />
            </button>
            {viewer.fullUrl ? (
              <button
                type="button"
                className="btn"
                onClick={() =>
                  void ipc.downloadFile(viewer.fullUrl, viewer.title || "image.jpg", downloadDir)
                }
              >
                <Download size={14} /> Tải về
              </button>
            ) : null}
            <button type="button" className="icon-btn" title="Đóng" onClick={onClose}>
              <X size={17} />
            </button>
          </div>
        </div>
        <div
          style={{
            flex: 1,
            overflow: "auto",
            display: "grid",
            placeItems: "center",
            padding: 20,
          }}
        >
          <img
            src={src}
            alt={viewer.title}
            style={{
              maxWidth: zoom === 1 ? "100%" : "none",
              maxHeight: zoom === 1 ? "100%" : "none",
              transform: `scale(${zoom})`,
              transformOrigin: "center center",
              transition: "transform 0.12s ease",
              borderRadius: 6,
            }}
          />
        </div>
      </div>
    </div>
  );
}
