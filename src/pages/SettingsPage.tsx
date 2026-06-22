import { useEffect, useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { loadSettings, saveSettings, setHideInFullscreen } from "../lib/api";
import type { AppSettings } from "../lib/types";
import { DEFAULT_SETTINGS } from "../lib/types";

export default function SettingsPage() {
  const [settings, setSettings] = useState<AppSettings>(DEFAULT_SETTINGS);

  useEffect(() => {
    loadSettings().then(setSettings).catch(() => {});
  }, []);

  const handleClose = () => {
    try { getCurrentWindow().hide(); } catch {}
  };

  const update = async <K extends keyof AppSettings>(key: K, value: AppSettings[K]) => {
    const next = { ...settings, [key]: value };
    setSettings(next);
    await saveSettings(next);
    if (key === "hideFullscreen") setHideInFullscreen(value as boolean).catch(() => {});
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
        {/* Always on top */}
        <div className="settings-group">
          <div className="settings-row settings-row-between">
            <label className="settings-label">窗口置顶</label>
            <button className={`toggle ${settings.alwaysOnTop ? "toggle-on" : ""}`}
              onClick={() => update("alwaysOnTop", !settings.alwaysOnTop)}>
              <span className="toggle-knob" />
            </button>
          </div>
        </div>

        {/* Hide in fullscreen */}
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
