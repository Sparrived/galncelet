import { invoke } from "@tauri-apps/api/core";
import type { GitStatus, GitDiff, GitBranch, GitLogEntry, SubmoduleInfo, AppSettings, AmkrMetrics, WindowState, SystemMetrics } from "./types";

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

export async function stageAll(repoRoot: string): Promise<void> {
  return invoke<void>("stage_all", { repoRoot });
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

export async function listSubmodules(repoRoot: string): Promise<SubmoduleInfo[]> {
  return invoke<SubmoduleInfo[]>("list_submodules", { repoRoot });
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

export async function fetchAmkrMetrics(): Promise<AmkrMetrics | null> {
  return invoke<AmkrMetrics | null>("fetch_amkr_metrics");
}

export async function generateCommitMessage(repoRoot: string): Promise<string> {
  return invoke<string>("generate_commit_message", { repoRoot });
}

export interface AmkrModelInfo {
  id: string;
  aliases: string[];
  is_current: boolean;
}

export async function getAmkrModels(): Promise<AmkrModelInfo[] | null> {
  return invoke<AmkrModelInfo[] | null>("get_amkr_models");
}

export async function setAmkrUnifiedModel(modelId: string): Promise<void> {
  return invoke<void>("set_amkr_unified_model", { modelId });
}

export async function startAmkrWs(): Promise<void> {
  return invoke<void>("start_amkr_ws");
}

export async function stopAmkrWs(): Promise<void> {
  return invoke<void>("stop_amkr_ws");
}

export async function fetchSystemMetrics(): Promise<SystemMetrics | null> {
  return invoke<SystemMetrics | null>("fetch_system_metrics");
}

export async function watchGitRepo(repoPath: string): Promise<void> {
  return invoke<void>("watch_git_repo", { repoPath });
}

export async function unwatchGitRepo(repoPath: string): Promise<void> {
  return invoke<void>("unwatch_git_repo", { repoPath });
}

export interface GitCommandResult {
  success: boolean;
  stdout: string;
  stderr: string;
}

export async function execGitCommand(repoRoot: string, command: string): Promise<GitCommandResult> {
  return invoke<GitCommandResult>("exec_git_command", { repoRoot, command });
}

// ─── Widget Snap ───

export type SnapEdge = "Top" | "Bottom" | "Left" | "Right";

export interface SnapTarget {
  target_label: string;
  edge: SnapEdge;
  offset: number;
}

export interface WidgetRect {
  x: number;
  y: number;
  w: number;
  h: number;
  attach_enabled: boolean;
}

export async function snapWidget(label: string, targetLabel: string, edge: SnapEdge, offset: number): Promise<void> {
  return invoke<void>("snap_widget", { label, targetLabel, edge, offset });
}

export async function unsnapWidget(label: string): Promise<void> {
  return invoke<void>("unsnap_widget", { label });
}

export async function getSnapInfo(label: string): Promise<SnapTarget | null> {
  return invoke<SnapTarget | null>("get_snap_info", { label });
}

export async function getAllWidgetRects(): Promise<Record<string, WidgetRect>> {
  return invoke<Record<string, WidgetRect>>("get_all_widget_rects");
}

export async function moveSnapGroup(label: string, dx: number, dy: number): Promise<void> {
  return invoke<void>("move_snap_group", { label, dx, dy });
}
