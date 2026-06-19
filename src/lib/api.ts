import { invoke } from "@tauri-apps/api/core";
import type { GitStatus, GitDiff, GitBranch, GitLogEntry, AppSettings, AmkrMetrics, WindowState } from "./types";

export async function getStatus(repoPath?: string): Promise<GitStatus> {
  return invoke<GitStatus>("get_status", { repoPath: repoPath ?? null });
}

export async function getFileDiff(
  repoRoot: string,
  filePath: string,
  staged: boolean
): Promise<GitDiff> {
  return invoke<GitDiff>("get_file_diff", {
    repoRoot,
    filePath,
    staged,
  });
}

export async function selectFolder(): Promise<string | null> {
  return invoke<string | null>("select_folder");
}

export async function stageFile(repoRoot: string, filePath: string): Promise<void> {
  return invoke<void>("stage_file", { repoRoot, filePath });
}

export async function unstageFile(repoRoot: string, filePath: string): Promise<void> {
  return invoke<void>("unstage_file", { repoRoot, filePath });
}

export async function discardFile(
  repoRoot: string,
  filePath: string,
  statusCode: string
): Promise<void> {
  return invoke<void>("discard_file", { repoRoot, filePath, statusCode });
}

export async function commit(repoRoot: string, message: string): Promise<string> {
  return invoke<string>("commit", { repoRoot, message });
}

export async function pull(repoRoot: string): Promise<string> {
  return invoke<string>("pull", { repoRoot });
}

export async function push(repoRoot: string): Promise<string> {
  return invoke<string>("push", { repoRoot });
}

export async function gitFetch(repoRoot: string): Promise<string> {
  return invoke<string>("git_fetch", { repoRoot });
}

export async function listBranches(repoRoot: string): Promise<GitBranch[]> {
  return invoke<GitBranch[]>("list_branches", { repoRoot });
}

export async function checkoutBranch(repoRoot: string, branch: string): Promise<string> {
  return invoke<string>("checkout_branch", { repoRoot, branch });
}

export async function gitLog(repoRoot: string, maxCount?: number): Promise<GitLogEntry[]> {
  return invoke<GitLogEntry[]>("git_log", { repoRoot, maxCount: maxCount ?? 50 });
}

export async function untrackFile(repoRoot: string, filePath: string): Promise<void> {
  return invoke<void>("untrack_file", { repoRoot, filePath });
}

export async function loadSettings(): Promise<AppSettings> {
  return invoke<AppSettings>("load_settings");
}

export async function saveSettings(settings: AppSettings): Promise<void> {
  return invoke<void>("save_settings", { settings });
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
  defaultWhitelist: string[] = [],
): Promise<void> {
  return invoke<void>("create_plugin_window", { pluginId, title, width, height, defaultAttachEnabled, defaultWhitelist });
}

export async function openManageWindow(): Promise<void> {
  return invoke<void>("open_manage_window");
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

export interface WindowEntry {
  title: string;
  process: string;
}

export async function listVisibleWindows(): Promise<WindowEntry[]> {
  return invoke<WindowEntry[]>("list_visible_windows");
}

export async function fetchAmkrMetrics(): Promise<AmkrMetrics | null> {
  return invoke<AmkrMetrics | null>("fetch_amkr_metrics");
}
