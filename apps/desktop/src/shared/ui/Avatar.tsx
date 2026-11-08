import { useState } from "react";
import { Users } from "lucide-react";
import { cn } from "../utils/cn";
import { avatarColor, initialsOf, jxlToJpgUrl } from "../utils/text";

interface AvatarProps {
  id: string;
  name: string;
  avatarUrl?: string | null;
  isGroup?: boolean;
  size?: number;
  mediaCache?: Record<string, string>;
  className?: string;
}

export function Avatar({
  id,
  name,
  avatarUrl,
  isGroup = false,
  size = 38,
  mediaCache,
  className,
}: AvatarProps) {
  const [failed, setFailed] = useState(false);
  const cachedDataUrl = mediaCache?.[`ava_${id}`];
  const src = cachedDataUrl || (avatarUrl && !failed ? jxlToJpgUrl(avatarUrl) : "");

  if (src) {
    return (
      <img
        src={src}
        alt={name}
        loading="lazy"
        decoding="async"
        onError={() => setFailed(true)}
        style={{ width: size, height: size }}
        className={cn(
          "rounded-full shrink-0 object-cover border border-border select-none",
          className,
        )}
      />
    );
  }

  return (
    <div
      style={{
        width: size,
        height: size,
        backgroundColor: avatarColor(name || id),
        fontSize: Math.max(11, Math.round(size * 0.36)),
      }}
      aria-label={name}
      className={cn(
        "rounded-full shrink-0 inline-flex items-center justify-center font-bold text-white select-none",
        className,
      )}
    >
      {isGroup ? <Users size={Math.round(size * 0.5)} /> : initialsOf(name || "Z")}
    </div>
  );
}
