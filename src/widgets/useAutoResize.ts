import { useEffect, useRef, useCallback } from "react";
import { getCurrentWindow, LogicalSize } from "@tauri-apps/api/window";
import { loadSettings } from "../lib/api";
import { HEADER_H, useWidgetContext } from "./WidgetContext";

/**
 * Auto-resize the widget window to fit content.
 *
 * @param containerRef - Ref to the container element to observe
 *
 * Usage:
 * ```tsx
 * const containerRef = useRef<HTMLDivElement>(null);
 * useAutoResize(containerRef);
 * return <div ref={containerRef}>...</div>;
 * ```
 */
export function useAutoResize(
  containerRef: React.RefObject<HTMLElement | null>,
): void {
  const { collapsed } = useWidgetContext();
  const cardWidthRef = useRef(0);
  const lastHeightRef = useRef(0);

  // Load card width once
  useEffect(() => {
    loadSettings().then((s) => {
      cardWidthRef.current = s.cardWidth;
    }).catch(() => {});
  }, []);

  // Resize function - calculate actual content height
  const apply = useCallback(() => {
    if (!cardWidthRef.current || collapsed || !containerRef.current) return;

    // Find the widget-body parent which contains all content
    const el = containerRef.current;
    let maxHeight = 0;

    // Walk through all children to find the maximum bottom position
    const rect = el.getBoundingClientRect();
    for (let i = 0; i < el.children.length; i++) {
      const child = el.children[i] as HTMLElement;
      if (child) {
        const childRect = child.getBoundingClientRect();
        const bottom = childRect.bottom - rect.top;
        if (bottom > maxHeight) {
          maxHeight = bottom;
        }
      }
    }

    // Also check scrollHeight as fallback
    const h = Math.max(maxHeight, el.scrollHeight);

    const total = Math.max(h, 60) + HEADER_H;

    if (Math.abs(total - lastHeightRef.current) < 4) return;
    lastHeightRef.current = total;

    const win = getCurrentWindow();
    win
      .setSize(new LogicalSize(cardWidthRef.current, total))
      .catch(() => {});
  }, [collapsed, containerRef]);

  // Observe content changes
  useEffect(() => {
    const el = containerRef.current;
    if (!el) return;

    // Initial size with delay to ensure layout
    const initTimer = setTimeout(() => {
      apply();
    }, 100);

    // Use requestAnimationFrame for smoother updates
    let rafId: number | null = null;
    const scheduleApply = () => {
      if (rafId) cancelAnimationFrame(rafId);
      rafId = requestAnimationFrame(() => {
        apply();
        rafId = null;
      });
    };

    const observer = new ResizeObserver(() => {
      scheduleApply();
    });
    observer.observe(el);

    // Also observe all children
    const mutationObserver = new MutationObserver(() => {
      scheduleApply();
    });
    mutationObserver.observe(el, {
      childList: true,
      subtree: true,
      attributes: true,
      characterData: true,
    });

    return () => {
      clearTimeout(initTimer);
      if (rafId) cancelAnimationFrame(rafId);
      observer.disconnect();
      mutationObserver.disconnect();
    };
  }, [containerRef, apply]);
}
