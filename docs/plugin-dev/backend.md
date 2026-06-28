# Rust 后端集成

插件通过 Tauri IPC 与 Rust 后端通信。前端使用 `invoke()` 调用命令，后端通过 `#[tauri::command]` 处理；后端也可以使用 Tauri event 主动通知前端。

## 架构概览

```text
React Panel -> plugin/api.ts -> invoke("command_name") -> Rust #[tauri::command]
                                                       -> State<T> / 文件 / 系统 API
Rust background task -> app.emit("plugin://event") -> React listen(...)
```

## 后端目录结构

```text
src-tauri/src/<rust_module>/
  mod.rs
```

示例：

```text
src-tauri/src/git/mod.rs
src-tauri/src/system_monitor/mod.rs
src-tauri/src/clipboard_history/mod.rs
src-tauri/src/page_notes/mod.rs
src-tauri/src/music_player/mod.rs
src-tauri/src/amkr/mod.rs
```

Rust 模块名必须使用 snake_case。前端插件 ID 可用短横线，例如 `music-player` 对应 `music_player`。

## 自动发现机制

`src-tauri/build.rs` 会扫描 `src-tauri/src/*/mod.rs` 并生成 `src-tauri/src/_plugins.rs`：

```rust
#[path = "music_player/mod.rs"]
pub mod music_player;

pub fn setup_all(app: &tauri::AppHandle) {
    music_player::setup(app);
}
```

因此每个插件模块都必须导出：

```rust
pub fn setup(app: &tauri::AppHandle) {
    // 初始化 state、后台任务或 no-op
}
```

不要手动编辑 `_plugins.rs`。

## 定义状态

```rust
use std::sync::{Arc, Mutex};

#[derive(Default)]
pub struct CounterState {
    count: Mutex<i32>,
}

pub fn setup(app: &tauri::AppHandle) {
    app.manage(Arc::new(CounterState::default()));
}
```

建议：

- 多 command 共享数据时使用 `app.manage`。
- 用 `Arc<Mutex<T>>`、`Arc<RwLock<T>>` 或专门的 manager 类型封装状态。
- 锁中不要执行慢 IO、网络请求或长时间计算。

## 定义命令

```rust
#[tauri::command]
pub fn get_counter(state: tauri::State<'_, Arc<CounterState>>) -> Result<i32, String> {
    state
        .count
        .lock()
        .map(|count| *count)
        .map_err(|_| "Counter state lock poisoned".to_string())
}

#[tauri::command]
pub fn set_counter(state: tauri::State<'_, Arc<CounterState>>, value: i32) -> Result<(), String> {
    let mut count = state
        .count
        .lock()
        .map_err(|_| "Counter state lock poisoned".to_string())?;
    *count = value;
    Ok(())
}
```

命令规则：

- 必须 `pub`。
- 推荐返回 `Result<T, String>`。
- 返回给前端的结构体需要 `Serialize`。
- 接收复杂参数的结构体需要 `Deserialize`。
- 字段建议使用 `#[serde(rename_all = "camelCase")]`。
- Windows 专属代码要用 `#[cfg(windows)]` 隔离。

## 注册命令

新增命令后，必须加入 `src-tauri/src/main.rs`：

```rust
.invoke_handler(tauri::generate_handler![
    counter::get_counter,
    counter::set_counter,
])
```

`build.rs` 不会自动注册 command。

## 前端封装

```ts
import { invoke } from "@tauri-apps/api/core";

export async function getCounter(): Promise<number> {
  return invoke<number>("get_counter");
}

export async function setCounter(value: number): Promise<void> {
  return invoke<void>("set_counter", { value });
}
```

Tauri 参数名按 camelCase 传递：Rust `window_label` 对应前端 `{ windowLabel }`。

## 事件系统

后端主动发事件：

```rust
use tauri::Emitter;

#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CounterChanged {
    pub value: i32,
}

app.emit("counter://changed", CounterChanged { value })
    .map_err(|e| e.to_string())?;
```

前端监听：

```tsx
import { listen } from "@tauri-apps/api/event";
import { useEffect } from "react";

useEffect(() => {
  let unlisten: (() => void) | undefined;
  listen<{ value: number }>("counter://changed", (event) => {
    console.log(event.payload.value);
  }).then((fn) => { unlisten = fn; });
  return () => unlisten?.();
}, []);
```

事件名建议使用 `plugin-id://event-name`，避免跨插件冲突。

## 数据持久化

插件私有数据应写入 Tauri app data 目录：

```rust
use tauri::Manager;

fn data_path(app: &tauri::AppHandle) -> Result<std::path::PathBuf, String> {
    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to resolve app data dir: {e}"))?;
    std::fs::create_dir_all(&data_dir)
        .map_err(|e| format!("Failed to create app data dir: {e}"))?;
    Ok(data_dir.join("counter.json"))
}
```

不要写入源码目录、`dist`、`target` 或当前工作目录。

## 后台任务

```rust
pub fn setup(app: &tauri::AppHandle) {
    let app_handle = app.clone();
    tauri::async_runtime::spawn(async move {
        // loop / websocket / polling
        let _ = app_handle;
    });
}
```

要求：

- 避免重复启动同一后台任务。
- 提供停止条件或 manager，防止资源泄漏。
- 网络错误和系统 API 错误应记录或返回，不要 panic。

## 全局热键与窗口序列

宿主已经提供：

- 插件独立热键：显示/隐藏指定插件窗口。
- 序列热键：多个插件共享位置并循环切换。
- 管理 API：`setPluginHotkey`、`setWidgetSequence`、`setSequenceHotkey`。

普通插件不需要直接调用 `global_shortcut` 插件。只有修改宿主热键系统时才应编辑 `src-tauri/src/settings.rs`。

## 完整计数器示例

`src-tauri/src/counter/mod.rs`

```rust
use std::sync::{Arc, Mutex};

#[derive(Default)]
pub struct CounterState {
    count: Mutex<i32>,
}

pub fn setup(app: &tauri::AppHandle) {
    app.manage(Arc::new(CounterState::default()));
}

#[tauri::command]
pub fn get_counter(state: tauri::State<'_, Arc<CounterState>>) -> Result<i32, String> {
    state
        .count
        .lock()
        .map(|count| *count)
        .map_err(|_| "Counter state lock poisoned".to_string())
}

#[tauri::command]
pub fn increment_counter(state: tauri::State<'_, Arc<CounterState>>) -> Result<i32, String> {
    let mut count = state
        .count
        .lock()
        .map_err(|_| "Counter state lock poisoned".to_string())?;
    *count += 1;
    Ok(*count)
}
```

`src/addons/counter/api.ts`

```ts
import { invoke } from "@tauri-apps/api/core";

export async function getCounter(): Promise<number> {
  return invoke<number>("get_counter");
}

export async function incrementCounter(): Promise<number> {
  return invoke<number>("increment_counter");
}
```

验证：

```powershell
npm run build
cargo check --manifest-path src-tauri/Cargo.toml
```