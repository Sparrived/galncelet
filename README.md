# Galncelet

简单易用、AI 友好的 Windows 桌面挂件框架。

## 功能

- **桌面挂件**：无边框透明窗口，毛玻璃视觉，贴边悬浮，可独立管理
- **系统托盘**：通过托盘图标控制每个挂件的显隐
- **自动窗口吸附**：检测当前前台窗口，挂件自动贴到其右侧
- **Git 状态面板**：文件变更树、Diff 查看器、暂存/提交/推送/拉取
- **AMKR 仪表盘**：实时显示 AMKR 路由器的请求量、延迟、Token 等指标
- **音乐播放器**：SMTC 媒体检测、歌词同步、多播放器切换
- **系统监控**：CPU/GPU/内存/磁盘/网络实时监控
- **剪贴板历史**：自动记录剪贴板内容，支持搜索和回填
- **页面笔记**：基于浏览器 URL 显示关联笔记
- **可扩展架构**：基于插件注册机制，轻松添加新的桌面挂件

## 技术栈

- [Tauri 2](https://v2.tauri.app/) - 轻量桌面应用框架
- Rust - 后端：Git 操作、Win32 窗口管理、HTTP 请求、SMTC 媒体控制
- React + TypeScript - 前端 UI
- Vite - 构建工具

## 前置条件

- **Node.js** >= 18（推荐 20+）
- **Rust** >= 1.81（[rustup 安装](https://rustup.rs/)）
- **Git**（需要在 PATH 中）
- **Visual Studio Build Tools 2022**（含 C++ 桌面开发工作负载和 Windows SDK）
- **Windows 11**（窗口吸附和毛玻璃效果需要；其他平台可运行但部分功能受限）

## 安装

```bash
npm install
```

## 运行

```bash
# 开发模式（带热重载）
npm run tauri dev

# 构建生产版本
npm run tauri build
```

## 添加新挂件

详见 [插件开发指南](docs/plugin-dev/README.md)。

简要流程：
1. 在 `src/addons/my-plugin/` 创建插件目录（含 `manifest.json`、`index.tsx`、组件、样式）
2. 在 `src-tauri/src/my_plugin/` 创建 Rust 模块（含 `mod.rs`，`pub fn setup()` 和 `#[tauri::command]` 函数）
3. 在 `src-tauri/src/main.rs` 的 `generate_handler![]` 中注册命令
4. `npm run tauri dev` 运行

Rust 侧的 `mod` 声明和 `setup()` 调用由 `build.rs` 自动生成。

## 项目结构

```
├── src/
│   ├── main.tsx               # 入口
│   ├── App.tsx                # 路由器（按窗口标签分发）
│   ├── styles.css             # 毛玻璃样式系统
│   ├── addons/                # 插件目录（每个子目录为一个插件）
│   │   ├── system-monitor/    # 系统监控
│   │   ├── clipboard-history/ # 剪贴板历史
│   │   ├── page-notes/        # 页面笔记
│   │   ├── amkr/              # AMKR 仪表盘
│   │   ├── git/               # Git 状态面板
│   │   └── music-player/      # 音乐播放器
│   ├── widgets/
│   │   ├── WidgetShell.tsx    # 通用挂件壳（标题栏 + 收起/关闭/附着）
│   │   ├── WidgetContext.ts   # 挂件上下文（右键菜单、折叠状态）
│   │   └── useAutoResize.ts   # 自动调整窗口尺寸 hook
│   ├── components/            # 共享组件（RadialGauge、ProgressBar 等）
│   └── lib/
│       ├── context.tsx        # WidgetContext（showResult/showError）
│       ├── api.ts             # Tauri IPC 封装（设置、窗口管理）
│       ├── format.ts          # 格式化工具函数
│       └── types.ts           # 共享 TypeScript 类型
├── src-tauri/
│   ├── src/
│   │   ├── main.rs            # 入口：框架命令、插件注册
│   │   ├── _plugins.rs        # 自动生成：插件 mod 声明 + setup 调用
│   │   ├── acrylic.rs         # DWM 毛玻璃
│   │   ├── plugins.rs         # 插件清单加载（编译时嵌入 manifest.json）
│   │   ├── settings.rs        # 持久化设置
│   │   ├── tray.rs            # 系统托盘
│   │   ├── window_attach.rs   # 窗口吸附
│   │   ├── amkr/              # AMKR 插件后端
│   │   ├── browser_ext/       # 浏览器扩展插件后端
│   │   ├── clipboard_history/ # 剪贴板历史插件后端
│   │   ├── git/               # Git 插件后端
│   │   ├── music_player/      # 音乐播放器插件后端（SMTC + 歌词）
│   │   ├── page_notes/        # 页面笔记插件后端
│   │   └── system_monitor/    # 系统监控插件后端
│   ├── build.rs               # 构建脚本（自动生成 _plugins.rs + 嵌入 manifest）
│   ├── Cargo.toml
│   └── tauri.conf.json
├── docs/
│   ├── ai-reference/          # AI 参考文档
│   └── plugin-dev/            # 插件开发指南
└── package.json
```

## 现有插件

| 插件 | 说明 | 复杂度 |
|------|------|--------|
| system-monitor | CPU/GPU/内存/磁盘/网络监控 | ⭐ |
| clipboard-history | 剪贴板历史记录与搜索 | ⭐ |
| page-notes | 基于 URL 的页面笔记 | ⭐⭐ |
| amkr | LLM API 路由器实时仪表盘 | ⭐⭐ |
| git | 完整 Git 仓库管理 | ⭐⭐⭐ |
| music-player | SMTC 媒体播放器 + 歌词 | ⭐⭐ |

## Runtime addons

Galncelet supports hot-pluggable runtime addons from `%APPDATA%\Galncelet\addons`. Users can copy or delete addon folders without rebuilding the main app; Galncelet watches the folder and refreshes the management page automatically. Runtime addons can be frontend-only or can ship their own sidecar backend that communicates with Galncelet through JSON-RPC.

See `docs/runtime-addons.md` for the package layout, manifest schema, frontend API, and sidecar protocol.
