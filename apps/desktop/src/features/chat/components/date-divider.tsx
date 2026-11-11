import { formatDayDivider } from "@/shared/utils/text";

export function DateDivider({ ts }: { ts: number }) {
  return (
    <div className="my-3 self-center rounded-xl bg-[rgba(255,255,255,0.05)] px-3 py-0.5 text-xs text-muted-foreground">
      {formatDayDivider(ts)}
    </div>
  );
}
