# ClipMan - 现代化剪切板管理器

> Windows 优先的跨平台剪切板管理器，使用 Rust + Tauri 2.0 + Svelte 5 构建

## ✨ 特性

- 🔄 **自动剪切板监控** - 实时捕获文本和图像
- 🔍 **强大搜索功能** - 基于 SQLite FTS5 全文搜索
- 📌 **置顶常用内容** - 快速访问常用剪切板项
- 🎨 **现代化 UI** - 简洁易用的 Fluent Design
- 🔐 **隐私保护** - 本地存储，端到端加密
- ⚡ **高性能** - Rust 后端，极速响应
- 🪶 **轻量级** - 安装包 < 10MB

## 🛠️ 技术栈

### 后端
- **Rust 1.82+** - 安全、高性能
- **Tauri 2.0** - 现代桌面应用框架
- **SQLite** - 本地数据库，支持 FTS5 全文搜索
- **arboard** - 跨平台剪切板操作
- **ring** - AES-256 加密

### 前端
- **Svelte 5** - 响应式框架，使用最新 Runes API
- **TypeScript** - 类型安全
- **Vite 6** - 快速构建工具

## 🚀 快速开始

### 前置要求

- Node.js 18+ 或 Bun
- Rust 1.82+
- (Windows) Visual Studio Build Tools

### 安装依赖

```bash
# 安装 npm 依赖
npm install

# 或使用 bun
bun install
```

### 开发模式

```bash
# 启动开发服务器
npm run tauri dev

# 或使用 bun
bun tauri dev
```

### 构建应用

```bash
# 构建生产版本
npm run tauri build

# 或使用 bun
bun tauri build
```

## 📁 项目结构

```
ClipMan/
├── src-tauri/           # Rust 后端
│   ├── src/
│   │   ├── main.rs      # 入口点，Tauri 设置，系统托盘
│   │   ├── clipboard.rs # 剪切板监控核心
│   │   ├── storage.rs   # SQLite 数据库封装
│   │   └── crypto.rs    # AES-256 加密模块
│   ├── Cargo.toml       # Rust 依赖
│   └── tauri.conf.json  # Tauri 配置
├── src/                 # Svelte 前端
│   ├── lib/
│   │   ├── components/  # Svelte 组件
│   │   │   ├── SearchBar.svelte
│   │   │   └── ClipboardItem.svelte
│   │   └── stores/
│   │       └── clipboard.svelte.ts  # 状态管理（Svelte 5 Runes）
│   ├── routes/
│   │   └── +page.svelte # 主页面
│   ├── app.css          # 全局样式
│   └── main.ts          # 前端入口
├── package.json         # npm 依赖
├── vite.config.js       # Vite 配置
├── svelte.config.js     # Svelte 配置
└── CLAUDE.md            # 产品需求文档
```

## 🔑 核心功能实现

### 1. 剪切板监控

使用 `arboard` crate 每 500ms 轮询剪切板变化：

```rust
// src-tauri/src/clipboard.rs
pub struct ClipboardMonitor {
    app_handle: AppHandle,
}

impl ClipboardMonitor {
    pub fn start(&self) {
        // 监控文本和图像变化
        // 自动保存到 SQLite
        // 触发前端事件更新 UI
    }
}
```

### 2. 全文搜索

使用 SQLite FTS5 虚拟表实现高性能搜索：

```sql
CREATE VIRTUAL TABLE clips_fts
USING fts5(content, content='clips');
```

### 3. Svelte 5 Runes 状态管理

```typescript
// src/lib/stores/clipboard.svelte.ts
class ClipboardStore {
  items = $state<ClipItem[]>([]);
  searchQuery = $state('');

  // Derived state
  pinnedItems = $derived(
    this.items.filter(item => item.isPinned)
  );

  filteredItems = $derived(() => {
    // 实时过滤逻辑
  });
}
```

### 4. 系统托盘集成

使用 Tauri 2.0 新 API：

```rust
// 创建托盘菜单
let menu = MenuBuilder::new(app)
    .items(&[&show_item, &pinned_item, &quit_item])
    .build()?;

// 处理点击事件
TrayIconBuilder::new()
    .menu(&menu)
    .on_menu_event(|app, event| { /* ... */ })
    .on_tray_icon_event(|tray, event| { /* ... */ })
    .build(app)?;
```

## 🎯 开发路线图

### MVP (v1.0) - 已完成脚手架
- [x] 剪切板监控（文本/图像）
- [x] 历史记录存储
- [x] 搜索功能
- [x] 置顶功能
- [x] 系统托盘集成
- [x] Svelte 5 Runes UI

### v1.1 - 计划中
- [ ] 热键支持（Win+V, Ctrl+Shift+V）
- [ ] 图像缩略图优化
- [ ] 数据导出（JSON/CSV）
- [ ] 敏感内容过滤

### v1.2 - 未来
- [ ] 文件路径支持
- [ ] 加密存储选项
- [ ] 自定义主题
- [ ] Linux/macOS 支持

## 🤝 贡献指南

欢迎贡献！请遵循以下步骤：

1. Fork 项目
2. 创建功能分支 (`git checkout -b feature/AmazingFeature`)
3. 提交更改 (`git commit -m 'Add some AmazingFeature'`)
4. 推送到分支 (`git push origin feature/AmazingFeature`)
5. 开启 Pull Request

## 📄 许可证

MIT License - 详见 [LICENSE](LICENSE) 文件

## 🙏 致谢

- [Tauri](https://tauri.app/) - 现代桌面应用框架
- [Svelte](https://svelte.dev/) - 响应式 UI 框架
- [rusqlite](https://github.com/rusqlite/rusqlite) - SQLite Rust 绑定
- [arboard](https://github.com/1Password/arboard) - 跨平台剪切板库

## 📧 联系方式

如有问题或建议，请提交 [Issue](https://github.com/yourusername/clipman/issues)
