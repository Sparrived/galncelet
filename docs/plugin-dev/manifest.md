# manifest.json 字段参考

每个插件目录下必须包含一个 `manifest.json` 文件，用于声明插件的元数据和窗口行为配置。该文件会在 `index.tsx` 中通过 `import manifest from "./manifest.json"` 导入，并展开传入 `registerPlugin()`。

## 字段一览

| 字段 | 类型 | 必填 | 默认值 | 说明 |
|------|------|------|--------|------|
| `id` | `string` | ✅ | — | 唯一标识符，用作设置存储键和窗口标签后缀（`widget-<id>`） |
| `title` | `string` | ✅ | — | 显示标题，出现在窗口标题栏和管理页面 |
| `description` | `string` | ❌ | `undefined` | 管理页面显示的简短描述 |
| `icon` | `string` | ❌ | `undefined` | 管理页面显示的 Emoji 图标 |
| `defaultWidth` | `number` | ❌ | `undefined` | 默认窗口宽度（逻辑像素） |
| `defaultHeight` | `number` | ❌ | `undefined` | 默认窗口高度（逻辑像素） |
| `showCloseButton` | `boolean` | ❌ | `true` | 标题栏是否显示关闭按钮 |
| `showCollapseButton` | `boolean` | ❌ | `true` | 标题栏是否显示折叠按钮 |
| `showAttachButton` | `boolean` | ❌ | `true` | 标题栏是否显示「附着到前台窗口」按钮 |
| `defaultAttachEnabled` | `boolean` | ❌ | `true` | 是否默认启用前台窗口附着 |
| `defaultAttachRemember` | `boolean` | ❌ | `false` | 是否默认启用「记住位置」模式（只管理显隐，不管位置） |
| `defaultWhitelist` | `string[]` | ❌ | `[]` | 附着白名单——前台窗口标题的子串匹配列表。空数组 = 不限制 |

## 字段详解

### `id`

插件的唯一标识。用途：
- 作为 `AppSettings.windowStates` 和 `panelVisibility` 的键
- Tauri 窗口标签格式为 `widget-<id>`
- 管理页面通过此 ID 引用插件

**命名规范**：使用 kebab-case，如 `system-monitor`、`clipboard-history`。

### `title`

窗口标题栏左侧显示的文字，同时出现在管理页面的插件列表中。

### `description` & `icon`

仅在管理页面（`ManagePage`）中显示。`icon` 推荐使用单个 Emoji 字符。

### `defaultWidth` & `defaultHeight`

插件窗口的初始尺寸，单位为逻辑像素。注意：
- `defaultWidth` 通常会被全局设置 `AppSettings.cardWidth`（默认 360px）覆盖
- `defaultHeight` 作为窗口创建时的初始高度
- 如果使用 `useAutoResize` hook，窗口高度会随内容自动调整

### 按钮控制

`showCloseButton`、`showCollapseButton`、`showAttachButton` 控制标题栏右侧的按钮显隐。

典型配置组合：

| 场景 | close | collapse | attach |
|------|-------|----------|--------|
| 标准工具挂件 | `true` | `true` | `false` |
| 附着到特定应用 | `true` | `true` | `true` |
| 常驻仪表盘 | `false` | `false` | `false` |

### 附着系统

附着功能让挂件跟随用户的前台窗口自动显示/隐藏：

- **`defaultAttachEnabled`**：是否默认开启附着。开启后，挂件会跟随前台窗口。
- **`defaultAttachRemember`**：「记住位置」模式。开启后，附着系统只管理挂件的显隐，不重新定位。
- **`defaultWhitelist`**：白名单数组。非空时，只有前台窗口标题包含白名单中某个子串时才触发附着。通常填入目标应用的进程名。

## 示例

### 独立工具（不附着）

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
  "title": "Git",
  "description": "Git 仓库管理",
  "icon": "🔀",
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
  "description": "根据浏览器 URL 显示关联笔记",
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

### 常驻仪表盘（无按钮）

```json
{
  "id": "amkr",
  "title": "AMKR Dashboard",
  "description": "Auto Model Key Router 实时监控",
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
