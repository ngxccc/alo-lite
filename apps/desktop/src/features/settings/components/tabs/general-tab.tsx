import { FolderOpen } from "lucide-react";
import { ipc } from "@/shared/api/ipc";
import { Button } from "@/shared/ui/button";
import { RowDivider, SectionHeading, SelectRow, SettingCard, SwitchRow } from "../primitives";
import type { SettingsTabProps } from "../types";

export function GeneralTab({ settings, updateSettings }: SettingsTabProps) {
  return (
    <>
      <SectionHeading>Ngôn ngữ</SectionHeading>
      <SettingCard>
        <SelectRow
          label="Thay đổi ngôn ngữ"
          value={settings.language}
          onChange={(v) => updateSettings({ language: v })}
          options={[
            { value: "Tiếng Việt", label: "Tiếng Việt" },
            { value: "English", label: "English" },
          ]}
        />
      </SettingCard>

      <SectionHeading>Khởi động</SectionHeading>
      <SettingCard>
        <SwitchRow
          label="Khởi động AloLite cùng hệ thống"
          checked={settings.autostart_on_boot}
          onChange={(v) => updateSettings({ autostart_on_boot: v })}
        />
        <RowDivider />
        <SwitchRow
          label="Ghi nhớ tài khoản đăng nhập"
          checked={settings.remember_login}
          onChange={(v) => updateSettings({ remember_login: v })}
        />
        <RowDivider />
        <SwitchRow
          label="Hiển thị tất cả danh bạ"
          checked={settings.show_all_contacts}
          onChange={(v) => updateSettings({ show_all_contacts: v })}
        />
      </SettingCard>

      <SectionHeading>Thư mục tải xuống</SectionHeading>
      <SettingCard>
        <div className="px-4 py-3">
          <div className="mb-2 text-xs text-muted-foreground">
            Lưu tại:{" "}
            <code className="rounded bg-muted px-1.5 py-0.5 text-foreground">
              {settings.download_dir}
            </code>
          </div>
          <div className="flex gap-2">
            <Button
              variant="outline"
              size="sm"
              onClick={async () => {
                const dir = await ipc.pickFile({
                  directory: true,
                  title: "Chọn thư mục lưu file",
                });
                if (dir) updateSettings({ download_dir: dir });
              }}
            >
              <FolderOpen className="h-3.5 w-3.5" /> Thay đổi thư mục
            </Button>
            <Button
              variant="outline"
              size="sm"
              onClick={() => void ipc.openPathInOs(settings.download_dir)}
            >
              Mở thư mục
            </Button>
          </div>
        </div>
      </SettingCard>
    </>
  );
}
