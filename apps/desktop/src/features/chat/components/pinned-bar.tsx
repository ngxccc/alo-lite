import { ChevronDown, MessageSquare } from "lucide-react";
import type { ChatMessage } from "@/shared/types/ipc.generated";
import { Button } from "@/shared/ui/button";

interface PinnedBarProps {
  pinnedMessages: ChatMessage[];
  onJumpToPinned: (msg: ChatMessage) => void;
  onToggleInfoDrawer: () => void;
}

export function PinnedBar({ pinnedMessages, onJumpToPinned, onToggleInfoDrawer }: PinnedBarProps) {
  const pinnedMessage = pinnedMessages.at(0);
  if (!pinnedMessage) return null;

  return (
    <div className="flex shrink-0 items-center justify-between border-b border-border/40 bg-sidebar px-4 py-1.5">
      <div
        onClick={() => onJumpToPinned(pinnedMessage)}
        className="flex min-w-0 flex-1 cursor-pointer items-center gap-2.5 overflow-hidden hover:opacity-90"
      >
        <div className="flex h-7 w-7 shrink-0 items-center justify-center rounded-full bg-primary/10 text-primary">
          <MessageSquare className="h-4 w-4" />
        </div>
        <div className="min-w-0 flex-1">
          <div className="text-sm leading-tight font-semibold text-foreground">Tin nhắn</div>
          <div className="truncate text-xs leading-tight text-muted-foreground">
            {pinnedMessage.sender ? `${pinnedMessage.sender}: ` : ""}
            {pinnedMessage.text || "[Hình ảnh/Tập tin]"}
          </div>
        </div>
      </div>
      <div className="ml-2 flex shrink-0 items-center gap-1.5">
        {pinnedMessages.length > 1 ? (
          <button
            type="button"
            onClick={onToggleInfoDrawer}
            className="flex items-center gap-1 rounded-full bg-muted/80 px-2.5 py-0.5 text-xs font-medium text-muted-foreground transition hover:bg-muted"
          >
            +{pinnedMessages.length - 1} ghim
            <ChevronDown className="h-3 w-3" />
          </button>
        ) : (
          <Button
            variant="ghost"
            size="icon-sm"
            className="h-6 w-6 text-muted-foreground hover:text-foreground"
            title="Xem tất cả tin ghim"
            onClick={onToggleInfoDrawer}
          >
            <ChevronDown className="h-3.5 w-3.5" />
          </Button>
        )}
      </div>
    </div>
  );
}
