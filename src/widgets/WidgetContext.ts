import { createContext, useContext, useEffect, useRef } from "react";

/** Height of the widget header bar (logical pixels). */
export const HEADER_H = 36;

export interface ContextMenuItem {
  label: string;
  icon?: string;
  onClick: () => void;
  danger?: boolean;
}

export interface WidgetContextValue {
  /** Whether the widget body is currently collapsed (hidden). */
  collapsed: boolean;
  /** Registered context menu items from plugins */
  contextMenuItems: ContextMenuItem[];
  /** Register plugin context menu items (replaces previous) */
  registerContextMenuItems: (items: ContextMenuItem[]) => void;
}

const WidgetContext = createContext<WidgetContextValue>({
  collapsed: false,
  contextMenuItems: [],
  registerContextMenuItems: () => {},
});

export const WidgetProvider = WidgetContext.Provider;

/** Read widget-level state from the nearest WidgetShell ancestor. */
export function useWidgetContext(): WidgetContextValue {
  return useContext(WidgetContext);
}

/**
 * Hook for plugins to register custom right-click menu items.
 * Items are automatically unregistered on unmount.
 */
export function useWidgetContextMenu(items: ContextMenuItem[]) {
  const { registerContextMenuItems } = useWidgetContext();
  const stable = useRef(items);
  stable.current = items;

  useEffect(() => {
    registerContextMenuItems(stable.current);
    return () => registerContextMenuItems([]);
  }, [registerContextMenuItems]);
}
