# Galncelet 插件开发指南（人类版）

Galncelet 是一个基于 Tauri 2 + React 的桌面悬浮挂件系统。插件由两部分组成：

- 前端插件：位于 `src/addons/<plugin-id>/`，提供 `manifest.json`、`index.tsx`、面板组件、样式和前端 API。
- 后端插件：位于 `src-tauri/src/<rust_module>/`，提供 Tauri command、后台任务、状态管理或系统能力。

应用启动时会自动加载所有 `src/addons/*/index.tsx` 前端插件；Tauri 构建时会由 `src-tauri/build.rs` 嵌入所有 `src/addons/*/manifest.json`，并自动发现 `src-tauri/src/*/mod.rs` 后端插件模块。

## 目录结构

推荐每个插件使用以下结构：

```text
src/addons/<plugin-id>/
  manifest.json        # 插件元数据，会被 build.rs 嵌入到 Rust 侧
  index.tsx            # 前端注册入口，必须调用 registerPlugin
  <Panel>.tsx          # React 面板组件
  api.ts               # invoke 封装（可选）
  types.ts             # 前端类型（可选）
  styles.css           # 插件样式（可选）

src-tauri/src/<rust_module>/
  mod.rs               # Rust 后端插件入口，必须提供 pub fn setup(app: &tauri::AppHandle)
```

当前内置插件包括：

| 插件 ID | 前端目录 | 后端模块 | 说明 |
| --- | --- | --- | --- |
| `git` | `src/addons/git` | `src-tauri/src/git` | Git 状态、diff、提交、分支、远程仓库、watcher |
| `system-monitor` | `src/addons/system-monitor` | `src-tauri/src/system_monitor` | CPU、GPU、内存、磁盘、网络指标 |
| `clipboard-history` | `src/addons/clipboard-history` | `src-tauri/src/clipboard_history` | 剪贴板历史、搜索、回填、持久化 |
| `page-notes` | `src/addons/page-notes` | `src-tauri/src/page_notes` | 按浏览器页面 URL 匹配笔记，包含浏览器扩展资源 |
| `music-player` | `src/addons/music-player` | `src-tauri/src/music_player` | 系统媒体会话、播放控制、歌词 |
| `amkr` | `src/addons/amkr` | `src-tauri/src/amkr` | AMKR 指标、模型设置、事件 WebSocket |

> 注意：前端插件 ID 可以包含短横线；Rust 模块名必须是合法 Rust 标识符，通常用下划线，例如 `system-monitor` 对应 `system_monitor`。

## 插件生命周期

1. `src-tauri/build.rs` 扫描 `src/addons/*/manifest.json`，把 manifest 编译进 Rust 二进制。
2. `src-tauri/build.rs` 扫描 `src-tauri/src/*/mod.rs`，生成 `src-tauri/src/_plugins.rs`，并调用每个模块的 `setup(app)`。
3. `src/App.tsx` 使用 `import.meta.glob("./addons/*/index.tsx")` 动态导入所有前端插件入口。
4. 每个 `index.tsx` 调用 `registerPlugin(def)` 写入前端插件注册表。
5. 主进程根据用户设置和 manifest 创建 `widget-<plugin-id>` 窗口。
6. 插件面板渲染在 `WidgetShell` 内，获得关闭、折叠、吸附、记忆位置、右键菜单等通用能力。

## manifest.json

`manifest.json` 是插件对宿主的声明。示例：

```json
{
  "id": "example",
  "title": "示例插件",
  "description": "展示插件开发的最小结构",
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

字段说明：

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `id` | `string` | 是 | 插件唯一 ID；同时用于窗口 query、设置 key、窗口标签后缀 `widget-<id>` |
| `title` | `string` | 是 | 面板标题和管理页显示名 |
| `description` | `string` | 否 | 管理页中的简短说明 |
| `icon` | `string` | 否 | 管理页显示的 emoji 或短文本图标 |
| `defaultWidth` | `number` | 否 | 默认窗口宽度，逻辑像素，默认 360 |
| `defaultHeight` | `number` | 否 | 默认窗口高度，逻辑像素，默认 600 |
| `showCloseButton` | `boolean` | 否 | 是否显示关闭按钮，默认 true |
| `showCollapseButton` | `boolean` | 否 | 是否显示折叠按钮，默认 true |
| `showAttachButton` | `boolean` | 否 | 是否显示吸附按钮，默认 true |
| `defaultAttachEnabled` | `boolean` | 否 | 是否默认吸附到前台窗口，默认 true |
| `defaultAttachRemember` | `boolean` | 否 | 是否只记忆显示/隐藏而不移动位置，默认 false |
| `defaultWhitelist` | `string[]` | 否 | 默认吸附白名单，匹配前台窗口标题或进程名片段；空数组表示不限制 |

约束：

- `id` 必须稳定，发布后不要随意修改，否则用户设置会丢失。
- `id` 建议使用小写字母、数字和短横线：`page-notes`、`music-player`。
- manifest 必须是合法 JSON，不能有注释或尾随逗号。
- manifest 中声明的窗口尺寸应覆盖常用内容，折叠状态由 `WidgetShell` 处理。

## 前端注册入口

每个插件必须提供 `src/addons/<plugin-id>/index.tsx`：

```tsx
import { registerPlugin } from "../registry";
import "./styles.css";
import ExamplePanel from "./ExamplePanel";

registerPlugin({
  id: "example",
  title: "示例插件",
  description: "展示插件开发的最小结构",
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

前端注册字段与 manifest 基本一致，但额外需要 `component`。为了避免管理页和实际窗口表现不一致，`index.tsx` 中的元数据应与 `manifest.json` 保持一致。

## React 面板组件

面板组件只负责内容区；标题栏、拖拽、关闭、折叠、吸附按钮由 `WidgetShell` 提供。

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

建议：

- 使用插件目录内的 CSS class 前缀，例如 `.example-panel`，避免污染其他插件。
- 定时刷新必须在 `useEffect` cleanup 中清理。
- IPC 调用错误应在插件内友好展示，不要让 Promise rejection 泄漏到控制台。
- 不要在插件组件中直接创建 Tauri 窗口；使用宿主提供的管理页和窗口生命周期。

## 前端 API 封装

插件前端通过 `@tauri-apps/api/core` 的 `invoke` 调用后端命令。推荐统一封装在插件自己的 `api.ts`：

```ts
import { invoke } from "@tauri-apps/api/core";

export async function getExampleData(): Promise<string> {
  return invoke<string>("get_example_data");
}
```

命令名必须与 Rust `#[tauri::command]` 函数名一致，并且该命令必须注册到 `src-tauri/src/main.rs` 的 `tauri::generate_handler![...]`。

## Rust 后端插件

每个后端插件模块建议使用：

```rust
use std::sync::{Arc, Mutex};

#[derive(Default)]
pub struct ExampleState {
    value: Mutex<String>,
}

pub fn setup(app: &tauri::AppHandle) {
    app.manage(Arc::new(ExampleState::default()));
}

#[tauri::command]
pub fn get_example_data(state: tauri::State<'_, Arc<ExampleState>>) -> Result<String, String> {
    state
        .value
        .lock()
        .map(|value| value.clone())
        .map_err(|_| "Example state lock poisoned".to_string())
}
```

约定：

- `setup(app)` 必须存在，即使插件暂时不需要初始化，也应保留空函数。
- 长任务不要阻塞 UI 线程；使用 `tauri::async_runtime::spawn` 或后台线程。
- command 返回 `Result<T, String>`，错误信息应适合直接显示给用户。
- 共享状态用 `app.manage(...)` 注入，前端调用时通过 `tauri::State` 获取。
- Windows 专属能力必须加 `#[cfg(windows)]`，非 Windows 平台提供降级分支。

## 注册后端命令

`build.rs` 会自动生成模块声明和 `setup_all(app)`，但不会自动注册 command。新增 `#[tauri::command]` 后，需要手动加入 `src-tauri/src/main.rs`：

```rust
.invoke_handler(tauri::generate_handler![
    example::get_example_data,
])
```

如果插件 ID 为 `example-plugin`，Rust 模块通常叫 `example_plugin`，注册时使用 `example_plugin::command_name`。

## 窗口与吸附能力

宿主会为每个启用的插件创建一个透明、无边框、置顶、跳过任务栏的窗口，窗口标签为 `widget-<plugin-id>`。

通用能力：

- 关闭：隐藏窗口并更新插件可见性。
- 折叠：保留标题栏高度或插件指定的折叠高度。
- 吸附：跟随前台窗口移动和显示/隐藏。
- 白名单：只在匹配的前台窗口标题或进程名中吸附。
- 记忆位置：`defaultAttachRemember` 或用户按钮可切换只管理显示/隐藏。
- 序列切换：全局设置可配置多个 widget 共用位置并用热键轮换。

插件不应自行假设窗口大小、屏幕缩放或 DPI；需要尺寸时优先监听容器布局。

## 设置与持久化

全局设置类型在 `src/lib/types.ts` 和 `src-tauri/src/settings.rs` 中维护。插件可以使用两类持久化方式：

- 宿主级设置：适合插件可见性、窗口位置、热键、吸附配置等通用数据。
- 插件私有文件：适合插件业务数据，例如剪贴板历史、页面笔记、缓存。

插件私有文件建议放在 Tauri app data 目录下：

```rust
let data_dir = app.path().app_data_dir()?;
let path = data_dir.join("example.json");
```

## 事件通信

如果后端需要主动通知前端，可以使用 Tauri event：

```rust
use tauri::Emitter;

app.emit("example://updated", payload).map_err(|e| e.to_string())?;
```

前端监听时需在组件卸载时取消监听：

```ts
import { listen } from "@tauri-apps/api/event";

useEffect(() => {
  let unlisten: (() => void) | undefined;
  listen("example://updated", (event) => {
    console.log(event.payload);
  }).then((fn) => { unlisten = fn; });
  return () => unlisten?.();
}, []);
```

## 浏览器扩展资源

如果插件需要随包发布静态资源，可以把资源放入插件目录，并在 `src-tauri/tauri.conf.json` 的 `bundle.resources` 中配置。例如 `page-notes` 会打包：

```json
"resources": [
  "../src/addons/page-notes/browser-extension/*"
]
```

## 开发流程

1. 创建 `src/addons/<plugin-id>/manifest.json`。
2. 创建 `src/addons/<plugin-id>/index.tsx` 并调用 `registerPlugin`。
3. 创建面板组件、样式、前端 `api.ts`。
4. 如需系统能力，创建 `src-tauri/src/<rust_module>/mod.rs` 并实现 `setup(app)`。
5. 将新增 Tauri command 加入 `src-tauri/src/main.rs` 的 `generate_handler!`。
6. 运行 `npm run build` 检查前端类型和 Vite 构建。
7. 运行 `cargo check` 或 `npm run tauri -- build --target x86_64-pc-windows-msvc` 检查后端。
8. 打开管理页启用插件，验证窗口、关闭、折叠、吸附、设置持久化。

## 发布与版本

当前发布流程位于：

- `scripts/release.ps1`
- `.github/workflows/release.yml`

发布脚本会：

1. 可选同步 `package.json`、`src-tauri/tauri.conf.json`、`src-tauri/Cargo.toml` 版本。
2. 执行 `npm ci`。
3. 执行 `npm run tauri -- build --target x86_64-pc-windows-msvc`。
4. 校验 Windows GUI subsystem，避免发布版弹出控制台窗口。
5. 生成 `src-tauri/target/x86_64-pc-windows-msvc/release/bundle/SHA256SUMS.txt`。
6. 可选通过 GitHub CLI 发布到 GitHub Releases。

本项目的更新检查器读取 GitHub Releases latest release，并用 `v*` tag 与当前应用版本比较。因此发布新版本时必须创建形如 `v1.0.0` 的 tag/release，并确保应用内版本号同步。

常用命令：

```powershell
npm run build
cargo check --manifest-path src-tauri/Cargo.toml
npm run release -- -Version 1.0.0 -Tag v1.0.0
npm run release:publish -- -Version 1.0.0 -Tag v1.0.0
```

## 质量检查清单

提交插件前请确认：

- `manifest.json` 与 `index.tsx` 元数据一致。
- 插件 ID 稳定且唯一。
- 所有 `invoke` 都有对应 Rust command，并已注册 handler。
- 所有定时器、监听器、WebSocket、文件 watcher 都有 cleanup 或停止逻辑。
- Rust command 不会长时间阻塞 UI。
- 错误信息可读，并能被前端展示。
- 样式 class 使用插件前缀，避免影响宿主或其他插件。
- Windows 专属功能有 `#[cfg(windows)]` 或降级分支。
- `npm run build` 和 `cargo check` 通过。

---

## Runtime addons for third-party development

The `src/addons/*` + `src-tauri/src/*` model described in this guide is the legacy built-in addon model. It is still valid for features shipped with Galncelet itself, but it requires rebuilding the main app.

For third-party addons that users can install or remove by copying files, use the runtime addon model instead:

- install folder: `%APPDATA%\Galncelet\addons\<addon-id>\`
- frontend entry: `manifest.json` + `ui/index.html`
- optional backend: sidecar executable using JSON-RPC over stdin/stdout

See `docs/runtime-addons.md` for the current hot-pluggable addon protocol.
