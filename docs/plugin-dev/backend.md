# Rust 后端集成

插件通过 Tauri 的 IPC 机制与 Rust 后端通信。前端调用 `invoke()` 发送命令，后端通过 `#[tauri::command]` 处理。也可以通过事件系统实现后端向前端的实时推送。

## 流程概览

```
┌─────────────┐    invoke("cmd", { args })    ┌──────────────┐
│  前端 api.ts │ ───────────────────────────→ │ Rust command  │
└─────────────┘                                └──────────────┘
                                                      │
┌─────────────┐    listen("event-name")       ┌──────────────┐
│  前端 Panel  │ ←─────────────────────────── │  app.emit()  │
└─────────────┘                                └──────────────┘
```

## 步骤 1：定义 Rust 命令

在 `src-tauri/src/` 下创建新模块文件，如 `my_plugin.rs`：

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

在 `src-tauri/src/main.rs` 中：

```rust
// 1. 在文件顶部添加模块声明
mod my_plugin;

// 2. 在 invoke_handler 的 generate_handler![] 中添加命令
.invoke_handler(tauri::generate_handler![
    // ... 已有命令 ...
    my_plugin::fetch_my_items,
    my_plugin::delete_item,
])
```

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

## 完整示例：计数器插件

### Rust (`src-tauri/src/counter.rs`)

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
```

### main.rs 注册

```rust
mod counter;

// 在 Builder 中管理状态
.manage(counter::CounterState(Mutex::new(0)))

// 注册命令
.invoke_handler(tauri::generate_handler![
    counter::get_counter,
    counter::increment_counter,
])
```

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
