import { type ReactNode, useState, useCallback, useEffect, useRef, useMemo } from "react";
import { getCurrentWindow, LogicalSize, LogicalPosition } from "@tauri-apps/api/window";
import { setBodyCollapsed, setAttachEnabled as setAttachEnabledApi, setAttachWhitelist, setAttachRemember, loadSettings, saveSettings, saveWindowState } from "../lib/api";
import type { WindowState } from "../lib/types";
import { HEADER_H, WidgetProvider } from "./WidgetContext";
import { CloseButton, CollapseButton, AttachButton, RememberButton } from "./WidgetButtons";

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
  const actualHeight = useRef<number>(0);
  const collapsedRef = useRef(false);

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
          actualHeight.current = ws.height;
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
    }).catch(() => {});
  }, []);

  // Track window position and size changes
  useEffect(() => {
    const unlistenMove = win.onMoved(() => {
      saveState({});
    });
    const unlistenResize = win.onResized(() => {
      if (collapsedRef.current) return;
      win.outerSize().then((size) => {
        const scale = window.devicePixelRatio || 1;
        actualHeight.current = size.height / scale;
      }).catch(() => {});
    });
    return () => {
      unlistenMove.then((fn) => fn());
      unlistenResize.then((fn) => fn());
    };
  }, [saveState, winLabel]);

  const toggleCollapse = useCallback(async () => {
    const next = !collapsed;
    setCollapsed(next);
    collapsedRef.current = next;
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
      // Restore to the height before collapsing, or last known height
      const restoreH = preCollapseHeight.current || actualHeight.current || 400;
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
    try {
      const s = await loadSettings();
      s.panelVisibility[pluginId] = false;
      await saveSettings(s);
      await win.hide();
    } catch {}
  }, [win, pluginId]);

  const contextValue = useMemo(() => ({ collapsed }), [collapsed]);

  return (
    <WidgetProvider value={contextValue}>
      <div className="widget">
        <header className="widget-header">
          <span className="widget-title">{title}</span>
          <div className="widget-header-right">
            {headerRight}
            {showAttachButton && attachEnabled && (
              <RememberButton active={attachRemember} onClick={toggleRemember} />
            )}
            {showAttachButton && (
              <AttachButton enabled={attachEnabled} onClick={toggleAttach} />
            )}
            {showCollapseButton && (
              <CollapseButton collapsed={collapsed} onClick={toggleCollapse} />
            )}
            {showCloseButton && (
              <CloseButton onClick={handleClose} />
            )}
          </div>
        </header>
        <div className="widget-body" style={collapsed ? { display: "none" } : undefined}>{children}</div>
      </div>
    </WidgetProvider>
  );
}
