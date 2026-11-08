import { Download, RefreshCw } from "lucide-react";
import { ipc } from "@/shared/api/ipc";
import { Button } from "@/shared/ui/button";
import { RowDivider, SectionHeading, SettingCard, SwitchRow } from "../primitives";
import type { SettingsTabProps } from "../types";

export function UpdateTab({ state, settings, updateSettings, dispatch }: SettingsTabProps) {
  return (
    <>
      <SectionHeading>Cập nhật ứng dụng</SectionHeading>
      <SettingCard>
        <div className="flex items-center justify-between px-4 py-3">
          <div>
            <div className="text-sm font-medium text-foreground">AloLite v{state.appVersion}</div>
            <div className="text-xs text-muted-foreground">
              {state.updateChecking
                ? "Đang kiểm tra bản phát hành mới…"
                : state.updateInfo?.has_update
                  ? `Có bản mới v${state.updateInfo.latest_version}`
                  : "Kiểm tra cập nhật trực tiếp từ GitHub Releases"}
            </div>
          </div>
          <Button
            variant="default"
            size="sm"
            disabled={state.updateChecking}
            onClick={() => {
              dispatch({ type: "SET_UPDATE_CHECKING", checking: true });
              void ipc.checkForUpdates(false);
            }}
          >
            <RefreshCw className="h-3.5 w-3.5" /> Kiểm tra ngay
          </Button>
        </div>
        {state.updateInfo?.has_update && state.updateInfo.download_url ? (
          <>
            <RowDivider />
            <div className="px-4 py-3">
              <Button
                variant="default"
                disabled={state.updateDownloading}
                onClick={() =>
                  void ipc.downloadAndApplyUpdate(
                    state.updateInfo!.download_url!,
                    state.updateInfo!.asset_name ?? "alolite",
                    state.updateInfo!.latest_version,
                    state.updateInfo!.expected_sha256,
                  )
                }
              >
                <Download className="h-3.5 w-3.5" /> Tải và cài đặt v
                {state.updateInfo.latest_version}
              </Button>
            </div>
          </>
        ) : null}
        {state.updateDownloadMessage ? (
          <>
            <RowDivider />
            <div className="px-4 py-2 text-xs text-muted-foreground">
              {state.updateDownloadMessage}
            </div>
          </>
        ) : null}
      </SettingCard>

      <SectionHeading>Tự động cập nhật</SectionHeading>
      <SettingCard>
        <SwitchRow
          label="Tự động kiểm tra cập nhật"
          checked={settings.auto_check_updates}
          onChange={(v) => updateSettings({ auto_check_updates: v })}
        />
      </SettingCard>
    </>
  );
}
