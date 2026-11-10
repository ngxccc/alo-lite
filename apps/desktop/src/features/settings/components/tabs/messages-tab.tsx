import { ChevronRight } from "lucide-react";
import {
  ChevronRow,
  RowDivider,
  SectionHeading,
  SelectRow,
  SettingCard,
  SwitchRow,
} from "../primitives";
import type { SettingsTabProps } from "../types";

export function MessagesTab({ settings, updateSettings }: SettingsTabProps) {
  return (
    <>
      <SectionHeading>Tin nhắn nhanh</SectionHeading>
      <div className="mb-1 text-xs text-muted-foreground">
        Tạo, chỉnh sửa và quản lý phím tắt cho tin nhắn thường dùng
      </div>
      <SettingCard>
        <SwitchRow
          label="Tin nhắn nhanh"
          checked={settings.quick_messages_enabled}
          onChange={(v) => updateSettings({ quick_messages_enabled: v })}
        />
        <RowDivider />
        <ChevronRow label="Quản lý tin nhắn nhanh" />
      </SettingCard>

      <SectionHeading>Ẩn trò chuyện</SectionHeading>
      <div className="mb-1 text-xs text-muted-foreground">Đặt mã PIN để ẩn trò chuyện riêng tư</div>
      <SettingCard>
        <div className="flex min-h-[44px] items-center justify-between px-4 py-2">
          <span className="text-sm text-foreground">PIN</span>
          <span className="flex items-center gap-1 text-sm text-muted-foreground">
            {settings.hidden_chat_pin_enabled ? "Đã bật" : "Chưa bật"}
            <ChevronRight className="h-4 w-4" />
          </span>
        </div>
      </SettingCard>

      <SectionHeading>Tách Quan trọng và Khác</SectionHeading>
      <div className="mb-1 text-xs text-muted-foreground">
        Tách các cuộc trò chuyện phân tâm sang tab Khác để tập trung hơn
      </div>
      <SettingCard>
        <SwitchRow
          label="Phân tách hộp thư Ưu tiên / Khác"
          checked={settings.split_priority_inbox}
          onChange={(v) => updateSettings({ split_priority_inbox: v })}
        />
      </SettingCard>

      <SectionHeading>Hiển thị nội dung file Excel</SectionHeading>
      <div className="mb-1 text-xs text-muted-foreground">
        Sao chép định dạng Excel sau khi dán vào Zalo
      </div>
      <SettingCard>
        <div className="flex flex-col">
          {[
            { value: "photo", label: "Hình ảnh" },
            { value: "text", label: "Văn bản" },
            { value: "ask", label: "Luôn hỏi" },
          ].map((opt, i) => (
            <div key={opt.value}>
              {i > 0 && <RowDivider />}
              <label className="flex min-h-[44px] cursor-pointer items-center gap-3 px-4 py-2">
                <input
                  type="radio"
                  name="excel_paste_mode"
                  value={opt.value}
                  checked={settings.excel_paste_mode === opt.value}
                  onChange={() => updateSettings({ excel_paste_mode: opt.value })}
                  className="h-4 w-4 accent-primary"
                />
                <span className="text-sm text-foreground">{opt.label}</span>
              </label>
            </div>
          ))}
        </div>
      </SettingCard>

      <SectionHeading>Cài đặt khác</SectionHeading>
      <SettingCard>
        <SwitchRow
          label="Nhấp đúp vào tin nhắn để trả lời"
          checked={settings.double_click_to_reply}
          onChange={(v) => updateSettings({ double_click_to_reply: v })}
        />
        <RowDivider />
        <SwitchRow
          label="Hiển thị trạng thái đang soạn tin"
          checked={settings.show_typing_indicator}
          onChange={(v) => updateSettings({ show_typing_indicator: v })}
        />
        <RowDivider />
        <SelectRow
          label="Phím gửi tin nhắn"
          value={settings.send_message_key}
          onChange={(v) =>
            updateSettings({
              send_message_key: v,
            })
          }
          options={[
            { value: "enter", label: "Enter để gửi" },
            { value: "cmd_enter", label: "Ctrl/Cmd + Enter để gửi" },
          ]}
        />
      </SettingCard>
    </>
  );
}
