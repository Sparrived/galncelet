# Galncelet

简单易用、AI 友好的 Windows 桌面挂件框架。

## 功能

- **桌面挂件**：无边框透明窗口，毛玻璃视觉，贴边悬浮，可独立管理
- **系统托盘**：通过托盘图标控制每个挂件的显隐
- **自动窗口吸附**：检测当前前台窗口，挂件自动贴到其右侧
- **Git 状态面板**：文件变更树、Diff 查看器、暂存/提交/推送/拉取
- **AMKR 仪表盘**：实时显示 AMKR 路由器的请求量、延迟、Token 等指标
- **可扩展架构**：基于面板注册机制，轻松添加新的桌面挂件

## 技术栈

- [Tauri 2](https://v2.tauri.app/) - 轻量桌面应用框架
- Rust - 后端：Git 操作、Win32 窗口管理、HTTP 请求
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

1. 在 `src/panels/` 创建面板组件
2. 在 `src/panels/registry.ts` 注册
3. 在 `src-tauri/src/main.rs` 添加窗口标签

## 项目结构

```
├── src/
│   ├── main.tsx               # 入口
│   ├── App.tsx                # 路由器（按窗口标签分发）
│   ├── styles.css             # 毛玻璃样式系统
│   ├── panels/
│   │   ├── types.ts           # PanelDef 接口
│   │   ├── context.tsx        # WidgetContext 共享上下文
│   │   ├── registry.ts        # 面板注册表
│   │   ├── GitPanel.tsx       # Git 状态面板
│   │   └── AmkrPanel.tsx      # AMKR 仪表盘面板
│   ├── widgets/
│   │   ├── WidgetShell.tsx    # 通用挂件壳（标题栏 + 收起/关闭）
│   │   ├── GitWidget.tsx      # Git 挂件
│   │   └── AmkrWidget.tsx     # AMKR 挂件
│   ├── components/
│   │   ├── GitTree.tsx        # Git 文件树
│   │   ├── DiffViewer.tsx     # Diff 查看器
│   │   ├── Dashboard.tsx      # AMKR 指标仪表盘
│   │   └── Settings.tsx       # 设置面板
│   └── lib/
│       ├── types.ts           # TypeScript 类型
│       └── api.ts             # Tauri IPC 封装
├── src-tauri/
│   ├── src/
│   │   ├── main.rs            # 多窗口创建 + 系统托盘
│   │   ├── git.rs             # Git 命令封装
│   │   ├── window_attach.rs   # 多窗口吸附
│   │   ├── settings.rs        # 持久化设置
│   │   ├── amkr.rs            # AMKR API 集成
│   │   ├── tray.rs            # 系统托盘
│   │   └── acrylic.rs         # DWM 毛玻璃
│   ├── Cargo.toml
│   ├── tauri.conf.json
│   └── icons/
└── package.json
```
