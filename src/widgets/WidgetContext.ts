import { createContext, useContext } from "react";

/** Height of the widget header bar (logical pixels). */
export const HEADER_H = 36;

export interface WidgetContextValue {
  /** Whether the widget body is currently collapsed (hidden). */
  collapsed: boolean;
}

const WidgetContext = createContext<WidgetContextValue>({ collapsed: false });

export const WidgetProvider = WidgetContext.Provider;

/** Read widget-level state from the nearest WidgetShell ancestor. */
export function useWidgetContext(): WidgetContextValue {
  return useContext(WidgetContext);
}
