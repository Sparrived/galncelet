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
  /** True if this directory is a git submodule */
  isSubmodule?: boolean;
  children?: TreeNode[];
  expanded?: boolean;
}

export interface GitBranch {
  name: string;
  isCurrent: boolean;
  isRemote: boolean;
  upstream: string | null;
}

export interface SubmoduleInfo {
  path: string;
  name: string;
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
  savedRepos: [],
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
  router_status: "green" | "yellow" | "red";
  active_requests: number;
  total: UsageStats;
  caller_types: Record<string, UsageStats>;
  models: Record<string, UsageStats>;
  requested_models: Record<string, UsageStats>;
  keys: Record<string, Record<string, UsageStats>>;
}

/** AMKR model info for unified-model selection */
export interface AmkrModelInfo {
  id: string;
  aliases: string[];
  is_current: boolean;
}

/** AMKR WebSocket event */
export interface AmkrEvent {
  type: "metrics_snapshot" | "key_state_change" | "config_change" | "connected";
  data: any;
}

/** AMKR key state change event data */
export interface AmkrKeyStateChange {
  model_id: string;
  key_name: string;
  state: {
    failures: number;
    cooldown_remaining_seconds: number;
    last_status_code: number | null;
    disabled: boolean;
  };
}

/** CPU 信息 */
export interface CpuInfo {
  /** CPU 使用率 (%) */
  usage: number;
  /** CPU 核心数 */
  cores: number;
  /** CPU 频率 (Hz) */
  frequency: number;
  /** CPU 温度 (°C)，可能为 null */
  temperature: number | null;
}

/** 内存信息 */
export interface MemoryInfo {
  /** 已用内存 (bytes) */
  used: number;
  /** 总内存 (bytes) */
  total: number;
  /** 内存使用率 (0-1) */
  usage: number;
}

/** GPU 信息 */
export interface GpuInfo {
  /** GPU 名称 */
  name: string;
  /** GPU 使用率 (%) */
  usage: number;
  /** 已用显存 (bytes) */
  memory_used: number;
  /** 总显存 (bytes) */
  memory_total: number;
  /** GPU 温度 (°C)，可能为 null */
  temperature: number | null;
}

/** 磁盘信息 */
export interface DiskInfo {
  /** 已用磁盘空间 (bytes) */
  used: number;
  /** 总磁盘空间 (bytes) */
  total: number;
  /** 磁盘使用率 (0-1) */
  usage: number;
}

/** 网络信息 */
export interface NetworkInfo {
  /** 总上传字节数 */
  upload_bytes: number;
  /** 总下载字节数 */
  download_bytes: number;
  /** 上传速度 (bytes/s) */
  upload_speed: number;
  /** 下载速度 (bytes/s) */
  download_speed: number;
}

/** 系统性能指标 */
export interface SystemMetrics {
  /** CPU 信息 */
  cpu: CpuInfo;
  /** 内存信息 */
  memory: MemoryInfo;
  /** GPU 信息（可能为 null 如果没有检测到） */
  gpu: GpuInfo | null;
  /** 磁盘信息 */
  disk: DiskInfo;
  /** 网络信息 */
  network: NetworkInfo;
}
