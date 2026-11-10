import { SmilePlus, Star } from "lucide-react";
import { ipc } from "@/shared/api/ipc";
import type { ChatMessage, Thread } from "@/shared/types/ipc";
import { Avatar } from "@/shared/ui/avatar";
import { Popover, PopoverTrigger } from "@/shared/ui/popover";
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
import { MessageReactionsChip, ReactionPickerContent } from "./message-reactions";
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
    <div
      ref={setRef}
      onDoubleClick={() => {
        if (state.settings?.double_click_to_reply && !isRecalled) {
          dispatch({ type: "SET_REPLYING_TO", msg });
        }
      }}
      className={cn(
        "group relative flex max-w-[76%] items-end gap-2 transition-colors",
        msg.is_outgoing ? "flex-row-reverse self-end" : "self-start",
        highlighted && "rounded-lg bg-primary/10 p-1 ring-2 ring-primary/60",
      )}
    >
      {/* Incoming avatar — 32px */}
      {!msg.is_outgoing && senderInfo ? (
        !isGroupedWithPrev ? (
          <Avatar
            id={senderInfo.id}
            name={senderInfo.name}
            avatarUrl={senderInfo.avatar}
            size={32}
            mediaCache={state.mediaCache}
            className="mb-1 shrink-0"
          />
        ) : (
          <div className="w-8 shrink-0" />
        )
      ) : null}

      {/* Bubble column */}
      <div
        className={cn(
          "relative flex min-w-0 flex-col",
          msg.is_outgoing ? "items-end" : "items-start",
        )}
      >
        {/* Bubble */}
        <div
          className={cn(
            isBareSticker
              ? "relative"
              : cn(
                  "relative max-w-[640px] rounded-[6px] text-[15px] leading-relaxed break-words whitespace-pre-wrap shadow-sm",
                  isRecalled
                    ? "bg-bubble-in px-3 py-2.5 text-muted-foreground italic opacity-70"
                    : msg.is_outgoing
                      ? "bg-bubble-out px-3 py-2.5 text-bubble-out-foreground"
                      : "bg-bubble-in px-3 py-2.5 text-bubble-in-foreground",
                ),
          )}
        >
          {/* Sender name inside bubble top — groups only, incoming, first in run */}
          {!msg.is_outgoing &&
          thread.is_group &&
          !isGroupedWithPrev &&
          senderInfo &&
          isTextOrRich &&
          !isRecalled ? (
            <div className="mb-1 text-[13px] font-semibold text-accent-foreground">
              {senderInfo.name}
            </div>
          ) : null}

          {/* Reply quote */}
          {msg.reply_to_text ? (
            <div
              onClick={() => jumpToQuoted(msg)}
              className="mb-2 cursor-pointer rounded-md bg-background/40 px-2.5 py-1.5 text-[13px] hover:bg-background/70"
            >
              <div className="text-[12px] font-bold text-primary">
                {resolveReplySenderName(msg.reply_to_sender)}
              </div>
              <div className="max-w-[300px] truncate text-muted-foreground">
                {msg.reply_to_text}
              </div>
            </div>
          ) : null}

          {/* Message body */}
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

          {/* Timestamp inside bubble — bottom, after content */}
          {isTextOrRich && !isRecalled ? (
            <div
              className={cn(
                "mt-1.5 flex items-center gap-1 text-[12px]",
                msg.is_outgoing
                  ? "justify-end text-bubble-out-foreground/60"
                  : "text-muted-foreground",
              )}
            >
              <span>{msg.time}</span>
              {msg.is_starred ? <Star className="h-3 w-3 text-warning" /> : null}
              {msg.is_outgoing && msg.delivery === "seen" ? (
                <span className="opacity-80">· Đã xem</span>
              ) : msg.is_outgoing && msg.delivery === "sent" ? (
                <span className="opacity-60">· Đã gửi</span>
              ) : null}
            </div>
          ) : null}

          {/* Quick reaction/like button — small circle, bottom-right corner of bubble, appears on group hover */}
          {isTextOrRich && !isRecalled ? (
            <Popover
              open={reactPickerFor === `quick_${msgKey}`}
              onOpenChange={(open) => setReactPickerFor(open ? `quick_${msgKey}` : null)}
            >
              <PopoverTrigger asChild>
                <button
                  type="button"
                  title="Thả cảm xúc"
                  className={cn(
                    "absolute -right-2.5 -bottom-2.5 z-10 flex h-6 w-6 cursor-pointer items-center justify-center rounded-full border border-border bg-sidebar shadow-sm transition-opacity",
                    "opacity-0 group-hover:opacity-100",
                  )}
                >
                  <SmilePlus className="h-3.5 w-3.5 text-muted-foreground" />
                </button>
              </PopoverTrigger>
              <ReactionPickerContent onSelectReaction={handleReaction} />
            </Popover>
          ) : null}
        </div>

        {/* Sticker time below */}
        {isBareSticker || isRecalled ? (
          <div className="flex items-center gap-1 px-1 pt-0.5 text-[11.5px] text-muted-foreground">
            <span>{msg.time}</span>
            {msg.is_starred ? <Star className="h-3 w-3 text-warning" /> : null}
          </div>
        ) : null}

        {/* Reactions chip — overlaps bubble bottom edge */}
        <MessageReactionsChip reactions={msg.reactions} isOutgoing={msg.is_outgoing} />
      </div>

      {/* Hover action toolbar — floating above bubble */}
      {!isRecalled ? (
        <MessageActions
          msg={msg}
          msgKey={msgKey}
          thread={thread}
          reactPickerFor={reactPickerFor}
          setReactPickerFor={setReactPickerFor}
          onSelectReaction={handleReaction}
          dispatch={dispatch}
        />
      ) : null}
    </div>
  );
}
