# manifest.json 字段参考

`manifest.json` 位于 `src/addons/<plugin-id>/manifest.json`，用于描述插件元数据和默认窗口行为。Tauri 构建时，`src-tauri/build.rs` 会扫描所有 manifest 并嵌入到 Rust 二进制中；运行时宿主用这些信息创建插件窗口、管理插件列表和恢复用户设置。

## 字段一览

| 字段 | 类型 | 必填 | 默认值 | 说明 |
| --- | --- | --- | --- | --- |
| `id` | `string` | 是 | 无 | 插件唯一 ID，也是窗口标签后缀和设置 key |
| `title` | `string` | 是 | 无 | 插件标题 |
| `description` | `string` | 否 | `undefined` | 管理页说明 |
| `icon` | `string` | 否 | `undefined` | 管理页图标，通常是 emoji |
| `defaultWidth` | `number` | 否 | `360` | 初始窗口宽度 |
| `defaultHeight` | `number` | 否 | `600` | 初始窗口高度 |
| `showCloseButton` | `boolean` | 否 | `true` | 是否显示关闭按钮 |
| `showCollapseButton` | `boolean` | 否 | `true` | 是否显示折叠按钮 |
| `showAttachButton` | `boolean` | 否 | `true` | 是否显示吸附按钮 |
| `defaultAttachEnabled` | `boolean` | 否 | `true` | 是否默认吸附到前台窗口 |
| `defaultAttachRemember` | `boolean` | 否 | `false` | 是否默认只记忆显示/隐藏，不移动位置 |
| `defaultWhitelist` | `string[]` | 否 | `[]` | 默认吸附白名单，匹配窗口标题或进程名片段 |

## 字段详解

### `id`

- 必须全局唯一。
- 推荐使用小写字母、数字和短横线，例如 `page-notes`。
- 会用于 `widget-<id>` 窗口标签、`index.html?widget=<id>` URL、`panelVisibility[id]` 和 `windowStates[id]`。
- 发布后不要修改，否则用户已有设置会失效。

### `title`

显示在标题栏、管理页和托盘相关入口中。应短而清晰。

### `description` 和 `icon`

仅影响展示。`icon` 推荐使用单个 emoji 或 1-2 个字符，避免宽度过大。

### `defaultWidth` 和 `defaultHeight`

- 单位为逻辑像素。
- 用户调整窗口或插件保存高度后，宿主会优先使用已保存状态。
- 对内容高度动态变化的插件，可以结合 `useAutoResize` 或手动保存窗口状态。

### 按钮控制

- `showCloseButton`: 控制标题栏关闭按钮。关闭按钮会隐藏窗口并更新插件可见性。
- `showCollapseButton`: 控制折叠/展开按钮。折叠时由 `WidgetShell` 调整窗口高度。
- `showAttachButton`: 控制吸附和记忆位置按钮区域。

### 吸附系统

- `defaultAttachEnabled=true`: 窗口默认跟随前台窗口显示、隐藏和移动。
- `defaultAttachEnabled=false`: 插件默认作为独立悬浮窗显示。
- `defaultAttachRemember=true`: 只根据前台窗口决定显示/隐藏，不改变窗口位置。
- `defaultWhitelist=[]`: 不限制前台窗口；非空时仅匹配列表中的窗口标题或进程名片段。

常见白名单：

```json
["powershell.exe", "pwsh.exe", "cmd.exe", "WindowsTerminal.exe"]
```

```json
["chrome.exe", "msedge.exe", "firefox.exe", "brave.exe", "vivaldi.exe"]
```

## 示例

### 独立工具

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

### 附着到终端

```json
{
  "id": "git",
  "title": "Git 状态",
  "description": "Git 仓库变更查看、暂存、提交、推送",
  "icon": "📋",
  "defaultWidth": 360,
  "defaultHeight": 800,
  "showCloseButton": true,
  "showCollapseButton": true,
  "showAttachButton": true,
  "defaultAttachEnabled": true,
  "defaultWhitelist": ["powershell.exe", "pwsh.exe", "cmd.exe", "WindowsTerminal.exe", "Tabby.exe"]
}
```

### 附着到浏览器

```json
{
  "id": "page-notes",
  "title": "页面笔记",
  "description": "根据浏览器页面 URL 显示预设笔记",
  "icon": "📝",
  "defaultWidth": 360,
  "defaultHeight": 160,
  "showCloseButton": true,
  "showCollapseButton": true,
  "showAttachButton": true,
  "defaultAttachEnabled": true,
  "defaultAttachRemember": false,
  "defaultWhitelist": ["chrome.exe", "msedge.exe", "firefox.exe", "brave.exe", "vivaldi.exe"]
}
```

### 常驻仪表盘

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

## 与 `PluginDef` 的映射

`src/addons/<plugin-id>/index.tsx` 调用 `registerPlugin` 时，需要重复 manifest 中的大部分字段，并额外传入 `component`。

保持一致的字段：

- `id`
- `title`
- `description`
- `icon`
- `defaultWidth`
- `defaultHeight`
- `showCloseButton`
- `showCollapseButton`
- `showAttachButton`
- `defaultAttachEnabled`
- `defaultAttachRemember`
- `defaultWhitelist`

仅前端字段：

- `component`
- `collapsedHeight`（可选，用于定制折叠高度）

## 校验清单

- JSON 合法，无注释、无尾随逗号。
- `id` 与目录名、`index.tsx` 注册 ID 一致。
- manifest 与 `registerPlugin` 的展示字段一致。
- 新资源已加入 `tauri.conf.json` 的 `bundle.resources`。
- 修改后运行 `npm run build` 和 `cargo check --manifest-path src-tauri/Cargo.toml`。