import { invoke } from "@tauri-apps/api/core";
import type { AppSettings, WindowState } from "./types";

export async function loadSettings(): Promise<AppSettings> {
  return invoke<AppSettings>("load_settings");
}

export async function saveSettings(settings: AppSettings): Promise<void> {
  return invoke<void>("save_settings", { settings });
}

export async function setPluginVisible(pluginId: string, visible: boolean): Promise<void> {
  return invoke<void>("set_plugin_visible", { pluginId, visible });
}

export async function updateCardWidth(width: number): Promise<void> {
  return invoke<void>("update_card_width", { width });
}

/**
 * Tell the attach loop the window is collapsed with a fixed physical height,
 * or expanded (pass null to restore normal mode).
 */
export async function setBodyCollapsed(
  windowLabel: string,
  height: number | null,
  expandHeight?: number,
): Promise<void> {
  return invoke<void>("set_body_collapsed", { windowLabel, height, expandHeight });
}

export async function setAttachEnabled(windowLabel: string, enabled: boolean): Promise<void> {
  return invoke<void>("set_attach_enabled", { windowLabel, enabled });
}

export async function createPluginWindow(
  pluginId: string,
  title: string,
  width: number,
  height: number,
  defaultAttachEnabled: boolean = true,
  defaultAttachRemember: boolean = false,
  defaultWhitelist: string[] = [],
): Promise<void> {
  return invoke<void>("create_plugin_window", { pluginId, title, width, height, defaultAttachEnabled, defaultAttachRemember, defaultWhitelist });
}

export interface RuntimeAddonInfo {
  id: string;
  title: string;
  description?: string | null;
  icon?: string | null;
  entry: string;
  defaultWidth?: number | null;
  defaultHeight?: number | null;
  showCloseButton?: boolean | null;
  showCollapseButton?: boolean | null;
  showAttachButton?: boolean | null;
  defaultAttachEnabled?: boolean | null;
  defaultAttachRemember?: boolean | null;
  defaultWhitelist: string[];
  permissions: string[];
  hasBackend: boolean;
}

export async function listRuntimeAddons(): Promise<RuntimeAddonInfo[]> {
  return invoke<RuntimeAddonInfo[]>("list_runtime_addons");
}

export async function getRuntimeAddonsDir(): Promise<string> {
  return invoke<string>("get_runtime_addons_dir");
}

export async function openRuntimeAddonsDir(): Promise<string> {
  return invoke<string>("open_runtime_addons_dir");
}

export async function createRuntimeAddonWindow(addonId: string): Promise<void> {
  return invoke<void>("create_runtime_addon_window", { addonId });
}

export async function invokeRuntimeAddon<T = unknown>(addonId: string, method: string, params: unknown = null): Promise<T> {
  return invoke<T>("invoke_runtime_addon", { addonId, method, params });
}

export async function runtimeAddonStorageGet<T = unknown>(addonId: string, key: string): Promise<T | null> {
  return invoke<T | null>("runtime_addon_storage_get", { addonId, key });
}

export async function runtimeAddonStorageSet(addonId: string, key: string, value: unknown): Promise<void> {
  return invoke<void>("runtime_addon_storage_set", { addonId, key, value });
}

export async function runtimeAddonStorageDelete(addonId: string, key: string): Promise<void> {
  return invoke<void>("runtime_addon_storage_delete", { addonId, key });
}

export async function openManageWindow(): Promise<void> {
  return invoke<void>("open_manage_window");
}

export async function openSettingsWindow(): Promise<void> {
  return invoke<void>("open_settings_window");
}

export async function openPluginSettings(pluginId: string): Promise<void> {
  return invoke<void>("open_plugin_settings", { pluginId });
}

export async function saveWindowState(windowId: string, state: WindowState): Promise<void> {
  return invoke<void>("save_window_state", { windowId, state });
}

export async function setAttachWhitelist(windowLabel: string, patterns: string[]): Promise<void> {
  return invoke<void>("set_attach_whitelist", { windowLabel, patterns });
}

export async function setAttachRemember(windowLabel: string, remember: boolean): Promise<void> {
  return invoke<void>("set_attach_remember", { windowLabel, remember });
}

export async function setHideInFullscreen(enabled: boolean): Promise<void> {
  return invoke<void>("set_hide_in_fullscreen", { enabled });
}

export async function setStartOnBoot(enabled: boolean): Promise<void> {
  return invoke<void>("set_start_on_boot", { enabled });
}
export interface UpdateCheckResult {
  currentVersion: string;
  latestVersion: string | null;
  latestTag: string | null;
  releaseName: string | null;
  releaseUrl: string | null;
  publishedAt: string | null;
  hasUpdate: boolean;
}

export async function checkForUpdates(): Promise<UpdateCheckResult> {
  return invoke<UpdateCheckResult>("check_for_updates");
}

export interface WindowEntry {
  title: string;
  process: string;
}

export async function listVisibleWindows(): Promise<WindowEntry[]> {
  return invoke<WindowEntry[]>("list_visible_windows");
}

export async function setPluginHotkey(pluginId: string, hotkey: string | null): Promise<void> {
  return invoke<void>("set_plugin_hotkey", { pluginId, hotkey });
}

export async function setWidgetSequence(sequence: string[]): Promise<void> {
  return invoke<void>("set_widget_sequence", { sequence });
}

export async function setSequenceHotkey(hotkey: string | null): Promise<void> {
  return invoke<void>("set_sequence_hotkey", { hotkey });
}
