# Galncelet 插件脚手架指南（AI 版）

本文件面向 AI/自动化代理，用于在 Galncelet 中稳定创建或修改插件。除非用户明确要求重构，否则保持最小、可验证、与现有风格一致的改动。

## 快速判断

当用户要求新增一个插件时，通常需要同时修改：

- `src/addons/<plugin-id>/manifest.json`
- `src/addons/<plugin-id>/index.tsx`
- `src/addons/<plugin-id>/<Name>Panel.tsx`
- `src/addons/<plugin-id>/api.ts`（如需 IPC）
- `src/addons/<plugin-id>/types.ts`（如需共享类型）
- `src/addons/<plugin-id>/styles.css`（如需样式）
- `src-tauri/src/<rust_module>/mod.rs`（如需后端能力）
- `src-tauri/src/main.rs` 的 `tauri::generate_handler![...]`（如新增 command）
- `src-tauri/tauri.conf.json` 的 `bundle.resources`（如需打包静态资源）

不要手动编辑 `src-tauri/src/_plugins.rs`。它由 `src-tauri/build.rs` 自动生成。

## 命名规则

- 前端插件目录和插件 ID：小写短横线，例如 `page-notes`。
- React 组件：PascalCase，例如 `PageNotesPanel.tsx`。
- Rust 模块：snake_case，例如 `page_notes`。
- Tauri command：snake_case，例如 `load_page_notes`。
- CSS class：用插件 ID 或简称做前缀，例如 `.page-notes-panel`。

映射示例：

```text
plugin id:    example-plugin
frontend dir: src/addons/example-plugin
rust module:  src-tauri/src/example_plugin
window label: widget-example-plugin
query:        index.html?widget=example-plugin
```

## 最小前端插件模板

`src/addons/example/manifest.json`

```json
{
  "id": "example",
  "title": "示例插件",
  "description": "展示一个最小 Galncelet 插件",
  "icon": "🧩",
  "defaultWidth": 360,
  "defaultHeight": 320,
  "showCloseButton": true,
  "showCollapseButton": true,
  "showAttachButton": true,
  "defaultAttachEnabled": true,
  "defaultAttachRemember": false,
  "defaultWhitelist": []
}
```

`src/addons/example/index.tsx`

```tsx
import { registerPlugin } from "../registry";
import ExamplePanel from "./ExamplePanel";
import "./styles.css";

registerPlugin({
  id: "example",
  title: "示例插件",
  description: "展示一个最小 Galncelet 插件",
  icon: "🧩",
  defaultWidth: 360,
  defaultHeight: 320,
  showCloseButton: true,
  showCollapseButton: true,
  showAttachButton: true,
  defaultAttachEnabled: true,
  defaultAttachRemember: false,
  defaultWhitelist: [],
  component: ExamplePanel,
});
```

`src/addons/example/ExamplePanel.tsx`

```tsx
export default function ExamplePanel() {
  return <div className="example-panel">示例插件已加载</div>;
}
```

`src/addons/example/styles.css`

```css
.example-panel {
  padding: 12px;
}
```

## 带 IPC 的前端模板

`src/addons/example/api.ts`

```ts
import { invoke } from "@tauri-apps/api/core";

export async function getExampleData(): Promise<string> {
  return invoke<string>("get_example_data");
}
```

`src/addons/example/ExamplePanel.tsx`

```tsx
import { useEffect, useState } from "react";
import { getExampleData } from "./api";

export default function ExamplePanel() {
  const [message, setMessage] = useState("加载中...");

  useEffect(() => {
    getExampleData().then(setMessage).catch((error) => setMessage(String(error)));
  }, []);

  return <div className="example-panel">{message}</div>;
}
```

## Rust 后端模板

`src-tauri/src/example/mod.rs`

```rust
use std::sync::{Arc, Mutex};

#[derive(Default)]
pub struct ExampleState {
    message: Mutex<String>,
}

pub fn setup(app: &tauri::AppHandle) {
    app.manage(Arc::new(ExampleState {
        message: Mutex::new("Hello from Rust".to_string()),
    }));
}

#[tauri::command]
pub fn get_example_data(state: tauri::State<'_, Arc<ExampleState>>) -> Result<String, String> {
    state
        .message
        .lock()
        .map(|message| message.clone())
        .map_err(|_| "Example state lock poisoned".to_string())
}
```

然后在 `src-tauri/src/main.rs` 的 `invoke_handler` 中加入：

```rust
example::get_example_data,
```

## 修改现有插件时的步骤

1. 先阅读该插件的 `manifest.json`、`index.tsx`、面板组件、`api.ts`、Rust `mod.rs`。
2. 确认是否已有同类 command、state、持久化路径、事件名。
3. 只修改该插件相关文件，除非必须注册 command 或调整宿主通用能力。
4. 新增 command 时同步前端 `api.ts` 类型。
5. 新增持久化时使用 Tauri app data 目录，不写入源码目录。
6. 新增后台任务时提供停止条件、cleanup 或可重复初始化保护。
7. 运行针对性验证：`npm run build`、`cargo check`，必要时运行相关 Rust 单测。

## 插件元数据同步规则

`manifest.json` 用于 Rust 侧窗口创建和管理页初始数据，`index.tsx` 用于前端 registry。两者重复字段必须同步：

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

如果只改其中一个，可能导致管理页、窗口创建、实际渲染表现不一致。

## 可用宿主 API

`src/lib/api.ts` 提供宿主级 API，包括：

- 设置：`loadSettings`、`saveSettings`
- 插件窗口：`createPluginWindow`、`setPluginVisible`
- 设置页和管理页：`openManageWindow`、`openSettingsWindow`、`openPluginSettings`
- 窗口状态：`saveWindowState`
- 吸附：`setAttachEnabled`、`setAttachWhitelist`、`setAttachRemember`
- 全局行为：`setHideInFullscreen`、`setStartOnBoot`
- 热键和序列：`setPluginHotkey`、`setWidgetSequence`、`setSequenceHotkey`
- 更新检查：`checkForUpdates`

插件业务 API 应放在插件自己的 `api.ts`，不要把所有插件命令都堆到 `src/lib/api.ts`。

## 后端 command 注册注意事项

- `build.rs` 只负责模块发现和 `setup(app)`，不负责 command handler。
- 新增 command 后必须手动注册到 `tauri::generate_handler![...]`。
- command 参数名会被 Tauri 前端序列化为 camelCase；前端 invoke 参数需要匹配 Rust 参数名的 camelCase。
- 返回结构体要 `#[derive(Serialize)]`，接收结构体要 `#[derive(Deserialize)]`。
- 对前端暴露的字段建议加 `#[serde(rename_all = "camelCase")]`。

## 资源打包规则

如果插件需要运行时访问静态文件：

1. 放在插件目录下，例如 `src/addons/page-notes/browser-extension/*`。
2. 在 `src-tauri/tauri.conf.json` 的 `bundle.resources` 中加入资源 glob。
3. 发布前运行 Tauri build 验证资源进入 bundle。

## 不要做的事

- 不要手动编辑 `src-tauri/src/_plugins.rs`。
- 不要修改插件 ID 来“修复”显示问题。
- 不要在 React render 阶段调用 `invoke`、创建 interval 或注册监听器。
- 不要在没有 cleanup 的情况下添加 `setInterval`、`listen`、WebSocket 或 watcher。
- 不要把插件私有状态写到仓库目录或 `dist`。
- 不要引入全局 CSS 选择器影响宿主和其他插件。
- 不要把 Windows-only API 暴露给非 Windows 构建且无降级。

## 验证命令

优先从快到慢：

```powershell
npm run build
cargo check --manifest-path src-tauri/Cargo.toml
cargo test --manifest-path src-tauri/Cargo.toml <module>::tests
npm run tauri -- build --target x86_64-pc-windows-msvc
```

发布验证使用：

```powershell
npm run release -- -Version 1.0.0 -Tag v1.0.0
```

该命令会生成 Windows 可用的 release exe、安装包和校验文件。