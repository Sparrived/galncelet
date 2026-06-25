# 快速开始：从零创建一个插件

本教程将引导你创建一个完整的 Galncelet 插件——**番茄钟计时器**。通过这个实例，你将了解插件开发的完整流程。

## 最终效果

一个显示倒计时的挂件，支持：
- 开始/暂停/重置计时器
- 25 分钟倒计时
- 倒计时结束时通知

## 步骤总览

```
1. 创建 Rust 后端命令
2. 注册命令到 main.rs
3. 创建前端 types.ts
4. 创建前端 api.ts
5. 创建 manifest.json
6. 创建 Panel 组件
7. 创建 styles.css
8. 创建 index.tsx 入口
9. 构建运行
```

---

## 步骤 1：创建 Rust 后端命令

创建 `src-tauri/src/pomodoro.rs`：

```rust
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use tauri::{Emitter, State};

/// 计时器状态（由 Tauri 管理）
pub struct PomodoroState {
    pub remaining: Mutex<u32>,
    pub running: Mutex<bool>,
}

impl PomodoroState {
    pub fn new() -> Self {
        Self {
            remaining: Mutex::new(25 * 60),
            running: Mutex::new(false),
        }
    }
}

/// 返回给前端的状态
#[derive(Serialize, Clone)]
pub struct PomodoroStatus {
    pub remaining: u32,
    pub running: bool,
}

#[tauri::command]
pub fn get_pomodoro_status(state: State<PomodoroState>) -> PomodoroStatus {
    PomodoroStatus {
        remaining: *state.remaining.lock().unwrap(),
        running: *state.running.lock().unwrap(),
    }
}

#[tauri::command]
pub fn start_pomodoro(app: tauri::AppHandle, state: State<PomodoroState>) -> Result<(), String> {
    {
        let mut running = state.running.lock().unwrap();
        if *running {
            return Ok(());
        }
        *running = true;
    }

    let app_clone = app.clone();
    let state_ref = app.state::<PomodoroState>();

    tokio::spawn(async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;

            let mut remaining = state_ref.remaining.lock().unwrap();
            let running = state_ref.running.lock().unwrap();

            if !*running || *remaining == 0 {
                break;
            }

            *remaining -= 1;
            let r = *remaining;

            drop(remaining);
            drop(running);

            let _ = app_clone.emit("pomodoro-tick", PomodoroStatus {
                remaining: r,
                running: true,
            });

            if r == 0 {
                let _ = app_clone.emit("pomodoro-complete", ());
                break;
            }
        }

        *state_ref.running.lock().unwrap() = false;
    });

    Ok(())
}

#[tauri::command]
pub fn pause_pomodoro(state: State<PomodoroState>) {
    *state.running.lock().unwrap() = false;
}

#[tauri::command]
pub fn reset_pomodoro(state: State<PomodoroState>) {
    *state.remaining.lock().unwrap() = 25 * 60;
    *state.running.lock().unwrap() = false;
}
```

---

## 步骤 2：注册命令到 main.rs

编辑 `src-tauri/src/main.rs`：

```rust
// 1. 在文件顶部添加模块声明
mod pomodoro;

fn main() {
    tauri::Builder::default()
        // 2. 注册状态
        .manage(pomodoro::PomodoroState::new())
        // 3. 在 invoke_handler 中添加命令
        .invoke_handler(tauri::generate_handler![
            // ... 已有命令 ...
            pomodoro::get_pomodoro_status,
            pomodoro::start_pomodoro,
            pomodoro::pause_pomodoro,
            pomodoro::reset_pomodoro,
        ])
        // ...
}
```

---

## 步骤 3：创建前端类型

创建 `src/addons/pomodoro/types.ts`：

```ts
export interface PomodoroStatus {
  remaining: number;  // 剩余秒数
  running: boolean;   // 是否正在计时
}
```

---

## 步骤 4：创建前端 API

创建 `src/addons/pomodoro/api.ts`：

```ts
import { invoke } from "@tauri-apps/api/core";
import type { PomodoroStatus } from "./types";

export async function getPomodoroStatus(): Promise<PomodoroStatus> {
  return invoke<PomodoroStatus>("get_pomodoro_status");
}

export async function startPomodoro(): Promise<void> {
  return invoke<void>("start_pomodoro");
}

export async function pausePomodoro(): Promise<void> {
  return invoke<void>("pause_pomodoro");
}

export async function resetPomodoro(): Promise<void> {
  return invoke<void>("reset_pomodoro");
}
```

---

## 步骤 5：创建 manifest.json

创建 `src/addons/pomodoro/manifest.json`：

```json
{
  "id": "pomodoro",
  "title": "番茄钟",
  "description": "25 分钟番茄工作法计时器",
  "icon": "🍅",
  "defaultWidth": 240,
  "defaultHeight": 180,
  "showCloseButton": true,
  "showCollapseButton": true,
  "showAttachButton": false,
  "defaultAttachEnabled": false,
  "defaultWhitelist": []
}
```

这是一个独立工具，不需要附着到其他窗口，所以 `showAttachButton` 和 `defaultAttachEnabled` 都设为 `false`。

---

## 步骤 6：创建 Panel 组件

创建 `src/addons/pomodoro/PomodoroPanel.tsx`：

```tsx
import { useEffect, useState, useRef, useCallback } from "react";
import { listen } from "@tauri-apps/api/event";
import {
  getPomodoroStatus,
  startPomodoro,
  pausePomodoro,
  resetPomodoro,
} from "./api";
import type { PomodoroStatus } from "./types";
import { useAutoResize } from "../../widgets/useAutoResize";
import { useWidgetContextMenu } from "../../widgets/WidgetContext";

function formatTime(seconds: number): string {
  const m = Math.floor(seconds / 60);
  const s = seconds % 60;
  return `${String(m).padStart(2, "0")}:${String(s).padStart(2, "0")}`;
}

export default function PomodoroPanel() {
  const [status, setStatus] = useState<PomodoroStatus>({
    remaining: 25 * 60,
    running: false,
  });
  const containerRef = useRef<HTMLDivElement>(null);
  useAutoResize(containerRef);

  // 右键菜单
  useWidgetContextMenu([
    { label: "重置", icon: "🔄", onClick: handleReset },
  ]);

  // 初始化：获取当前状态
  useEffect(() => {
    getPomodoroStatus().then(setStatus).catch(() => {});
  }, []);

  // 监听实时事件
  useEffect(() => {
    const unlistenTick = listen<PomodoroStatus>("pomodoro-tick", (e) => {
      setStatus(e.payload);
    });
    const unlistenComplete = listen("pomodoro-complete", () => {
      setStatus({ remaining: 0, running: false });
    });

    return () => {
      unlistenTick.then((fn) => fn());
      unlistenComplete.then((fn) => fn());
    };
  }, []);

  async function handleReset() {
    await resetPomodoro();
    setStatus({ remaining: 25 * 60, running: false });
  }

  const handleToggle = async () => {
    if (status.running) {
      await pausePomodoro();
      setStatus((s) => ({ ...s, running: false }));
    } else {
      await startPomodoro();
      setStatus((s) => ({ ...s, running: true }));
    }
  };

  const progress = 1 - status.remaining / (25 * 60);
  const isDone = status.remaining === 0;

  return (
    <div className="pd-panel" ref={containerRef}>
      {/* 计时器显示 */}
      <div className={`pd-timer ${isDone ? "pd-timer--done" : ""}`}>
        {formatTime(status.remaining)}
      </div>

      {/* 进度条 */}
      <div className="pd-progress-track">
        <div
          className="pd-progress-bar"
          style={{ width: `${progress * 100}%` }}
        />
      </div>

      {/* 控制按钮 */}
      <div className="pd-controls">
        <button className="pd-btn" onClick={handleToggle}>
          {status.running ? "⏸ 暂停" : isDone ? "🔄 重新开始" : "▶ 开始"}
        </button>
        <button className="pd-btn pd-btn--secondary" onClick={handleReset}>
          ↺ 重置
        </button>
      </div>

      {/* 状态文字 */}
      <div className="pd-status">
        {isDone ? "🍅 时间到！休息一下吧" : status.running ? "专注中…" : "准备开始"}
      </div>
    </div>
  );
}
```

---

## 步骤 7：创建样式

创建 `src/addons/pomodoro/styles.css`：

```css
.pd-panel {
  padding: 16px;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 12px;
}

.pd-timer {
  font-size: 48px;
  font-weight: 700;
  font-variant-numeric: tabular-nums;
  color: var(--text-primary);
  letter-spacing: 2px;
}

.pd-timer--done {
  color: var(--mcha-green);
  animation: pd-pulse 1s ease-in-out infinite;
}

@keyframes pd-pulse {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.5; }
}

.pd-progress-track {
  width: 100%;
  height: 4px;
  background: var(--mcha-surface);
  border-radius: 2px;
  overflow: hidden;
}

.pd-progress-bar {
  height: 100%;
  background: var(--mcha-cyan);
  border-radius: 2px;
  transition: width 1s linear;
}

.pd-controls {
  display: flex;
  gap: 8px;
}

.pd-btn {
  padding: 6px 16px;
  border: 1px solid var(--glass-border);
  border-radius: 6px;
  background: var(--glass-highlight);
  color: var(--text-primary);
  font-size: 12px;
  cursor: pointer;
  transition: background 0.15s;
}

.pd-btn:hover {
  background: rgba(255, 255, 255, 0.08);
}

.pd-btn--secondary {
  color: var(--text-secondary);
}

.pd-status {
  font-size: 12px;
  color: var(--text-muted);
}
```

---

## 步骤 8：创建入口文件

创建 `src/addons/pomodoro/index.tsx`：

```tsx
import { registerPlugin } from "../registry";
import PomodoroPanel from "./PomodoroPanel";
import manifest from "./manifest.json";
import "./styles.css";

registerPlugin({
  ...manifest,
  component: PomodoroPanel,
});
```

---

## 步骤 9：构建运行

```bash
# 安装依赖（如果还没安装）
npm install

# 开发模式运行
npm run tauri dev
```

启动后：
1. 打开管理窗口（系统托盘或快捷键）
2. 在插件列表中找到「🍅 番茄钟」
3. 点击启用，番茄钟挂件就会出现

---

## 完整文件清单

```
src/addons/pomodoro/
├── index.tsx              ← 入口：注册插件
├── manifest.json          ← 元数据配置
├── types.ts               ← TypeScript 类型
├── api.ts                 ← Tauri IPC 封装
├── PomodoroPanel.tsx      ← 主组件
└── styles.css             ← 样式

src-tauri/src/
├── main.rs                ← 添加 mod + 注册命令
└── pomodoro.rs            ← Rust 后端逻辑
```

---

## 下一步

- [manifest.json 字段参考](manifest.md) — 了解更多配置选项
- [可用 Hooks](hooks.md) — 深入了解 useAutoResize、useWidget 等
- [Rust 后端集成](backend.md) — 更多后端开发细节
- [常见模式](patterns.md) — 轮询、事件、子组件组织等
