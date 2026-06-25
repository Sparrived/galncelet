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
