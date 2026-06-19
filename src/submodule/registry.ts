import type { FC } from "react";

/** Plugin definition — the contract every plugin must satisfy. */
export interface PluginDef {
  /** Unique identifier (used as settings key, window label suffix) */
  id: string;
  /** Display title */
  title: string;
  /** Short description shown in management page */
  description?: string;
  /** Icon emoji shown in management page */
  icon?: string;
  /** Height consumed when widget is collapsed (logical px). 0 = hide entirely. */
  collapsedHeight?: number;
  /** Default window width (logical px) */
  defaultWidth?: number;
  /** Default window height (logical px) */
  defaultHeight?: number;
  /** Show close button in header (default true) */
  showCloseButton?: boolean;
  /** Show collapse/expand button in header (default true) */
  showCollapseButton?: boolean;
  /** Show attach-to-foreground toggle button (default true) */
  showAttachButton?: boolean;
  /** Whether this widget attaches to foreground window by default (default true) */
  defaultAttachEnabled?: boolean;
  /** Default whitelist — window title substrings to attach to */
  defaultWhitelist?: string[];
  /** The widget React component */
  component: FC;
}

const registry = new Map<string, PluginDef>();

/** Register a plugin. Call this at module load time (side-effect import). */
export function registerPlugin(def: PluginDef): void {
  registry.set(def.id, def);
}

/** Get a single plugin by id. */
export function getPlugin(id: string): PluginDef | undefined {
  return registry.get(id);
}

/** Get all registered plugins. */
export function getAllPlugins(): PluginDef[] {
  return Array.from(registry.values());
}
