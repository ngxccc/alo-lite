import { useState } from "react";
import { Users } from "lucide-react";
import { avatarColor, initialsOf, jxlToJpgUrl } from "../utils/text";

interface AvatarProps {
  id: string;
  name: string;
  avatarUrl?: string | null;
  isGroup?: boolean;
  size?: number;
  mediaCache?: Record<string, string>;
}

export function Avatar({
  id,
  name,
  avatarUrl,
  isGroup = false,
  size = 38,
  mediaCache,
}: AvatarProps) {
  const [failed, setFailed] = useState(false);
  const cachedDataUrl = mediaCache?.[`ava_${id}`];
  const src = cachedDataUrl || (avatarUrl && !failed ? jxlToJpgUrl(avatarUrl) : "");

  const style = {
    width: `${size}px`,
    height: `${size}px`,
    borderRadius: "50%",
    flexShrink: 0,
  };

  if (src) {
    return (
      <img
        src={src}
        alt={name}
        loading="lazy"
        decoding="async"
        onError={() => setFailed(true)}
        style={{ ...style, objectFit: "cover", border: "1px solid var(--border)" }}
      />
    );
  }
  return (
    <div
      style={{
        ...style,
        background: avatarColor(name || id),
        color: "#fff",
        display: "grid",
        placeItems: "center",
        fontWeight: 700,
        fontSize: `${Math.max(11, Math.round(size * 0.36))}px`,
      }}
      aria-label={name}
    >
      {isGroup && !name ? <Users size={Math.round(size * 0.5)} /> : initialsOf(name || "Z")}
    </div>
  );
}
