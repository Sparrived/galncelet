# Galncelet 插件 Schema 与接口参考（AI 版）

本文件定义插件 manifest、前端 registry、窗口状态、Tauri command 和发布相关约定。AI 生成插件时应以此为准。

## Manifest Schema

文件位置：`src/addons/<plugin-id>/manifest.json`

```ts
interface PluginManifest {
  id: string;
  title: string;
  description?: string;
  icon?: string;
  defaultWidth?: number;
  defaultHeight?: number;
  showCloseButton?: boolean;
  showCollapseButton?: boolean;
  showAttachButton?: boolean;
  defaultAttachEnabled?: boolean;
  defaultAttachRemember?: boolean;
  defaultWhitelist?: string[];
}
```

默认值由宿主在窗口创建和 `WidgetShell` 中处理：

| 字段 | 默认值 | 用途 |
| --- | --- | --- |
| `defaultWidth` | `360` | 初次创建窗口宽度 |
| `defaultHeight` | `600` | 初次创建窗口高度 |
| `showCloseButton` | `true` | 标题栏关闭按钮 |
| `showCollapseButton` | `true` | 标题栏折叠按钮 |
| `showAttachButton` | `true` | 标题栏吸附按钮 |
| `defaultAttachEnabled` | `true` | 初始是否吸附前台窗口 |
| `defaultAttachRemember` | `false` | 初始是否只记忆显示/隐藏 |
| `defaultWhitelist` | `[]` | 初始吸附白名单，空表示不限制 |

### JSON 示例

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

## Frontend Registry Schema

文件位置：`src/addons/registry.ts`

```ts
import type { FC } from "react";

interface PluginDef {
  id: string;
  title: string;
  description?: string;
  icon?: string;
  collapsedHeight?: number;
  defaultWidth?: number;
  defaultHeight?: number;
  showCloseButton?: boolean;
  showCollapseButton?: boolean;
  showAttachButton?: boolean;
  defaultAttachEnabled?: boolean;
  defaultAttachRemember?: boolean;
  defaultWhitelist?: string[];
  component: FC;
}
```

`collapsedHeight` 目前只存在于前端 registry，用于内容折叠高度扩展；manifest 中没有该字段时不要强行添加。

注册函数：

```ts
registerPlugin(def: PluginDef): void
getPlugin(id: string): PluginDef | undefined
getAllPlugins(): PluginDef[]
```

## App Settings Schema

文件位置：`src/lib/types.ts`

```ts
interface WindowState {
  x?: number;
  y?: number;
  height?: number;
  attachEnabled?: boolean;
  whitelist?: string[];
  attachRemember?: boolean;
}

interface AppSettings {
  refreshIntervalMs: number;
  cardWidth: number;
  logMaxCount: number;
  alwaysOnTop: boolean;
  startOnBoot: boolean;
  pullRebase: boolean;
  savedRepos: string[];
  currentRepo?: string;
  panelVisibility: Record<string, boolean>;
  windowStates: Record<string, WindowState>;
  hideFullscreen: boolean;
  pluginHotkeys: Record<string, string>;
  widgetSequence: string[];
  sequenceHotkey: string | null;
}
```

插件窗口状态保存在 `windowStates[pluginId]`，不要直接用窗口 label 作为 key，除非调用宿主 API 要求 `windowLabel`。

## Window Labels and URLs

| 概念 | 格式 |
| --- | --- |
| 插件 ID | `<plugin-id>` |
| 插件窗口标签 | `widget-<plugin-id>` |
| 插件 URL | `index.html?widget=<plugin-id>` |
| 管理页标签 | `manage` |
| 设置页标签 | `settings` |
| 插件设置页标签 | `settings-<plugin-id>` |

`src/App.tsx` 会根据 query `widget` 或当前窗口 label 推导插件 ID。

## Tauri Commands

### 宿主通用命令

前端封装：`src/lib/api.ts`

| 命令 | 用途 |
| --- | --- |
| `load_settings` / `save_settings` | 读取/保存全局设置 |
| `set_plugin_visible` | 设置插件窗口可见性 |
| `create_plugin_window` | 创建或显示插件窗口 |
| `open_manage_window` | 打开管理页 |
| `open_settings_window` | 打开全局设置页 |
| `open_plugin_settings` | 打开插件设置页 |
| `save_window_state` | 保存窗口位置、高度、吸附配置 |
| `set_attach_enabled` | 设置窗口吸附开关 |
| `set_attach_whitelist` | 设置吸附白名单 |
| `set_attach_remember` | 设置吸附记忆模式 |
| `set_hide_in_fullscreen` | 设置全屏时隐藏挂件 |
| `set_start_on_boot` | 设置 Windows 登录自启动 |
| `list_visible_windows` | 获取可见窗口列表 |
| `set_plugin_hotkey` | 设置插件显示/隐藏热键 |
| `set_widget_sequence` | 设置轮换插件序列 |
| `set_sequence_hotkey` | 设置轮换热键 |
| `check_for_updates` | 检查 GitHub Releases 新版本 |

### 插件命令

当前插件命令按模块注册在 `src-tauri/src/main.rs`：

| 模块 | 命令 |
| --- | --- |
| `git` | `get_status`、`get_file_diff`、`exec_git_command`、`stage_file`、`stage_all`、`unstage_file`、`discard_file`、`untrack_file`、`commit`、`pull`、`push`、`git_fetch`、`list_branches`、`checkout_branch`、`git_log`、`list_submodules`、`list_remotes`、`add_remote`、`remove_remote`、`watch_git_repo`、`unwatch_git_repo` |
| `system_monitor` | `fetch_system_metrics` |
| `clipboard_history` | `get_clipboard_history`、`copy_to_clipboard`、`delete_clipboard_entry`、`clear_clipboard_history` |
| `page_notes` | `load_page_notes`、`save_page_notes`、`get_ws_port` |
| `browser_ext` | `open_extension_dir`、`launch_browser_with_extension` |
| `music_player` | `get_media_info`、`media_control`、`get_media_sessions`、`select_media_session`、`get_lyrics` |
| `amkr` | `fetch_amkr_metrics`、`generate_commit_message`、`get_amkr_models`、`set_amkr_unified_model`、`start_amkr_ws`、`stop_amkr_ws` |

新增插件命令必须：

1. 在 Rust 函数上添加 `#[tauri::command]`。
2. 确保函数可从模块外访问，即 `pub fn` 或 `pub async fn`。
3. 加入 `tauri::generate_handler![...]`。
4. 在前端插件 `api.ts` 中封装 `invoke`。
5. 为返回结构体派生 `Serialize`，为输入结构体派生 `Deserialize`。

## build.rs 自动发现规则

`src-tauri/build.rs` 执行两类发现：

### Manifest 嵌入

扫描路径：`../src/addons/*/manifest.json`

结果：生成到 `$OUT_DIR/plugin_manifests.rs`，由 `src-tauri/src/plugins.rs` 读取。

### Rust 插件模块发现

扫描路径：`src-tauri/src/*/mod.rs`

排除目录：

```rust
[
  "target", ".git", ".idea", ".vscode",
  "acrylic", "plugins", "settings", "tray", "window_attach",
]
```

结果：生成 `src-tauri/src/_plugins.rs`，内容包括：

- `pub mod <module>;`
- `pub fn setup_all(app: &tauri::AppHandle)` 调用每个模块的 `setup(app)`。

因此，非插件框架模块必须加入 skip 列表；插件模块必须提供 `setup(app)`。

## Serialization Rules

建议 Rust 返回给前端的结构体使用：

```rust
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExamplePayload {
    pub current_value: String,
}
```

前端类型对应：

```ts
interface ExamplePayload {
  currentValue: string;
}
```

Tauri invoke 参数也按 camelCase 传递：

```rust
#[tauri::command]
pub fn set_attach_enabled(window_label: String, enabled: bool) -> Result<(), String> { ... }
```

```ts
invoke("set_attach_enabled", { windowLabel, enabled });
```

## Update Checker Contract

更新检查器位于 `src-tauri/src/updater.rs`，前端封装位于 `src/lib/api.ts`。

返回类型：

```ts
interface UpdateCheckResult {
  currentVersion: string;
  latestVersion: string | null;
  latestTag: string | null;
  releaseName: string | null;
  releaseUrl: string | null;
  publishedAt: string | null;
  hasUpdate: boolean;
}
```

发布约定：

- GitHub Release tag 使用 `v<semver>`，例如 `v1.0.0`。
- 应用内版本使用不带 `v` 的 semver，例如 `1.0.0`。
- `package.json`、`src-tauri/tauri.conf.json`、`src-tauri/Cargo.toml` 必须同步版本。

## Release Contract

发布脚本：`scripts/release.ps1`

关键参数：

| 参数 | 说明 |
| --- | --- |
| `-Version 1.0.0` | 同步版本号 |
| `-Tag v1.0.0` | 设置发布 tag |
| `-Target x86_64-pc-windows-msvc` | Windows x64 目标，默认值 |
| `-AllowDirty` | 允许脏工作区，仅 CI 或特殊情况使用 |
| `-SkipBuild` | 跳过构建 |
| `-Publish` | 使用 GitHub CLI 发布 release |
| `-Draft` | 创建 draft release |
| `-Prerelease` | 标记 prerelease |
| `-SkipRelease` | 不发布 GitHub Release |

发布产物：

- `src-tauri/target/release/galncelet.exe`
- `src-tauri/target/release/bundle/**`
- `src-tauri/target/release/bundle/SHA256SUMS.txt`

脚本会验证 `galncelet.exe` 是 Windows GUI subsystem，确保 release 版不会打开控制台窗口。