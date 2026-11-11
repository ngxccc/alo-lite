import { CornerUpLeft, Forward, RotateCcw, SmilePlus, Star, Trash2 } from "lucide-react";
import { ipc } from "@/shared/api/ipc";
import type { ChatMessage, Thread } from "@/shared/types/ipc.generated";
import { Popover, PopoverTrigger } from "@/shared/ui/popover";
import { ChatBubbleAction, ChatBubbleActionWrapper } from "@/shared/ui/chat";
import { cn } from "@/shared/utils/cn";
import type { AppAction } from "@/app/store/app-reducer";
import { ReactionPickerContent } from "./message-reactions";

interface MessageActionsProps {
  msg: ChatMessage;
  msgKey: string;
  thread: Thread;
  reactPickerFor: string | null;
  setReactPickerFor: (key: string | null) => void;
  onSelectReaction: (emoji: string) => void;
  dispatch: (action: AppAction) => void;
  variant?: "sent" | "received";
}

export function MessageActions({
  msg,
  msgKey,
  thread,
  reactPickerFor,
  setReactPickerFor,
  onSelectReaction,
  dispatch,
  variant,
}: MessageActionsProps) {
  const isPickerOpen = reactPickerFor === msgKey;

  return (
    <ChatBubbleActionWrapper variant={variant} forceVisible={isPickerOpen}>
      <ChatBubbleAction
        icon={<CornerUpLeft className="h-3.5 w-3.5" />}
        title="Trả lời"
        onClick={() => dispatch({ type: "SET_REPLYING_TO", msg })}
      />

      <Popover open={isPickerOpen} onOpenChange={(open) => setReactPickerFor(open ? msgKey : null)}>
        <PopoverTrigger asChild>
          <ChatBubbleAction icon={<SmilePlus className="h-3.5 w-3.5" />} title="Thả cảm xúc" />
        </PopoverTrigger>
        <ReactionPickerContent onSelectReaction={onSelectReaction} />
      </Popover>

      <ChatBubbleAction
        icon={<Forward className="h-3.5 w-3.5" />}
        title="Chuyển tiếp"
        onClick={() => dispatch({ type: "SET_FORWARD_MSG", msg })}
      />

      <ChatBubbleAction
        icon={<Star className={cn("h-3.5 w-3.5", msg.is_starred && "text-warning")} />}
        title="Gắn sao"
        onClick={() =>
          dispatch({
            type: "TOGGLE_LOCAL_STAR",
            threadId: thread.id,
            msgId: msg.id,
          })
        }
      />

      {msg.is_outgoing ? (
        <ChatBubbleAction
          icon={<RotateCcw className="h-3.5 w-3.5" />}
          title="Thu hồi tin nhắn"
          onClick={() => void ipc.recallMessage(thread.id, thread.is_group, msg)}
        />
      ) : null}

      <ChatBubbleAction
        icon={<Trash2 className="h-3.5 w-3.5 text-destructive" />}
        title="Xóa chỉ ở phía tôi"
        onClick={() => {
          dispatch({
            type: "CORE_EVENT",
            event: {
              type: "MessageDeleted",
              payload: {
                thread_id: thread.id,
                msg_id: msg.id,
                ts: msg.ts,
              },
            },
          });
          void ipc.deleteForMe(thread.id, thread.is_group, msg);
        }}
      />
    </ChatBubbleActionWrapper>
  );
}
