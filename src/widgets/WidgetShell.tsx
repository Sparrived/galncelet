import { type ReactNode, useState, useCallback, useEffect, useRef, useMemo } from "react";
import { getCurrentWindow, LogicalSize, LogicalPosition } from "@tauri-apps/api/window";
import { setBodyCollapsed, setAttachEnabled as setAttachEnabledApi, setAttachWhitelist, setAttachRemember, loadSettings, saveWindowState, setPluginVisible } from "../lib/api";
import type { WindowState } from "../lib/types";
import { HEADER_H, WidgetProvider, type ContextMenuItem } from "./WidgetContext";
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
  onClose?: () => void;
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
  onClose,
}: WidgetShellProps) {
  const [collapsed, setCollapsed] = useState(false);
  const [attachEnabled, setAttachEnabledState] = useState(defaultAttachEnabled);
  const [attachRemember, setAttachRememberState] = useState(defaultAttachRemember);
  const [isInSequence, setIsInSequence] = useState(false);
  const win = getCurrentWindow();
  const winLabel = win.label;
  const pluginId = winLabel.replace("widget-", "");
  const saveTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const initialized = useRef(false);
  const preCollapseHeight = useRef<number | null>(null);
  const actualHeight = useRef<number>(0);
  const collapsedRef = useRef(false);

  // Body drag: make blank areas draggable
  const handleBodyMouseDown = useCallback((e: React.MouseEvent) => {
    const target = e.target as HTMLElement;
    if (target.closest("button, input, select, textarea, a, [data-no-drag]")) return;
    if (target.closest(".widget-header")) return;
    e.preventDefault();
    win.startDragging().catch(() => {});
  }, [win]);

  // Refs to avoid stale closures in saveState
  const attachEnabledRef = useRef(attachEnabled);
  const attachRememberRef = useRef(attachRemember);
  attachEnabledRef.current = attachEnabled;
  attachRememberRef.current = attachRemember;

  // Debounced save of window state
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
          // Window might be hidden
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
      if (s.widgetSequence && s.widgetSequence.includes(pluginId)) {
        setIsInSequence(true);
      }
      const ws = s.windowStates[pluginId];
      if (ws) {
        if (ws.x != null && ws.y != null) {
          win.setPosition(new LogicalPosition(ws.x, ws.y)).catch(() => {});
        } else {
          const cx = Math.round((window.screen.width - (s.cardWidth || 360)) / 2);
          const cy = Math.round((window.screen.height - 400) / 2);
          win.setPosition(new LogicalPosition(Math.max(0, cx), Math.max(0, cy))).catch(() => {});
        }
        if (ws.height != null) {
          actualHeight.current = ws.height;
          win.setSize(new LogicalSize(s.cardWidth, ws.height)).catch(() => {});
        }
        if (ws.attachEnabled != null) {
          setAttachEnabledState(ws.attachEnabled);
          setAttachEnabledApi(winLabel, ws.attachEnabled);
        }
        const wl = (ws.whitelist && ws.whitelist.length > 0) ? ws.whitelist : defaultWhitelist;
        if (wl.length > 0) {
          setAttachWhitelist(winLabel, wl);
        }
        if (ws.attachRemember) {
          setAttachRememberState(true);
          setAttachRemember(winLabel, true);
        }
      }
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
      try {
        const size = await win.outerSize();
        preCollapseHeight.current = size.height / scale;
      } catch {
        preCollapseHeight.current = null;
      }
      await setBodyCollapsed(winLabel, Math.round(HEADER_H * scale));
      saveState({ height: HEADER_H });
    } else {
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
    if (isInSequence) return;
    try {
      if (onClose) await onClose();
      const w = Math.round((window.screen.width - 360) / 2);
      const h = Math.round((window.screen.height - 400) / 2);
      win.setPosition(new LogicalPosition(Math.max(0, w), Math.max(0, h))).catch(() => {});
      saveWindowState(pluginId, { x: undefined, y: undefined, height: undefined }).catch(() => {});
      await win.hide();
      setPluginVisible(pluginId, false).catch(() => {});
    } catch {}
  }, [win, winLabel, onClose, pluginId, isInSequence]);


  // ─── Context Menu ───
  const [menuPos, setMenuPos] = useState<{ x: number; y: number } | null>(null);
  const [pluginMenuItems, setPluginMenuItems] = useState<ContextMenuItem[]>([]);
  const menuRef = useRef<HTMLDivElement>(null);

  const registerContextMenuItems = useCallback((items: ContextMenuItem[]) => {
    setPluginMenuItems(items);
  }, []);

  const handleContextMenu = useCallback((e: React.MouseEvent) => {
    e.preventDefault();
    const rect = (e.currentTarget as HTMLElement).getBoundingClientRect();
    setMenuPos({ x: e.clientX - rect.left + 2, y: e.clientY - rect.top + 4 });
  }, []);

  const closeMenu = useCallback(() => setMenuPos(null), []);

  // Close menu on outside click
  useEffect(() => {
    if (!menuPos) return;
    const handler = (e: MouseEvent) => {
      if (menuRef.current && !menuRef.current.contains(e.target as Node)) closeMenu();
    };
    document.addEventListener("mousedown", handler);
    return () => document.removeEventListener("mousedown", handler);
  }, [menuPos, closeMenu]);

  const defaultMenuItems: ContextMenuItem[] = isInSequence
    ? []
    : [{ label: "关闭挂件", icon: "✕", onClick: handleClose, danger: true }];
  const allMenuItems = [...pluginMenuItems, ...defaultMenuItems];

  const contextValue = useMemo(() => ({ collapsed, contextMenuItems: pluginMenuItems, registerContextMenuItems }), [collapsed, pluginMenuItems, registerContextMenuItems]);

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
            {showCloseButton && !isInSequence && (
              <CloseButton onClick={handleClose} />
            )}
          </div>
        </header>
        <div className="widget-body" style={collapsed ? { display: "none" } : undefined} onMouseDown={handleBodyMouseDown} onContextMenu={handleContextMenu}>
          {children}
          {menuPos && (
            <div className="ctx-menu" ref={menuRef} style={{ left: menuPos.x, top: menuPos.y }}>
              {allMenuItems.map((item, i) => (
                <button
                  key={i}
                  className={`ctx-menu-item${item.danger ? " ctx-menu-danger" : ""}`}
                  onClick={() => { item.onClick(); closeMenu(); }}
                >
                  {item.icon && <span className="ctx-menu-icon">{item.icon}</span>}
                  {item.label}
                </button>
              ))}
            </div>
          )}
        </div>
      </div>
    </WidgetProvider>
  );
}
