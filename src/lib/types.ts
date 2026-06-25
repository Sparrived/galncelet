/** Per-window persisted state */
export interface WindowState {
  x?: number;
  y?: number;
  height?: number;
  attachEnabled?: boolean;
  /** Attach whitelist — foreground window title substrings. Empty = no restriction. */
  whitelist?: string[];
  /** When true, attach only manages show/hide, not position. */
  attachRemember?: boolean;
}

/** Persisted application settings */
export interface AppSettings {
  refreshIntervalMs: number;
  cardWidth: number;
  logMaxCount: number;
  alwaysOnTop: boolean;
  pullRebase: boolean;
  /** Saved repository paths */
  savedRepos: string[];
  /** Last active repository path (persisted across sessions) */
  currentRepo?: string;
  /** Panel visibility keyed by panel id. Missing key = visible. */
  panelVisibility: Record<string, boolean>;
  /** Per-window persisted state */
  windowStates: Record<string, WindowState>;
  /** Hide all widgets when the focused window is fullscreen */
  hideFullscreen: boolean;
  /** Plugin hotkeys: pluginId → shortcut string (e.g. "ctrl+shift+1") */
  pluginHotkeys: Record<string, string>;
  /** Widget sequence: ordered plugin IDs sharing the same position */
  widgetSequence: string[];
  /** Hotkey to cycle through the widget sequence */
  sequenceHotkey: string | null;
}

export const DEFAULT_SETTINGS: AppSettings = {
  refreshIntervalMs: 2000,
  cardWidth: 360,
  logMaxCount: 50,
  alwaysOnTop: true,
  pullRebase: true,
  savedRepos: [],
  panelVisibility: {},
  windowStates: {},
  hideFullscreen: true,
  pluginHotkeys: {},
  widgetSequence: [],
  sequenceHotkey: null,
};
