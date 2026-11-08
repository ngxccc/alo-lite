import { CornerUpLeft, Forward, RotateCcw, SmilePlus, Star, Trash2 } from "lucide-react";
import { ipc } from "@/shared/api/ipc";
import type { ChatMessage, Thread } from "@/shared/types/ipc";
import { Button } from "@/shared/ui/button";
import { Popover, PopoverTrigger } from "@/shared/ui/popover";
import { cn } from "@/shared/utils/cn";
import type { AppAction } from "@/app/store/appReducer";
import { ReactionPickerContent } from "./MessageReactions";

interface MessageActionsProps {
  msg: ChatMessage;
  msgKey: string;
  thread: Thread;
  reactPickerFor: string | null;
  setReactPickerFor: (key: string | null) => void;
  onSelectReaction: (emoji: string) => void;
  dispatch: (action: AppAction) => void;
}

export function MessageActions({
  msg,
  msgKey,
  thread,
  reactPickerFor,
  setReactPickerFor,
  onSelectReaction,
  dispatch,
}: MessageActionsProps) {
  return (
    <div
      className={cn(
        "absolute -top-9 z-20 hidden items-center gap-0.5 rounded-lg border border-border bg-popover p-0.5 shadow-md group-hover:flex",
        msg.is_outgoing ? "right-0" : "left-10",
      )}
    >
      <Button
        variant="ghost"
        size="icon-sm"
        className="h-7 w-7"
        title="Trả lời"
        onClick={() => dispatch({ type: "SET_REPLYING_TO", msg })}
      >
        <CornerUpLeft className="h-3.5 w-3.5" />
      </Button>

      <Popover
        open={reactPickerFor === msgKey}
        onOpenChange={(open) => setReactPickerFor(open ? msgKey : null)}
      >
        <PopoverTrigger asChild>
          <Button variant="ghost" size="icon-sm" className="h-7 w-7" title="Thả cảm xúc">
            <SmilePlus className="h-3.5 w-3.5" />
          </Button>
        </PopoverTrigger>
        <ReactionPickerContent onSelectReaction={onSelectReaction} />
      </Popover>

      <Button
        variant="ghost"
        size="icon-sm"
        className="h-7 w-7"
        title="Chuyển tiếp"
        onClick={() => dispatch({ type: "SET_FORWARD_MSG", msg })}
      >
        <Forward className="h-3.5 w-3.5" />
      </Button>

      <Button
        variant="ghost"
        size="icon-sm"
        className="h-7 w-7"
        title="Gắn sao"
        onClick={() =>
          dispatch({
            type: "TOGGLE_LOCAL_STAR",
            threadId: thread.id,
            msgId: msg.id,
          })
        }
      >
        <Star className={cn("h-3.5 w-3.5", msg.is_starred && "text-warning")} />
      </Button>

      {msg.is_outgoing ? (
        <Button
          variant="ghost"
          size="icon-sm"
          className="h-7 w-7"
          title="Thu hồi tin nhắn"
          onClick={() => void ipc.recallMessage(thread.id, thread.is_group, msg)}
        >
          <RotateCcw className="h-3.5 w-3.5" />
        </Button>
      ) : null}

      <Button
        variant="ghost"
        size="icon-sm"
        className="h-7 w-7 text-destructive hover:text-destructive"
        title="Xóa chỉ ở phía tôi"
        onClick={() => void ipc.deleteForMe(thread.id, thread.is_group, msg)}
      >
        <Trash2 className="h-3.5 w-3.5" />
      </Button>
    </div>
  );
}
