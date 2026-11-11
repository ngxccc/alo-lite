import { useState } from "react";
import { SmilePlus, Undo2 } from "lucide-react";
import { Button } from "@/shared/ui/button";
import { PopoverContent } from "@/shared/ui/popover";
import { cn } from "@/shared/utils/cn";
import {
  CATEGORY_ICONS,
  EMOJI_CATEGORIES,
  PINNED_REACTIONS,
  REACTION_LIST,
} from "../constants/emojis";

export const resolveReactionEmoji = (raw: string): string => {
  const clean = raw.trim();
  if (clean === "\u{1F44D}" || clean === "/-strong" || clean === "like" || clean === "3") {
    return "👍";
  }
  if (
    clean === "\u{2764}\u{FE0F}" ||
    clean === "\u{2764}" ||
    clean === "/-heart" ||
    clean === "heart" ||
    clean === "5"
  ) {
    return "❤️";
  }
  if (
    clean === "\u{1F606}" ||
    clean === ":>" ||
    clean === "smile" ||
    clean === "haha" ||
    clean === "0"
  ) {
    return "😆";
  }
  if (
    clean === "\u{1F62E}" ||
    clean === ":o" ||
    clean === "surprised" ||
    clean === "wow" ||
    clean === "32"
  ) {
    return "😮";
  }
  if (
    clean === "\u{1F622}" ||
    clean === ":-((" ||
    clean === ":-(( " ||
    clean === "cry" ||
    clean === "buồn" ||
    clean === "2"
  ) {
    return "😢";
  }
  if (
    clean === "\u{1F620}" ||
    clean === ":-h" ||
    clean === "rage" ||
    clean === "phẫn nộ" ||
    clean === "20"
  ) {
    return "😠";
  }
  if (clean === ":handclap" || clean === "clap" || clean === "46") {
    return "👏";
  }
  if (clean === "/-rose" || clean === "120") {
    return "🌹";
  }
  if (clean === "/-break" || clean === "65") {
    return "💔";
  }
  if (clean === "/-weak" || clean === "4") {
    return "👎";
  }
  if (clean === "/-ok" || clean === "68") {
    return "👌";
  }
  if (clean === "/-v" || clean === "69") {
    return "✌️";
  }
  if (clean === "/-thanks" || clean === "70") {
    return "🙏";
  }
  if (clean === "/-shit" || clean === "66") {
    return "💩";
  }
  if (clean === "101") {
    return "🎉";
  }
  return clean || "❤️";
};

export function ReactionBadge({ raw }: { raw: string }) {
  const emoji = resolveReactionEmoji(raw);
  return <span className="text-sm leading-none select-none">{emoji}</span>;
}

interface ReactionPickerContentProps {
  onSelectReaction: (emoji: string) => void;
}

export function ReactionPickerContent({ onSelectReaction }: ReactionPickerContentProps) {
  const [showAll, setShowAll] = useState(false);
  const [category, setCategory] = useState<string>("Smileys");

  if (!showAll) {
    return (
      <PopoverContent side="top" className="flex w-auto items-center gap-1 p-1.5">
        {REACTION_LIST.map((r) => (
          <Button
            key={r.key}
            variant="ghost"
            size="icon-sm"
            title={r.label}
            className="h-8 w-8 text-base transition-transform hover:scale-125"
            onClick={() => onSelectReaction(r.emoji)}
          >
            <ReactionBadge raw={r.emoji} />
          </Button>
        ))}
        <Button
          variant="ghost"
          size="icon-sm"
          title="Thêm biểu tượng cảm xúc"
          className="h-8 w-8 text-muted-foreground hover:text-foreground"
          onClick={(e) => {
            e.preventDefault();
            e.stopPropagation();
            setShowAll(true);
          }}
        >
          <SmilePlus className="h-4 w-4" />
        </Button>
      </PopoverContent>
    );
  }

  const categoryEmojis = EMOJI_CATEGORIES[category] ?? [];

  return (
    <PopoverContent side="top" className="w-[300px] space-y-2 p-2">
      <div className="flex items-center justify-between border-b border-border pb-1 text-xs text-muted-foreground">
        <span className="font-medium text-foreground">Ghim & Phổ biến</span>
        <Button
          variant="ghost"
          size="icon-sm"
          className="h-6 w-6 text-muted-foreground"
          title="Thu gọn"
          onClick={() => setShowAll(false)}
        >
          <Undo2 className="h-3.5 w-3.5" />
        </Button>
      </div>

      {/* Pinned row */}
      <div className="flex flex-wrap gap-1">
        {PINNED_REACTIONS.map((em) => (
          <button
            key={`pin_${em}`}
            type="button"
            className="flex h-7 w-7 cursor-pointer items-center justify-center rounded text-base transition-transform hover:scale-120 hover:bg-accent"
            onClick={() => onSelectReaction(em)}
          >
            {em}
          </button>
        ))}
      </div>

      {/* Category tabs */}
      <div className="no-scrollbar flex items-center gap-0.5 overflow-x-auto border-t border-border pt-1">
        {Object.keys(EMOJI_CATEGORIES).map((catName) => (
          <button
            key={catName}
            type="button"
            title={catName}
            className={cn(
              "flex h-6 w-6 shrink-0 cursor-pointer items-center justify-center rounded text-sm transition-colors",
              category === catName
                ? "bg-accent font-bold text-accent-foreground"
                : "text-muted-foreground hover:bg-muted",
            )}
            onClick={() => setCategory(catName)}
          >
            {CATEGORY_ICONS[catName] ?? "😀"}
          </button>
        ))}
      </div>

      {/* Emoji grid for active category */}
      <div className="grid max-h-44 grid-cols-8 gap-1 overflow-y-auto pr-1">
        {categoryEmojis.map((em, idx) => (
          <button
            key={`${category}_${em}_${idx}`}
            type="button"
            className="flex h-7 w-7 cursor-pointer items-center justify-center rounded text-base transition-transform hover:scale-120 hover:bg-accent"
            onClick={() => onSelectReaction(em)}
          >
            {em}
          </button>
        ))}
      </div>
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
        <span className="ml-0.5 text-xs font-semibold text-muted-foreground">
          {reactions.length}
        </span>
      ) : null}
    </div>
  );
}
