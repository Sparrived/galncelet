# Galncelet 插件开发指南

Galncelet 是一个基于 Tauri 2 的桌面悬浮挂件系统。每个插件以独立的透明窗口运行，拥有自己的 UI 和后端逻辑。本指南帮助你从零开始创建自己的插件，并涵盖框架提供的所有能力。

## 架构概览

```
┌──────────────────────────────────────────────────────────┐
│                    Galncelet App                          │
│                                                          │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐      │
│  │  Widget A   │  │  Widget B   │  │  Widget C   │ ...  │
│  │ (独立窗口)   │  │ (独立窗口)   │  │ (独立窗口)   │      │
│  │ widget-<id>  │  │ widget-<id>  │  │ widget-<id>  │      │
│  └──────┬──────┘  └──────┬──────┘  └──────┬──────┘      │
│         ↑                ↑                ↑              │
│  ┌──────────────────────────────────────────────┐        │
│  │               WidgetShell (壳)                 │        │
│  │  标题栏 · 拖拽 · 折叠 · 附着 · 吸附 · 右键菜单 │        │
│  └──────────────────────────────────────────────┘        │
│         ↑                                              │
│  ┌──────────────────────────────────────────────┐      │
│  │              Plugin Registry                  │      │
│  │        registerPlugin / getPlugin             │      │
│  └──────────────────────────────────────────────┘      │
│         ↑                                              │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐             │
│  │index.tsx │  │index.tsx │  │index.tsx │  ...        │
│  │(插件 A)  │  │(插件 B)  │  │(插件 C)  │             │
│  └──────────┘  └──────────┘  └──────────┘             │
└──────────────────────────────────────────────────────────┘
         ↕ Tauri IPC (invoke / listen / emit)
┌──────────────────────────────────────────────────────────┐
│                  Rust 后端 (Tauri 2)                     │
│  #[tauri::command] · app.emit() · State<T> · build.rs   │
└──────────────────────────────────────────────────────────┘
```

### 核心概念

| 概念 | 说明 |
|------|------|
| **Addon（插件）** | 前端逻辑单元——`src/addons/<id>/` 下的一个目录，包含 manifest、组件、API、样式 |
| **Widget（挂件）** | 运行时实体——一个 Tauri 原生窗口，由 `widget-<id>` 标签标识 |
| **WidgetShell** | 框架提供的窗口壳——标题栏、拖拽、折叠、附着、吸附、右键菜单 |
| **Registry（注册表）** | 前端全局 `Map<string, PluginDef>`，由 `registerPlugin()` 填充 |
| **Rust Module** | 后端逻辑单元——`src-tauri/src/<id>/mod.rs`，包含 `setup()` 和 `#[tauri::command]` |
| **_plugins.rs** | `build.rs` 自动生成的胶水文件——插件 mod 声明 + `setup_all()` 调用 |

### 加载流程

```
启动阶段（build.rs，编译时）
  ├── 扫描 src/addons/*/manifest.json → 嵌入编译产物
  └── 扫描 src-tauri/src/<id>/mod.rs  → 生成 _plugins.rs
         ├── mod amkr;
         ├── mod clipboard_history;
         └── ...
         └── pub fn setup_all(app) { amkr::setup(app); ... }

启动阶段（main.rs，运行时）
  ├── _plugins::setup_all(&handle)     → 初始化所有插件后端
  ├── plugins::load_manifests()        → 读取嵌入的 manifest
  ├── 为每个 manifest 创建 widget 窗口 → create_widget_window()
  └── tray::setup(app, &manifests)     → 构建托盘菜单

前端加载（App.tsx，运行时）
  ├── import.meta.glob("./addons/*/index.tsx")  → Vite 构建时发现
  ├── Promise.all 动态导入所有插件模块
  ├── 每个 index.tsx 执行 registerPlugin()       → 注册到全局 Map
  └── 根据窗口标签 widget-<id> 查找 PluginDef
        └── <WidgetShell><Component /></WidgetShell>
```

### 自动发现机制

**前端**：Vite 的 `import.meta.glob("./addons/*/index.tsx")` 在构建时自动发现 `src/addons/` 下所有子目录的 `index.tsx`。运行时通过 `Promise.all` 动态导入，无需手动注册。

**后端**：`build.rs` 扫描 `src-tauri/src/` 下所有包含 `mod.rs` 的子目录（排除 `acrylic`、`plugins`、`settings`、`tray`、`window_attach` 等框架模块），自动生成：
- `mod <id>;` 声明（带 `#[path = "<id>/mod.rs"]`）
- `setup_all()` 函数，依次调用每个插件的 `setup(app)`
- manifest 嵌入代码

**你不需要**：
- 在 `main.rs` 中写 `mod my_plugin;`
- 在 `main.rs` 中写 `my_plugin::setup(app);`
- 手动更新 `_plugins.rs`（它会被覆盖）

**你仍然需要**：
- 在 `main.rs` 的 `generate_handler![]` 中注册你的 `#[tauri::command]` 函数
- 创建 `src-tauri/src/<id>/mod.rs` 文件

## 插件目录结构

```
src/addons/my-plugin/                    ← 前端插件目录
├── index.tsx                            # 入口：import manifest + component，调用 registerPlugin()
├── manifest.json                        # 插件元数据和窗口配置
├── types.ts                             # TypeScript 接口（IPC 数据结构）
├── api.ts                               # Tauri invoke() 封装函数
├── MyPluginPanel.tsx                    # 主面板组件
├── styles.css                           # 插件专属样式
└── components/                          # 可选：子组件
    └── SubWidget.tsx

src-tauri/src/my-plugin/                 ← 后端插件目录
└── mod.rs                               # setup() + #[tauri::command] 函数
```

### 各文件职责

| 文件 | 职责 | 必须 |
|------|------|------|
| `src/addons/my-plugin/index.tsx` | 导入 manifest + 组件，调用 `registerPlugin()` | ✅ |
| `src/addons/my-plugin/manifest.json` | 声明 id、title、icon、尺寸、按钮、附着配置 | ✅ |
| `src/addons/my-plugin/MyPluginPanel.tsx` | 渲染挂件内容，管理状态和交互 | ✅ |
| `src/addons/my-plugin/styles.css` | 插件专属样式 | ✅ |
| `src/addons/my-plugin/types.ts` | IPC 数据结构的 TypeScript 接口 | 推荐 |
| `src/addons/my-plugin/api.ts` | 封装 `invoke()` 调用 | 推荐 |
| `src-tauri/src/my-plugin/mod.rs` | 后端 `setup()` + `#[tauri::command]` | 需要后端时 |
| `src-tauri/src/main.rs` | 在 `generate_handler![]` 中注册命令 | 需要后端时 |

## 最小插件示例

一个最简单的插件只需要 4 个文件（纯前端，无 Rust 后端）：

### 1. manifest.json

```json
{
  "id": "hello",
  "title": "Hello World",
  "description": "最小示例插件",
  "icon": "👋",
  "defaultWidth": 240,
  "defaultHeight": 100
}
```

### 2. HelloPanel.tsx

```tsx
import { useRef } from "react";
import { useAutoResize } from "../../widgets/useAutoResize";

export default function HelloPanel() {
  const ref = useRef<HTMLDivElement>(null);
  useAutoResize(ref);

  return (
    <div ref={ref} style={{ padding: "16px", color: "var(--text-primary)" }}>
      Hello, World!
    </div>
  );
}
```

### 3. index.tsx

```tsx
import { registerPlugin } from "../registry";
import HelloPanel from "./HelloPanel";
import manifest from "./manifest.json";
import "./styles.css";

registerPlugin({ ...manifest, component: HelloPanel });
```

### 4. styles.css

```css
/* 最小插件可以为空 */
```

然后运行 `npm run tauri dev`，在管理页面启用插件即可。

## 现有插件一览

| 插件 | 说明 | 复杂度 | IPC 命令数 |
|------|------|--------|-----------|
| [system-monitor](../src/addons/system-monitor/) | CPU/GPU/内存/磁盘/网络监控 | ⭐ 简单 | 1 |
| [clipboard-history](../src/addons/clipboard-history/) | 剪贴板历史记录与搜索 | ⭐ 简单 | 4 |
| [page-notes](../src/addons/page-notes/) | 基于 URL 的页面笔记 | ⭐⭐ 中等 | 4 |
| [amkr](../src/addons/amkr/) | LLM API 路由器实时仪表盘 | ⭐⭐ 中等 | 5 |
| [music-player](../src/addons/music-player/) | SMTC 媒体播放器 + 歌词 | ⭐⭐ 中等 | 5 |
| [git](../src/addons/git/) | 完整 Git 仓库管理 | ⭐⭐⭐ 复杂 | 18 |

## 框架能力速览

### 可用 Hooks

| Hook | 来源 | 用途 |
|------|------|------|
| `useAutoResize(ref)` | `src/widgets/useAutoResize` | 自动调整窗口大小适应内容 |
| `useWidget()` | `src/lib/context` | 刷新、消息提示、状态通知 |
| `useWidgetContextMenu(items)` | `src/widgets/WidgetContext` | 注册自定义右键菜单 |
| `useWidgetContext()` | `src/widgets/WidgetContext` | 读取折叠状态等上下文 |

详见 [可用 Hooks](hooks.md)。

### 共享组件

`src/components/` 下提供 7 个可复用组件：`RadialGauge`、`AnimatedNumber`、`ProgressBar`、`StatCard`、`MetricRow`、`Toggle`、`EmptyState`。

详见 [常见模式](patterns.md#共享组件库)。

### 格式化工具

`src/lib/format.ts` 提供：`fmtNumber`、`fmtMs`、`fmtBytes`、`fmtHz`、`fmtPercent`、`fmtUptime`。

### 全局 CSS 变量

```css
--text-primary    /* 主文字色 #e4e4e7 */
--text-secondary  /* 次文字色 #a1a1aa */
--text-muted      /* 弱文字色 #71717a */
--glass-bg        /* 背景色 rgba(18,18,24,0.99) */
--glass-border    /* 边框色 rgba(255,255,255,0.08) */
--glass-highlight /* 高亮背景 rgba(255,255,255,0.04) */
--mcha-cyan       /* 强调色 #22d3ee */
--mcha-green      /* 成功色 #4ade80 */
--mcha-amber      /* 警告色 #fbbf24 */
--mcha-red        /* 错误色 #f87171 */
--mcha-surface    /* 表面色 rgba(255,255,255,0.03) */
--mcha-border     /* 表面边框 rgba(255,255,255,0.06) */
```

## 开发流程

### 方式一：纯前端插件（无新 Rust 命令）

如果你的插件只使用现有的 Tauri 命令，或纯前端逻辑：

1. 创建 `src/addons/my-plugin/` 目录
2. 编写 `manifest.json`
3. 编写 Panel 组件和样式
4. 编写 `index.tsx` 注册
5. `npm run tauri dev` 运行

### 方式二：需要新 Rust 命令

1. 创建 `src-tauri/src/my-plugin/mod.rs`，定义 `setup()` 和 `#[tauri::command]` 函数
2. `build.rs` 会自动生成 `_plugins.rs`，添加 `mod my-plugin;` 声明和 `setup()` 调用
3. 在 `src-tauri/src/main.rs` 的 `generate_handler![]` 中注册你的命令
4. 创建前端 `types.ts` 和 `api.ts`
5. 编写 Panel 组件
6. 注册插件并运行

详见 [快速开始教程](quickstart.md) 和 [Rust 后端集成](backend.md)。

## 文档索引

| 文档 | 内容 |
|------|------|
| [快速开始](quickstart.md) | 从零创建番茄钟插件的完整教程 |
| [manifest.json 参考](manifest.md) | 所有 manifest 字段的详细说明 |
| [可用 Hooks](hooks.md) | useAutoResize、useWidget、useWidgetContextMenu 详解 |
| [Rust 后端集成](backend.md) | Tauri 命令定义、注册、事件系统、数据持久化 |
| [常见模式](patterns.md) | 轮询/事件驱动、子组件组织、CSS 规范、共享组件 |
