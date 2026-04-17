# VolumeOSD

一个基于 **Tauri 2.0 + React + TypeScript** 开发的高性能 Windows 系统音量控制工具。它提供了一个现代化的、半透明的悬浮调节条（OSD），并支持全局快捷键和开机自启。

## ✨ 功能特性

- **现代 UI**: 采用无边框、半透明、圆角设计的 OSD 调节条。
- **全局快捷键**: 无论你在哪个程序中，都可以通过以下组合键控制音量：
  - `Ctrl + Alt + ↑`: 音量增加 5%
  - `Ctrl + Alt + ↓`: 音量减少 5%
  - `Ctrl + Alt + ←`: 切换静音/取消静音
- **长按支持**: 支持长按快捷键进行连续音量调节，并针对 CPU 占用进行了优化。
- **系统托盘**: 支持托盘图标及右键菜单操作：
  - 开机启动切换（实时同步系统注册表）
  - 静音切换
  - 退出程序
- **开机自启**: 内置自启动管理逻辑，无需手动添加启动项。
- **高性能**: 采用 Rust 直接调用 Windows Core Audio API (WASAPI)，响应极其迅速，资源占用极低。

## 🛠️ 技术栈

- **后端**: Rust (Tauri 2.0)
- **前端**: React 19 + TypeScript + Vite
- **系统交互**: `windows` crate (直接调用 Win32 API)
- **插件**: 
  - `tauri-plugin-global-shortcut`: 处理全局热键
  - `tauri-plugin-autostart`: 管理开机启动

## 🚀 开发与构建

### 准备工作
确保你的系统中已安装：
- [Node.js](https://nodejs.org/) (建议 v18+)
- [Rust](https://www.rust-lang.org/tools/install) (包括 Cargo)
- [WebView2 Runtime](https://developer.microsoft.com/en-us/microsoft-edge/webview2/) (Windows 10/11 通常自带)

### 安装依赖
```bash
npm install
```

### 运行开发服务器
```bash
npm run tauri dev
```

### 构建安装包
```bash
npm run tauri build
```
构建完成后，安装包将生成在 `src-tauri/target/release/bundle/` 目录下。

## 📝 许可证

MIT License
