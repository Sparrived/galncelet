# Rust 后端集成

插件通过 Tauri 的 IPC 机制与 Rust 后端通信。前端调用 `invoke()` 发送命令，后端通过 `#[tauri::command]` 处理。也可以通过事件系统实现后端向前端的实时推送。

## 架构概览

```
┌─────────────┐    invoke("cmd", { args })    ┌──────────────┐
│  前端 api.ts │ ───────────────────────────→ │ Rust command  │
└─────────────┘                                └──────────────┘
                                                      │
┌─────────────┐    listen("event-name")       ┌──────────────┐
│  前端 Panel  │ ←─────────────────────────── │  app.emit()  │
└─────────────┘                                └──────────────┘
```

## 插件后端目录结构

```
src-tauri/src/my-plugin/          ← 后端插件目录
└── mod.rs                         ← setup() + #[tauri::command] 函数

src-tauri/src/
├── main.rs                        ← 在 generate_handler![] 中注册命令
├── _plugins.rs                    ← 【自动生成】mod 声明 + setup_all()
├── acrylic.rs                     ← 框架模块
├── plugins.rs                     ← 框架模块（manifest 加载）
├── settings.rs                    ← 框架模块（设置持久化）
├── tray.rs                        ← 框架模块（系统托盘）
├── window_attach.rs               ← 框架模块（窗口吸附）
├── amkr/                          ← AMKR 插件后端
│   └── mod.rs
├── browser_ext/                   ← 浏览器扩展插件后端
│   └── mod.rs
├── clipboard_history/             ← 剪贴板历史插件后端
│   └── mod.rs
├── git/                           ← Git 插件后端
│   └── mod.rs
├── music_player/                  ← 音乐播放器插件后端
│   └── mod.rs
├── page_notes/                    ← 页面笔记插件后端
│   └── mod.rs
└── system_monitor/                ← 系统监控插件后端
    └── mod.rs
```

## 自动发现机制（重要）

`build.rs` 会自动扫描 `src-tauri/src/` 下所有包含 `mod.rs` 的子目录，自动生成 `_plugins.rs`：

```rust
// _plugins.rs（自动生成，不要手动编辑）
#[path = "amkr/mod.rs"]
pub mod amkr;
#[path = "clipboard_history/mod.rs"]
pub mod clipboard_history;
// ...

pub fn setup_all(app: &tauri::AppHandle) {
    amkr::setup(app);
    clipboard_history::setup(app);
    // ...
}
```

`main.rs` 中只需要：

```rust
mod _plugins;
use _plugins::{amkr, clipboard_history, /* ... */};

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            _plugins::setup_all(&app.handle());  // 自动调用所有插件的 setup()
            // ...
        })
}
```

**你不需要**在 `main.rs` 中写 `mod my_plugin;`。

**你仍然需要**在 `main.rs` 的 `generate_handler![]` 中注册你的 `#[tauri::command]` 函数。

---

## 步骤 1：定义 Rust 命令

在 `src-tauri/src/my_plugin/mod.rs` 中创建模块：

```rust
use serde::{Deserialize, Serialize};

/// 数据结构 —— 需要 Serialize 以便返回给前端
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct MyItem {
    pub id: String,
    pub name: String,
    pub value: f64,
}

/// 查询命令 —— 返回数据给前端
#[tauri::command]
pub async fn fetch_my_items() -> Result<Vec<MyItem>, String> {
    // 你的业务逻辑
    let items = vec![
        MyItem {
            id: "1".into(),
            name: "示例".into(),
            value: 42.0,
        },
    ];
    Ok(items)
}

/// 操作命令 —— 执行动作，返回 Result<(), String>
#[tauri::command]
pub async fn delete_item(id: String) -> Result<(), String> {
    // 删除逻辑
    // 错误时返回 Err("错误信息".into())
    Ok(())
}

/// 初始化插件 —— build.rs 会自动调用此函数
pub fn setup(app: &tauri::AppHandle) {
    use tauri::Manager;
    // 如果需要初始化状态，在这里做
    // 例如：app.manage(MyState::new());
}
```

### 命令规则

- 必须标注 `#[tauri::command]`
- 函数名使用 snake_case（前端调用时也用 snake_case）
- 返回类型通常为 `Result<T, String>`，`Err` 会作为异常传到前端
- 参数需要实现 `Deserialize`，返回值需要实现 `Serialize`
- 使用 `async` 可以在命令中执行异步操作

### 访问 Tauri App Handle

如果需要在命令中访问 Tauri 应用句柄（如发射事件、访问路径等）：

```rust
#[tauri::command]
pub async fn my_command(app: tauri::AppHandle) -> Result<(), String> {
    let data_dir = app.path().app_data_dir()
        .map_err(|e| e.to_string())?;
    // ...
    Ok(())
}
```

## 步骤 2：注册命令

在 `src-tauri/src/main.rs` 的 `generate_handler![]` 中添加：

```rust
.invoke_handler(tauri::generate_handler![
    // ... 已有命令 ...
    my_plugin::fetch_my_items,
    my_plugin::delete_item,
])
```

`build.rs` 会自动处理 `mod my_plugin;` 声明，你只需要：
1. 创建 `src-tauri/src/my_plugin/mod.rs`
2. 在 `generate_handler![]` 中注册命令

## 步骤 3：前端 API 封装

在插件目录下创建 `api.ts`，为每个 Rust 命令封装一个 TypeScript 函数：

```ts
import { invoke } from "@tauri-apps/api/core";
import type { MyItem } from "./types";

export async function fetchMyItems(): Promise<MyItem[]> {
  return invoke<MyItem[]>("fetch_my_items");
}

export async function deleteItem(id: string): Promise<void> {
  return invoke<void>("delete_item", { id });
}
```

### invoke 规则

- 第一个参数是命令名字符串（snake_case，与 Rust 函数名一致）
- 第二个参数是参数对象，键名与 Rust 函数参数名一致
- 泛型参数指定返回类型
- Rust 的 `Err(String)` 会导致 Promise reject

## 步骤 4：前端调用

在 Panel 组件中使用封装好的 API 函数：

```tsx
import { useEffect, useState } from "react";
import { fetchMyItems, deleteItem } from "./api";
import type { MyItem } from "./types";

export default function MyPanel() {
  const [items, setItems] = useState<MyItem[]>([]);

  useEffect(() => {
    fetchMyItems().then(setItems).catch(console.error);
  }, []);

  const handleDelete = async (id: string) => {
    try {
      await deleteItem(id);
      // 刷新列表
      const updated = await fetchMyItems();
      setItems(updated);
    } catch (e) {
      console.error("删除失败:", e);
    }
  };

  return (
    <div>
      {items.map(item => (
        <div key={item.id}>
          {item.name}
          <button onClick={() => handleDelete(item.id)}>删除</button>
        </div>
      ))}
    </div>
  );
}
```

## 事件系统

除了请求-响应模式，Tauri 还支持事件驱动的实时通信。

### 后端发射事件

```rust
use tauri::Emitter;

#[tauri::command]
pub async fn start_monitoring(app: tauri::AppHandle) -> Result<(), String> {
    // 在后台任务中发射事件
    let app_clone = app.clone();
    tokio::spawn(async move {
        loop {
            let data = collect_metrics();
            app_clone
                .emit("my-plugin-update", &data)
                .unwrap_or_default();
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        }
    });
    Ok(())
}
```

### 前端监听事件

```tsx
import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";

interface UpdatePayload {
  value: number;
  timestamp: string;
}

export default function MyPanel() {
  const [latest, setLatest] = useState<UpdatePayload | null>(null);

  useEffect(() => {
    const unlisten = listen<UpdatePayload>("my-plugin-update", (event) => {
      setLatest(event.payload);
    });

    // 清理：组件卸载时取消监听
    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  return <div>{latest?.value ?? "等待数据…"}</div>;
}
```

### 事件命名规范

- 使用 kebab-case，如 `my-plugin-update`、`git-changed`
- 事件名是全局的，建议加插件前缀避免冲突
- payload 需要 Rust 端实现 `Serialize`，前端指定泛型类型

## 数据持久化

### 使用全局设置

如果需要跨插件共享或持久化数据，可以使用全局设置 API：

```ts
import { loadSettings, saveSettings } from "../../lib/api";

// 读取
const settings = await loadSettings();
const myList = (settings as any).myPluginData ?? [];

// 写入
await saveSettings({
  ...settings,
  myPluginData: newList,
});
```

> **注意**：`AppSettings` 类型定义在 `src/lib/types.ts`。如果你的插件需要额外的持久化字段，需要先在 `AppSettings` 接口中添加，同时在 Rust 端的 `AppSettings` 结构体中同步添加。

当前 `AppSettings` 包含的字段（`src/lib/types.ts`）：

```ts
interface AppSettings {
  refreshIntervalMs: number;    // 全局刷新间隔（默认 2000ms）
  cardWidth: number;            // 挂件窗口宽度（默认 360px）
  logMaxCount: number;          // 日志最大条数（默认 50）
  alwaysOnTop: boolean;         // 窗口置顶（默认 true）
  pullRebase: boolean;          // git pull 使用 rebase（默认 true）
  savedRepos: string[];         // Git 保存的仓库路径
  currentRepo?: string;         // 当前活跃的仓库路径
  panelVisibility: Record<string, boolean>;  // 插件可见性
  windowStates: Record<string, WindowState>; // 每个挂件的窗口状态
  hideFullscreen: boolean;      // 全屏时隐藏挂件（默认 true）
  pluginHotkeys: Record<string, string>;     // 插件快捷键
  widgetSequence: string[];     // 挂件序列（共享位置，热键切换）
  sequenceHotkey: string | null;// 序列切换热键
}
```

### 使用独立存储

也可以通过自定义 Tauri 命令实现独立的存储：

```rust
#[tauri::command]
pub async fn load_my_config(app: tauri::AppHandle) -> Result<MyConfig, String> {
    let path = app.path().app_data_dir()
        .map_err(|e| e.to_string())?
        .join("my_config.json");
    // 读取并解析 JSON
    Ok(config)
}

#[tauri::command]
pub async fn save_my_config(app: tauri::AppHandle, config: MyConfig) -> Result<(), String> {
    let path = app.path().app_data_dir()
        .map_err(|e| e.to_string())?
        .join("my_config.json");
    // 序列化并写入
    Ok(())
}
```

## 全局热键

框架支持两种热键：

1. **插件独立热键**：每个插件可以绑定一个全局快捷键，触发时显示/隐藏该插件窗口
2. **序列切换热键**：当多个插件使用 widget sequence 共享位置时，热键可以在它们之间循环切换

### 设置插件热键

```ts
import { setPluginHotkey } from "../../lib/api";

// 设置热键
await setPluginHotkey("my-plugin", "ctrl+shift+m");

// 清除热键
await setPluginHotkey("my-plugin", null);
```

### 设置序列热键

```ts
import { setSequenceHotkey, setWidgetSequence } from "../../lib/api";

// 设置序列
await setWidgetSequence(["plugin-a", "plugin-b", "plugin-c"]);

// 设置序列切换热键
await setSequenceHotkey("ctrl+shift+tab");
```

## 完整示例：计数器插件

### Rust (`src-tauri/src/counter/mod.rs`)

```rust
use std::sync::Mutex;
use tauri::State;

pub struct CounterState(pub Mutex<i64>);

#[tauri::command]
pub fn get_counter(state: State<CounterState>) -> i64 {
    *state.0.lock().unwrap()
}

#[tauri::command]
pub fn increment_counter(state: State<CounterState>) -> i64 {
    let mut val = state.0.lock().unwrap();
    *val += 1;
    *val
}

pub fn setup(_app: &tauri::AppHandle) {
    // setup 由 build.rs 自动调用
}
```

### main.rs 注册

```rust
// 只需要在 generate_handler![] 中注册命令
.invoke_handler(tauri::generate_handler![
    // ... 已有命令 ...
    counter::get_counter,
    counter::increment_counter,
])
```

> **不需要**在 main.rs 中写 `mod counter;` —— build.rs 会自动生成。

### 前端 api.ts

```ts
import { invoke } from "@tauri-apps/api/core";

export async function getCounter(): Promise<number> {
  return invoke<number>("get_counter");
}

export async function incrementCounter(): Promise<number> {
  return invoke<number>("increment_counter");
}
```
