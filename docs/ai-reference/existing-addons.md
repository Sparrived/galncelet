# Existing Addons Reference

> Quick lookup of all 6 plugins: their IPC commands, events, state, and architectural choices.

## Summary Table

| Plugin | ID | IPC Commands | Events | Update Strategy | useAutoResize | useWidget |
|--------|----|-------------|--------|----------------|---------------|-----------|
| System Monitor | `system-monitor` | 1 | 0 | Poll 2s | Yes | No |
| Clipboard History | `clipboard-history` | 4 | 0 | Poll 1s | Yes | No |
| Page Notes | `page-notes` | 4 | 0 | Poll 500ms | Yes | No |
| AMKR Dashboard | `amkr` | 5 | 1 | Event-driven | Yes | No |
| Git Status | `git` | 18 | 2 | Event-driven | No | Yes |
| Music Player | `music-player` | 5 | 0 | Poll 1s/5s | Yes | No |

---

## system-monitor

**Path**: `src/addons/system-monitor/`
**Files**: `index.tsx`, `manifest.json`, `types.ts`, `api.ts`, `SystemMonitorPanel.tsx`, `styles.css`
**Manifest**: 320×150, no attach, close+collapse

### IPC Commands

| Command | Return Type | Description |
|---------|-------------|-------------|
| `fetch_system_metrics` | `SystemMetrics \| null` | All metrics in one call |

### Types

```ts
SystemMetrics { cpu: CpuInfo, memory: MemoryInfo, gpu: GpuInfo|null, disk: DiskInfo, network: NetworkInfo }
CpuInfo { usage: number, cores: number, frequency: number, temperature: number|null }
MemoryInfo { used: number, total: number, usage: number }
GpuInfo { name: string, usage: number, memory_used: number, memory_total: number, temperature: number|null }
DiskInfo { used: number, total: number, usage: number }
NetworkInfo { upload_bytes: number, download_bytes: number, upload_speed: number, download_speed: number }
```

### State

`metrics: SystemMetrics | null`

### Notes

- Simplest plugin after clipboard-history
- Uses `RadialGauge` from shared components
- GPU section conditionally rendered (may be null)
- Color thresholds: green (<70%), amber (70-89%), red (≥90%)

---

## clipboard-history

**Path**: `src/addons/clipboard-history/`
**Files**: `index.tsx`, `manifest.json`, `api.ts`, `ClipboardPanel.tsx`, `styles.css`
**Manifest**: 340×400, no attach, close+collapse

### IPC Commands

| Command | Args | Return Type | Description |
|---------|------|-------------|-------------|
| `get_clipboard_history` | `{ query?: string }` | `ClipboardEntry[]` | Fetch entries, optional search |
| `copy_to_clipboard` | `{ id: string }` | `void` | Re-copy entry by ID |
| `delete_clipboard_entry` | `{ id: string }` | `void` | Delete one entry |
| `clear_clipboard_history` | — | `void` | Clear all entries |

### Types

```ts
// Inline in api.ts (no separate types.ts)
ClipboardEntry { id: string, text: string, timestamp: string }
```

### State

`entries: ClipboardEntry[]`, `query: string`, `copiedId: string | null`

### Notes

- Simplest addon. No types.ts, no sub-components
- `copiedId` provides brief visual feedback (green highlight) on copy

---

## page-notes

**Path**: `src/addons/page-notes/`
**Files**: `index.tsx`, `manifest.json`, `types.ts`, `api.ts`, `PageNotesPanel.tsx`, `styles.css`, `browser-extension/`
**Manifest**: 360×160, attach to browsers, close+collapse
**Default Whitelist**: `chrome.exe`, `msedge.exe`, `firefox.exe`, `brave.exe`, `vivaldi.exe`

### IPC Commands

| Command | Args | Return Type | Description |
|---------|------|-------------|-------------|
| `load_page_notes` | — | `PageNotesConfig` | Load rules config |
| `save_page_notes` | `{ config: PageNotesConfig }` | `void` | Persist rules config |
| `get_browser_url` | — | `string \| null` | Current browser URL |
| `get_ws_port` | — | `number` | WebSocket port for browser extension |

### Types

```ts
PageNoteRule { id: string, name: string, pattern: string, matchMode: "substring"|"regex", note: string, enabled: boolean }
PageNotesConfig { rules: PageNoteRule[], wsPort: number }
```

### State

`config: PageNotesConfig`, `currentUrl: string`, `matchedNote: string | null`, `view: "match"|"rules"`, `editingId: string|null`, `showAdd: boolean`

### Notes

- Has companion Chrome extension in `browser-extension/` (Manifest V3, WebSocket communication)
- Fastest poll interval (500ms) for URL detection
- Two views: match view (shows matched note) and rules view (edit rules)

---

## amkr

**Path**: `src/addons/amkr/`
**Files**: `index.tsx`, `manifest.json`, `types.ts`, `api.ts`, `AmkrPanel.tsx`, `styles.css`, `components/Dashboard.tsx`
**Manifest**: 360×320, no close/collapse/attach buttons (standalone dashboard)

### IPC Commands

| Command | Args | Return Type | Description |
|---------|------|-------------|-------------|
| `fetch_amkr_metrics` | — | `AmkrMetrics` | One-shot metrics fetch |
| `get_amkr_models` | — | `AmkrModelInfo[]` | List available models |
| `set_amkr_unified_model` | `{ model: string }` | `void` | Switch active model |
| `start_amkr_ws` | — | `void` | Start WebSocket listener |
| `stop_amkr_ws` | — | `void` | Stop WebSocket listener |

### Events

| Event | Payload | Description |
|-------|---------|-------------|
| `amkr-event` | `{ type: string, data: any }` | Real-time updates. `type="metrics_snapshot"` for metrics |

### State

`metrics: AmkrMetrics`, `models: AmkrModelInfo[]`, `loading: boolean`, `showDropdown: boolean`

### Notes

- Event-driven (WebSocket → Tauri event), not polling
- Uses `fmtNumber`, `fmtMs`, `fmtUptime`, `fmtPercent` from `src/lib/format`
- Uses shared components: `AnimatedNumber`, `ProgressBar`, `StatCard`, `MetricRow`
- Sub-component `Dashboard` renders the metrics layout

---

## git

**Path**: `src/addons/git/`
**Files**: `index.tsx`, `manifest.json`, `types.ts`, `api.ts`, `GitPanel.tsx`, `styles.css`, `components/GitTree.tsx`, `components/DiffViewer.tsx`, `components/CommitTree.tsx`, `components/GitConsole.tsx`
**Manifest**: 360×800 (tallest), attach to terminals, close+collapse+attach
**Default Whitelist**: `powershell.exe`, `pwsh.exe`, `cmd.exe`, `WindowsTerminal.exe`, `Tabby.exe`

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
| `commit` | `{ repo: string, message: string }` | `void` | Create commit |
| `pull` | `{ repo: string, rebase: boolean }` | `void` | Pull from remote |
| `push` | `{ repo: string }` | `void` | Push to remote |
| `git_fetch` | `{ repo: string }` | `void` | Fetch from remote |
| `list_branches` | `{ repo: string }` | `GitBranch[]` | List all branches |
| `checkout_branch` | `{ repo: string, branch: string }` | `void` | Switch branch |
| `git_log` | `{ repo: string, maxCount?: number }` | `GitLogEntry[]` | Commit history |
| `list_submodules` | `{ repo: string }` | `GitSubmodule[]` | List submodules |
| `generate_commit_message` | `{ repo: string }` | `string` | AI-generated commit message |
| `exec_git_command` | `{ repo: string, args: string[] }` | `string` | Raw git command execution |

### Events

| Event | Payload | Description |
|-------|---------|-------------|
| `git-changed` | `{ repo: string }` | File watcher detected changes |
| `ai-commit-progress` | `{ chunk: string }` | Streaming AI commit message progress |

### Types

```ts
GitStatus { branch: string, ahead: number, behind: number, files: GitFileEntry[] }
GitFileEntry { path: string, status: string, staged: boolean }
GitBranch { name: string, current: boolean, remote?: string }
GitLogEntry { hash: string, fullHash: string, parents: string[], author: string, date: string, message: string }
GitSubmodule { name: string, path: string, url: string }
```

### State

`settings`, `status`, `tree` (TreeNode[]), `diff`, `selectedFile`, `commitMsg`, `branches`, `logEntries`, `view` ("changes"|"history"), `fetching`, `aiGenerating`, `aiError`, `consoleLog`, `submodules`, `parentRepo`

### Notes

- Most complex addon (18 IPC commands, 4 sub-components)
- Only addon that uses `useWidget()` for `showResult`/`showError`/`onStatusChange`
- Only addon that does NOT use `useAutoResize` (fixed height 800px)
- Event-driven via file watcher, not polling
- Uses `loadSettings()`/`saveSettings()` for persistent `savedRepos`
- Has dual view mode: changes (file tree + diff) and history (commit graph)
- AI commit message generation with streaming progress
- Submodule navigation support

---

## music-player

**Path**: `src/addons/music-player/`
**Files**: `index.tsx`, `manifest.json`, `types.ts`, `api.ts`, `MusicPanel.tsx`, `styles.css`
**Manifest**: 320×240, no attach, close+collapse

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
MediaInfo { title: string, artist: string, album: string, thumbnail: string, duration_ms: number, position_ms: number, is_playing: boolean, shuffle: boolean, repeat_mode: string }
MediaSessionInfo { id: string, app_name: string }
LyricLine { time_ms: number, text: string }
Lyrics { lines: LyricLine[], source: string, duration_ms?: number }
```

### State

`info`, `position`, `seeking`, `sessions`, `selectedId`, `lyrics`

### Update Strategy

- Media info: poll every 1s
- Sessions: poll every 5s

### Notes

- Uses Windows SMTC (System Media Transport Controls) for media integration
- Client-side position estimation between polls
- Multi-source lyrics fetching (NetEase + LRCLIB) with caching
- Session picker to switch between active media sessions
- Compact layout with `useAutoResize` (uses `mp-bg-layer` wrapper for absolute-positioned backgrounds)
