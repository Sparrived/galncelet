import { useEffect, useState, useRef } from "react";
import { getCurrentWindow, Window } from "@tauri-apps/api/window";
import { getAllPlugins, type PluginDef } from "../addons/registry";
import {
  loadSettings, saveSettings, saveWindowState,
  setAttachWhitelist, createPluginWindow, listVisibleWindows,
  updateCardWidth, setPluginHotkey, setWidgetSequence, setSequenceHotkey,
  type WindowEntry,
  openSettingsWindow,
} from "../lib/api";
import type { AppSettings } from "../lib/types";
import { DEFAULT_SETTINGS } from "../lib/types";

export default function ManagePage() {
  const [settings, setSettings] = useState<AppSettings>(DEFAULT_SETTINGS);
  const [activePlugin, setActivePlugin] = useState<string | null>(null);
  const [visibleWindows, setVisibleWindows] = useState<WindowEntry[]>([]);
  const [scanningWindows, setScanningWindows] = useState(false);
  const [recordingHotkey, setRecordingHotkey] = useState(false);
  const [recordingSeqHotkey, setRecordingSeqHotkey] = useState(false);
  const [pendingDisplay, setPendingDisplay] = useState("");
  const hotkeyInputRef = useRef<HTMLDivElement>(null);
  const seqHotkeyInputRef = useRef<HTMLDivElement>(null);
  const plugins = getAllPlugins();

  useEffect(() => {
    loadSettings().then(setSettings).catch(() => {});
  }, []);

  // Focus hotkey input when recording starts
  useEffect(() => {
    if (recordingHotkey) {
      hotkeyInputRef.current?.focus();
      setPendingDisplay("");
      startRecording(
        (hk) => { saveHotkey(activePlugin!, hk); setRecordingHotkey(false); },
        () => setRecordingHotkey(false),
        setPendingDisplay,
      );
    }
  }, [recordingHotkey]);
  useEffect(() => {
    if (recordingSeqHotkey) {
      seqHotkeyInputRef.current?.focus();
      setPendingDisplay("");
      startRecording(
        (hk) => { saveSequenceHotkey(hk); setRecordingSeqHotkey(false); },
        () => setRecordingSeqHotkey(false),
        setPendingDisplay,
      );
    }
  }, [recordingSeqHotkey]);

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
      const win = await Window.getByLabel(`widget-${plugin.id}`);
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

  const scanWindows = async () => {
    setScanningWindows(true);
    try {
      const wins = await listVisibleWindows();
      setVisibleWindows(wins);
    } catch { setVisibleWindows([]); }
    finally { setScanningWindows(false); }
  };

  // Key code → readable display name (matches Rust parse_shortcut)
  const KEY_NAMES: Record<string, string> = {
    ControlLeft: "Ctrl", ControlRight: "Ctrl",
    ShiftLeft: "Shift", ShiftRight: "Shift",
    AltLeft: "Alt", AltRight: "Alt",
    MetaLeft: "Super", MetaRight: "Super",
  };
  const codeToName = (code: string): string => {
    if (KEY_NAMES[code]) return KEY_NAMES[code];
    if (code.startsWith("Key")) return code.replace("Key", "").toLowerCase();
    if (code.startsWith("Digit")) return code.replace("Digit", "");
    if (code.length <= 3 && code.startsWith("F")) return code.toLowerCase();
    const map: Record<string, string> = {
      Space: "space", Tab: "tab", Enter: "enter", Escape: "escape", Backspace: "backspace",
      Slash: "slash", Backslash: "backslash", Period: "period", Comma: "comma",
      Semicolon: "semicolon", Quote: "quote",
      BracketLeft: "bracketleft", BracketRight: "bracketright",
      Minus: "minus", Equal: "equal", Backquote: "backquote",
      ArrowUp: "up", ArrowDown: "down", ArrowLeft: "left", ArrowRight: "right",
      Delete: "delete", Insert: "insert", Home: "home", End: "end",
      PageUp: "pageup", PageDown: "pagedown",
    };
    return map[code] || code.toLowerCase();
  };

  // Held-keys tracking for hotkey recording
  const heldKeysRef = useRef(new Set<string>());
  const pendingPartsRef = useRef<string[]>([]);

  const startRecording = (onConfirm: (hotkey: string) => void, onCancel: () => void, onDisplay: (text: string) => void) => {
    heldKeysRef.current.clear();
    pendingPartsRef.current = [];

    const onKeyDown = (e: KeyboardEvent) => {
      e.preventDefault();
      e.stopPropagation();

      // Enter confirms (not part of the combo)
      if (e.code === "Enter") {
        cleanup();
        if (pendingPartsRef.current.length >= 2) {
          onConfirm(pendingPartsRef.current.join("+"));
        } else {
          onCancel();
        }
        return;
      }
      // Escape cancels
      if (e.code === "Escape") {
        cleanup();
        onCancel();
        return;
      }

      heldKeysRef.current.add(e.code);

      // Build display: modifiers first, then non-modifier keys
      const modifiers: string[] = [];
      const keys: string[] = [];
      for (const k of heldKeysRef.current) {
        const name = codeToName(k);
        if (KEY_NAMES[k]) modifiers.push(name);
        else keys.push(name);
      }
      pendingPartsRef.current = [...modifiers, ...keys];
      onDisplay(pendingPartsRef.current.join(" + "));
    };

    const onKeyUp = (e: KeyboardEvent) => {
      e.preventDefault();
      e.stopPropagation();
      heldKeysRef.current.delete(e.code);
      // Rebuild display from remaining held keys
      const modifiers: string[] = [];
      const keys: string[] = [];
      for (const k of heldKeysRef.current) {
        const name = codeToName(k);
        if (KEY_NAMES[k]) modifiers.push(name);
        else keys.push(name);
      }
      pendingPartsRef.current = [...modifiers, ...keys];
      onDisplay(pendingPartsRef.current.join(" + "));
    };

    const cleanup = () => {
      document.removeEventListener("keydown", onKeyDown, true);
      document.removeEventListener("keyup", onKeyUp, true);
    };

    document.addEventListener("keydown", onKeyDown, true);
    document.addEventListener("keyup", onKeyUp, true);
  };

  const saveHotkey = async (pluginId: string, hotkey: string | null) => {
    await setPluginHotkey(pluginId, hotkey);
    const newHotkeys = { ...settings.pluginHotkeys };
    if (hotkey) newHotkeys[pluginId] = hotkey;
    else delete newHotkeys[pluginId];
    setSettings({ ...settings, pluginHotkeys: newHotkeys });
  };

  // ─── Sequence helpers ───
  const seq = settings.widgetSequence;

  // Ensure all sequence widget windows exist (create if missing)
  const ensureSequenceWindows = async (ids: string[]) => {
    for (const id of ids) {
      const p = plugins.find((pl) => pl.id === id);
      if (!p) continue;
      const win = await Window.getByLabel(`widget-${id}`);
      if (!win) {
        await createPluginWindow(
          p.id, p.title,
          p.defaultWidth ?? 360, p.defaultHeight ?? 600,
          p.defaultAttachEnabled !== false,
          p.defaultAttachRemember === true,
          p.defaultWhitelist ?? [],
        );
      }
    }
  };

  const applySequence = async (newSeq: string[]) => {
    await ensureSequenceWindows(newSeq);
    await setWidgetSequence(newSeq);
    await saveSettings({ ...settings, widgetSequence: newSeq });
    setSettings({ ...settings, widgetSequence: newSeq });
  };

  const addToSequence = async (pluginId: string) => {
    if (seq.includes(pluginId)) return;
    await applySequence([...seq, pluginId]);
  };

  const removeFromSequence = async (pluginId: string) => {
    await applySequence(seq.filter((id) => id !== pluginId));
  };

  const moveInSequence = async (pluginId: string, direction: -1 | 1) => {
    const idx = seq.indexOf(pluginId);
    if (idx < 0) return;
    const newIdx = idx + direction;
    if (newIdx < 0 || newIdx >= seq.length) return;
    const newSeq = [...seq];
    [newSeq[idx], newSeq[newIdx]] = [newSeq[newIdx], newSeq[idx]];
    await applySequence(newSeq);
  };

  const saveSequenceHotkey = async (hotkey: string | null) => {
    await setSequenceHotkey(hotkey);
    setSettings({ ...settings, sequenceHotkey: hotkey });
  };

  const availableForSequence = plugins.filter((p) => !seq.includes(p.id));

  // ─── Plugin settings view ───
  if (activePlugin) {
    const plugin = getPluginById(activePlugin);
    const whitelist = getWhitelist(activePlugin);
    const currentHotkey = settings.pluginHotkeys[activePlugin];

    return (
      <div className="manage-page">
        <header className="manage-header">
          <div className="manage-header-actions">
            <button className="btn" onClick={() => { setActivePlugin(null); setRecordingHotkey(false); }} title="返回">
              &#8592;
            </button>
          </div>
          <h2>{plugin?.icon || "📦"} {plugin?.title || activePlugin} 设置</h2>
          <div className="manage-header-actions">
            <button className="btn btn-close" onClick={handleClose} title="关闭">&#10005;</button>
          </div>
        </header>

        <div className="manage-content settings-page-body">
          {/* Hotkey config */}
          <div className="settings-group">
            <label className="settings-label">快捷键</label>
            <div className="settings-sublabel">按下组合键快速显示/隐藏此挂件（需含 Ctrl/Alt/Shift 修饰键）</div>
            <div className="hotkey-row">
              {recordingHotkey ? (
                <div
                  ref={hotkeyInputRef}
                  className="hotkey-input hotkey-recording"
                  tabIndex={0}
                  onBlur={() => setRecordingHotkey(false)}
                >
                  {pendingDisplay || "按下组合键，Enter 确认"}
                </div>
              ) : (
                <div className="hotkey-input" onClick={() => setRecordingHotkey(true)}>
                  {currentHotkey || "点击设置快捷键"}
                </div>
              )}
              {currentHotkey && !recordingHotkey && (
                <button className="btn-action hotkey-clear" onClick={() => saveHotkey(activePlugin, null)}>
                  清除
                </button>
              )}
            </div>
          </div>

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
          <button className="btn" onClick={() => openSettingsWindow()} title="全局设置">⚙️</button>
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
                    className="btn-action btn-open-plugin"
                    onClick={() => setActivePlugin(p.id)}
                  >
                    设置
                  </button>
                  <button
                    className="btn-action btn-open-plugin"
                    onClick={() => openPlugin(p)}
                    disabled={!enabled}
                    style={!enabled ? { opacity: 0.3, pointerEvents: "none" } : undefined}
                  >
                    打开
                  </button>
                  <button
                    className={`toggle ${enabled ? "toggle-on" : ""}`}
                    onClick={() => togglePlugin(p)}
                    title={enabled ? "禁用" : "启用"}
                  >
                    <span className="toggle-knob" />
                  </button>
                </div>
              </div>
            );
          })}
          {plugins.length === 0 && <div className="manage-empty">暂无已注册的插件</div>}
        </div>

        {/* ─── Widget Sequence ─── */}
        <div className="settings-group sequence-section">
          <label className="settings-label">挂件序列</label>
          <div className="settings-sublabel">将挂件加入序列，通过快捷键在同一位置依次切换</div>

          {/* Sequence hotkey */}
          <div className="hotkey-row" style={{ marginBottom: 10 }}>
            {recordingSeqHotkey ? (
              <div
                ref={seqHotkeyInputRef}
                className="hotkey-input hotkey-recording"
                tabIndex={0}
                onBlur={() => setRecordingSeqHotkey(false)}
              >
                {pendingDisplay || "按下组合键，Enter 确认"}
              </div>
            ) : (
              <div className="hotkey-input" onClick={() => setRecordingSeqHotkey(true)}>
                {settings.sequenceHotkey || "点击设置切换快捷键"}
              </div>
            )}
            {settings.sequenceHotkey && !recordingSeqHotkey && (
              <button className="btn-action hotkey-clear" onClick={() => saveSequenceHotkey(null)}>
                清除
              </button>
            )}
          </div>

          {/* Current sequence */}
          {seq.length > 0 && (
            <div className="sequence-list">
              {seq.map((id, i) => {
                const p = getPluginById(id);
                return (
                  <div key={id} className="sequence-item">
                    <span className="sequence-index">{i + 1}</span>
                    <span className="sequence-icon">{p?.icon || "📦"}</span>
                    <span className="sequence-name">{p?.title || id}</span>
                    <div className="sequence-actions">
                      <button className="btn-sm" disabled={i === 0} onClick={() => moveInSequence(id, -1)}>↑</button>
                      <button className="btn-sm" disabled={i === seq.length - 1} onClick={() => moveInSequence(id, 1)}>↓</button>
                      <button className="btn-sm btn-sm-danger" onClick={() => removeFromSequence(id)}>✕</button>
                    </div>
                  </div>
                );
              })}
            </div>
          )}

          {/* Add to sequence */}
          {availableForSequence.length > 0 && (
            <div className="sequence-add">
              {availableForSequence.map((p) => (
                <button key={p.id} className="btn-action sequence-add-btn" onClick={() => addToSequence(p.id)}>
                  {p.icon || "📦"} {p.title}
                </button>
              ))}
            </div>
          )}
        </div>
      </div>
    </div>
  );
}

// Helper: find plugin by id
function getPluginById(id: string): PluginDef | undefined {
  return getAllPlugins().find((p) => p.id === id);
}
