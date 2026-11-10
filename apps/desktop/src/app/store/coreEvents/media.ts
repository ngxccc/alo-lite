import type { CoreEventDto } from "@/shared/types/ipc.generated";
import type { AppState } from "../app-state";
import { nextToast } from "../helpers";

export type MediaCoreEvent = Extract<
  CoreEventDto,
  | { type: "StickerImageReady" }
  | { type: "StickerStoreLoaded" }
  | { type: "StickerPackItemsLoaded" }
  | { type: "StickerSearchResults" }
  | { type: "StickerSuggestKeywordsLoaded" }
  | { type: "ComposerStickerSuggestions" }
  | { type: "LinkPreviewReady" }
  | { type: "FileDownloaded" }
>;

export function handleMediaEvent(state: AppState, event: MediaCoreEvent): AppState {
  switch (event.type) {
    case "StickerImageReady":
      return {
        ...state,
        mediaCache: {
          ...state.mediaCache,
          [event.payload.key]: event.payload.data_url,
        },
      };

    case "StickerStoreLoaded":
      return {
        ...state,
        stickerStorePacks: event.payload.packs,
        stickerStoreLoading: false,
      };

    case "StickerPackItemsLoaded":
      return {
        ...state,
        stickerPackItems: {
          ...state.stickerPackItems,
          [event.payload.cate_id]: event.payload.items,
        },
      };

    case "StickerSearchResults":
      return {
        ...state,
        stickerSearchResults: event.payload.items,
        stickerSearchLoading: false,
      };

    case "StickerSuggestKeywordsLoaded":
      return {
        ...state,
        stickerSuggestKeywords: event.payload.keywords,
      };

    case "ComposerStickerSuggestions":
      return {
        ...state,
        composerStickerSuggestions: event.payload.items,
      };

    case "LinkPreviewReady":
      return {
        ...state,
        linkMetadataCache: {
          ...state.linkMetadataCache,
          [event.payload.meta.url]: event.payload.meta,
        },
      };

    case "FileDownloaded":
      return {
        ...state,
        toast: nextToast(
          event.payload.ok
            ? `Đã lưu tệp: ${event.payload.file_name} (${event.payload.path})`
            : `Tải tệp thất bại: ${event.payload.error ?? "không xác định"}`,
        ),
      };
  }
}
