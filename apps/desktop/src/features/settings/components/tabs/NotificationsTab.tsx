import { cn } from "@/shared/utils/cn";
import { NotifThumbnail, SectionHeading, SettingCard, SwitchRow } from "../primitives";
import type { SettingsTabProps } from "../types";

export function NotificationsTab({ settings, updateSettings }: SettingsTabProps) {
  return (
    <>
      <SectionHeading>Bật thông báo</SectionHeading>
      <div className="mb-1 text-xs text-muted-foreground">
        Hiển thị thông báo khi có tin nhắn mới
      </div>
      <SettingCard>
        <div className="px-4 py-4">
          <div className="grid grid-cols-2 gap-4">
            {[
              { active: true, label: "Bật" },
              { active: false, label: "Tắt" },
            ].map((opt) => {
              const selected = settings.enable_notifications === opt.active;
              return (
                <button
                  key={opt.label}
                  type="button"
                  onClick={() => updateSettings({ enable_notifications: opt.active })}
                  className={cn(
                    "flex flex-col items-center gap-3 rounded-lg border-2 py-4 transition-colors",
                    selected ? "border-primary" : "border-border hover:border-muted-foreground",
                  )}
                >
                  <NotifThumbnail active={opt.active} />
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

      <SectionHeading>Âm thanh thông báo</SectionHeading>
      <SettingCard>
        <SwitchRow
          label="Bật âm thanh cho tin nhắn &amp; thông báo mới"
          checked={settings.play_notification_sound}
          onChange={(v) => updateSettings({ play_notification_sound: v })}
        />
      </SettingCard>
    </>
  );
}
