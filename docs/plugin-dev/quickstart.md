# 快速开始：从零创建一个插件

本教程创建一个“计数器”插件，包含前端面板和 Rust 后端 command。完成后，插件会出现在管理页中，并能在独立 widget 窗口里运行。

## 步骤总览

1. 创建前端插件目录。
2. 编写 `manifest.json`。
3. 编写前端 `api.ts`、面板、样式和 `index.tsx`。
4. 创建 Rust 后端模块。
5. 在 `main.rs` 注册 command。
6. 构建验证。

## 步骤 1：创建目录

```text
src/addons/counter/
src-tauri/src/counter/
```

## 步骤 2：创建 manifest

`src/addons/counter/manifest.json`

```json
{
  "id": "counter",
  "title": "计数器",
  "description": "一个最小的前后端插件示例",
  "icon": "🔢",
  "defaultWidth": 280,
  "defaultHeight": 180,
  "showCloseButton": true,
  "showCollapseButton": true,
  "showAttachButton": false,
  "defaultAttachEnabled": false,
  "defaultWhitelist": []
}
```

## 步骤 3：创建前端 API

`src/addons/counter/api.ts`

```ts
import { invoke } from "@tauri-apps/api/core";

export async function getCounter(): Promise<number> {
  return invoke<number>("get_counter");
}

export async function incrementCounter(): Promise<number> {
  return invoke<number>("increment_counter");
}

export async function resetCounter(): Promise<number> {
  return invoke<number>("reset_counter");
}
```

## 步骤 4：创建面板组件

`src/addons/counter/CounterPanel.tsx`

```tsx
import { useEffect, useState } from "react";
import { getCounter, incrementCounter, resetCounter } from "./api";

export default function CounterPanel() {
  const [count, setCount] = useState(0);
  const [error, setError] = useState<string | null>(null);

  const load = async () => {
    try {
      setCount(await getCounter());
      setError(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    }
  };

  useEffect(() => {
    load();
  }, []);

  const increment = async () => {
    try {
      setCount(await incrementCounter());
      setError(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    }
  };

  const reset = async () => {
    try {
      setCount(await resetCounter());
      setError(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    }
  };

  return (
    <div className="counter-panel">
      <div className="counter-value">{count}</div>
      <div className="counter-actions">
        <button onClick={increment}>+1</button>
        <button onClick={reset}>重置</button>
      </div>
      {error && <div className="counter-error">{error}</div>}
    </div>
  );
}
```

## 步骤 5：创建样式

`src/addons/counter/styles.css`

```css
.counter-panel {
  display: flex;
  flex-direction: column;
  gap: 12px;
  padding: 12px;
}

.counter-value {
  font-size: 40px;
  font-weight: 700;
  text-align: center;
}

.counter-actions {
  display: flex;
  gap: 8px;
  justify-content: center;
}

.counter-error {
  color: #fecaca;
  font-size: 12px;
}
```

## 步骤 6：创建入口文件

`src/addons/counter/index.tsx`

```tsx
import { registerPlugin } from "../registry";
import CounterPanel from "./CounterPanel";
import "./styles.css";

registerPlugin({
  id: "counter",
  title: "计数器",
  description: "一个最小的前后端插件示例",
  icon: "🔢",
  defaultWidth: 280,
  defaultHeight: 180,
  showCloseButton: true,
  showCollapseButton: true,
  showAttachButton: false,
  defaultAttachEnabled: false,
  defaultWhitelist: [],
  component: CounterPanel,
});
```

## 步骤 7：创建 Rust 后端

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

#[tauri::command]
pub fn reset_counter(state: tauri::State<'_, Arc<CounterState>>) -> Result<i32, String> {
    let mut count = state
        .count
        .lock()
        .map_err(|_| "Counter state lock poisoned".to_string())?;
    *count = 0;
    Ok(*count)
}
```

## 步骤 8：注册 command

在 `src-tauri/src/main.rs` 的 `tauri::generate_handler![...]` 中加入：

```rust
counter::get_counter,
counter::increment_counter,
counter::reset_counter,
```

不需要手动编辑 `src-tauri/src/_plugins.rs`，它会由 `build.rs` 自动生成。

## 步骤 9：构建运行

```powershell
npm run build
cargo check --manifest-path src-tauri/Cargo.toml
npm run tauri -- dev
```

运行后打开管理页，启用“计数器”插件。

## 纯前端插件

如果插件不需要 Rust 后端，可以省略：

- `src-tauri/src/<module>/mod.rs`
- `main.rs` command 注册
- 前端 `api.ts`

仍然必须保留：

- `manifest.json`
- `index.tsx`
- 面板组件

## 常见问题

### 插件不出现在管理页

检查：

- `manifest.json` 是否在 `src/addons/<plugin-id>/` 下。
- `manifest.json` 是否是合法 JSON。
- `index.tsx` 是否调用 `registerPlugin`。
- `npm run build` 是否触发了前端动态导入。

### 前端 invoke 报 command not found

检查：

- Rust 函数是否有 `#[tauri::command]`。
- 函数是否是 `pub`。
- 是否加入 `tauri::generate_handler![...]`。
- 前端命令名是否和 Rust 函数名一致。

### Rust 模块未初始化

检查：

- 模块目录是否是 `src-tauri/src/<module>/mod.rs`。
- 模块是否有 `pub fn setup(app: &tauri::AppHandle)`。
- `cargo check` 是否重新生成了 `_plugins.rs`。