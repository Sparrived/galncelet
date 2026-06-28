# 可用 Hooks

Galncelet 提供少量宿主 hooks 和 context，帮助插件处理窗口尺寸、通用反馈、右键菜单和标题栏行为。

## `useAutoResize`

位置：`src/widgets/useAutoResize.ts`

用途：根据内容高度自动调整当前 widget 窗口高度，适合内容高度随数据变化的插件。

典型用法：

```tsx
import { useRef } from "react";
import { useAutoResize } from "../../widgets/useAutoResize";

export default function ExamplePanel() {
  const rootRef = useRef<HTMLDivElement>(null);
  useAutoResize(rootRef);
  return <div ref={rootRef}>...</div>;
}
```

注意：

- 传入的 ref 必须绑定到内容根节点。
- 内容频繁变化时要避免布局抖动。
- 固定尺寸插件可以不使用。

## `WidgetProvider` / `WidgetContext`

位置：`src/lib/context.tsx`、`src/widgets/WidgetContext.ts`

宿主在 `src/App.tsx` 中用 `WidgetProvider` 包裹插件，向插件提供：

- `refresh()`：请求刷新。
- `showResult(message)`：展示成功/结果信息。
- `showError(message)`：展示错误信息。
- `onStatusChange(status)`：报告状态文本。

使用方式：

```tsx
import { useWidgetContext } from "../../widgets/WidgetContext";

export default function ExamplePanel() {
  const { showError } = useWidgetContext();
  return <button onClick={() => showError("操作失败")}>测试</button>;
}
```

如果插件不需要宿主反馈，可以不使用。

## 右键菜单

`WidgetShell` 支持 context menu 注册。适合插件向标题栏或窗口区域注入自定义操作。

建议：

- 菜单项文案简短。
- 卸载组件时自动移除菜单项。
- 不要用右键菜单承载核心路径，核心操作仍应在插件 UI 中可见。

## 标题栏按钮

`WidgetShell` 根据插件 registry 字段控制按钮：

- `showCloseButton`
- `showCollapseButton`
- `showAttachButton`
- `defaultAttachEnabled`
- `defaultAttachRemember`
- `defaultWhitelist`

插件通常不需要直接调用 `CloseButton`、`CollapseButton`、`AttachButton`、`RememberButton`。只需在 `manifest.json` 和 `index.tsx` 中声明行为。

## Tauri hooks 与监听 cleanup

如果插件直接使用 Tauri event：

```tsx
useEffect(() => {
  let unlisten: (() => void) | undefined;
  listen("plugin://event", handler).then((fn) => { unlisten = fn; });
  return () => unlisten?.();
}, []);
```

如果插件使用 interval：

```tsx
useEffect(() => {
  const interval = setInterval(loadData, 1000);
  return () => clearInterval(interval);
}, []);
```

所有监听、定时器、WebSocket、动画循环都必须 cleanup。

## Hook 选择建议

| 场景 | 建议 |
| --- | --- |
| 内容高度动态变化 | `useAutoResize` |
| 需要向用户显示错误 | `useWidgetContext().showError` 或插件内错误条 |
| 需要额外操作菜单 | 右键菜单 context |
| 周期性刷新 | `useEffect + setInterval + cleanup` |
| 后端主动通知 | Tauri `listen + cleanup` |