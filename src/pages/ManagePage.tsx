import { useEffect, useState } from "react";
import { getCurrentWindow, WebviewWindow } from "@tauri-apps/api/window";
import { getAllPlugins, type PluginDef } from "../addons/registry";
import {
  loadSettings, saveSettings, saveWindowState,
  setAttachWhitelist, createPluginWindow, listVisibleWindows,
  selectFolder, getStatus, updateCardWidth,
  type WindowEntry,
} from "../lib/api";
import type { AppSettings } from "../lib/types";
import { DEFAULT_SETTINGS } from "../lib/types";

export default function ManagePage() {
  const [settings, setSettings] = useState<AppSettings>(DEFAULT_SETTINGS);
  const [activePlugin, setActivePlugin] = useState<string | null>(null);
  const [visibleWindows, setVisibleWindows] = useState<WindowEntry[]>([]);
  const [scanningWindows, setScanningWindows] = useState(false);
  const plugins = getAllPlugins();

  useEffect(() => {
    loadSettings().then(setSettings).catch(() => {});
  }, []);

  const handleClose = () => {
    try { getCurrentWindow().close(); } catch {}
  };

  const updateSettings = async (patch: Partial<AppSettings>) => {
    const newSettings = { ...settings, ...patch };
    await saveSettings(newSettings);
    setSettings(newSettings);
    if (patch.cardWidth) updateCardWidth(patch.cardWidth);
  };

  const togglePlugin = async (plugin: PluginDef) => {
    const vis = { ...settings.panelVisibility };
    const enabling = vis[plugin.id] === false;
    vis[plugin.id] = enabling;
    await updateSettings({ panelVisibility: vis });
    if (enabling) {
      await createPluginWindow(
        plugin.id, plugin.title,
        plugin.defaultWidth ?? 360, plugin.defaultHeight ?? 600,
        plugin.defaultAttachEnabled !== false,
        plugin.defaultAttachRemember === true,
        plugin.defaultWhitelist ?? [],
      );
    } else {
      const win = WebviewWindow.getByLabel(`widget-${plugin.id}`);
      if (win) { try { await win.hide(); } catch {} }
    }
  };

  const openPlugin = async (plugin: PluginDef) => {
    await createPluginWindow(
      plugin.id, plugin.title,
      plugin.defaultWidth ?? 360, plugin.defaultHeight ?? 600,
      plugin.defaultAttachEnabled !== false,
      plugin.defaultAttachRemember === true,
      plugin.defaultWhitelist ?? [],
    );
  };

  const getWhitelist = (id: string): string[] =>
    settings.windowStates[id]?.whitelist ?? [];

  const updateWhitelist = async (id: string, updated: string[]) => {
    const ws = { ...(settings.windowStates[id] || {}), whitelist: updated };
    const newSettings = { ...settings, windowStates: { ...settings.windowStates, [id]: ws } };
    await saveSettings(newSettings);
    await saveWindowState(id, ws);
    await setAttachWhitelist(`widget-${id}`, updated);
    setSettings(newSettings);
  };

  const handleSelectFolder = async () => {
    const path = await selectFolder();
    if (path) { try { await getStatus(path); } catch {} }
  };

  const scanWindows = async () => {
    setScanningWindows(true);
    try {
      const wins = await listVisibleWindows();
      setVisibleWindows(wins);
    } catch { setVisibleWindows([]); }
    finally { setScanningWindows(false); }
  };

  // ─── Plugin settings view ───
  if (activePlugin) {
    const plugin = getPluginById(activePlugin);
    const whitelist = getWhitelist(activePlugin);

    return (
      <div className="manage-page">
        <header className="manage-header">
          <div className="manage-header-actions">
            <button className="btn" onClick={() => setActivePlugin(null)} title="返回">
              &#8592;
            </button>
          </div>
          <h2>{plugin?.icon || "📦"} {plugin?.title || activePlugin} 设置</h2>
          <div className="manage-header-actions">
            <button className="btn btn-close" onClick={handleClose} title="关闭">&#10005;</button>
          </div>
        </header>

        <div className="manage-content settings-page-body">
          {/* Git settings */}
          {activePlugin === "git" && (
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

          {/* Whitelist editor */}
          {plugin?.showAttachButton !== false && (
            <div className="settings-group">
              <label className="settings-label">吸附白名单</label>
              <div className="settings-sublabel">仅对白名单内的窗口启用吸附，留空=全部窗口</div>

              {/* Current whitelist entries */}
              <div className="manage-whitelist-list">
                {whitelist.map((pattern, i) => (
                  <div key={i} className="manage-whitelist-item">
                    <span className="manage-whitelist-pattern">{pattern}</span>
                    <button className="manage-whitelist-remove"
                      onClick={() => updateWhitelist(activePlugin, whitelist.filter((_, j) => j !== i))}>
                      &#10005;
                    </button>
                  </div>
                ))}
                {whitelist.length === 0 && <div className="manage-whitelist-empty">无限制 — 对所有窗口吸附</div>}
              </div>

              {/* Window picker */}
              <button className="btn-action btn-scan-windows" onClick={scanWindows} disabled={scanningWindows}>
                {scanningWindows ? "扫描中…" : "🔍 扫描运行中的程序"}
              </button>

              {visibleWindows.length > 0 && (
                <div className="window-picker-list">
                  {visibleWindows.map((w, i) => {
                    const alreadyAdded = whitelist.includes(w.process);
                    return (
                      <div
                        key={i}
                        className={`window-picker-item${alreadyAdded ? " window-picker-added" : ""}`}
                        onClick={() => {
                          if (!alreadyAdded) {
                            updateWhitelist(activePlugin, [...whitelist, w.process]);
                          }
                        }}
                      >
                        <span className="window-picker-process">{w.process}</span>
                        <span className="window-picker-title">{w.title}</span>
                        {alreadyAdded && <span className="window-picker-check">✓</span>}
                      </div>
                    );
                  })}
                </div>
              )}
            </div>
          )}
        </div>
      </div>
    );
  }

  // ─── Plugin list view ───
  return (
    <div className="manage-page">
      <header className="manage-header">
        <h2>Galncelet</h2>
        <div className="manage-header-actions">
          <button className="btn btn-close" onClick={handleClose} title="关闭">&#10005;</button>
        </div>
      </header>

      <div className="manage-content">
        <div className="manage-list">
          {plugins.map((p) => {
            const enabled = settings.panelVisibility[p.id] !== false;
            return (
              <div key={p.id} className={`manage-item ${enabled ? "" : "manage-item-disabled"}`}>
                <div className="manage-item-info">
                  <span className="manage-item-icon">{p.icon || "📦"}</span>
                  <div className="manage-item-text">
                    <span className="manage-item-title">{p.title}</span>
                    {p.description && <span className="manage-item-desc">{p.description}</span>}
                  </div>
                </div>
                <div className="manage-item-actions">
                  <button
                    className={`toggle ${enabled ? "toggle-on" : ""}`}
                    onClick={() => togglePlugin(p)}
                    title={enabled ? "禁用" : "启用"}
                  >
                    <span className="toggle-knob" />
                  </button>
                  <button
                    className="btn-action btn-open-plugin"
                    onClick={() => setActivePlugin(p.id)}
                  >
                    设置
                  </button>
                  {enabled && (
                    <button
                      className="btn-action btn-open-plugin"
                      onClick={() => openPlugin(p)}
                    >
                      打开
                    </button>
                  )}
                </div>
              </div>
            );
          })}
          {plugins.length === 0 && <div className="manage-empty">暂无已注册的插件</div>}
        </div>
      </div>
    </div>
  );
}

// Helper: find plugin by id
function getPluginById(id: string): PluginDef | undefined {
  return getAllPlugins().find((p) => p.id === id);
}

