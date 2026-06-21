import { type ReactNode, useState, useCallback, useEffect, useRef } from "react";
import { getCurrentWindow, LogicalSize, LogicalPosition } from "@tauri-apps/api/window";
import { setBodyCollapsed, setAttachEnabled as setAttachEnabledApi, setAttachWhitelist, setAttachRemember, setHasPosition, loadSettings, saveWindowState } from "../lib/api";
import type { WindowState } from "../lib/types";

const HEADER_H = 36;
const DEFAULT_H = 800;
const SAVE_DEBOUNCE_MS = 500;

interface WidgetShellProps {
  title: ReactNode;
  children: ReactNode;
  headerRight?: ReactNode;
  showCloseButton?: boolean;
  showCollapseButton?: boolean;
  showAttachButton?: boolean;
  defaultAttachEnabled?: boolean;
  defaultAttachRemember?: boolean;
  defaultWhitelist?: string[];
}

export function WidgetShell({
  title,
  children,
  headerRight,
  showCloseButton = true,
  showCollapseButton = true,
  showAttachButton = true,
  defaultAttachEnabled = true,
  defaultAttachRemember = false,
  defaultWhitelist = [],
}: WidgetShellProps) {
  const [collapsed, setCollapsed] = useState(false);
  const [attachEnabled, setAttachEnabledState] = useState(defaultAttachEnabled);
  const [attachRemember, setAttachRememberState] = useState(defaultAttachRemember);
  const win = getCurrentWindow();
  const winLabel = win.label;
  const pluginId = winLabel.replace("widget-", "");
  const saveTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const initialized = useRef(false);
  const preCollapseHeight = useRef<number | null>(null);

  // Refs to avoid stale closures in saveState
  const attachEnabledRef = useRef(attachEnabled);
  const attachRememberRef = useRef(attachRemember);
  attachEnabledRef.current = attachEnabled;
  attachRememberRef.current = attachRemember;

  // Debounced save of window state — reads latest values from refs
  const saveState = useCallback((partial: Partial<WindowState>) => {
    if (saveTimer.current) clearTimeout(saveTimer.current);
    saveTimer.current = setTimeout(async () => {
      try {
        let x: number | undefined;
        let y: number | undefined;
        try {
          const pos = await win.outerPosition();
          const scale = window.devicePixelRatio || 1;
          x = pos.x / scale;
          y = pos.y / scale;
        } catch {
          // Window might be hidden — save without position
        }
        const current: WindowState = {
          x, y,
          attachEnabled: attachEnabledRef.current,
          attachRemember: attachRememberRef.current,
          ...partial,
        };
        await saveWindowState(pluginId, current);
      } catch (e) {
        console.error(`[WidgetShell] save failed for ${pluginId}:`, e);
      }
    }, SAVE_DEBOUNCE_MS);
  }, [winLabel, pluginId]);

  // On mount: restore saved position, attach state, and whitelist
  useEffect(() => {
    if (initialized.current) return;
    initialized.current = true;

    loadSettings().then((s) => {
      const ws = s.windowStates[pluginId];
      if (ws) {
        // Restore position
        if (ws.x != null && ws.y != null) {
          win.setPosition(new LogicalPosition(ws.x, ws.y)).catch(() => {});
        }
        // Restore height
        if (ws.height != null) {
          win.setSize(new LogicalSize(s.cardWidth, ws.height)).catch(() => {});
        }
        // Restore attach state
        if (ws.attachEnabled != null) {
          setAttachEnabledState(ws.attachEnabled);
          setAttachEnabledApi(winLabel, ws.attachEnabled);
        }
        // Restore whitelist: use saved if non-empty, otherwise plugin default
        const wl = (ws.whitelist && ws.whitelist.length > 0) ? ws.whitelist : defaultWhitelist;
        if (wl.length > 0) {
          setAttachWhitelist(winLabel, wl);
        }
        // Restore attach remember mode
        if (ws.attachRemember) {
          setAttachRememberState(true);
          setAttachRemember(winLabel, true);
        }
      }
      // Apply default attach if no saved state
      if (!ws?.attachEnabled && defaultAttachEnabled !== false) {
        setAttachEnabledApi(winLabel, true);
      }
      // Persist initial state so attachRemember/attachEnabled are saved
      setTimeout(() => {
        console.log(`[WidgetShell] ${pluginId} delayed save: attachEnabled=${attachEnabledRef.current} attachRemember=${attachRememberRef.current}`);
        saveState({});
      }, 1000);
    }).catch(() => {});
  }, []);

  // Track window position changes
  useEffect(() => {
    const unlisten = win.onMoved(() => {
      saveState({});
      setHasPosition(winLabel);
    });
    return () => { unlisten.then((fn) => fn()); };
  }, [saveState, winLabel]);

  const toggleCollapse = useCallback(async () => {
    const next = !collapsed;
    setCollapsed(next);
    const scale = window.devicePixelRatio || 1;
    if (next) {
      // Save current height before collapsing
      try {
        const size = await win.outerSize();
        preCollapseHeight.current = size.height / scale;
      } catch {
        preCollapseHeight.current = null;
      }
      await setBodyCollapsed(winLabel, Math.round(HEADER_H * scale));
      saveState({ height: HEADER_H });
    } else {
      // Restore to the height before collapsing, or plugin default
      const restoreH = preCollapseHeight.current ?? DEFAULT_H;
      await setBodyCollapsed(winLabel, null, Math.round(restoreH * scale));
      saveState({ height: restoreH });
      preCollapseHeight.current = null;
    }
  }, [collapsed, winLabel, saveState]);

  const toggleAttach = useCallback(async () => {
    const next = !attachEnabled;
    setAttachEnabledState(next);
    await setAttachEnabledApi(winLabel, next);
    saveState({ attachEnabled: next });
  }, [attachEnabled, winLabel, saveState]);

  const toggleRemember = useCallback(async () => {
    const next = !attachRemember;
    setAttachRememberState(next);
    await setAttachRemember(winLabel, next);
    saveState({ attachRemember: next });
  }, [attachRemember, winLabel, saveState]);

  const handleClose = useCallback(async () => {
    try { await win.hide(); } catch {}
  }, [win]);

  return (
    <div className="widget">
      <header className="widget-header">
        <span className="widget-title">{title}</span>
        <div className="widget-header-right">
          {headerRight}
          {showAttachButton && attachEnabled && (
            <button
              className={`btn btn-remember${attachRemember ? " btn-remember-on" : ""}`}
              onClick={toggleRemember}
              title={attachRemember ? "跟随位置" : "记住位置"}
            >
              &#128204;
            </button>
          )}
          {showAttachButton && (
            <button
              className={`btn btn-attach${attachEnabled ? " btn-attach-on" : ""}`}
              onClick={toggleAttach}
              title={attachEnabled ? "停止吸附" : "开启吸附"}
            >
              &#128279;
            </button>
          )}
          {showCollapseButton && (
            <button
              className={`btn${collapsed ? " btn-collapsed" : ""}`}
              onClick={toggleCollapse}
              title={collapsed ? "展开" : "收起"}
            >
              &#9776;
            </button>
          )}
          {showCloseButton && (
            <button className="btn btn-close" onClick={handleClose} title="关闭">
              &#10005;
            </button>
          )}
        </div>
      </header>
      {!collapsed && <div className="widget-body">{children}</div>}
    </div>
  );
}
