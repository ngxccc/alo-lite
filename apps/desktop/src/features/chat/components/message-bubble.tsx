import { Star } from "lucide-react";
import { ipc } from "@/shared/api/ipc";
import type { ChatMessage, Thread } from "@/shared/types/ipc.generated";
import {
  ChatBubble,
  ChatBubbleAvatar,
  ChatBubbleMessage,
  ChatBubbleTimestamp,
} from "@/shared/ui/chat";
import { cn } from "@/shared/utils/cn";
import type { AppAction, AppState } from "@/app/store/app-reducer";
import {
  isFileMessage,
  isGifUrl,
  isPhotoMessage,
  isVideoMessage,
  parseCallCard,
} from "@/features/chat/utils/messages";
import {
  CallCardMedia,
  FileMessageMedia,
  LinkPreviewCard,
  PhotoMessageMedia,
  StickerMessageMedia,
  VideoMessageMedia,
} from "./message-media";
import { MessageReactionsChip } from "./message-reactions";
import { MessageActions } from "./message-actions";
import type { ResolvedSender } from "./use-sender-lookup";

interface MessageBubbleProps {
  msg: ChatMessage;
  msgKey: string;
  thread: Thread;
  isGroupedWithPrev: boolean;
  highlighted: boolean;
  senderInfo: ResolvedSender | null;
  state: AppState;
  dispatch: (action: AppAction) => void;
  reactPickerFor: string | null;
  setReactPickerFor: (key: string | null) => void;
  jumpToQuoted: (msg: ChatMessage) => void;
  resolveReplySenderName: (raw?: string | null) => string;
  setRef: (el: HTMLDivElement | null) => void;
}

export function MessageBubble({
  msg,
  msgKey,
  thread,
  isGroupedWithPrev,
  highlighted,
  senderInfo,
  state,
  dispatch,
  reactPickerFor,
  setReactPickerFor,
  jumpToQuoted,
  resolveReplySenderName,
  setRef,
}: MessageBubbleProps) {
  const isRecalled = msg.text.trim() === "[Tin nhắn đã thu hồi]";
  const callCard = parseCallCard(msg);
  const isBareSticker = Boolean(msg.sticker_id);
  const isTextOrRich = !isBareSticker;
  const bubbleVariant = msg.is_outgoing ? "sent" : "received";

  const handleReaction = (reaction: string) => {
    dispatch({
      type: "CORE_EVENT",
      event: {
        type: "Reacted",
        payload: {
          thread_id: thread.id,
          msg_id: msg.id,
          ts: msg.ts,
          reaction,
        },
      },
    });
    void ipc.reactMessage(thread.id, thread.is_group, msg, reaction);
    setReactPickerFor(null);
  };

  return (
    <ChatBubble
      ref={setRef}
      variant={bubbleVariant}
      onDoubleClick={() => {
        if (state.settings?.double_click_to_reply && !isRecalled) {
          dispatch({ type: "SET_REPLYING_TO", msg });
        }
      }}
      className={cn(highlighted && "rounded-lg bg-primary/10 p-1 ring-2 ring-primary/60")}
    >
      {/* ── Avatar for received messages ── */}
      {!msg.is_outgoing && senderInfo ? (
        !isGroupedWithPrev ? (
          <ChatBubbleAvatar
            id={senderInfo.id}
            name={senderInfo.name}
            avatarUrl={senderInfo.avatar}
            size={32}
            mediaCache={state.mediaCache}
          />
        ) : (
          <div className="w-8 shrink-0" />
        )
      ) : null}

      {/* ── Bubble column ── */}
      <div
        className={cn(
          "relative flex min-w-0 flex-col",
          msg.is_outgoing ? "items-end" : "items-start",
        )}
      >
        {/* ── shadcn-chat ChatBubbleMessage ── */}
        <ChatBubbleMessage
          variant={bubbleVariant}
          className={cn(
            isBareSticker && "border-none bg-transparent p-0 shadow-none",
            isRecalled && "italic opacity-70",
          )}
        >
          {/* Sender display name for group chats */}
          {!msg.is_outgoing &&
          thread.is_group &&
          !isGroupedWithPrev &&
          senderInfo &&
          isTextOrRich &&
          !isRecalled ? (
            <div className="mb-1 text-sm font-semibold text-accent-foreground select-none">
              {senderInfo.name}
            </div>
          ) : null}

          {/* Reply quote snippet */}
          {msg.reply_to_text ? (
            <div
              onClick={() => jumpToQuoted(msg)}
              className="mb-2 cursor-pointer rounded-md bg-background/40 px-2.5 py-1.5 text-sm hover:bg-background/70"
            >
              <div className="text-xs font-bold text-primary">
                {resolveReplySenderName(msg.reply_to_sender)}
              </div>
              <div className="max-w-[300px] truncate text-muted-foreground">
                {msg.reply_to_text}
              </div>
            </div>
          ) : null}

          {/* Message content dispatcher */}
          {isRecalled ? (
            <span>Tin nhắn đã được thu hồi</span>
          ) : msg.sticker_id ? (
            <StickerMessageMedia msg={msg} />
          ) : callCard ? (
            <CallCardMedia callCard={callCard} />
          ) : isVideoMessage(msg) && msg.link_url ? (
            <VideoMessageMedia
              msg={msg}
              downloadDir={state.settings?.download_dir ?? ""}
              inlineVideoSupported={state.inlineVideoSupported}
              inlineVideoReason={state.inlineVideoReason}
              onNotify={(message) => dispatch({ type: "SHOW_TOAST", message })}
            />
          ) : isPhotoMessage(msg) &&
            Boolean(msg.link_thumb ?? msg.link_url ?? isGifUrl(msg.text)) ? (
            <PhotoMessageMedia
              msg={msg}
              onOpenViewer={(texKey, fullUrl) =>
                dispatch({
                  type: "SET_IMAGE_VIEWER",
                  viewer: {
                    texKey,
                    fullUrl,
                    title: msg.link_title ?? "Hình ảnh",
                    sender: senderInfo?.name ?? msg.sender,
                    time: msg.time,
                  },
                })
              }
            />
          ) : isFileMessage(msg) && msg.link_url ? (
            <FileMessageMedia msg={msg} downloadDir={state.settings?.download_dir ?? ""} />
          ) : (
            <div>
              <div className="select-text">{msg.text}</div>
              <LinkPreviewCard msg={msg} />
            </div>
          )}

          {/* Inline Timestamp & delivery status */}
          {isTextOrRich && !isRecalled ? (
            <ChatBubbleTimestamp
              timestamp={msg.time}
              className={cn(
                msg.is_outgoing
                  ? "justify-end text-bubble-out-foreground/60"
                  : "text-muted-foreground",
              )}
            >
              {msg.is_starred ? <Star className="h-3 w-3 text-warning" /> : null}
              {msg.is_outgoing &&
              ((msg.delivery as string) === "seen" || msg.delivery === "Read") ? (
                <span className="opacity-80">· Đã xem</span>
              ) : msg.is_outgoing &&
                ((msg.delivery as string) === "sent" ||
                  msg.delivery === "ServerAccepted" ||
                  msg.delivery === "Delivered") ? (
                <span className="opacity-60">· Đã gửi</span>
              ) : null}
            </ChatBubbleTimestamp>
          ) : null}
        </ChatBubbleMessage>

        {/* Timestamp below for stickers & recalled messages */}
        {isBareSticker || isRecalled ? (
          <div className="flex items-center gap-1 px-1 pt-0.5 text-xs text-muted-foreground select-none">
            <span>{msg.time}</span>
            {msg.is_starred ? <Star className="h-3 w-3 text-warning" /> : null}
          </div>
        ) : null}

        {/* Reaction badge/chip */}
        <MessageReactionsChip reactions={msg.reactions} isOutgoing={msg.is_outgoing} />
      </div>

      {/* Floating action bar */}
      {!isRecalled ? (
        <MessageActions
          msg={msg}
          msgKey={msgKey}
          thread={thread}
          reactPickerFor={reactPickerFor}
          setReactPickerFor={setReactPickerFor}
          onSelectReaction={handleReaction}
          dispatch={dispatch}
          variant={bubbleVariant}
        />
      ) : null}
    </ChatBubble>
  );
}
