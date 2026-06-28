# 常见模式与最佳实践

本页总结 Galncelet 插件的常见实现模式，适合开发新插件或审查现有插件时使用。

## 数据更新策略

### 轮询

适合系统指标、Git 状态、媒体会话等需要周期刷新但没有可靠事件源的数据。

```tsx
useEffect(() => {
  let cancelled = false;

  const load = async () => {
    try {
      const next = await fetchData();
      if (!cancelled) setData(next);
    } catch (error) {
      if (!cancelled) setError(String(error));
    }
  };

  load();
  const interval = setInterval(load, 1000);
  return () => {
    cancelled = true;
    clearInterval(interval);
  };
}, []);
```

建议间隔：

| 场景 | 间隔 |
| --- | --- |
| 系统监控 | 1000-2000ms |
| 媒体播放状态 | 500-1000ms |
| Git 状态兜底刷新 | 2000ms 以上 |
| 网络 API | 5000ms 以上或手动刷新 |

### 事件驱动

适合剪贴板、文件 watcher、WebSocket、浏览器扩展消息等后端可主动推送的场景。

```tsx
useEffect(() => {
  let unlisten: (() => void) | undefined;
  listen("plugin://updated", () => refresh()).then((fn) => { unlisten = fn; });
  return () => unlisten?.();
}, [refresh]);
```

### 乐观更新

适合 UI 操作后立即反馈，再等待后端确认：

```tsx
const toggle = async () => {
  setEnabled((value) => !value);
  try {
    await saveEnabled(!enabled);
  } catch (error) {
    setEnabled(enabled);
    setError(String(error));
  }
};
```

## 组件组织

简单插件：

```text
src/addons/example/
  manifest.json
  index.tsx
  ExamplePanel.tsx
  api.ts
  styles.css
```

复杂插件：

```text
src/addons/git/
  components/
    CommitTree.tsx
    DiffViewer.tsx
    GitConsole.tsx
    GitTree.tsx
  GitPanel.tsx
  api.ts
  types.ts
  styles.css
```

拆分原则：

- 面板文件负责数据流和页面布局。
- 子组件负责展示或局部交互。
- API 调用集中在 `api.ts`。
- 共享类型集中在 `types.ts`。

## CSS 命名

每个插件使用自己的 class 前缀：

| 插件 | 前缀示例 |
| --- | --- |
| `git` | `.git-panel`、`.git-tree` |
| `system-monitor` | `.system-monitor-panel` |
| `clipboard-history` | `.clipboard-history-panel` |
| `music-player` | `.music-player-panel` |
| `page-notes` | `.page-notes-panel` |
| `amkr` | `.amkr-panel` |

不要写宽泛选择器，例如 `button {}`、`.row {}`、`.panel {}`。必要时限制在插件根节点下。

## 共享组件

可复用组件位于 `src/components/`：

- `AnimatedNumber`
- `EmptyState`
- `MetricRow`
- `ProgressBar`
- `RadialGauge`
- `StatCard`
- `Toggle`

使用共享组件时保持 props 简单，不要把插件业务逻辑塞进通用组件。

## 状态管理

### 基础模式

```tsx
const [data, setData] = useState<Data | null>(null);
const [loading, setLoading] = useState(false);
const [error, setError] = useState<string | null>(null);
```

### 避免闭包陈旧

```tsx
const selectedIdRef = useRef(selectedId);
useEffect(() => {
  selectedIdRef.current = selectedId;
}, [selectedId]);
```

适合 interval、watcher callback、事件监听中读取最新状态。

### 复杂状态

表单密集或状态转移复杂时使用 `useReducer`，但不要为了简单 toggle 引入 reducer。

## 错误处理

推荐前端错误模式：

```tsx
try {
  await action();
  setError(null);
} catch (error) {
  setError(error instanceof Error ? error.message : String(error));
}
```

推荐 Rust 错误模式：

```rust
.map_err(|e| format!("Failed to read settings: {e}"))?;
```

错误信息应包含失败动作，避免只返回 `e.to_string()`。

## 窗口交互

- 拖拽和标题栏由 `WidgetShell` 处理。
- 关闭行为应通过 `setPluginVisible` 或 `onClose` 走宿主逻辑。
- 插件不要自行创建或销毁主 widget 窗口。
- 动态高度优先用 `useAutoResize` 或保存 `WindowState.height`。

## 吸附与序列

吸附用于插件跟随前台窗口；序列用于多个插件共享一个位置并通过热键轮换。

序列中的插件：

- 启动时会创建窗口。
- 第一个默认显示，其余默认隐藏。
- 切换时沿用当前可见插件的位置。
- attach loop 会跳过序列窗口的位置管理，避免互相抢位置。

开发插件时只需正确声明 `defaultAttachEnabled`、`defaultAttachRemember` 和 `defaultWhitelist`。

## 性能建议

- 避免在 render 中做昂贵计算；使用 `useMemo`。
- 大文本 diff 或日志列表应限制长度。
- 网络请求和文件 IO 放在 Rust 或异步函数中。
- 不要在高频 interval 中 setState 大对象。
- 后端 watcher 和 WebSocket 应有去重或节流。

## 安全建议

- 对来自前端的路径、URL、命令参数做校验。
- Git、shell、文件删除等危险动作必须范围明确。
- 不要把 token、密钥、用户隐私写入日志。
- 外部 URL 打开前应确保来源可信。

## 发布前检查

- `manifest.json` 与 `index.tsx` 元数据一致。
- 新增 Rust command 已注册。
- 所有监听和定时器有 cleanup。
- 插件样式不污染全局。
- `npm run build` 通过。
- `cargo check --manifest-path src-tauri/Cargo.toml` 通过。