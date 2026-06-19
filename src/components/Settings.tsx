import { useState } from "react";
import type { AppSettings } from "../lib/types";
import { getAllPlugins } from "../addons/registry";

interface SettingsProps {
  settings: AppSettings;
  onSave: (settings: AppSettings) => void;
  onClose: () => void;
  onSelectFolder: () => void;
}

export function Settings({ settings, onSave, onClose, onSelectFolder }: SettingsProps) {
  const [draft, setDraft] = useState<AppSettings>({ ...settings });

  const update = <K extends keyof AppSettings>(key: K, value: AppSettings[K]) => {
    setDraft((prev) => ({ ...prev, [key]: value }));
  };

  const handleSave = () => {
    onSave(draft);
    onClose();
  };

  return (
    <div className="settings-overlay" onClick={onClose}>
      <div className="settings-panel" onClick={(e) => e.stopPropagation()}>
        <div className="settings-header">
          <h3>设置</h3>
          <button className="btn" onClick={onClose}>
            &#10005;
          </button>
        </div>
        <div className="settings-body">
          {/* Select repo */}
          <div className="settings-group">
            <button
              className="btn-action btn-select-folder"
              onClick={() => { onClose(); onSelectFolder(); }}
            >
              &#128193; 选择仓库
            </button>
          </div>

          {/* Refresh interval */}
          <div className="settings-group">
            <label className="settings-label">自动刷新间隔</label>
            <div className="settings-row">
              <input
                className="settings-slider"
                type="range"
                min={500}
                max={10000}
                step={500}
                value={draft.refreshIntervalMs}
                onChange={(e) => update("refreshIntervalMs", Number(e.target.value))}
              />
              <span className="settings-value">{draft.refreshIntervalMs}ms</span>
            </div>
          </div>

          {/* Card width */}
          <div className="settings-group">
            <label className="settings-label">卡片宽度</label>
            <div className="settings-row">
              <input
                className="settings-slider"
                type="range"
                min={300}
                max={600}
                step={10}
                value={draft.cardWidth}
                onChange={(e) => update("cardWidth", Number(e.target.value))}
              />
              <span className="settings-value">{draft.cardWidth}px</span>
            </div>
          </div>

          {/* Log max count */}
          <div className="settings-group">
            <label className="settings-label">提交历史最大条数</label>
            <div className="settings-row">
              <input
                className="settings-number"
                type="number"
                min={10}
                max={500}
                value={draft.logMaxCount}
                onChange={(e) => update("logMaxCount", Math.max(10, Math.min(500, Number(e.target.value) || 10)))}
              />
            </div>
          </div>

          {/* Always on top */}
          <div className="settings-group">
            <div className="settings-row settings-row-between">
              <label className="settings-label">窗口置顶</label>
              <button
                className={`toggle ${draft.alwaysOnTop ? "toggle-on" : ""}`}
                onClick={() => update("alwaysOnTop", !draft.alwaysOnTop)}
                type="button"
              >
                <span className="toggle-knob" />
              </button>
            </div>
          </div>

          {/* Pull --rebase */}
          <div className="settings-group">
            <div className="settings-row settings-row-between">
              <label className="settings-label">Pull 使用 Rebase</label>
              <button
                className={`toggle ${draft.pullRebase ? "toggle-on" : ""}`}
                onClick={() => update("pullRebase", !draft.pullRebase)}
                type="button"
              >
                <span className="toggle-knob" />
              </button>
            </div>
          </div>

          {/* Panel visibility */}
          {getAllPlugins().map((p) => (
            <div className="settings-group" key={p.id}>
              <div className="settings-row settings-row-between">
                <label className="settings-label">显示 {p.title}</label>
                <button
                  className={`toggle ${draft.panelVisibility[p.id] !== false ? "toggle-on" : ""}`}
                  onClick={() => {
                    const vis = { ...draft.panelVisibility };
                    vis[p.id] = vis[p.id] === false;
                    update("panelVisibility", vis);
                  }}
                  type="button"
                >
                  <span className="toggle-knob" />
                </button>
              </div>
            </div>
          ))}
        </div>

        <div className="settings-footer">
          <button className="btn-action" onClick={onClose}>
            取消
          </button>
          <button className="btn-action btn-commit" onClick={handleSave}>
            保存
          </button>
        </div>
      </div>
    </div>
  );
}
