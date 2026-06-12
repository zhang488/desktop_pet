# Desktop Pet · 桌面宠物提醒助手

一款桌面端「桌宠式」定时提醒应用。角色平时隐藏，到点（喝水 / 久坐起身 / 吃饭 / 睡觉等）自动跳出到桌面，用对话气泡进行拟人化提醒，用户响应后隐藏退出。

> 当前进度：**阶段 1（工程骨架）已完成**，业务逻辑尚未开发。详见 [`docs/FEATURES.md`](docs/FEATURES.md) 功能看板。

---

## 技术栈

| 层 | 选型 | 说明 |
|----|------|------|
| 应用框架 | **Tauri 2** | 跨平台、安装包小（10–15MB）、内存占用低（50–100MB） |
| 后端 | **Rust** | 任务调度、系统集成、托盘、自启、数据持久化 |
| 前端 | **React 19 + TypeScript + Vite** | 角色窗口渲染、设置面板 |
| 角色渲染 | **Live2D Cubism + PixiJS**（规划中） | 当前为 CSS 占位角色 |
| 数据库 | **SQLite**（规划中） | 提醒历史、配置、统计 |

**目标平台**：Windows + macOS

---

## 环境要求

| 工具 | 版本 | 用途 |
|------|------|------|
| Node.js | ≥ 20 | 前端构建 |
| npm | ≥ 10 | 包管理 |
| Rust | ≥ 1.77（开发用 1.96） | 后端编译 |
| Cargo | 随 Rust 安装 | Rust 构建工具 |

> Windows 还需安装 **WebView2 运行时**（Win11 一般已自带）和 **Microsoft C++ Build Tools**；macOS 需要 **Xcode Command Line Tools**。

---

## 快速开始

```powershell
# 1. 安装前端依赖（首次克隆后）
npm install

# 2. 启动开发模式（首次会编译 Rust，耗时较长；之后走缓存很快）
npm run tauri:dev
```

启动后预期：桌面出现一个**透明背景、无边框、始终置顶**的小窗口，内含占位角色（蓝色椭圆 + 呼吸动画）、对话气泡和两个按钮；**任务栏不显示图标**。

### 常用命令

| 命令 | 作用 |
|------|------|
| `npm run dev` | 仅启动 Vite 前端（浏览器里调试 UI，无桌宠窗口能力） |
| `npm run tauri:dev` | 启动完整桌面应用（开发模式，带热重载） |
| `npm run tauri:build` | 打包生产安装包（输出在 `src-tauri/target/release/bundle/`） |
| `npm run build` | 仅构建前端静态资源到 `dist/` |
| `npm run lint` | ESLint 检查 |

---

## 项目结构

```
desktop_pet/
├── docs/
│   └── FEATURES.md          # 功能清单与开发进度看板
├── src/                     # React 前端
│   ├── App.tsx              # 桌宠主 UI（当前为占位实现）
│   ├── App.css              # 角色样式与动画
│   ├── index.css            # 全局透明背景
│   └── main.tsx             # 前端入口
├── src-tauri/               # Rust 后端
│   ├── src/
│   │   ├── lib.rs           # 应用入口（Tauri Builder、插件注册）
│   │   └── main.rs          # 二进制入口
│   ├── capabilities/        # Tauri 权限能力声明
│   ├── icons/               # 应用图标（当前为默认图标）
│   ├── tauri.conf.json      # Tauri 配置（窗口/打包/安全策略）
│   └── Cargo.toml           # Rust 依赖
├── vite.config.ts           # Vite 配置（已适配 Tauri）
├── package.json
└── README.md
```

---

## 关键配置说明

桌宠窗口的特殊行为在 `src-tauri/tauri.conf.json` 的 `app.windows[0]` 中定义：

| 配置项 | 值 | 作用 |
|--------|-----|------|
| `transparent` | `true` | 窗口透明背景（露出角色，无白底） |
| `decorations` | `false` | 无标题栏边框 |
| `alwaysOnTop` | `true` | 始终置顶 |
| `skipTaskbar` | `true` | 不在任务栏显示图标 |
| `resizable` | `false` | 固定尺寸 |
| `shadow` | `false` | 无窗口阴影 |
| `visible` | `true` | 当前为可见（便于验证；后续业务上线后改为启动隐藏，到点再 show） |
| 尺寸 | 360 × 480 | 角色窗口大小 |

> 前端透明依赖 `src/index.css` 中 `background: transparent`，修改主题时勿覆盖。

---

## 开发路线图

- **阶段 1 — 工程骨架** ✅ 已完成
  脚手架、透明置顶窗口、占位角色 UI。
- **阶段 2 — 核心链路** 🚧 规划中
  系统托盘 → 后端定时调度器 + IPC 事件 → 点击穿透 → 打通单个「喝水」提醒完整流程 → Live2D 接入。
- **阶段 3 — 体验完善**
  多提醒类型 + 优先级队列、防打扰检测、设置面板、SQLite 持久化与统计。
- **阶段 4 — 可选增强**
  TTS 语音、多角色切换、AI 动态台词。

完整功能清单与逐项状态见 [`docs/FEATURES.md`](docs/FEATURES.md)。

---

## 常见问题

**Q：`npm run tauri:dev` 第一次很慢？**
A：首次需要编译整个 Rust 依赖树，属于正常现象；之后增量编译会快很多。

**Q：窗口背景不透明 / 是白色的？**
A：检查 `src/index.css` 的透明背景是否被覆盖，以及 `tauri.conf.json` 中 `transparent: true` 是否生效（Windows 需 WebView2 支持）。

**Q：只想调前端 UI，不想每次编译 Rust？**
A：直接 `npm run dev`，在浏览器 `http://localhost:1420` 调试（但托盘、窗口穿透等 Tauri 原生能力不可用）。
