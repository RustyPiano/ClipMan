# ClipMan

<div align="center">

<img src="app-icon.png" alt="ClipMan" width="128" />

**Lightweight Modern Clipboard Manager**

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Version](https://img.shields.io/badge/Version-2.0.1-blue.svg)](https://github.com/RustyPiano/ClipMan/releases)
[![Platform](https://img.shields.io/badge/Platform-Windows%20%7C%20macOS%20%7C%20Linux-lightgrey.svg)](https://github.com/RustyPiano/ClipMan)

[Download](#-installation) · [Features](#-features) · [Development](#-development)

[中文文档](README.md)

</div>

## Introduction

ClipMan is a **lightweight**, **high-performance** modern clipboard manager built with Rust + Svelte 5, focusing on essential and practical features.

**Why ClipMan?**

- ✅ **Persistent Storage** - History remains available after system restart
- ✅ **Pin Feature** - Pin frequently used content with one click, access anytime
- ✅ **Lightweight & Fast** - Small installers and fast startup
- ✅ **Modern UI** - Clean interface with multiple themes
- ✅ **Open Source** - MIT license, fully open source

## ✨ Features

### Core Features

- **📌 Pin Content** - Pin code snippets, commands, links, etc. permanently
- **💾 Persistent Storage** - SQLite database, survives restarts, smart deduplication
- **🔍 Full-Text Search** - SQLite FTS5 + trigram index with real-time Chinese/English search
- **⌨️ QuickBar Access** - Open from any app (default `Cmd/Ctrl+Shift+V`), select by keyboard, then auto-paste or copy based on settings
- **🎯 Tray Menu** - Quick access to recent and pinned items
- **🎨 Multiple Themes** - Light/Dark/Pink themes, follow system
- **🌐 Multi-Language** - English and Chinese support, auto-detect system language

### Additional Features

- 🛡️ Skip password-like clipboard contents (optional)
- 🔄 Auto-update
- 🚀 Launch at startup
- 📁 Custom storage location
- 🧹 Data management

## 📥 Installation

Visit [GitHub Releases](https://github.com/RustyPiano/ClipMan/releases/latest) to download:

- **Windows**: `ClipMan_2.0.1_x64_en-US.msi`
- **macOS (Apple Silicon)**: `ClipMan_2.0.1_aarch64.dmg`
- **macOS (Intel)**: `ClipMan_2.0.1_x64.dmg`
- **Linux**: `ClipMan_2.0.1_amd64.AppImage`

### macOS Permissions

Grant Accessibility permission on first run:

**System Settings** → **Privacy & Security** → **Accessibility** → Add ClipMan

## 🚀 Usage

1. Use hotkey `Cmd+Shift+V` (macOS) / `Ctrl+Shift+V` (Windows) to open
2. All copied content is automatically saved in the history list
3. Click 📌 icon to pin frequently used content
4. Use search bar to quickly find history records
5. Click an item to run the QuickBar default action: auto-paste by default, or copy only when auto-paste is disabled
6. Click tray icon for quick access (tray menu actions always copy only)

## 🛠️ Technology

**Backend**
- Tauri 2.11 - Lightweight desktop framework
- SQLite + FTS5 - High-performance database
- Rust - Memory-safe, high performance

**Frontend**
- Svelte 5 - Modern reactive framework
- Tailwind CSS 4 - Modern styling solution
- Vite 8 - Fast build tool

**Performance Positioning**
- ⚡ Lightweight popup: QuickBar uses a small hidden window and opens on demand
- 💾 Local-first storage: SQLite persistence with no background network service
- 🔍 Indexed search: FTS5 + trigram indexing reduces search latency
- 📦 Small distribution: exact installer size depends on the current Release artifacts

## 🔧 Development

### Requirements

- Bun 1.3+ or Node.js 20.19+
- Rust 1.96.0 (pinned by `rust-toolchain.toml`)
- System: Windows 10+ / macOS 10.13+ / Linux

### Quick Start

```bash
# Clone repository
git clone https://github.com/RustyPiano/ClipMan.git
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
│   │   └── i18n/         # Internationalization
│   └── routes/           # Page routes
├── src-tauri/        # Rust backend
│   └── src/
│       ├── main.rs       # Entry point, tray
│       ├── clipboard.rs  # Clipboard monitoring
│       ├── storage.rs    # Database
│       ├── paste.rs      # Copy / auto-paste
│       ├── window.rs     # QuickBar / settings windows
│       └── settings.rs   # Settings management
└── package.json
```

## 🗺️ Roadmap

**Completed**
- [x] Clipboard monitoring and history
- [x] Pin feature
- [x] Full-text search
- [x] QuickBar auto-paste
- [x] Global hotkey
- [x] System tray
- [x] Auto-update
- [x] Multiple themes
- [x] Skip password-like clipboard contents
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

Contributions are welcome through issues and pull requests.

- 🐛 [Report Bug](https://github.com/RustyPiano/ClipMan/issues)
- ✨ [Feature Request](https://github.com/RustyPiano/ClipMan/issues)
- 💬 [Discussions](https://github.com/RustyPiano/ClipMan/discussions)

## 📄 License

[MIT License](LICENSE)

## 🙏 Acknowledgments

- [Tauri](https://tauri.app/) - Desktop application framework
- [Svelte](https://svelte.dev/) - Frontend framework
- [rusqlite](https://github.com/rusqlite/rusqlite) - SQLite bindings
- [arboard](https://github.com/1Password/arboard) - Clipboard library

---

<div align="center">

**If you find it helpful, give it a ⭐️ Star!**

Made with ❤️ by [RustyPiano](https://github.com/RustyPiano)

</div>
