# 现有插件索引（AI 版）

本文件帮助 AI 快速理解当前内置插件的职责、文件位置和常见修改入口。修改插件前仍需读取对应源码。

## 总览

| 插件 ID | 前端入口 | 后端模块 | 主要能力 |
| --- | --- | --- | --- |
| `git` | `src/addons/git/index.tsx` | `src-tauri/src/git/mod.rs` | Git 状态、diff、暂存、提交、分支、远程、watcher |
| `system-monitor` | `src/addons/system-monitor/index.tsx` | `src-tauri/src/system_monitor/mod.rs` | CPU、GPU、内存、磁盘、网络指标 |
| `clipboard-history` | `src/addons/clipboard-history/index.tsx` | `src-tauri/src/clipboard_history/mod.rs` | 剪贴板监听、历史列表、搜索、复制回填、持久化 |
| `page-notes` | `src/addons/page-notes/index.tsx` | `src-tauri/src/page_notes/mod.rs` | 按浏览器 URL 匹配笔记、WebSocket、浏览器扩展资源 |
| `music-player` | `src/addons/music-player/index.tsx` | `src-tauri/src/music_player/mod.rs` | 系统媒体会话、播放控制、进度、歌词 |
| `amkr` | `src/addons/amkr/index.tsx` | `src-tauri/src/amkr/mod.rs` | AMKR 指标、模型配置、提交信息生成、事件 WebSocket |

## `git`

前端：

- `src/addons/git/GitPanel.tsx`
- `src/addons/git/api.ts`
- `src/addons/git/types.ts`
- `src/addons/git/components/GitTree.tsx`
- `src/addons/git/components/DiffViewer.tsx`
- `src/addons/git/components/GitConsole.tsx`
- `src/addons/git/components/CommitTree.tsx`
- `src/addons/git/styles.css`

后端：

- `src-tauri/src/git/mod.rs`
- `src-tauri/src/git/git_watcher.rs`

命令：

- `get_status`
- `get_file_diff`
- `exec_git_command`
- `stage_file`
- `stage_all`
- `unstage_file`
- `discard_file`
- `untrack_file`
- `commit`
- `pull`
- `push`
- `git_fetch`
- `list_branches`
- `checkout_branch`
- `git_log`
- `list_submodules`
- `list_remotes`
- `add_remote`
- `remove_remote`
- `watch_git_repo`
- `unwatch_git_repo`

常见修改：

- UI 布局、文件树、diff 展示：改前端 components。
- Git 命令行为：改 `src-tauri/src/git/mod.rs`。
- 自动刷新和 watcher：改 `git_watcher.rs` 与 `GitPanel.tsx`。

## `system-monitor`

前端：

- `src/addons/system-monitor/SystemMonitorPanel.tsx`
- `src/addons/system-monitor/api.ts`
- `src/addons/system-monitor/types.ts`
- `src/addons/system-monitor/styles.css`

后端：

- `src-tauri/src/system_monitor/mod.rs`

命令：

- `fetch_system_metrics`

常见修改：

- 指标采集：改 Rust `SystemMonitorState` 和 command。
- 展示样式：改 `SystemMonitorPanel.tsx` 与 `styles.css`。
- 新增字段：同步 Rust 返回结构体、前端 `types.ts` 和 UI。

## `clipboard-history`

前端：

- `src/addons/clipboard-history/ClipboardPanel.tsx`
- `src/addons/clipboard-history/api.ts`
- `src/addons/clipboard-history/styles.css`

后端：

- `src-tauri/src/clipboard_history/mod.rs`

命令：

- `get_clipboard_history`
- `copy_to_clipboard`
- `delete_clipboard_entry`
- `clear_clipboard_history`

常见修改：

- 监听和去重策略：改 Rust monitor loop。
- 历史上限和持久化：改 Rust state 和 app data 文件逻辑。
- 搜索和列表交互：改 `ClipboardPanel.tsx`。

## `page-notes`

前端：

- `src/addons/page-notes/PageNotesPanel.tsx`
- `src/addons/page-notes/api.ts`
- `src/addons/page-notes/types.ts`
- `src/addons/page-notes/styles.css`
- `src/addons/page-notes/browser-extension/*`

后端：

- `src-tauri/src/page_notes/mod.rs`
- `src-tauri/src/page_notes/page_url.rs`
- `src-tauri/src/browser_ext/mod.rs`

命令：

- `load_page_notes`
- `save_page_notes`
- `get_ws_port`
- `open_extension_dir`
- `launch_browser_with_extension`

常见修改：

- URL 识别：改 `page_url.rs`。
- 笔记匹配/存储：改 `page_notes/mod.rs` 和前端类型。
- 浏览器扩展：改 `browser-extension`，并确认 `tauri.conf.json` 的 `bundle.resources`。

## `music-player`

前端：

- `src/addons/music-player/MusicPanel.tsx`
- `src/addons/music-player/api.ts`
- `src/addons/music-player/types.ts`
- `src/addons/music-player/styles.css`

后端：

- `src-tauri/src/music_player/mod.rs`
- `src-tauri/src/music_player/lyrics.rs`

命令：

- `get_media_info`
- `get_media_sessions`
- `select_media_session`
- `media_control`
- `get_lyrics`

常见修改：

- SMTC 媒体会话：改 `music_player/mod.rs`。
- 歌词获取与解析：改 `lyrics.rs`。
- 播放进度、控制按钮、session 选择：改 `MusicPanel.tsx`。

## `amkr`

前端：

- `src/addons/amkr/AmkrPanel.tsx`
- `src/addons/amkr/components/Dashboard.tsx`
- `src/addons/amkr/api.ts`
- `src/addons/amkr/types.ts`
- `src/addons/amkr/styles.css`

后端：

- `src-tauri/src/amkr/mod.rs`

命令：

- `fetch_amkr_metrics`
- `generate_commit_message`
- `get_amkr_models`
- `set_amkr_unified_model`
- `start_amkr_ws`
- `stop_amkr_ws`

常见修改：

- 指标接口和模型配置：改 Rust command。
- WebSocket 生命周期：改 `AmkrWsHandle` 相关逻辑。
- 仪表盘展示：改 `Dashboard.tsx` 和样式。

## 宿主能力相关文件

| 文件 | 作用 |
| --- | --- |
| `src/App.tsx` | 动态导入前端插件并按窗口类型渲染 |
| `src/addons/registry.ts` | 前端插件注册表和 `PluginDef` 类型 |
| `src/widgets/WidgetShell.tsx` | 标题栏、关闭、折叠、吸附、窗口状态保存 |
| `src/widgets/WidgetButtons.tsx` | 标题栏按钮 |
| `src/widgets/useAutoResize.ts` | 内容高度自适应 |
| `src/lib/api.ts` | 宿主级 Tauri command 封装 |
| `src/lib/types.ts` | 全局设置和窗口状态类型 |
| `src-tauri/src/main.rs` | 窗口创建、command handler 注册、启动流程 |
| `src-tauri/src/settings.rs` | 设置持久化、开机自启动、热键、插件可见性 |
| `src-tauri/src/window_attach.rs` | 前台窗口吸附、全屏隐藏、序列切换协作 |
| `src-tauri/src/tray.rs` | 系统托盘与插件菜单 |
| `src-tauri/src/plugins.rs` | 读取编译期嵌入的 manifest |
| `src-tauri/build.rs` | 自动嵌入 manifest、生成 `_plugins.rs` |
| `src-tauri/src/updater.rs` | GitHub Releases 更新检查 |

## 修改建议

- 修改单个插件时，优先限制在该插件前端目录和对应 Rust 模块。
- 新增 command 时必须改 `src-tauri/src/main.rs`。
- 修改通用窗口行为时才改 `WidgetShell`、`window_attach.rs` 或 `settings.rs`。
- 修改 manifest 字段时同步 `index.tsx` 注册字段。
- 修改插件资源时同步 `src-tauri/tauri.conf.json`。