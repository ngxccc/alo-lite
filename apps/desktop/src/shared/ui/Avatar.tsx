import { useState } from "react";
import { Users } from "lucide-react";
import { cn } from "@/shared/utils/cn";
import { avatarColor, initialsOf, jxlToJpgUrl } from "@/shared/utils/text";

interface AvatarProps {
  id: string;
  name: string;
  avatarUrl?: string | null;
  isGroup?: boolean;
  memberCount?: number;
  size?: number;
  mediaCache?: Record<string, string | undefined>;
  className?: string;
}

export function Avatar({
  id,
  name,
  avatarUrl,
  isGroup = false,
  memberCount,
  size = 38,
  mediaCache,
  className,
}: AvatarProps) {
  const [failed, setFailed] = useState(false);
  const cachedDataUrl = mediaCache?.[`ava_${id}`];
  const src = cachedDataUrl || (avatarUrl && !failed ? jxlToJpgUrl(avatarUrl) : "");

  const avatarNode = src ? (
    <img
      src={src}
      alt={name}
      loading="lazy"
      decoding="async"
      onError={() => setFailed(true)}
      style={{ width: size, height: size }}
      className={cn(
        "shrink-0 rounded-full border border-border object-cover select-none",
        className,
      )}
    />
  ) : (
    <div
      style={{
        width: size,
        height: size,
        backgroundColor: avatarColor(name || id),
        fontSize: Math.max(11, Math.round(size * 0.36)),
      }}
      aria-label={name}
      className={cn(
        "inline-flex shrink-0 items-center justify-center rounded-full font-bold text-white select-none",
        className,
      )}
    >
      {isGroup ? <Users size={Math.round(size * 0.5)} /> : initialsOf(name || "Z")}
    </div>
  );

  if (isGroup && memberCount && memberCount > 0 && size >= 36) {
    return (
      <div className="relative inline-flex shrink-0">
        {avatarNode}
        <span className="absolute -right-1 -bottom-1 flex h-4 min-w-4 items-center justify-center gap-0.5 rounded-full bg-[#1c1e23] px-1 text-[9.5px] font-semibold text-gray-300 shadow-xs ring-1 ring-border">
          <Users className="h-2.5 w-2.5" />
          <span>{memberCount > 999 ? "999+" : memberCount}</span>
        </span>
      </div>
    );
  }

  return avatarNode;
}
