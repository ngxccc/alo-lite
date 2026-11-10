import { cn } from "@/shared/utils/cn";
import { SectionHeading, SelectRow, SettingCard, SwitchRow, ThemeThumbnail } from "../primitives";
import type { SettingsTabProps } from "../types";

export function AppearanceTab({ settings, updateSettings }: SettingsTabProps) {
  return (
    <>
      <SectionHeading>Chủ đề giao diện</SectionHeading>
      <SettingCard>
        <div className="px-4 py-4">
          <div className="grid grid-cols-3 gap-3">
            {(
              [
                { id: "light" as const, label: "Sáng" },
                { id: "dark" as const, label: "Tối" },
                { id: "system" as const, label: "Hệ thống" },
              ] as const
            ).map((opt) => {
              const selected = settings.theme_mode === opt.id;
              return (
                <button
                  key={opt.id}
                  type="button"
                  onClick={() => updateSettings({ theme_mode: opt.id })}
                  className={cn(
                    "flex flex-col gap-2 rounded-lg border-2 p-2 transition-colors",
                    selected ? "border-primary" : "border-border hover:border-muted-foreground",
                  )}
                >
                  <ThemeThumbnail mode={opt.id} />
                  <div className="flex items-center gap-1.5">
                    <div
                      className={cn(
                        "h-3.5 w-3.5 rounded-full border-2",
                        selected ? "border-primary bg-primary" : "border-muted-foreground",
                      )}
                    />
                    <span className="text-xs text-foreground">{opt.label}</span>
                  </div>
                </button>
              );
            })}
          </div>
        </div>
      </SettingCard>

      <SectionHeading>Cỡ chữ & Hiển thị</SectionHeading>
      <SettingCard>
        <SelectRow
          label="Cỡ chữ toàn ứng dụng"
          value={settings.font_size}
          onChange={(v) => updateSettings({ font_size: v })}
          options={[
            { value: "small", label: "Nhỏ" },
            { value: "medium", label: "Tiêu chuẩn" },
            { value: "large", label: "Lớn" },
          ]}
        />
      </SettingCard>

      <SectionHeading>Nền trò chuyện</SectionHeading>
      <SettingCard>
        <SwitchRow
          label="Sử dụng ảnh đại diện làm nền mờ"
          checked={settings.use_avatar_as_wallpaper}
          onChange={(v) => updateSettings({ use_avatar_as_wallpaper: v })}
        />
      </SettingCard>
    </>
  );
}
