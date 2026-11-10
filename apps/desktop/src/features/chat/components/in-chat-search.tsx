import { Search, X } from "lucide-react";
import { Button } from "@/shared/ui/button";

interface InChatSearchProps {
  isOpen: boolean;
  query: string;
  onOpen: () => void;
  onClose: () => void;
  onQueryChange: (query: string) => void;
}

export function InChatSearch({ isOpen, query, onOpen, onClose, onQueryChange }: InChatSearchProps) {
  if (isOpen) {
    return (
      <div className="flex h-8 w-56 items-center gap-2 rounded-md border border-border bg-muted px-2.5 focus-within:border-primary/80">
        <Search className="h-4 w-4 shrink-0 text-muted-foreground" />
        <input
          autoFocus
          value={query}
          onChange={(e) => onQueryChange(e.target.value)}
          placeholder="Tìm trong cuộc trò chuyện…"
          className="w-full bg-transparent text-sm text-foreground placeholder:text-muted-foreground focus:outline-none"
        />
        <button
          type="button"
          onClick={onClose}
          className="inline-flex h-4 w-4 cursor-pointer items-center justify-center rounded text-muted-foreground hover:text-foreground"
        >
          <X className="h-3.5 w-3.5" />
        </button>
      </div>
    );
  }

  return (
    <Button
      variant="ghost"
      size="icon"
      title="Tìm kiếm tin nhắn"
      className="h-8 w-8 text-muted-foreground hover:text-foreground"
      onClick={onOpen}
    >
      <Search className="h-5 w-5" />
    </Button>
  );
}
