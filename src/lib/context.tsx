import { createContext, useContext } from "react";

/** Shared widget context available to all panels. */
export interface WidgetContext {
  /** Current repo root path, or null if no repo loaded */
  repoRoot: string | null;
  /** Current branch name, or null */
  branch: string | null;
  /** Trigger a full git status refresh */
  refresh: () => Promise<void>;
  /** Show a transient success message */
  showResult: (msg: string) => void;
  /** Show a transient error message */
  showError: (msg: string) => void;
  /** Notify the shell of status changes (repo name, branch) */
  onStatusChange: (repoRoot: string | null, branch: string | null) => void;
}

const ctx = createContext<WidgetContext>({
  repoRoot: null,
  branch: null,
  refresh: async () => {},
  showResult: () => {},
  showError: () => {},
  onStatusChange: () => {},
});

export const WidgetProvider = ctx.Provider;

export function useWidget(): WidgetContext {
  return useContext(ctx);
}
