import {
  ChevronRow,
  RowDivider,
  SectionHeading,
  SelectRow,
  SettingCard,
  SwitchRow,
} from "../primitives";
import type { SettingsTabProps } from "../types";

export function PrivacyTab({ settings, updateSettings }: SettingsTabProps) {
  return (
    <>
      <SectionHeading>Thông tin cá nhân</SectionHeading>
      <SettingCard>
        <SelectRow
          label="Hiển thị ngày sinh"
          value={settings.show_birthday}
          onChange={(v) => updateSettings({ show_birthday: v })}
          options={[
            { value: "hide", label: "Không hiển thị" },
            { value: "friends", label: "Bạn bè" },
            { value: "everyone", label: "Tất cả mọi người" },
          ]}
        />
        <RowDivider />
        <SwitchRow
          label="Hiển thị trạng thái truy cập"
          checked={settings.show_online_status}
          onChange={(v) => updateSettings({ show_online_status: v })}
        />
      </SettingCard>

      <SectionHeading>Tin nhắn và cuộc gọi</SectionHeading>
      <SettingCard>
        <SwitchRow
          label='Hiển thị trạng thái "Đã xem"'
          checked={settings.show_seen_status}
          onChange={(v) => updateSettings({ show_seen_status: v, block_seen: !v })}
        />
        <RowDivider />
        <SelectRow
          label="Cho phép nhắn tin"
          value={settings.allow_message_from}
          onChange={(v) => updateSettings({ allow_message_from: v })}
          options={[
            { value: "everyone", label: "Tất cả mọi người" },
            { value: "friends", label: "Bạn bè" },
            { value: "none", label: "Không ai" },
          ]}
        />
        <RowDivider />
        <SelectRow
          label="Cho phép gọi điện"
          value={settings.allow_call_from}
          onChange={(v) => updateSettings({ allow_call_from: v })}
          options={[
            { value: "everyone", label: "Tất cả mọi người" },
            { value: "friends_strangers", label: "Bạn bè và người lạ bạn biết" },
            { value: "friends", label: "Bạn bè" },
          ]}
        />
        <RowDivider />
        <SwitchRow
          label='Ẩn trạng thái "Đang soạn tin"'
          desc="Chặn gửi gói typing đến người khác."
          checked={settings.block_typing}
          onChange={(v) => updateSettings({ block_typing: v })}
        />
        <RowDivider />
        <SwitchRow
          label='Ẩn trạng thái "Đã nhận"'
          desc="Chặn gửi xác nhận đã nhận tin."
          checked={settings.block_delivered}
          onChange={(v) => updateSettings({ block_delivered: v })}
        />
      </SettingCard>

      <SectionHeading>Chặn tin nhắn</SectionHeading>
      <SettingCard>
        <ChevronRow label="Chặn tin nhắn" desc="Quản lý danh sách chặn" />
      </SettingCard>

      <SectionHeading>Nguồn tìm kiếm</SectionHeading>
      <SettingCard>
        <SwitchRow
          label="Cho phép tìm kiếm qua số điện thoại"
          checked={settings.allow_search_by_phone}
          onChange={(v) => updateSettings({ allow_search_by_phone: v })}
        />
      </SettingCard>

      <SectionHeading>Cho phép người lạ kết bạn</SectionHeading>
      <SettingCard>
        <div className="flex flex-col gap-0">
          {[
            { key: "friend_src_qr" as const, label: "Qua mã QR của tôi" },
            { key: "friend_src_group" as const, label: "Từ nhóm chung" },
            { key: "friend_src_card" as const, label: "Qua thẻ danh thiếp Zalo" },
          ].map((item, i) => (
            <div key={item.key}>
              {i > 0 && <RowDivider />}
              <label className="flex min-h-[44px] cursor-pointer items-center gap-3 px-4 py-2">
                <input
                  type="checkbox"
                  checked={settings[item.key]}
                  onChange={(e) => updateSettings({ [item.key]: e.target.checked })}
                  className="h-4 w-4 rounded border-border accent-primary"
                />
                <span className="text-sm text-foreground">{item.label}</span>
              </label>
            </div>
          ))}
        </div>
      </SettingCard>
    </>
  );
}
