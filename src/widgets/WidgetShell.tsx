import { type ReactNode, useState, useCallback, useEffect, useRef, useMemo } from "react";
import { getCurrentWindow, LogicalSize, LogicalPosition } from "@tauri-apps/api/window";
import { emit, listen } from "@tauri-apps/api/event";
import { setBodyCollapsed, setAttachEnabled as setAttachEnabledApi, setAttachWhitelist, setAttachRemember, loadSettings, saveWindowState, setPluginVisible, getAllWidgetRects, snapWidget, unsnapWidget, moveSnapGroup, getSnapInfo, type SnapEdge } from "../lib/api";
import type { WindowState } from "../lib/types";
import { HEADER_H, WidgetProvider, type ContextMenuItem } from "./WidgetContext";
import { CloseButton, CollapseButton, AttachButton, RememberButton } from "./WidgetButtons";

const SNAP_THRESHOLD = 20;

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
  const [snapEdge, setSnapEdge] = useState<SnapEdge | null>(null);
  const [incomingEdge, setIncomingEdge] = useState<SnapEdge | null>(null);
  const win = getCurrentWindow();
  const winLabel = win.label;
  const pluginId = winLabel.replace("widget-", "");
  const saveTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const initialized = useRef(false);
  const preCollapseHeight = useRef<number | null>(null);
  const actualHeight = useRef<number>(0);
  const collapsedRef = useRef(false);
  const prevPosRef = useRef<{ x: number; y: number } | null>(null);
  const snapEdgeRef = useRef<SnapEdge | null>(null);
  const snapTargetRef = useRef<string>("");
  const snapCooldownRef = useRef(false);
  snapEdgeRef.current = snapEdge;

  // Body drag: make blank areas draggable
  const handleBodyMouseDown = useCallback((e: React.MouseEvent) => {
    const target = e.target as HTMLElement;
    // Don't drag if clicking on interactive elements
    if (target.closest("button, input, select, textarea, a, [data-no-drag]")) return;
    // Don't drag if inside header (already handled by -webkit-app-region)
    if (target.closest(".widget-header")) return;
    e.preventDefault();
    win.startDragging().catch(() => {});
  }, [win]);

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
        // Restore position (center on screen if no saved position)
        if (ws.x != null && ws.y != null) {
          win.setPosition(new LogicalPosition(ws.x, ws.y)).catch(() => {});
        } else {
          const cx = Math.round((window.screen.width - (s.cardWidth || 360)) / 2);
          const cy = Math.round((window.screen.height - 400) / 2);
          win.setPosition(new LogicalPosition(Math.max(0, cx), Math.max(0, cy))).catch(() => {});
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

  // Track window position and size changes + snap detection
  // Snap: preview glow while dragging, commit on mouse release
  useEffect(() => {
    let pendingSnap: { target: string; edge: SnapEdge; offset: number } | null = null;

    const OPPOSITE_EDGE: Record<SnapEdge, SnapEdge> = { Top: "Bottom", Bottom: "Top", Left: "Right", Right: "Left" };

    const commitSnap = () => {
      if (pendingSnap) {
        const oldTarget = snapTargetRef.current;
        // Clear old target's incoming glow
        if (oldTarget && oldTarget !== pendingSnap.target) {
          emit(`snap:clear:${oldTarget}`, {}).catch(() => {});
        }
        snapWidget(winLabel, pendingSnap.target, pendingSnap.edge, pendingSnap.offset).catch(() => {});
        // A shows glow on its own contact edge (opposite of snap edge)
        setSnapEdge(OPPOSITE_EDGE[pendingSnap.edge]);
        snapTargetRef.current = pendingSnap.target;
        // B shows glow on the edge A is attached to
        emit(`snap:notify:${pendingSnap.target}`, { edge: pendingSnap.edge }).catch(() => {});
        snapCooldownRef.current = true;
        setTimeout(() => { snapCooldownRef.current = false; }, 300);
        pendingSnap = null;
      } else if (snapEdgeRef.current && snapTargetRef.current) {
        // Was snapped but dragged away — unsnap on release
        emit(`snap:clear:${snapTargetRef.current}`, {}).catch(() => {});
        unsnapWidget(winLabel).catch(() => {});
        setSnapEdge(null);
        snapTargetRef.current = "";
        snapCooldownRef.current = true;
        setTimeout(() => { snapCooldownRef.current = false; }, 300);
      }
    };

    const handleMouseUp = () => { commitSnap(); };
    document.addEventListener("mouseup", handleMouseUp);

    // Listen for incoming snap notifications (when another widget snaps TO us)
    const unlistenNotify = listen(`snap:notify:${winLabel}`, (e) => {
      const { edge } = e.payload as { edge: SnapEdge };
      setIncomingEdge(edge);
    });
    const unlistenClear = listen(`snap:clear:${winLabel}`, () => {
      setIncomingEdge(null);
    });

    // On mount, check if we already have an incoming snap
    getSnapInfo(winLabel).then(() => {
      // We don't have a "reverse" query, so we check all widgets later
    }).catch(() => {});

    const unlistenMove = win.onMoved(async () => {
      saveState({});
      try {
        const pos = await win.outerPosition();
        const scale = window.devicePixelRatio || 1;
        const px = Math.round(pos.x / scale);
        const py = Math.round(pos.y / scale);

        // If snapped, move the snap group by the delta
        if (snapEdgeRef.current && prevPosRef.current) {
          const dx = pos.x - Math.round(prevPosRef.current.x * scale);
          const dy = pos.y - Math.round(prevPosRef.current.y * scale);
          if (dx !== 0 || dy !== 0) {
            moveSnapGroup(winLabel, dx, dy).catch(() => {});
          }
        }
        prevPosRef.current = { x: px, y: py };

        // Skip detection during cooldown or for foreground-attached widgets
        if (snapCooldownRef.current || attachEnabledRef.current) return;

        

        // Detect proximity and show preview glow
        const rects = await getAllWidgetRects();
        const my = rects[winLabel];
        if (!my) return;

        let bestDist = Infinity;
        let bestEdge: SnapEdge | null = null;
        let bestTarget = "";
        let bestOffset = 0;

        for (const [label, r] of Object.entries(rects)) {
          if (label === winLabel) continue;
          if (r.attach_enabled) continue;
          // My bottom near target's top → I'm above → snap to target's Top (A的顶边贴B的底边)
          const dAbove = Math.abs((my.y + my.h) - r.y);
          if (dAbove < SNAP_THRESHOLD && my.x > r.x - my.w / 2 && my.x < r.x + r.w - my.w / 2) {
            if (dAbove < bestDist) { bestDist = dAbove; bestEdge = "Top"; bestTarget = label; bestOffset = my.x; }
          }
          // My top near target's bottom → I'm below → snap to target's Bottom
          const dBelow = Math.abs(my.y - (r.y + r.h));
          if (dBelow < SNAP_THRESHOLD && my.x > r.x - my.w / 2 && my.x < r.x + r.w - my.w / 2) {
            if (dBelow < bestDist) { bestDist = dBelow; bestEdge = "Bottom"; bestTarget = label; bestOffset = my.x; }
          }
          // My right near target's left → I'm to the left → snap to target's Left
          const dLeftOf = Math.abs((my.x + my.w) - r.x);
          if (dLeftOf < SNAP_THRESHOLD && my.y > r.y - my.h / 2 && my.y < r.y + r.h - my.h / 2) {
            if (dLeftOf < bestDist) { bestDist = dLeftOf; bestEdge = "Left"; bestTarget = label; bestOffset = my.y; }
          }
          // My left near target's right → I'm to the right → snap to target's Right
          const dRightOf = Math.abs(my.x - (r.x + r.w));
          if (dRightOf < SNAP_THRESHOLD && my.y > r.y - my.h / 2 && my.y < r.y + r.h - my.h / 2) {
            if (dRightOf < bestDist) { bestDist = dRightOf; bestEdge = "Right"; bestTarget = label; bestOffset = my.y; }
          }
        }

        if (bestEdge && bestDist < SNAP_THRESHOLD) {
          // Preview glow on A's contact edge (opposite of snap edge)
          const glowEdge = ({ Top: "Bottom", Bottom: "Top", Left: "Right", Right: "Left" } as Record<SnapEdge, SnapEdge>)[bestEdge];
          setSnapEdge(glowEdge);
          pendingSnap = { target: bestTarget, edge: bestEdge, offset: bestOffset };
        } else {
          // No edge nearby — clear preview glow
          setSnapEdge(null);
          pendingSnap = null;
        }

      } catch {}
    });
    const unlistenResize = win.onResized(() => {
      if (collapsedRef.current) return;
      win.outerSize().then((size) => {
        const scale = window.devicePixelRatio || 1;
        actualHeight.current = size.height / scale;
      }).catch(() => {});
    });
    return () => {
      document.removeEventListener("mouseup", handleMouseUp);
      unlistenNotify.then((fn) => fn());
      unlistenClear.then((fn) => fn());
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
      if (onClose) await onClose();
      // Remove snap relationships and notify target
      if (snapTargetRef.current) {
        emit(`snap:clear:${snapTargetRef.current}`, {}).catch(() => {});
      }
      unsnapWidget(winLabel).catch(() => {});
      setSnapEdge(null);
      setIncomingEdge(null);
      snapTargetRef.current = "";
      // Move window to screen center, then clear saved position
      const w = Math.round((window.screen.width - 360) / 2);
      const h = Math.round((window.screen.height - 400) / 2);
      win.setPosition(new LogicalPosition(Math.max(0, w), Math.max(0, h))).catch(() => {});
      saveWindowState(pluginId, { x: undefined, y: undefined, height: undefined }).catch(() => {});
      await win.hide();
      setPluginVisible(pluginId, false).catch(() => {});
    } catch {}
  }, [win, winLabel, onClose, pluginId]);

  
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

  const defaultMenuItems: ContextMenuItem[] = [
    { label: "关闭挂件", icon: "✕", onClick: handleClose, danger: true },
  ];
  const allMenuItems = [...pluginMenuItems, ...defaultMenuItems];

  const contextValue = useMemo(() => ({ collapsed, contextMenuItems: pluginMenuItems, registerContextMenuItems }), [collapsed, pluginMenuItems, registerContextMenuItems]);

  return (
    <WidgetProvider value={contextValue}>
      <div className={`widget${snapEdge ? ` widget-snapped snap-edge-${snapEdge.toLowerCase()}` : ""}${incomingEdge ? ` widget-snapped snap-edge-${incomingEdge.toLowerCase()}` : ""}`}>
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
