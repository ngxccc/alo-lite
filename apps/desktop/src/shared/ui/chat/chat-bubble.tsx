import * as React from "react";
import { cva, type VariantProps } from "class-variance-authority";
import { Avatar } from "@/shared/ui/avatar";
import { Button, type ButtonProps } from "@/shared/ui/button";
import { cn } from "@/shared/utils/cn";

// ── ChatBubble ───────────────────────────────────────────────
const chatBubbleVariant = cva("group relative flex max-w-[76%] items-end gap-2 transition-colors", {
  variants: {
    variant: {
      received: "self-start",
      sent: "flex-row-reverse self-end",
    },
    layout: {
      default: "",
      ai: "w-full max-w-full items-center",
    },
  },
  defaultVariants: {
    variant: "received",
    layout: "default",
  },
});

export interface ChatBubbleProps
  extends React.HTMLAttributes<HTMLDivElement>, VariantProps<typeof chatBubbleVariant> {}

export const ChatBubble = React.forwardRef<HTMLDivElement, ChatBubbleProps>(
  ({ className, variant, layout, children, ...props }, ref) => (
    <div ref={ref} className={cn(chatBubbleVariant({ variant, layout, className }))} {...props}>
      {React.Children.map(children, (child) =>
        React.isValidElement(child) && typeof child.type !== "string"
          ? React.cloneElement(child, {
              variant,
              layout,
            } as React.ComponentProps<typeof child.type>)
          : child,
      )}
    </div>
  ),
);
ChatBubble.displayName = "ChatBubble";

// ── ChatBubbleAvatar ─────────────────────────────────────────
export interface ChatBubbleAvatarProps {
  id?: string;
  name?: string;
  avatarUrl?: string | null;
  size?: number;
  mediaCache?: Record<string, string | undefined>;
  className?: string;
}

export const ChatBubbleAvatar: React.FC<ChatBubbleAvatarProps> = ({
  id = "",
  name = "User",
  avatarUrl,
  size = 32,
  mediaCache,
  className,
}) => (
  <Avatar
    id={id}
    name={name}
    avatarUrl={avatarUrl}
    size={size}
    mediaCache={mediaCache}
    className={cn("mb-1 shrink-0", className)}
  />
);

// ── ChatBubbleMessage ────────────────────────────────────────
const chatBubbleMessageVariants = cva(
  "relative max-w-[640px] text-base leading-relaxed break-words whitespace-pre-wrap shadow-xs transition-colors",
  {
    variants: {
      variant: {
        received:
          "rounded-2xl rounded-bl-xs border border-border/40 bg-bubble-in px-3.5 py-2.5 text-bubble-in-foreground",
        sent: "rounded-2xl rounded-br-xs border border-bubble-out-border/30 bg-bubble-out px-3.5 py-2.5 text-bubble-out-foreground",
      },
      layout: {
        default: "",
        ai: "w-full rounded-none border-t bg-transparent",
      },
    },
    defaultVariants: {
      variant: "received",
      layout: "default",
    },
  },
);

export interface ChatBubbleMessageProps
  extends React.HTMLAttributes<HTMLDivElement>, VariantProps<typeof chatBubbleMessageVariants> {
  isLoading?: boolean;
}

export const ChatBubbleMessage = React.forwardRef<HTMLDivElement, ChatBubbleMessageProps>(
  ({ className, variant, layout, isLoading = false, children, ...props }, ref) => (
    <div
      ref={ref}
      className={cn(chatBubbleMessageVariants({ variant, layout, className }))}
      {...props}
    >
      {isLoading ? (
        <div className="flex items-center space-x-1.5 py-1">
          <span className="h-1.5 w-1.5 animate-bounce rounded-full bg-muted-foreground/60 [animation-delay:-0.3s]" />
          <span className="h-1.5 w-1.5 animate-bounce rounded-full bg-muted-foreground/60 [animation-delay:-0.15s]" />
          <span className="h-1.5 w-1.5 animate-bounce rounded-full bg-muted-foreground/60" />
        </div>
      ) : (
        children
      )}
    </div>
  ),
);
ChatBubbleMessage.displayName = "ChatBubbleMessage";

// ── ChatBubbleTimestamp ──────────────────────────────────────
export interface ChatBubbleTimestampProps extends React.HTMLAttributes<HTMLDivElement> {
  timestamp: string;
}

export const ChatBubbleTimestamp: React.FC<ChatBubbleTimestampProps> = ({
  timestamp,
  className,
  children,
  ...props
}) => (
  <div className={cn("mt-1.5 flex items-center gap-1 text-xs select-none", className)} {...props}>
    <span>{timestamp}</span>
    {children}
  </div>
);

// ── ChatBubbleAction ─────────────────────────────────────────
export type ChatBubbleActionProps = ButtonProps & {
  icon: React.ReactNode;
};

export const ChatBubbleAction: React.FC<ChatBubbleActionProps> = ({
  icon,
  onClick,
  className,
  variant = "ghost",
  size = "icon-sm",
  ...props
}) => (
  <Button
    variant={variant}
    size={size}
    className={cn("h-7 w-7", className)}
    onClick={onClick}
    {...props}
  >
    {icon}
  </Button>
);

// ── ChatBubbleActionWrapper ──────────────────────────────────
export interface ChatBubbleActionWrapperProps extends React.HTMLAttributes<HTMLDivElement> {
  variant?: "sent" | "received";
  className?: string;
  forceVisible?: boolean;
}

export const ChatBubbleActionWrapper = React.forwardRef<
  HTMLDivElement,
  ChatBubbleActionWrapperProps
>(({ className, forceVisible = false, children, ...props }, ref) => (
  <div
    ref={ref}
    className={cn(
      "z-20 shrink-0 items-center gap-0.5 self-center rounded-lg border border-border bg-popover p-0.5 shadow-md transition-opacity duration-150 select-none",
      forceVisible
        ? "flex opacity-100"
        : "hidden opacity-0 group-hover:flex group-hover:opacity-100",
      className,
    )}
    {...props}
  >
    {children}
  </div>
));
ChatBubbleActionWrapper.displayName = "ChatBubbleActionWrapper";
