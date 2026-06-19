export interface GitFileEntry {
  /** Relative path from repo root */
  path: string;
  /** Single-char git status code: M, A, D, R, C, ? etc. */
  statusCode: string;
  /** Whether change is staged */
  staged: boolean;
}

export interface GitStatus {
  /** Repository root absolute path */
  repoRoot: string;
  /** Current branch name, or "DETACHED" */
  branch: string;
  /** Whether HEAD exists (repo has at least one commit) */
  hasHead: boolean;
  files: GitFileEntry[];
}

export interface GitDiff {
  /** File path relative to repo root */
  filePath: string;
  /** Raw unified diff text */
  diff: string;
  /** Whether this is a staged diff */
  staged: boolean;
}

/** Tree node built from flat file entries */
export interface TreeNode {
  name: string;
  path: string;
  type: "file" | "dir";
  /** Only set for files */
  statusCode?: string;
  /** Only set for files */
  staged?: boolean;
  children?: TreeNode[];
  expanded?: boolean;
}

export interface GitBranch {
  name: string;
  isCurrent: boolean;
  isRemote: boolean;
  upstream: string | null;
}

export interface GitLogEntry {
  hash: string;
  fullHash: string;
  parents: string[];
  author: string;
  date: string;
  message: string;
}

/** Per-window persisted state */
export interface WindowState {
  x?: number;
  y?: number;
  height?: number;
  attachEnabled?: boolean;
  /** Attach whitelist — foreground window title substrings. Empty = no restriction. */
  whitelist?: string[];
}

/** Persisted application settings */
export interface AppSettings {
  refreshIntervalMs: number;
  cardWidth: number;
  logMaxCount: number;
  alwaysOnTop: boolean;
  pullRebase: boolean;
  /** Panel visibility keyed by panel id. Missing key = visible. */
  panelVisibility: Record<string, boolean>;
  /** Per-window persisted state */
  windowStates: Record<string, WindowState>;
}

export const DEFAULT_SETTINGS: AppSettings = {
  refreshIntervalMs: 2000,
  cardWidth: 360,
  logMaxCount: 50,
  alwaysOnTop: true,
  pullRebase: true,
  panelVisibility: { git: true, amkr: true },
  windowStates: {},
};

/** AMKR per-model or total usage stats */
export interface UsageStats {
  requests: number;
  successes: number;
  failures: number;
  retries: number;
  prompt_tokens: number;
  completion_tokens: number;
  total_tokens: number;
  cached_tokens: number;
  cache_hit_rate: number;
  cached_token_rate: number;
  avg_duration_ms: number;
  min_duration_ms: number;
  max_duration_ms: number;
  avg_first_token_ms: number;
  min_first_token_ms: number;
  max_first_token_ms: number;
  status_codes: Record<string, number>;
}

/** AMKR /metrics endpoint response */
export interface AmkrMetrics {
  started_at: string;
  rate_window_seconds: number;
  current_rpm: number;
  current_tpm: number;
  total: UsageStats;
  caller_types: Record<string, UsageStats>;
  models: Record<string, UsageStats>;
  requested_models: Record<string, UsageStats>;
  keys: Record<string, Record<string, UsageStats>>;
}
