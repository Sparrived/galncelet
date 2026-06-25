# Galncelet 插件开发指南

Galncelet 是一个基于 Tauri 的桌面悬浮挂件系统。每个插件以独立的原生窗口运行，拥有自己的 UI 和后端逻辑。本指南将帮助你从零开始创建自己的插件。

## 架构概览

```
┌──────────────────────────────────────────────────────┐
│                    Galncelet App                      │
│                                                      │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐          │
│  │ Widget A │  │ Widget B │  │ Widget C │  ...      │
│  │(独立窗口) │  │(独立窗口) │  │(独立窗口) │          │
│  └──────────┘  └──────────┘  └──────────┘          │
│       ↑              ↑              ↑                │
│  ┌──────────────────────────────────────┐           │
│  │          WidgetShell (壳)             │           │
│  │  标题栏 · 拖拽 · 折叠 · 附着 · 吸附   │           │
│  └──────────────────────────────────────┘           │
│       ↑                                             │
│  ┌──────────────────────────────────────┐           │
│  │           Plugin Registry            │           │
│  │     registerPlugin / getPlugin       │           │
│  └──────────────────────────────────────┘           │
│       ↑                                             │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐          │
│  │index.tsx │  │index.tsx │  │index.tsx │  ...      │
│  │(插件 A)  │  │(插件 B)  │  │(插件 C)  │          │
│  └──────────┘  └──────────┘  └──────────┘          │
└──────────────────────────────────────────────────────┘
         ↕ Tauri IPC (invoke / listen)
┌──────────────────────────────────────────────────────┐
│                  Rust 后端                            │
│  #[tauri::command]  ·  app.emit()  ·  State<T>       │
└──────────────────────────────────────────────────────┘
```

### 核心概念

- **Addon（插件）**：逻辑单元——一个目录，包含 manifest、组件、API、样式
- **Widget（挂件）**：运行时展示——一个 Tauri 原生窗口，由 WidgetShell 包裹
- **WidgetShell**：框架提供的窗口壳——标题栏、拖拽、折叠、附着、吸附、右键菜单
- **Registry（注册表）**：全局 Map，存储所有已注册的 `PluginDef`

### 加载流程

1. Vite 的 `import.meta.glob("./addons/*/index.tsx")` 在构建时发现所有插件
2. 应用启动时并行动态导入所有插件模块
3. 每个模块的 `index.tsx` 执行时调用 `registerPlugin()` 注册到全局注册表
4. 根据 Tauri 窗口标签（`widget-<id>`）查找对应的插件
5. 将插件的 `component` 渲染在 `WidgetShell` 内

## 插件目录结构

```
src/addons/my-plugin/
├── index.tsx              # 入口文件，调用 registerPlugin()
├── manifest.json          # 插件元数据和窗口配置
├── types.ts               # TypeScript 类型定义（IPC 数据结构）
├── api.ts                 # Tauri invoke() 封装函数
├── MyPluginPanel.tsx      # 主面板组件
├── styles.css             # 样式文件
└── components/            # 可选：子组件目录
    └── SomeWidget.tsx
```

### 各文件职责

| 文件 | 职责 | 必须 |
|------|------|------|
| `index.tsx` | 导入 manifest + 组件，调用 `registerPlugin()` | ✅ |
| `manifest.json` | 声明 id、title、icon、尺寸、按钮、附着配置 | ✅ |
| `Panel 组件` | 渲染挂件内容，管理状态和交互 | ✅ |
| `styles.css` | 插件专属样式 | ✅ |
| `types.ts` | IPC 数据结构的 TypeScript 接口 | 推荐 |
| `api.ts` | 封装 `invoke()` 调用 | 推荐 |

## 最小插件示例

一个最简单的插件只需要 4 个文件：

### manifest.json

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

### HelloPanel.tsx

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

### index.tsx

```tsx
import { registerPlugin } from "../registry";
import HelloPanel from "./HelloPanel";
import manifest from "./manifest.json";
import "./styles.css";

registerPlugin({ ...manifest, component: HelloPanel });
```

### styles.css

```css
/* 最小插件可以为空 */
```

## 现有插件一览

| 插件 | 说明 | 复杂度 |
|------|------|--------|
| [system-monitor](../../src/addons/system-monitor/) | CPU/GPU/内存/磁盘/网络监控 | ⭐ 简单 |
| [clipboard-history](../../src/addons/clipboard-history/) | 剪贴板历史记录与搜索 | ⭐ 简单 |
| [page-notes](../../src/addons/page-notes/) | 基于 URL 的页面笔记 | ⭐⭐ 中等 |
| [amkr](../../src/addons/amkr/) | LLM API 路由器实时仪表盘 | ⭐⭐ 中等 |
| [git](../../src/addons/git/) | 完整 Git 仓库管理 | ⭐⭐⭐ 复杂 |

## 可用的框架能力

### Hooks

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

### 方式一：只有前端（不添加新 Rust 命令）

如果你的插件只使用现有的 Tauri 命令，或纯前端逻辑：

1. 创建 `src/addons/my-plugin/` 目录
2. 编写 `manifest.json`
3. 编写 Panel 组件和样式
4. 编写 `index.tsx` 注册
5. `npm run tauri dev` 运行

### 方式二：需要新 Rust 命令

1. 创建 `src-tauri/src/my_plugin.rs`，定义命令
2. 在 `main.rs` 中 `mod my_plugin` 并注册命令
3. 创建前端 `api.ts` 封装 invoke 调用
4. 创建 `types.ts` 定义数据结构
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
