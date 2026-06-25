# 常见模式与最佳实践

## 数据更新策略

### 轮询（Polling）

适用于没有实时推送机制的场景。通过 `setInterval` 定期调用 API：

```tsx
useEffect(() => {
  // 首次立即加载
  fetchData().then(setData).catch(() => {});

  // 定时轮询
  const interval = setInterval(() => {
    fetchData().then(setData).catch(() => {});
  }, 2000);

  return () => clearInterval(interval);
}, []);
```

现有插件的轮询间隔：

| 插件 | 间隔 | 原因 |
|------|------|------|
| system-monitor | 2000ms | 系统指标变化不快 |
| clipboard-history | 1000ms | 需要及时发现新剪贴板内容 |
| page-notes | 500ms | URL 变化需要快速响应 |

### 事件驱动（Event-driven）

适用于后端能主动推送的场景（WebSocket、文件监听等）：

```tsx
import { listen } from "@tauri-apps/api/event";

useEffect(() => {
  const unlisten = listen<MyPayload>("my-event", (event) => {
    setData(event.payload);
  });
  return () => { unlisten.then((fn) => fn()); };
}, []);
```

现有插件的事件使用：

| 插件 | 事件名 | 触发源 |
|------|--------|--------|
| amkr | `amkr-event` | WebSocket 后端 |
| git | `git-changed` | 文件系统监听 |
| git | `ai-commit-progress` | AI 流式生成 |

### 选择建议

- 后端能发事件 → 用事件驱动（延迟低、资源省）
- 后端只有请求-响应 → 用轮询
- 可以混合使用：轮询兜底 + 事件触发即时刷新

---

## 子组件组织

### 简单插件：单文件

如果组件逻辑简单（如 clipboard-history），直接在 Panel 文件中完成所有渲染：

```
src/addons/my-plugin/
├── index.tsx
├── manifest.json
├── api.ts
├── MyPluginPanel.tsx    ← 所有 UI 逻辑在这里
└── styles.css
```

### 复杂插件：components/ 目录

当组件有独立的 UI 模块时，拆分到 `components/` 目录：

```
src/addons/my-plugin/
├── index.tsx
├── manifest.json
├── types.ts
├── api.ts
├── MyPluginPanel.tsx    ← 主容器，编排子组件
├── styles.css
└── components/
    ├── Header.tsx
    ├── ItemList.tsx
    └── DetailView.tsx
```

现有插件的拆分情况：

| 插件 | 子组件数 | 子组件 |
|------|---------|--------|
| clipboard-history | 0 | — |
| system-monitor | 0 | — |
| page-notes | 0 | （内部内联了 RuleItem、RuleEditor） |
| amkr | 1 | `Dashboard.tsx` |
| git | 4 | `GitTree.tsx`、`DiffViewer.tsx`、`CommitTree.tsx`、`GitConsole.tsx` |

---

## CSS 命名约定

每个插件使用自己的 CSS 类名前缀，避免冲突：

| 插件 | 前缀 | 示例 |
|------|------|------|
| system-monitor | `.sm-` | `.sm-panel`、`.sm-gauges`、`.sm-cell` |
| clipboard-history | `.cb-` | `.cb-panel`、`.cb-item`、`.cb-search` |
| page-notes | `.pn-` | `.pn-panel`、`.pn-view`、`.pn-rule` |
| amkr | `.amkr-` | `.amkr-panel`、`.amkr-metric` |
| git | `.tree-`、`.diff-`、`.commit-` | `.tree-node`、`.diff-line`、`.commit-row` |

### 推荐规范

```css
/* 插件根容器 */
.mp-panel { padding: 8px 10px; }

/* 子模块 */
.mp-header { ... }
.mp-list { ... }
.mp-item { ... }

/* 状态修饰符 */
.mp-item--active { ... }
.mp-item--disabled { ... }
```

### 使用全局 CSS 变量

始终使用全局主题变量保持视觉一致：

```css
.mp-item {
  color: var(--text-primary);
  border-bottom: 1px solid var(--glass-border);
  background: var(--glass-highlight);
}

.mp-error {
  color: var(--mcha-red);
}

.mp-success {
  color: var(--mcha-green);
}
```

---

## 共享组件库

`src/components/` 下提供了 7 个可复用组件：

### RadialGauge

环形仪表盘，常用于百分比指标展示。

```tsx
import { RadialGauge } from "../../components/RadialGauge";

<RadialGauge
  value={cpuPct}         // 0-100 的数值
  label={`${cpuPct}%`}   // 中心显示文字
  color="var(--mcha-cyan)" // 圆弧颜色
  sub="CPU"              // 底部标签
  sub2="45°C"            // 底部第二行（可选）
  sub2Color="var(--mcha-amber)" // 第二行颜色（可选）
/>
```

### AnimatedNumber

带过渡动画的数值显示。

```tsx
import { AnimatedNumber } from "../../components/AnimatedNumber";

<AnimatedNumber
  value={12345}
  format={(n) => n.toLocaleString()}  // 可选格式化函数
  duration={500}                       // 动画时长 ms，默认 500
/>
```

### ProgressBar

水平进度条。

```tsx
import { ProgressBar } from "../../components/ProgressBar";

<ProgressBar
  value={0.85}              // 0-1
  color="var(--mcha-green)"
  label="85%"               // 可选文字标签
/>
```

### StatCard

指标卡片，显示标签、数值和可选的副文字。

```tsx
import { StatCard } from "../../components/StatCard";

<StatCard label="请求数" value="12.5K" sub="过去 24 小时" />
```

### MetricRow

单行指标显示。

```tsx
import { MetricRow } from "../../components/MetricRow";

<MetricRow label="成功率" value="99.8%" />
```

### Toggle

开关切换。

```tsx
import { Toggle } from "../../components/Toggle";

<Toggle
  checked={enabled}
  onChange={setEnabled}
  label="启用通知"   // 可选
/>
```

### EmptyState

空状态/加载中占位。

```tsx
import { EmptyState } from "../../components/EmptyState";

<EmptyState message="暂无数据" />
```

---

## 格式化工具函数

`src/lib/format.ts` 提供常用的数据格式化：

```ts
import { fmtNumber, fmtMs, fmtBytes, fmtHz, fmtPercent, fmtUptime } from "../../lib/format";

fmtNumber(1234567)    // "1.2M"
fmtMs(1500)           // "1.5s"
fmtBytes(1048576)     // "1.0 MB"
fmtHz(3600000000)     // "3.60 GHz"
fmtPercent(0.85)      // "85.0%"
fmtUptime("2024-01-01T00:00:00Z")  // "3h42m"
```

---

## 状态管理

### 基础模式：useState + useEffect

大多数插件使用标准 React hooks 即可：

```tsx
const [data, setData] = useState<MyData[]>([]);
const [loading, setLoading] = useState(true);

useEffect(() => {
  setLoading(true);
  fetchData()
    .then(setData)
    .catch(console.error)
    .finally(() => setLoading(false));
}, []);
```

### 避免闭包陈旧：useRef

当回调中需要访问最新状态但依赖数组不想包含该状态时：

```tsx
const statusRef = useRef<GitStatus | null>(null);
const [status, setStatus] = useState<GitStatus | null>(null);

// 同步到 ref
useEffect(() => { statusRef.current = status; }, [status]);

// 在回调中使用 ref 获取最新值
const handleEvent = useCallback(() => {
  const current = statusRef.current;  // 总是最新的
  // ...
}, []);  // 依赖数组为空，不会因 status 变化而重建
```

Git 插件大量使用此模式（`selectedFileRef`、`statusRef`、`watchedRepoRef`）。

### 复杂状态：useReducer

当状态逻辑涉及多个相互依赖的值时：

```tsx
const [state, dispatch] = useReducer(reducer, initialState);
```

目前没有现有插件使用此模式，但对于表单密集型插件可能更合适。

---

## 错误处理

### 推荐模式

```tsx
const [error, setError] = useState<string | null>(null);

const handleAction = async () => {
  try {
    setError(null);
    await doAction();
    showResult("操作成功");
  } catch (e) {
    const msg = String(e);
    setError(msg);
    showError(msg);
  }
};
```

### invoke 错误

Rust 命令返回 `Err(String)` 时，`invoke()` 的 Promise 会 reject，错误信息为 Rust 端传入的字符串。

---

## 窗口交互

### 窗口拖拽

`WidgetShell` 已处理标题栏和空白区域的拖拽。如果你的组件中有交互元素（按钮、输入框等），确保它们不会意外触发拖拽——框架通过检查 `event.target` 的 `tagName` 和 `closest()` 来过滤。

### 自动滚动

对于长列表（如 git log、剪贴板历史），使用 `overflow-y: auto` 实现滚动：

```tsx
<div className="mp-list" style={{ maxHeight: "300px", overflowY: "auto" }}>
  {items.map(item => <div key={item.id}>...</div>)}
</div>
```

### 自动滚动到底部

控制台类组件通常需要自动滚动：

```tsx
const endRef = useRef<HTMLDivElement>(null);

useEffect(() => {
  endRef.current?.scrollIntoView({ behavior: "smooth" });
}, [logs.length]);

return (
  <div className="mp-console">
    {logs.map((log, i) => <div key={i}>{log}</div>)}
    <div ref={endRef} />
  </div>
);
```
