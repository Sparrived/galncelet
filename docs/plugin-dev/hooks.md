# 可用 Hooks

Galncelet 框架为插件组件提供了 4 个 React Hook，用于自动调整窗口尺寸、访问上下文功能和注册右键菜单。

## useAutoResize

**来源**：`src/widgets/useAutoResize.ts`

自动调整 Tauri 窗口大小以适应内容。使用 `ResizeObserver` 和 `MutationObserver` 监听内容变化，测量内容高度后加上标题栏高度（36px），调用 Tauri API 调整窗口尺寸。内置 4px 死区防止抖动。

### 签名

```ts
function useAutoResize(containerRef: React.RefObject<HTMLElement | null>): void;
```

### 用法

```tsx
import { useRef } from "react";
import { useAutoResize } from "../../widgets/useAutoResize";

export default function MyPanel() {
  const containerRef = useRef<HTMLDivElement>(null);
  useAutoResize(containerRef);

  return (
    <div ref={containerRef}>
      {/* 你的内容 */}
    </div>
  );
}
```

### 工作原理

1. 组件挂载后，通过 `loadSettings()` 读取全局 `cardWidth`（默认 360px）
2. 使用 `ResizeObserver` 监听容器元素的尺寸变化
3. 使用 `MutationObserver` 监听子节点增删和属性变化
4. 测量内容高度，加上 `HEADER_H`（36px）得到总高度
5. 如果高度变化超过 4px 死区，调用 `win.setSize(new LogicalSize(cardWidth, total))`
6. 窗口折叠时不调整尺寸

### 注意事项

- 容器 ref 必须指向实际渲染的根元素
- **4/5 的现有插件使用此 hook**（system-monitor、clipboard-history、page-notes、amkr）
- Git 插件因固定高度 800px 而未使用
- 如果你的内容高度固定不变，可以不用此 hook，直接在 manifest 中设置 `defaultHeight`

---

## useWidget

**来源**：`src/lib/context.tsx`

访问应用级 Widget 上下文，提供刷新、消息提示和状态通知功能。

### 签名

```ts
interface WidgetContext {
  refresh: () => Promise<void>;
  showResult: (msg: string) => void;
  showError: (msg: string) => void;
  onStatusChange: (status: string | null) => void;
}

function useWidget(): WidgetContext;
```

### 用法

```tsx
import { useWidget } from "../../lib/context";

export default function MyPanel() {
  const { showResult, showError, onStatusChange } = useWidget();

  const handleCommit = async () => {
    try {
      await commit(repo, message);
      showResult("提交成功");
      onStatusChange("main");  // 更新窗口副标题
    } catch (e) {
      showError("提交失败: " + e);
    }
  };

  return <button onClick={handleCommit}>提交</button>;
}
```

### 各方法说明

| 方法 | 用途 | 典型场景 |
|------|------|---------|
| `refresh()` | 触发当前挂件的完整数据刷新 | 目前为 no-op，预留接口 |
| `showResult(msg)` | 显示成功提示 | 操作成功后的反馈 |
| `showError(msg)` | 显示错误提示 | 捕获异常后的反馈 |
| `onStatusChange(status)` | 更新窗口副标题 | 显示当前分支、连接状态等 |

### 注意事项

- 目前只有 Git 插件使用此 hook
- `onStatusChange(null)` 清除副标题
- 这些方法来自 `App.tsx` 中的 `WidgetProvider`

---

## useWidgetContextMenu

**来源**：`src/widgets/WidgetContext.ts`

为插件注册自定义右键菜单项。组件卸载时自动清除。

### 签名

```ts
interface ContextMenuItem {
  label: string;       // 菜单项文字
  icon?: string;       // Emoji 图标
  onClick: () => void; // 点击回调
  danger?: boolean;    // 红色危险样式
}

function useWidgetContextMenu(items: ContextMenuItem[]): void;
```

### 用法

```tsx
import { useWidgetContextMenu } from "../../widgets/WidgetContext";

export default function MyPanel() {
  useWidgetContextMenu([
    { label: "刷新", icon: "🔄", onClick: () => fetchData() },
    { label: "清空", icon: "🗑️", danger: true, onClick: () => clearAll() },
  ]);

  return <div>...</div>;
}
```

### 注意事项

- 传入的 items 数组会在每次渲染时替换之前的菜单项（而非追加）
- 组件卸载时自动调用 `registerContextMenuItems([])` 清除
- 菜单项在用户右键挂件主体区域时弹出（标题栏区域不触发）
- 使用 `useRef` 内部稳定引用，避免不必要的 effect 重触发

---

## useWidgetContext

**来源**：`src/widgets/WidgetContext.ts`

读取 WidgetShell 提供的上下文，包括折叠状态和已注册的菜单项。

### 签名

```ts
interface WidgetContextValue {
  collapsed: boolean;
  contextMenuItems: ContextMenuItem[];
  registerContextMenuItems: (items: ContextMenuItem[]) => void;
}

function useWidgetContext(): WidgetContextValue;
```

### 用法

```tsx
import { useWidgetContext } from "../../widgets/WidgetContext";

export default function MyPanel() {
  const { collapsed } = useWidgetContext();

  return (
    <div>
      {collapsed ? "已折叠" : "展开内容"}
    </div>
  );
}
```

### 注意事项

- 一般不需要直接使用此 hook，`useWidgetContextMenu` 已封装了常见的注册逻辑
- `collapsed` 状态由 `WidgetShell` 管理，当用户点击折叠按钮时切换
- 折叠时 `WidgetShell` 会将 `.widget-body` 设为 `display: none`，所以你的组件通常不需要感知折叠状态

---

## Hook 使用决策

```
需要自动调整窗口大小？
  └─ 是 → useAutoResize(containerRef)
  └─ 否 → 在 manifest 中设置固定 defaultHeight

需要显示成功/错误消息或更新副标题？
  └─ 是 → useWidget()
  └─ 否 → 不需要

需要自定义右键菜单？
  └─ 是 → useWidgetContextMenu(items)
  └─ 否 → 不需要

需要读取折叠状态？
  └─ 是 → useWidgetContext()
  └─ 否 → 不需要
```
