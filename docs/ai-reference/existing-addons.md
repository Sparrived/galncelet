# Existing Addons Reference

> Quick lookup of all 6 plugins: their IPC commands, events, state, manifest values, and architectural choices.
> Target audience: AI coding agents. Exact data extracted from source code.

## Summary Table

| Plugin | ID | Path | IPC Commands | Events | Update Strategy | useAutoResize | useWidget | Manifest Size |
|--------|----|------|-------------|--------|----------------|---------------|-----------|---------------|
| System Monitor | `system-monitor` | `src/addons/system-monitor/` | 1 | 0 | Poll 2s | Yes | No | 320×150 |
| Clipboard History | `clipboard-history` | `src/addons/clipboard-history/` | 4 | 0 | Poll 1s | Yes | No | 340×400 |
| Page Notes | `page-notes` | `src/addons/page-notes/` | 4 | 0 | Poll 500ms | Yes | No | 360×160 |
| Music Player | `music-player` | `src/addons/music-player/` | 5 | 0 | Poll 1s/5s | Yes | No | 320×240 |
| AMKR Dashboard | `amkr` | `src/addons/amkr/` | 6 | 1 | Event-driven | Yes | No | 360×320 |
| Git Status | `git` | `src/addons/git/` | 22 | 2 | Event-driven | No | Yes | 360×800 |

---

## system-monitor

**Path**: `src/addons/system-monitor/`
**Files**: `index.tsx`, `manifest.json`, `types.ts`, `api.ts`, `SystemMonitorPanel.tsx`, `styles.css`
**Rust**: `src-tauri/src/system_monitor/mod.rs`

### Manifest

```json
{
  "id": "system-monitor",
  "title": "系统监控",
  "description": "实时监控主机 CPU、GPU、内存、磁盘、网络等性能指标",
  "icon": "🖥️",
  "defaultWidth": 320,
  "defaultHeight": 150,
  "showCloseButton": true,
  "showCollapseButton": true,
  "showAttachButton": false,
  "defaultAttachEnabled": false,
  "defaultWhitelist": []
}
```

### IPC Commands

| Command | Return Type | Description |
|---------|-------------|-------------|
| `fetch_system_metrics` | `SystemMetrics \| null` | All metrics in one call |

### Types

```ts
// src/addons/system-monitor/types.ts
SystemMetrics {
  cpu: CpuInfo,
  memory: MemoryInfo,
  gpu: GpuInfo | null,
  disk: DiskInfo,
  network: NetworkInfo
}
CpuInfo { usage: number, cores: number, frequency: number, temperature: number | null }
MemoryInfo { used: number, total: number, usage: number }
GpuInfo { name: string, usage: number, memory_used: number, memory_total: number, temperature: number | null }
DiskInfo { used: number, total: number, usage: number }
NetworkInfo { upload_bytes: number, download_bytes: number, upload_speed: number, download_speed: number }
```

### State

`metrics: SystemMetrics | null`

### Rust State

`SystemMonitorState` — contains `sys: Mutex<System>`, `networks: Mutex<Networks>`, `disks: Mutex<Disks>`, `last_network: Mutex<Option<NetworkSnapshot>>`, `nvml: Mutex<Option<Nvml>>`

### Notes

- Simplest plugin after clipboard-history
- Uses `RadialGauge` from shared components
- GPU section conditionally rendered (may be null)
- Color thresholds: green (<70%), amber (70-89%), red (≥90%)
- Uses WMI + sysinfo fallback for CPU temperature
- NVML for GPU metrics

---

## clipboard-history

**Path**: `src/addons/clipboard-history/`
**Files**: `index.tsx`, `manifest.json`, `api.ts`, `ClipboardPanel.tsx`, `styles.css`
**No separate types.ts** — types defined inline in api.ts

### Manifest

```json
{
  "id": "clipboard-history",
  "title": "剪贴板历史",
  "description": "自动记录剪贴板内容，支持搜索和快速回填",
  "icon": "📋",
  "defaultWidth": 340,
  "defaultHeight": 400,
  "showCloseButton": true,
  "showCollapseButton": true,
  "showAttachButton": false,
  "defaultAttachEnabled": false,
  "defaultWhitelist": []
}
```

### IPC Commands

| Command | Args | Return Type | Description |
|---------|------|-------------|-------------|
| `get_clipboard_history` | `{ query?: string }` | `ClipboardEntry[]` | Fetch entries, optional search |
| `copy_to_clipboard` | `{ id: string }` | `void` | Re-copy entry by ID |
| `delete_clipboard_entry` | `{ id: string }` | `void` | Delete one entry |
| `clear_clipboard_history` | — | `void` | Clear all entries |

### Types (inline in api.ts)

```ts
ClipboardEntry { id: string, text: string, timestamp: string }
```

### State

`entries: ClipboardEntry[]`, `query: string`, `copiedId: string | null`

### Rust Backend

`src-tauri/src/clipboard_history/mod.rs` — uses `clipboard_win` crate for Windows clipboard monitoring with background thread.

### Notes

- Simplest addon. No types.ts, no sub-components
- `copiedId` provides brief visual feedback (green highlight) on copy
- Background thread monitors clipboard changes

---

## page-notes

**Path**: `src/addons/page-notes/`
**Files**: `index.tsx`, `manifest.json`, `types.ts`, `api.ts`, `PageNotesPanel.tsx`, `styles.css`, `browser-extension/`
**Rust**: `src-tauri/src/page_notes/mod.rs`

### Manifest

```json
{
  "id": "page-notes",
  "title": "页面笔记",
  "description": "根据浏览器页面 URL 显示预设笔记，支持 substring 和 regex 匹配",
  "icon": "📝",
  "defaultWidth": 360,
  "defaultHeight": 160,
  "showCloseButton": true,
  "showCollapseButton": true,
  "showAttachButton": true,
  "defaultAttachEnabled": true,
  "defaultAttachRemember": false,
  "defaultWhitelist": [
    "chrome.exe",
    "msedge.exe",
    "firefox.exe",
    "brave.exe",
    "vivaldi.exe"
  ]
}
```

### IPC Commands

| Command | Args | Return Type | Description |
|---------|------|-------------|-------------|
| `load_page_notes` | — | `PageNotesConfig` | Load rules config |
| `save_page_notes` | `{ config: PageNotesConfig }` | `void` | Persist rules config |
| `get_browser_url` | — | `string \| null` | Current browser URL (framework command) |
| `get_ws_port` | — | `number` | WebSocket port for browser extension |

### Types

```ts
// src/addons/page-notes/types.ts
PageNoteRule {
  id: string,
  name: string,
  pattern: string,
  matchMode: "substring" | "regex",
  note: string,
  enabled: boolean
}
PageNotesConfig { rules: PageNoteRule[], wsPort: number }
```

### State

`config: PageNotesConfig`, `currentUrl: string`, `matchedNote: string | null`, `view: "match" | "rules"`, `editingId: string | null`, `showAdd: boolean`

### Rust Backend

`src-tauri/src/page_notes/mod.rs` — WebSocket server for browser extension communication.

### Notes

- Has companion Chrome extension in `browser-extension/` (Manifest V3, WebSocket communication)
- Fastest poll interval (500ms) for URL detection
- Two views: match view (shows matched note) and rules view (edit rules)
- Uses `get_browser_url` via `getBrowserUrl()` from frontend api.ts (exposed as `get_browser_url` in Rust)

---

## music-player

**Path**: `src/addons/music-player/`
**Files**: `index.tsx`, `manifest.json`, `types.ts`, `api.ts`, `MusicPanel.tsx`, `styles.css`
**Rust**: `src-tauri/src/music_player/mod.rs`

### Manifest

```json
{
  "id": "music-player",
  "title": "音乐播放",
  "description": "自动检测系统媒体播放器，显示正在播放的音乐信息并控制播放",
  "icon": "🎵",
  "defaultWidth": 320,
  "defaultHeight": 240,
  "showCloseButton": true,
  "showCollapseButton": true,
  "showAttachButton": false,
  "defaultAttachEnabled": false,
  "defaultWhitelist": []
}
```

### IPC Commands

| Command | Args | Return Type | Description |
|---------|------|-------------|-------------|
| `get_media_info` | — | `MediaInfo \| null` | Current media playback info |
| `media_control` | `{ action: MediaAction }` | `boolean` | Control playback |
| `get_media_sessions` | — | `MediaSessionInfo[]` | List all SMTC sessions |
| `select_media_session` | `{ session_id?: string }` | `void` | Select which session to control |
| `get_lyrics` | `{ title, artist, album }` | `Lyrics \| null` | Fetch lyrics (NetEase + LRCLIB) |

### Types

```ts
// src/addons/music-player/types.ts
MediaInfo {
  title: string,
  artist: string,
  album: string,
  thumbnail: string,
  duration_ms: number,
  position_ms: number,
  is_playing: boolean,
  shuffle: boolean,
  repeat_mode: string
}
MediaSessionInfo { id: string, app_name: string }
LyricLine { time_ms: number, text: string }
Lyrics { lines: LyricLine[], source: string, duration_ms?: number }
MediaAction = "Play" | "Pause" | "Toggle" | "Next" | "Previous"
           | { SetPosition: number }
           | { SetShuffle: boolean }
           | "CycleRepeat"
```

### State

`info: MediaInfo | null`, `position: number`, `seeking: boolean`, `sessions: MediaSessionInfo[]`, `selectedId: string | null`, `lyrics: Lyrics | null`

### Rust Backend

`src-tauri/src/music_player/mod.rs` — Uses Windows SMTC (System Media Transport Controls).

### Update Strategy

- Media info: poll every 1s
- Sessions: poll every 5s

### Notes

- Uses Windows SMTC (System Media Transport Controls) for media integration
- Client-side position estimation between polls
- Multi-source lyrics fetching (NetEase + LRCLIB) with caching
- Session picker to switch between active media sessions
- Compact layout with `useAutoResize` (uses `mp-bg-layer` wrapper for absolute-positioned backgrounds)

---

## amkr

**Path**: `src/addons/amkr/`
**Files**: `index.tsx`, `manifest.json`, `types.ts`, `api.ts`, `AmkrPanel.tsx`, `styles.css`, `components/Dashboard.tsx`
**Rust**: `src-tauri/src/amkr/mod.rs`

### Manifest

```json
{
  "id": "amkr",
  "title": "AMKR 仪表盘",
  "description": "Auto Model Key Router 实时指标监控",
  "icon": "📊",
  "defaultWidth": 360,
  "defaultHeight": 320,
  "showCloseButton": false,
  "showCollapseButton": false,
  "showAttachButton": false,
  "defaultAttachEnabled": false,
  "defaultWhitelist": []
}
```

### IPC Commands

| Command | Args | Return Type | Description |
|---------|------|-------------|-------------|
| `fetch_amkr_metrics` | — | `AmkrMetrics` | One-shot metrics fetch |
| `generate_commit_message` | `{ repo: string }` | `string` | AI-generated commit message (also used by git plugin) |
| `get_amkr_models` | — | `AmkrModelInfo[]` | List available models |
| `set_amkr_unified_model` | `{ model: string }` | `void` | Switch active model |
| `start_amkr_ws` | — | `void` | Start WebSocket listener |
| `stop_amkr_ws` | — | `void` | Stop WebSocket listener |

### Events

| Event | Payload | Description |
|-------|---------|-------------|
| `amkr-event` | `{ type: string, data: any }` | Real-time updates. `type="metrics_snapshot"` for metrics |

### Types

```ts
// src/addons/amkr/types.ts
AmkrMetrics {
  total_requests: number,
  success_rate: number,
  avg_latency_ms: number,
  tokens_used: number,
  active_keys: number,
  // ... additional fields
}
AmkrModelInfo { id: string, name: string, provider: string }
```

### State

`metrics: AmkrMetrics`, `models: AmkrModelInfo[]`, `loading: boolean`, `showDropdown: boolean`

### Rust Backend

`src-tauri/src/amkr/mod.rs` — WebSocket client connecting to AMKR backend + AI commit message generation.

### Notes

- Event-driven (WebSocket → Tauri event), not polling
- Uses `fmtNumber`, `fmtMs`, `fmtUptime`, `fmtPercent` from `src/lib/format`
- Uses shared components: `AnimatedNumber`, `ProgressBar`, `StatCard`, `MetricRow`
- Sub-component `Dashboard` renders the metrics layout
- Standalone dashboard: no close/collapse/attach buttons
- Provides `generate_commit_message` command used by git plugin for AI commit messages

---

## git

**Path**: `src/addons/git/`
**Files**: `index.tsx`, `manifest.json`, `types.ts`, `api.ts`, `GitPanel.tsx`, `styles.css`, `components/GitTree.tsx`, `components/DiffViewer.tsx`, `components/CommitTree.tsx`, `components/GitConsole.tsx`
**Rust**: `src-tauri/src/git/mod.rs`, `src-tauri/src/git/git_watcher.rs`

### Manifest

```json
{
  "id": "git",
  "title": "Git 状态",
  "description": "Git 仓库变更查看、暂存、提交、推送",
  "icon": "🔀",
  "defaultWidth": 360,
  "defaultHeight": 800,
  "showCloseButton": true,
  "showCollapseButton": true,
  "showAttachButton": true,
  "defaultAttachEnabled": true,
  "defaultWhitelist": [
    "powershell.exe",
    "pwsh.exe",
    "cmd.exe",
    "WindowsTerminal.exe",
    "Tabby.exe"
  ]
}
```

### IPC Commands

| Command | Args | Return Type | Description |
|---------|------|-------------|-------------|
| `get_status` | `{ repo: string }` | `GitStatus` | Repo status (staged/unstaged/untracked) |
| `get_file_diff` | `{ repo: string, file: string, staged: boolean }` | `string` | Unified diff for a file |
| `select_folder` | — | `string \| null` | Native folder picker dialog |
| `stage_file` | `{ repo: string, file: string }` | `void` | Stage a file |
| `stage_all` | `{ repo: string }` | `void` | Stage all changes |
| `unstage_file` | `{ repo: string, file: string }` | `void` | Unstage a file |
| `discard_file` | `{ repo: string, file: string }` | `void` | Discard changes |
| `untrack_file` | `{ repo: string, file: string }` | `void` | Remove from tracking |
| `commit` | `{ repo: string, message: string }` | `string` | Create commit |
| `pull` | `{ repo: string, rebase?: boolean }` | `string` | Pull from remote |
| `push` | `{ repo: string }` | `string` | Push to remote |
| `git_fetch` | `{ repo: string }` | `string` | Fetch from remote |
| `list_branches` | `{ repo: string }` | `GitBranch[]` | List all branches |
| `checkout_branch` | `{ repo: string, branch: string }` | `void` | Switch branch |
| `git_log` | `{ repo: string, maxCount?: number }` | `GitLogEntry[]` | Commit history |
| `list_submodules` | `{ repo: string }` | `GitSubmodule[]` | List submodules |
| `watch_git_repo` | `{ repo: string }` | `void` | Start watching repo for changes |
| `unwatch_git_repo` | `{ repo: string }` | `void` | Stop watching repo |
| `generate_commit_message` | `{ repo: string }` | `string` | AI-generated commit message (via AMKR) |
| `exec_git_command` | `{ repo: string, command: string }` | `GitCommandResult` | Raw git command execution |
| `list_remotes` | `{ repo: string }` | `GitRemoteInfo[]` | List remotes |
| `add_remote` | `{ repo: string, name: string, url: string }` | `void` | Add remote |
| `remove_remote` | `{ repo: string, name: string }` | `void` | Remove remote |

### Events

| Event | Payload | Description |
|-------|---------|-------------|
| `git-changed` | `{ repo: string }` | File watcher detected changes |
| `ai-commit-progress` | `{ chunk: string }` | Streaming AI commit message progress |

### Types

```ts
// src/addons/git/types.ts
GitStatus {
  repoRoot: string,
  branch: string,
  hasHead: boolean,
  files: GitFileEntry[],
  ahead: number,
  behind: number
}
GitFileEntry {
  path: string,
  statusCode: string,
  staged: boolean,
  additions: number,
  deletions: number
}
GitBranch { name: string, current: boolean, remote?: string }
GitLogEntry {
  hash: string, fullHash: string, parents: string[],
  author: string, date: string, message: string
}
GitSubmodule { name: string, path: string, url: string }
GitRemoteInfo { name: string, url: string }
GitDiff { filePath: string, diff: string, staged: boolean }
GitCommandResult { success: boolean, stdout: string, stderr: string }
```

### State

`settings`, `status`, `tree` (TreeNode[]), `diff`, `selectedFile`, `commitMsg`, `branches`, `logEntries`, `view` ("changes" | "history"), `fetching`, `aiGenerating`, `aiError`, `consoleLog`, `submodules`, `parentRepo`

### Rust Backend

`src-tauri/src/git/mod.rs` + `src-tauri/src/git/git_watcher.rs` — Uses `git2` crate + file watcher (`notify` crate).

### Notes

- Most complex addon (22 IPC commands, 4 sub-components)
- Only addon that uses `useWidget()` for `showResult`/`showError`/`onStatusChange`
- Only addon that does NOT use `useAutoResize` (fixed height 800px)
- Event-driven via file watcher (`notify` crate), not polling
- Uses `loadSettings()`/`saveSettings()` for persistent `savedRepos` and `currentRepo`
- Has dual view mode: changes (file tree + diff) and history (commit graph)
- AI commit message generation with streaming progress (uses AMKR's `generate_commit_message`)
- Submodule navigation support
- Remote management (add/remove/list)
- `select_folder` uses `tauri_plugin_dialog`
- `git_watcher` module handles filesystem watching for `git-changed` events
