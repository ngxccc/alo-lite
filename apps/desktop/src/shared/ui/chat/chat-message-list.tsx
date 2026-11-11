import * as React from "react";
import { ArrowDown } from "lucide-react";
import { Button } from "@/shared/ui/button";
import { cn } from "@/shared/utils/cn";

export interface ChatMessageListProps extends React.HTMLAttributes<HTMLDivElement> {
  smooth?: boolean;
  showScrollToBottom?: boolean;
}

export const ChatMessageList = React.forwardRef<HTMLDivElement, ChatMessageListProps>(
  ({ className, children, smooth = false, showScrollToBottom = true, ...props }, ref) => {
    const localRef = React.useRef<HTMLDivElement | null>(null);
    const [isAtBottom, setIsAtBottom] = React.useState(true);

    const handleScroll = (e: React.UIEvent<HTMLDivElement>) => {
      props.onScroll?.(e);
      const target = e.currentTarget;
      const atBottom = target.scrollHeight - target.scrollTop - target.clientHeight < 40;
      setIsAtBottom(atBottom);
    };

    const scrollToBottom = () => {
      const container =
        ref && typeof ref === "object" && "current" in ref
          ? (ref as React.RefObject<HTMLDivElement | null>).current
          : localRef.current;
      if (container) {
        container.scrollTo({
          top: container.scrollHeight,
          behavior: smooth ? "smooth" : "auto",
        });
      }
    };

    return (
      <div className="relative flex min-h-0 w-full flex-1 flex-col">
        <div
          ref={ref ?? localRef}
          onScroll={handleScroll}
          className={cn("flex flex-1 flex-col gap-0.5 overflow-y-auto px-4 py-3", className)}
          {...props}
        >
          {children}
        </div>

        {showScrollToBottom && !isAtBottom && (
          <Button
            onClick={scrollToBottom}
            size="icon"
            variant="outline"
            className="absolute right-6 bottom-3 z-30 h-8 w-8 rounded-full border-border bg-popover/90 text-foreground shadow-md backdrop-blur-xs hover:bg-popover"
            aria-label="Cuộn xuống cuối"
          >
            <ArrowDown className="h-4 w-4" />
          </Button>
        )}
      </div>
    );
  },
);
ChatMessageList.displayName = "ChatMessageList";
