# ClipMan

<div align="center">

<img src="app-icon.png" alt="ClipMan" width="128" />

**Lightweight Modern Clipboard Manager**

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Version](https://img.shields.io/badge/Version-1.9.0-blue.svg)](https://github.com/Kiaana/ClipMan/releases)
[![Platform](https://img.shields.io/badge/Platform-Windows%20%7C%20macOS%20%7C%20Linux-lightgrey.svg)](https://github.com/Kiaana/ClipMan)

[Download](#-installation) · [Features](#-features) · [Development](#-development)

[中文文档](README.md)

</div>

## Introduction

ClipMan is a **lightweight** (< 5MB), **high-performance** (< 50MB memory) modern clipboard manager built with Rust + Svelte 5, focusing on essential and practical features.

**Why ClipMan?**

- ✅ **Persistent Storage** - History remains available after system restart
- ✅ **Pin Feature** - Pin frequently used content with one click, access anytime
- ✅ **Lightweight & Fast** - < 5MB installer, < 1s startup
- ✅ **Modern UI** - Clean interface with multiple themes
- ✅ **Open Source** - MIT license, fully open source

## ✨ Features

### Core Features

- **📌 Pin Content** - Pin code snippets, commands, links, etc. permanently
- **💾 Persistent Storage** - SQLite database, survives restarts, smart deduplication
- **🔍 Full-Text Search** - FTS5 engine with real-time Chinese/English search
- **⌨️ Global Hotkey** - Smart input system, quick access from any app (default `Cmd/Ctrl+Shift+V`)
- **🎯 Tray Menu** - Quick access to recent and pinned items
- **🎨 Multiple Themes** - Light/Dark/Pink themes, follow system

### Additional Features

- 🔒 AES-256-GCM encryption (optional)
- 🔄 Auto-update
- 🚀 Launch at startup
- 📁 Custom storage location
- 🧹 Data management

## 📥 Installation

Visit [GitHub Releases](https://github.com/Kiaana/ClipMan/releases/latest) to download:

- **Windows**: `ClipMan_1.9.0_x64_en-US.msi`
- **macOS (Apple Silicon)**: `ClipMan_1.9.0_aarch64.dmg`
- **macOS (Intel)**: `ClipMan_1.9.0_x64.dmg`
- **Linux**: `ClipMan_1.9.0_amd64.AppImage`

### macOS Permissions

Grant Accessibility permission on first run:

**System Settings** → **Privacy & Security** → **Accessibility** → Add ClipMan

## 🚀 Usage

1. Use hotkey `Cmd+Shift+V` (macOS) / `Ctrl+Shift+V` (Windows) to open
2. All copied content is automatically saved in the history list
3. Click 📌 icon to pin frequently used content
4. Use search bar to quickly find history records
5. Click an item to copy it to clipboard
6. Click tray icon for quick access

## 🛠️ Technology

**Backend**
- Tauri 2.0 - Lightweight desktop framework
- SQLite + FTS5 - High-performance database
- Rust - Memory-safe, high performance

**Frontend**
- Svelte 5 - Modern reactive framework
- Tailwind CSS 4 - Modern styling solution
- Vite 6 - Fast build tool

**Performance**
- ⚡ Startup: < 1s
- 💾 Memory: 30-50MB
- 📦 Installer: < 5MB
- 🔋 CPU: 0% (idle)

## 🔧 Development

### Requirements

- Bun or Node.js 18+
- Rust 1.82+
- System: Windows 10+ / macOS 10.13+ / Linux

### Quick Start

```bash
# Clone repository
git clone https://github.com/Kiaana/ClipMan.git
cd ClipMan

# Install dependencies
bun install

# Development
bun tauri dev

# Build
bun tauri build
```

### Project Structure

```
ClipMan/
├── src/              # Svelte frontend
│   ├── lib/
│   │   ├── components/   # UI components
│   │   ├── stores/       # State management
│   │   └── utils/        # Utilities
│   └── routes/           # Page routes
├── src-tauri/        # Rust backend
│   └── src/
│       ├── main.rs       # Entry point, tray
│       ├── clipboard.rs  # Clipboard monitoring
│       ├── storage.rs    # Database
│       ├── crypto.rs     # Encryption
│       └── settings.rs   # Settings management
└── package.json
```

## 🗺️ Roadmap

**Completed**
- [x] Clipboard monitoring and history
- [x] Pin feature
- [x] Full-text search
- [x] Global hotkey
- [x] System tray
- [x] Auto-update
- [x] Multiple themes
- [x] AES-256 encryption
- [x] Custom storage location
- [x] Launch at startup

**Planned**
- [ ] Smart image compression (AVIF/WebP/MozJPEG, configurable)
- [ ] Multi-device sync
- [ ] Group management
- [ ] Filter rules
- [ ] Plugin system
- [ ] CLI tool

## 🤝 Contributing

Contributions welcome! See [Contributing Guide](CONTRIBUTING.md).

- 🐛 [Report Bug](https://github.com/Kiaana/ClipMan/issues)
- ✨ [Feature Request](https://github.com/Kiaana/ClipMan/issues)
- 💬 [Discussions](https://github.com/Kiaana/ClipMan/discussions)

## 📄 License

[MIT License](LICENSE)

## 🙏 Acknowledgments

- [Tauri](https://tauri.app/) - Desktop application framework
- [Svelte](https://svelte.dev/) - Frontend framework
- [rusqlite](https://github.com/rusqlite/rusqlite) - SQLite bindings
- [arboard](https://github.com/1Password/arboard) - Clipboard library
- [ring](https://github.com/briansmith/ring) - Cryptography library

---

<div align="center">

**If you find it helpful, give it a ⭐️ Star!**

Made with ❤️ by [Kiaana](https://github.com/Kiaana)

</div>
