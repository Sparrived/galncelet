import { createContext, useContext } from "react";

/** Shared widget context available to all panels. */
export interface WidgetContext {
  /** Trigger a full refresh of the current widget's data */
  refresh: () => Promise<void>;
  /** Show a transient success message */
  showResult: (msg: string) => void;
  /** Show a transient error message */
  showError: (msg: string) => void;
  /** Notify the shell of status changes (displayed as window subtitle) */
  onStatusChange: (status: string | null) => void;
}

const ctx = createContext<WidgetContext>({
  refresh: async () => {},
  showResult: () => {},
  showError: () => {},
  onStatusChange: () => {},
});

export const WidgetProvider = ctx.Provider;

export function useWidget(): WidgetContext {
  return useContext(ctx);
}
