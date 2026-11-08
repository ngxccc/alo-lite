import type React from "react";
import { ChevronRight } from "lucide-react";
import { Switch } from "@/shared/ui/switch";
import { cn } from "@/shared/utils/cn";

/** Zalo-style row: label+optional desc on left, toggle on right */
export function SwitchRow({
  label,
  desc,
  checked,
  onChange,
}: {
  label: string;
  desc?: string;
  checked: boolean;
  onChange: (v: boolean) => void;
}) {
  return (
    <div className="flex min-h-[44px] items-center justify-between gap-4 px-4 py-2">
      <div className="min-w-0 flex-1">
        <div className="text-sm text-foreground">{label}</div>
        {desc ? <div className="mt-0.5 text-xs text-muted-foreground">{desc}</div> : null}
      </div>
      {/* 40x22 toggle */}
      <Switch
        checked={checked}
        onCheckedChange={onChange}
        className="h-[22px] w-10 data-[state=checked]:bg-primary data-[state=unchecked]:bg-[#3c4149] [&>span]:h-[18px] [&>span]:w-[18px] [&>span]:data-[state=checked]:translate-x-[18px]"
      />
    </div>
  );
}

/** Select row */
export function SelectRow({
  label,
  value,
  onChange,
  options,
}: {
  label: string;
  value: string;
  onChange: (v: string) => void;
  options: { value: string; label: string }[];
}) {
  return (
    <div className="flex min-h-[44px] items-center justify-between gap-4 px-4 py-2">
      <span className="text-sm text-foreground">{label}</span>
      <select
        value={value}
        onChange={(e) => onChange(e.target.value)}
        className="h-[38px] min-w-[160px] rounded-[6px] border border-border bg-canvas px-3 text-sm text-foreground focus:outline-none"
      >
        {options.map((o) => (
          <option key={o.value} value={o.value}>
            {o.label}
          </option>
        ))}
      </select>
    </div>
  );
}

/** Chevron-only nav row (block messages, manage quick messages, etc.) */
export function ChevronRow({ label, desc }: { label: string; desc?: string }) {
  return (
    <div className="flex min-h-[44px] cursor-pointer items-center justify-between gap-4 px-4 py-2 hover:bg-white/[0.03]">
      <div className="min-w-0 flex-1">
        <div className="text-sm text-foreground">{label}</div>
        {desc ? <div className="mt-0.5 text-xs text-muted-foreground">{desc}</div> : null}
      </div>
      <ChevronRight className="h-4 w-4 shrink-0 text-muted-foreground" />
    </div>
  );
}

/** Section heading (not inside a card) */
export function SectionHeading({ children }: { children: React.ReactNode }) {
  return (
    <div className="mt-4 mb-2 text-base font-medium text-foreground first:mt-0">{children}</div>
  );
}

/** Card wrapper */
export function SettingCard({
  children,
  className,
}: {
  children: React.ReactNode;
  className?: string;
}) {
  return <div className={cn("overflow-hidden rounded-lg bg-card", className)}>{children}</div>;
}

/** Divider between rows inside card */
export function RowDivider() {
  return <div className="mx-4 h-px bg-border" />;
}

/** CSS mini-mockup for theme picker */
export function ThemeThumbnail({ mode }: { mode: "light" | "dark" | "system" }) {
  if (mode === "light") {
    return (
      <div className="relative h-[72px] w-full overflow-hidden rounded bg-[#ebecf0]">
        {/* sidebar strip */}
        <div className="absolute inset-y-0 left-0 w-[30%] bg-white" />
        {/* chat bubble out */}
        <div className="absolute right-3 bottom-3 h-[12px] w-[45%] rounded-sm bg-[#e5f1ff]" />
        {/* chat bubble in */}
        <div className="absolute bottom-7 left-[32%] h-[12px] w-[40%] rounded-sm bg-white" />
        {/* header bar */}
        <div className="absolute inset-x-0 top-0 h-[14px] bg-white" />
      </div>
    );
  }
  if (mode === "dark") {
    return (
      <div className="relative h-[72px] w-full overflow-hidden rounded bg-[#16191d]">
        <div className="absolute inset-y-0 left-0 w-[30%] bg-[#22262b]" />
        <div className="absolute right-3 bottom-3 h-[12px] w-[45%] rounded-sm bg-[#1f344d]" />
        <div className="absolute bottom-7 left-[32%] h-[12px] w-[40%] rounded-sm bg-[#282d33]" />
        <div className="absolute inset-x-0 top-0 h-[14px] bg-[#22262b]" />
      </div>
    );
  }
  // system: half-and-half
  return (
    <div className="relative h-[72px] w-full overflow-hidden rounded">
      {/* left half dark */}
      <div className="absolute inset-y-0 left-0 w-1/2 bg-[#16191d]">
        <div className="absolute inset-y-0 left-0 w-[35%] bg-[#22262b]" />
        <div className="absolute inset-x-0 top-0 h-[14px] bg-[#22262b]" />
      </div>
      {/* right half light */}
      <div className="absolute inset-y-0 right-0 w-1/2 bg-[#ebecf0]">
        <div className="absolute inset-x-0 top-0 h-[14px] bg-white" />
        <div className="absolute right-2 bottom-3 h-[10px] w-[55%] rounded-sm bg-[#e5f1ff]" />
      </div>
    </div>
  );
}

/** CSS mini-mockup for notification toggle (laptop outline) */
export function NotifThumbnail({ active }: { active: boolean }) {
  return (
    <div
      className={cn(
        "relative mx-auto flex h-[60px] w-[80px] flex-col items-center justify-center rounded border-2",
        active ? "border-primary" : "border-border",
      )}
    >
      {/* laptop screen */}
      <div
        className={cn(
          "h-[32px] w-[56px] rounded-t border-2",
          active ? "border-primary" : "border-muted-foreground",
        )}
      >
        {active && <div className="mt-1 ml-1 h-2 w-6 rounded-sm bg-primary/40" />}
      </div>
      {/* laptop base */}
      <div
        className={cn(
          "h-[6px] w-[64px] rounded-b border-x-2 border-b-2",
          active ? "border-primary" : "border-muted-foreground",
        )}
      />
    </div>
  );
}
