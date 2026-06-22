import { type ReactNode, useState, useCallback, useEffect, useRef, useMemo } from "react";
import { getCurrentWindow, LogicalSize, LogicalPosition } from "@tauri-apps/api/window";
import { setBodyCollapsed, setAttachEnabled as setAttachEnabledApi, setAttachWhitelist, setAttachRemember, loadSettings, saveWindowState, getAllWidgetRects, snapWidget, unsnapWidget, moveSnapGroup, type SnapEdge } from "../lib/api";
import type { WindowState } from "../lib/types";
import { HEADER_H, WidgetProvider } from "./WidgetContext";
import { CloseButton, CollapseButton, AttachButton, RememberButton } from "./WidgetButtons";

const SNAP_THRESHOLD = 20;
const UNSNAP_THRESHOLD = 40;

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
  useEffect(() => {
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

        // During cooldown after snap/unsnap, skip detection
        if (snapCooldownRef.current) return;

        // If already snapped, only check unsnap (distance > threshold)
        const curEdge = snapEdgeRef.current;
        const curTarget = snapTargetRef.current;
        if (curEdge && curTarget) {
          const rects = await getAllWidgetRects();
          const my = rects[winLabel];
          const tgt = rects[curTarget];
          if (my && tgt) {
            let dist = 0;
            switch (curEdge) {
              case "Bottom": dist = Math.abs((my.y + my.h) - tgt.y); break;
              case "Top": dist = Math.abs(my.y - (tgt.y + tgt.h)); break;
              case "Right": dist = Math.abs((my.x + my.w) - tgt.x); break;
              case "Left": dist = Math.abs(my.x - (tgt.x + tgt.w)); break;
            }
            if (dist > UNSNAP_THRESHOLD) {
              unsnapWidget(winLabel).catch(() => {});
              setSnapEdge(null);
              snapTargetRef.current = "";
              snapCooldownRef.current = true;
              setTimeout(() => { snapCooldownRef.current = false; }, 200);
            }
          }
          return; // Skip new-snap detection while snapped
        }

        // Not snapped — check proximity to other widgets for new snap
        const rects = await getAllWidgetRects();
        const my = rects[winLabel];
        if (!my) return;

        let bestDist = Infinity;
        let bestEdge: SnapEdge | null = null;
        let bestTarget = "";
        let bestOffset = 0;

        for (const [label, r] of Object.entries(rects)) {
          if (label === winLabel) continue;
          const dBottom = Math.abs((my.y + my.h) - r.y);
          if (dBottom < SNAP_THRESHOLD && my.x > r.x - my.w / 2 && my.x < r.x + r.w - my.w / 2) {
            if (dBottom < bestDist) { bestDist = dBottom; bestEdge = "Bottom"; bestTarget = label; bestOffset = my.x; }
          }
          const dTop = Math.abs(my.y - (r.y + r.h));
          if (dTop < SNAP_THRESHOLD && my.x > r.x - my.w / 2 && my.x < r.x + r.w - my.w / 2) {
            if (dTop < bestDist) { bestDist = dTop; bestEdge = "Top"; bestTarget = label; bestOffset = my.x; }
          }
          const dRight = Math.abs((my.x + my.w) - r.x);
          if (dRight < SNAP_THRESHOLD && my.y > r.y - my.h / 2 && my.y < r.y + r.h - my.h / 2) {
            if (dRight < bestDist) { bestDist = dRight; bestEdge = "Right"; bestTarget = label; bestOffset = my.y; }
          }
          const dLeft = Math.abs(my.x - (r.x + r.w));
          if (dLeft < SNAP_THRESHOLD && my.y > r.y - my.h / 2 && my.y < r.y + r.h - my.h / 2) {
            if (dLeft < bestDist) { bestDist = dLeft; bestEdge = "Left"; bestTarget = label; bestOffset = my.y; }
          }
        }

        if (bestEdge && bestDist < SNAP_THRESHOLD) {
          snapWidget(winLabel, bestTarget, bestEdge, bestOffset).catch(() => {});
          setSnapEdge(bestEdge);
          snapTargetRef.current = bestTarget;
          snapCooldownRef.current = true;
          setTimeout(() => { snapCooldownRef.current = false; }, 200);
        }
        if (bestDist > UNSNAP_THRESHOLD) {
          unsnapWidget(winLabel).catch(() => {});
          setSnapEdge(null);
          snapTargetRef.current = "";
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
      // Remove snap relationships and reset saved position
      unsnapWidget(winLabel).catch(() => {});
      setSnapEdge(null);
      snapTargetRef.current = "";
      saveWindowState(pluginId, { x: undefined, y: undefined, height: undefined }).catch(() => {});
      await win.hide();
    } catch {}
  }, [win, winLabel, onClose, pluginId]);

  const contextValue = useMemo(() => ({ collapsed }), [collapsed]);

  return (
    <WidgetProvider value={contextValue}>
      <div className={`widget${snapEdge ? ` widget-snapped snap-edge-${snapEdge.toLowerCase()}` : ""}`}>
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
        <div className="widget-body" style={collapsed ? { display: "none" } : undefined} onMouseDown={handleBodyMouseDown}>{children}</div>
      </div>
    </WidgetProvider>
  );
}
