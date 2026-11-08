import { REACTION_LIST } from "@/shared/types/ipc";
import { Button } from "@/shared/ui/button";
import { PopoverContent } from "@/shared/ui/popover";
import { cn } from "@/shared/utils/cn";
import likeIcon from "@/assets/reactions/like.png";
import heartIcon from "@/assets/reactions/heart.png";
import smileIcon from "@/assets/reactions/smile.png";
import surprisedIcon from "@/assets/reactions/surprised.png";
import cryIcon from "@/assets/reactions/cry.png";
import rageIcon from "@/assets/reactions/rage.png";

export const REACTION_ICONS = {
  like: likeIcon,
  heart: heartIcon,
  smile: smileIcon,
  surprised: surprisedIcon,
  cry: cryIcon,
  rage: rageIcon,
} as const;

export const resolveReactionKey = (raw: string): keyof typeof REACTION_ICONS | null => {
  const clean = raw.trim();
  if (clean === "\u{1F44D}" || clean === "/-strong" || clean === "like" || clean === "3") {
    return "like";
  }
  if (
    clean === "\u{2764}\u{FE0F}" ||
    clean === "\u{2764}" ||
    clean === "/-heart" ||
    clean === "heart" ||
    clean === "5"
  ) {
    return "heart";
  }
  if (
    clean === "\u{1F606}" ||
    clean === ":>" ||
    clean === "smile" ||
    clean === "haha" ||
    clean === "0"
  ) {
    return "smile";
  }
  if (
    clean === "\u{1F62E}" ||
    clean === ":o" ||
    clean === "surprised" ||
    clean === "wow" ||
    clean === "32"
  ) {
    return "surprised";
  }
  if (
    clean === "\u{1F622}" ||
    clean === ":-" ||
    clean === "cry" ||
    clean === "buồn" ||
    clean === "2"
  ) {
    return "cry";
  }
  if (
    clean === "\u{1F620}" ||
    clean === ":-h" ||
    clean === "rage" ||
    clean === "phẫn nộ" ||
    clean === "20"
  ) {
    return "rage";
  }
  return null;
};

export function ReactionBadge({ raw }: { raw: string }) {
  const key = resolveReactionKey(raw);
  if (key) {
    return <img src={REACTION_ICONS[key]} alt={key} className="h-4.5 w-4.5 object-contain" />;
  }
  const found = REACTION_LIST.find((r) => r.emoji === raw);
  return <span className="text-sm leading-none">{found?.emoji ?? raw}</span>;
}

interface ReactionPickerContentProps {
  onSelectReaction: (emoji: string) => void;
}

export function ReactionPickerContent({ onSelectReaction }: ReactionPickerContentProps) {
  return (
    <PopoverContent side="top" className="flex items-center gap-1 p-1.5">
      {REACTION_LIST.map((r) => (
        <Button
          key={r.key}
          variant="ghost"
          size="icon-sm"
          title={r.label}
          onClick={() => onSelectReaction(r.emoji)}
        >
          <ReactionBadge raw={r.emoji} />
        </Button>
      ))}
    </PopoverContent>
  );
}

interface MessageReactionsChipProps {
  reactions?: string[];
  isOutgoing?: boolean;
}

export function MessageReactionsChip({ reactions, isOutgoing }: MessageReactionsChipProps) {
  if (!reactions || reactions.length === 0) return null;

  return (
    <div
      className={cn(
        "-mt-1.5 flex cursor-pointer items-center gap-0.5 rounded-full border border-border/80 bg-sidebar px-2 py-0.5 text-xs shadow-sm transition-colors select-none hover:bg-secondary",
        isOutgoing ? "mr-1 self-end" : "ml-1 self-start",
      )}
    >
      {Array.from(new Set(reactions))
        .slice(0, 5)
        .map((em, i) => (
          <span key={`${em}_${i}`} className="flex items-center">
            <ReactionBadge raw={em} />
          </span>
        ))}
      {reactions.length > 1 ? (
        <span className="ml-0.5 text-[11px] font-semibold text-muted-foreground">
          {reactions.length}
        </span>
      ) : null}
    </div>
  );
}
