import { useEffect, useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import {
  checkForUpdates,
  loadSettings,
  saveSettings,
  setHideInFullscreen,
  setStartOnBoot,
  type UpdateCheckResult,
} from "../lib/api";
import type { AppSettings } from "../lib/types";
import { DEFAULT_SETTINGS } from "../lib/types";

export default function SettingsPage() {
  const [settings, setSettings] = useState<AppSettings>(DEFAULT_SETTINGS);
  const [updateInfo, setUpdateInfo] = useState<UpdateCheckResult | null>(null);
  const [updateStatus, setUpdateStatus] = useState<"idle" | "checking" | "error">("idle");
  const [updateError, setUpdateError] = useState<string | null>(null);

  const refreshUpdateInfo = async () => {
    setUpdateStatus("checking");
    setUpdateError(null);
    try {
      setUpdateInfo(await checkForUpdates());
      setUpdateStatus("idle");
    } catch (error) {
      setUpdateStatus("error");
      setUpdateError(error instanceof Error ? error.message : String(error));
    }
  };

  useEffect(() => {
    loadSettings().then(setSettings).catch(() => {});
    refreshUpdateInfo();
  }, []);

  const handleClose = () => {
    try { getCurrentWindow().hide(); } catch {}
  };

  const openReleasePage = () => {
    if (!updateInfo?.releaseUrl) return;
    window.open(updateInfo.releaseUrl, "_blank", "noopener,noreferrer");
  };

  const update = async <K extends keyof AppSettings>(key: K, value: AppSettings[K]) => {
    const next = { ...settings, [key]: value };
    setSettings(next);
    await saveSettings(next);
    if (key === "hideFullscreen") setHideInFullscreen(value as boolean).catch(() => {});
    if (key === "startOnBoot") setStartOnBoot(value as boolean).catch(() => {});
  };

  return (
    <div className="manage-page">
      <header className="manage-header">
        <h2>⚙️ 全局设置</h2>
        <div className="manage-header-actions">
          <button className="btn btn-close" onClick={handleClose} title="关闭">&#10005;</button>
        </div>
      </header>

      <div className="manage-content settings-page-body">
        <div className="settings-group update-check-card">
          <div className="settings-row settings-row-between">
            <div>
              <div className="settings-label">软件更新</div>
              <div className="settings-sublabel">
                当前版本 {updateInfo?.currentVersion ?? "检测中"}
              </div>
            </div>
            <button className="btn btn-sm" onClick={refreshUpdateInfo} disabled={updateStatus === "checking"}>
              {updateStatus === "checking" ? "检查中..." : "检查更新"}
            </button>
          </div>

          {updateInfo?.hasUpdate && (
            <div className="update-notice update-notice-new">
              <div>
                发现新版本 {updateInfo.latestTag ?? updateInfo.latestVersion}
                {updateInfo.releaseName ? `：${updateInfo.releaseName}` : ""}
              </div>
              <button className="btn btn-sm" onClick={openReleasePage}>查看发布页</button>
            </div>
          )}

          {updateInfo && !updateInfo.hasUpdate && (
            <div className="update-notice">已是最新版本</div>
          )}

          {updateStatus === "error" && (
            <div className="update-notice update-notice-error">
              检查更新失败：{updateError}
            </div>
          )}
        </div>

        <div className="settings-group">
          <div className="settings-row settings-row-between">
            <label className="settings-label">窗口置顶</label>
            <button className={`toggle ${settings.alwaysOnTop ? "toggle-on" : ""}`}
              onClick={() => update("alwaysOnTop", !settings.alwaysOnTop)}>
              <span className="toggle-knob" />
            </button>
          </div>
        </div>

        <div className="settings-group">
          <div className="settings-row settings-row-between">
            <label className="settings-label">开机自启动</label>
            <button className={`toggle ${settings.startOnBoot ? "toggle-on" : ""}`}
              onClick={() => update("startOnBoot", !settings.startOnBoot)}>
              <span className="toggle-knob" />
            </button>
          </div>
        </div>

        <div className="settings-group">
          <div className="settings-row settings-row-between">
            <label className="settings-label">全屏时隐藏挂件</label>
            <button className={`toggle ${settings.hideFullscreen ? "toggle-on" : ""}`}
              onClick={() => update("hideFullscreen", !settings.hideFullscreen)}>
              <span className="toggle-knob" />
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}
