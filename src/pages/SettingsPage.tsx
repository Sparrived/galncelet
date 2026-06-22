import { useEffect, useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import {
  loadSettings, saveSettings,
  setHideInFullscreen, selectFolder, getStatus, updateCardWidth,
} from "../lib/api";
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
    if (key === "cardWidth") updateCardWidth(value as number);
    if (key === "hideFullscreen") setHideInFullscreen(value as boolean).catch(() => {});
  };

  const handleSelectFolder = async () => {
    const path = await selectFolder();
    if (path) { try { await getStatus(path); } catch {} }
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
        {/* Repo selector */}
        <div className="settings-group">
          <button className="btn-action btn-select-folder" onClick={handleSelectFolder}>
            &#128193; 选择仓库
          </button>
        </div>

        {/* Refresh interval */}
        <div className="settings-group">
          <label className="settings-label">自动刷新间隔</label>
          <div className="settings-row">
            <input className="settings-slider" type="range" min={500} max={10000} step={500}
              value={settings.refreshIntervalMs}
              onChange={(e) => update("refreshIntervalMs", Number(e.target.value))}
            />
            <span className="settings-value">{settings.refreshIntervalMs}ms</span>
          </div>
        </div>

        {/* Card width */}
        <div className="settings-group">
          <label className="settings-label">卡片宽度</label>
          <div className="settings-row">
            <input className="settings-slider" type="range" min={300} max={600} step={10}
              value={settings.cardWidth}
              onChange={(e) => update("cardWidth", Number(e.target.value))}
            />
            <span className="settings-value">{settings.cardWidth}px</span>
          </div>
        </div>

        {/* Log max count */}
        <div className="settings-group">
          <label className="settings-label">提交历史最大条数</label>
          <div className="settings-row">
            <input className="settings-number" type="number" min={10} max={500}
              value={settings.logMaxCount}
              onChange={(e) => update("logMaxCount", Math.max(10, Math.min(500, Number(e.target.value) || 10)))}
            />
          </div>
        </div>

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

        {/* Pull rebase */}
        <div className="settings-group">
          <div className="settings-row settings-row-between">
            <label className="settings-label">Pull 使用 Rebase</label>
            <button className={`toggle ${settings.pullRebase ? "toggle-on" : ""}`}
              onClick={() => update("pullRebase", !settings.pullRebase)}>
              <span className="toggle-knob" />
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}
