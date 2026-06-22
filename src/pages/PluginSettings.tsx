import { useEffect, useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { getPlugin } from "../addons/registry";
import {
  loadSettings, saveSettings, saveWindowState,
  setAttachWhitelist,
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

  const whitelist = settings.windowStates[pluginId]?.whitelist ?? [];

  const updateWhitelist = async (updated: string[]) => {
    const ws = { ...(settings.windowStates[pluginId] || {}), whitelist: updated };
    const newSettings = { ...settings, windowStates: { ...settings.windowStates, [pluginId]: ws } };
    await saveSettings(newSettings);
    await saveWindowState(pluginId, ws);
    await setAttachWhitelist(`widget-${pluginId}`, updated);
    setSettings(newSettings);
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

        {plugin.showAttachButton === false && (
          <div className="manage-whitelist-empty" style={{ padding: "20px 0" }}>
            该插件暂无专属设置
          </div>
        )}
      </div>
    </div>
  );
}
