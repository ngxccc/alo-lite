import { useEffect, useState } from "react";
import { ipc, isTauriRuntime } from "@/shared/api/ipc";
import type { AutoReplyRule } from "@/shared/types/ipc";
import { Button } from "@/shared/ui/button";
import { Input } from "@/shared/ui/input";
import { Switch } from "@/shared/ui/switch";
import { cn } from "@/shared/utils/cn";
import { RowDivider, SectionHeading, SettingCard, SwitchRow } from "../primitives";
import type { SettingsTabProps } from "../types";

export function UtilitiesTab({ settings, updateSettings, dispatch }: SettingsTabProps) {
  const [autoReplyRules, setAutoReplyRules] = useState<AutoReplyRule[]>([]);
  const [newRuleKw, setNewRuleKw] = useState("");
  const [newRuleReply, setNewRuleReply] = useState("");

  useEffect(() => {
    if (!isTauriRuntime()) return;
    void ipc
      .getAutoreplyRules()
      .then((r) => setAutoReplyRules(Array.isArray(r) ? r : []))
      .catch(() => undefined);
  }, []);

  return (
    <>
      <SectionHeading>Gợi ý nhãn dán</SectionHeading>
      <SettingCard>
        <SwitchRow
          label="Hiển thị gợi ý sticker theo từ khóa đang soạn"
          checked={settings.suggest_stickers_while_typing}
          onChange={(v) => updateSettings({ suggest_stickers_while_typing: v })}
        />
      </SettingCard>

      <SectionHeading>Phím tắt @</SectionHeading>
      <SettingCard>
        {[
          {
            key: "suggest_mentions" as const,
            label: "Gợi ý nhắc tên (@mention)",
            desc: "Gợi ý mention khi soạn tin",
            color: "bg-[#e94949]",
            abbr: "@",
          },
          {
            key: "enable_gif_search" as const,
            label: "Tìm GIF",
            desc: "Nhập từ khóa để tìm GIF",
            color: "bg-[#2d7dd2]",
            abbr: "GIF",
          },
          {
            key: "enable_sticker_keyword_search" as const,
            label: "Tìm nhãn dán",
            desc: "Nhập từ khóa để tìm nhãn dán",
            color: "bg-[#8b5cf6]",
            abbr: ":)",
          },
        ].map((item, i) => (
          <div key={item.key}>
            {i > 0 && <RowDivider />}
            <div className="flex min-h-[52px] items-center justify-between gap-3 px-4 py-2">
              <div
                className={cn(
                  "flex h-8 w-8 shrink-0 items-center justify-center rounded-full text-[10px] font-bold text-white",
                  item.color,
                )}
              >
                {item.abbr}
              </div>
              <div className="min-w-0 flex-1">
                <div className="text-sm text-foreground">{item.label}</div>
                <div className="text-xs text-muted-foreground">{item.desc}</div>
              </div>
              <Switch
                checked={settings[item.key]}
                onCheckedChange={(v) => updateSettings({ [item.key]: v })}
                className="h-[22px] w-10 data-[state=checked]:bg-primary data-[state=unchecked]:bg-[#3c4149] [&>span]:h-[18px] [&>span]:w-[18px] [&>span]:data-[state=checked]:translate-x-[18px]"
              />
            </div>
          </div>
        ))}
      </SettingCard>

      <SectionHeading>Đa phiên bản</SectionHeading>
      <SettingCard>
        <div className="px-4 py-3">
          <div className="flex gap-1.5">
            <Input
              value={settings.multi_profile_name}
              onChange={(e) => updateSettings({ multi_profile_name: e.target.value })}
              placeholder="Tên profile phụ (vd. acc2)"
            />
            <Button
              variant="outline"
              onClick={() => void ipc.spawnMultiProfile(settings.multi_profile_name)}
            >
              Mở Profile mới
            </Button>
          </div>
        </div>
      </SettingCard>

      <SectionHeading>Tự động trả lời</SectionHeading>
      <SettingCard>
        <div className="px-4 py-3">
          {autoReplyRules.map((rule) => (
            <div
              key={rule.id}
              className="flex items-center justify-between border-b border-border py-2 last:border-0"
            >
              <span className="text-sm">
                <code className="rounded bg-muted px-1.5 py-0.5 text-xs">{rule.keyword}</code> →{" "}
                {rule.reply_text}
              </span>
              <Button
                variant="destructive"
                size="sm"
                onClick={async () => {
                  const next = autoReplyRules.filter((r) => r.id !== rule.id);
                  setAutoReplyRules(next);
                  await ipc.saveAutoreplyRules(next);
                }}
              >
                Xóa
              </Button>
            </div>
          ))}
          <div className="mt-2 flex gap-1.5">
            <Input
              value={newRuleKw}
              onChange={(e) => setNewRuleKw(e.target.value)}
              placeholder="Từ khóa (vd. /ping)"
            />
            <Input
              value={newRuleReply}
              onChange={(e) => setNewRuleReply(e.target.value)}
              placeholder="Nội dung trả lời…"
            />
            <Button
              variant="default"
              onClick={async () => {
                if (!newRuleKw.trim() || !newRuleReply.trim()) return;
                const next: AutoReplyRule[] = [
                  ...autoReplyRules,
                  {
                    id: `rule_${Date.now()}`,
                    keyword: newRuleKw.trim(),
                    match_mode: "contains",
                    reply_text: newRuleReply.trim(),
                    enabled: true,
                    scope: "direct",
                  },
                ];
                setAutoReplyRules(next);
                setNewRuleKw("");
                setNewRuleReply("");
                await ipc.saveAutoreplyRules(next);
              }}
            >
              Thêm
            </Button>
          </div>
        </div>
      </SettingCard>

      <SectionHeading>Nhãn dán tùy chỉnh</SectionHeading>
      <SettingCard>
        <div className="px-4 py-3">
          <Button
            variant="outline"
            size="sm"
            onClick={async () => {
              const p = await ipc.exportCustomStickersTemplate();
              dispatch({
                type: "SHOW_TOAST",
                message: `Đã xuất mẫu ${p}`,
              });
            }}
          >
            Xuất mẫu custom_stickers.json
          </Button>
        </div>
      </SettingCard>
    </>
  );
}
