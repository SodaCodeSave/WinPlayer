# 第三方组件与协议声明

WinPlayer 本体源代码遵循 **MIT License**（见 [LICENSE](LICENSE)），但集成了若干遵循其他协议的第三方组件。分发本软件时，请保留以下声明。

## WinUIonWeb

- **项目**：[WinUIonWeb](https://github.com/Furry-Xiyi/WinUIonWeb)
- **作者**：[Furry-Xiyi](https://github.com/Furry-Xiyi)
- **npm 包**：[`winuionweb`](https://www.npmjs.com/package/winuionweb)
- **协议**：GPL-3.0-only
- **用途**：本软件的图形界面使用了该组件库提供的 WinUI 风格 Web 组件（按钮、开关、下拉等）。该组件库是对 Microsoft WinUI 的独立 Web 实现，与 Microsoft 无关联，亦未获 Microsoft 认可或授权。

```
WinUIonWeb — WinUI-style Vue 3 components for the web
Copyright (c) Furry-Xiyi
Licensed under the GNU General Public License v3.0 only (GPL-3.0-only)
https://github.com/Furry-Xiyi/WinUIonWeb
```

> **协议提示**：GPL-3.0 为强 copyleft 协议。虽然 WinPlayer 自身以 MIT 协议发布，但由于本软件在运行时分发并包含了 GPL-3.0 的组件，**整个软件的分发物在 GPL-3.0 的约束之下**。如需再分发（含二进制分发），请确保遵守 GPL-3.0 的相应义务（提供源代码获取途径、保留协议声明等）。本声明即为此目的提供来源与协议标注。

## Tauri

- **项目**：[Tauri](https://github.com/tauri-apps/tauri)
- **协议**：MIT 或 Apache-2.0（双许可）
- **用途**：桌面应用框架（Rust 后端 + WebView 前端）

## Vue.js

- **项目**：[Vue](https://github.com/vuejs/core)
- **作者**：Evan You
- **协议**：MIT
- **用途**：前端 UI 框架

## 其他依赖

前端与 Rust 依赖的完整列表参见 `package.json` 与 `src-tauri/Cargo.lock`，各依赖协议以其上游仓库声明为准。
