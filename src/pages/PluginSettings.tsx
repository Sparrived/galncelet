import { useEffect, useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { getPlugin } from "../addons/registry";
import {
  loadSettings, saveSettings, saveWindowState,
  setAttachWhitelist, selectFolder, getStatus, updateCardWidth,
} from "../lib/api";
import type { AppSettings } from "../lib/types";
import { DEFAULT_SETTINGS } from "../lib/types";

interface Props {
  pluginId: string;
}

export default function PluginSettings({ pluginId }: Props) {
  const [settings, setSettings] = useState<AppSettings>(DEFAULT_SETTINGS);
  const plugin = getPlugin(pluginId);

  useEffect(() => {
    loadSettings().then(setSettings).catch(() => {});
  }, []);

  const updateSettings = async (patch: Partial<AppSettings>) => {
    const newSettings = { ...settings, ...patch };
    await saveSettings(newSettings);
    setSettings(newSettings);
    if (patch.cardWidth) updateCardWidth(patch.cardWidth);
  };

  const whitelist = settings.windowStates[pluginId]?.whitelist ?? [];

  const updateWhitelist = async (updated: string[]) => {
    const ws = { ...(settings.windowStates[pluginId] || {}), whitelist: updated };
    const newSettings = { ...settings, windowStates: { ...settings.windowStates, [pluginId]: ws } };
    await saveSettings(newSettings);
    await saveWindowState(pluginId, ws);
    await setAttachWhitelist(`widget-${pluginId}`, updated);
    setSettings(newSettings);
  };

  const handleSelectFolder = async () => {
    const path = await selectFolder();
    if (path) { try { await getStatus(path); } catch {} }
  };

  const handleClose = () => {
    try { getCurrentWindow().close(); } catch {}
  };

  if (!plugin) {
    return (
      <div className="plugin-settings-page">
        <header className="settings-page-header">
          <span>未找到插件</span>
          <button className="btn btn-close" onClick={handleClose}>&#10005;</button>
        </header>
      </div>
    );
  }

  return (
    <div className="plugin-settings-page">
      <header className="settings-page-header">
        <span className="settings-page-title">
          <span>{plugin.icon || "📦"}</span>
          <span>{plugin.title} 设置</span>
        </span>
        <button className="btn btn-close" onClick={handleClose} title="关闭">&#10005;</button>
      </header>

      <div className="settings-page-body">
        {/* Git plugin settings */}
        {pluginId === "git" && (
          <>
            <div className="settings-group">
              <button className="btn-action btn-select-folder" onClick={handleSelectFolder}>
                &#128193; 选择仓库
              </button>
            </div>
            <div className="settings-group">
              <label className="settings-label">自动刷新间隔</label>
              <div className="settings-row">
                <input className="settings-slider" type="range" min={500} max={10000} step={500}
                  value={settings.refreshIntervalMs}
                  onChange={(e) => updateSettings({ refreshIntervalMs: Number(e.target.value) })}
                />
                <span className="settings-value">{settings.refreshIntervalMs}ms</span>
              </div>
            </div>
            <div className="settings-group">
              <label className="settings-label">提交历史最大条数</label>
              <div className="settings-row">
                <input className="settings-number" type="number" min={10} max={500}
                  value={settings.logMaxCount}
                  onChange={(e) => updateSettings({ logMaxCount: Math.max(10, Math.min(500, Number(e.target.value) || 10)) })}
                />
              </div>
            </div>
            <div className="settings-group">
              <div className="settings-row settings-row-between">
                <label className="settings-label">Pull 使用 Rebase</label>
                <button className={`toggle ${settings.pullRebase ? "toggle-on" : ""}`}
                  onClick={() => updateSettings({ pullRebase: !settings.pullRebase })}>
                  <span className="toggle-knob" />
                </button>
              </div>
            </div>
            <div className="settings-group">
              <label className="settings-label">卡片宽度</label>
              <div className="settings-row">
                <input className="settings-slider" type="range" min={300} max={600} step={10}
                  value={settings.cardWidth}
                  onChange={(e) => updateSettings({ cardWidth: Number(e.target.value) })}
                />
                <span className="settings-value">{settings.cardWidth}px</span>
              </div>
            </div>
          </>
        )}

        {/* Whitelist editor (for plugins with attach) */}
        {plugin.showAttachButton !== false && (
          <div className="settings-group">
            <label className="settings-label">吸附白名单</label>
            <div className="settings-sublabel">留空=对全部窗口启用吸附</div>
            <div className="manage-whitelist-list">
              {whitelist.map((pattern, i) => (
                <div key={i} className="manage-whitelist-item">
                  <span className="manage-whitelist-pattern">{pattern}</span>
                  <button className="manage-whitelist-remove"
                    onClick={() => updateWhitelist(whitelist.filter((_, j) => j !== i))}>&#10005;</button>
                </div>
              ))}
              {whitelist.length === 0 && (
                <div className="manage-whitelist-empty">无限制</div>
              )}
            </div>
            <div className="manage-whitelist-add">
              <input className="manage-whitelist-input" type="text"
                placeholder="窗口标题关键词…"
                onKeyDown={(e) => {
                  if (e.key === "Enter") {
                    const v = (e.target as HTMLInputElement).value.trim();
                    if (v) { updateWhitelist([...whitelist, v]); (e.target as HTMLInputElement).value = ""; }
                  }
                }}
              />
            </div>
          </div>
        )}

        {pluginId !== "git" && plugin.showAttachButton === false && (
          <div className="manage-whitelist-empty" style={{ padding: "20px 0" }}>
            该插件暂无专属设置
          </div>
        )}
      </div>
    </div>
  );
}
