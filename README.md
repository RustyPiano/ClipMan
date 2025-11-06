# ClipMan - 现代化剪切板管理器

> 跨平台剪切板管理器（Windows/macOS/Linux），使用 Rust + Tauri 2.0 + Svelte 5 构建

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/Rust-1.82+-orange.svg)](https://www.rust-lang.org/)
[![Tauri](https://img.shields.io/badge/Tauri-2.0-blue.svg)](https://tauri.app/)
[![Svelte](https://img.shields.io/badge/Svelte-5-red.svg)](https://svelte.dev/)

## ✨ 特性

- 🔄 **自动剪切板监控** - 实时捕获文本和图像（每 500ms）
- 🔍 **强大搜索功能** - 基于 SQLite FTS5 全文搜索
- 📌 **置顶常用内容** - 快速访问常用剪切板项
- 🎯 **原生托盘菜单** - 左键点击直接显示历史列表
- ⌨️ **全局热键** - `Cmd+Shift+V` (macOS) / `Ctrl+Shift+V` (Windows/Linux)
- 🎨 **现代化 UI** - 简洁易用，支持滚动和搜索
- 🔐 **隐私保护** - 本地存储，AES-256-GCM 端到端加密
- ⚡ **高性能** - Rust 后端，内存占用 < 50MB
- 🪶 **轻量级** - 安装包 < 10MB
- 🍎 **macOS 优化** - 菜单栏专属模式，无 Dock 图标

## 🛠️ 技术栈

### 后端
- **Rust 1.82+** - 安全、高性能系统编程语言
- **Tauri 2.0** - 现代桌面应用框架，WebView2 渲染
- **SQLite + FTS5** - 本地数据库，全文搜索支持
- **arboard 3.4** - 跨平台剪切板操作
- **ring 0.17** - AES-256-GCM 加密
- **image 0.25** - 图像处理和缩略图生成

### 前端
- **Svelte 5** - 响应式框架，使用最新 Runes API (`$state`, `$derived`)
- **TypeScript** - 类型安全
- **Tailwind CSS 4** - 现代化样式
- **Vite 6** - 快速构建工具

## 🚀 快速开始

### 前置要求

- **Node.js** 18+ 或 Bun
- **Rust** 1.82+ (安装: https://rustup.rs/)
- **系统要求**:
  - Windows 10+ (需要 WebView2)
  - macOS 10.13+
  - Linux (需要 WebKit2GTK)

### 安装依赖

```bash
# 克隆仓库
git clone https://github.com/yourusername/clipman.git
cd clipman

# 安装 npm 依赖
npm install

# 或使用 bun（更快）
bun install
```

### 开发模式

```bash
# 启动开发服务器
npm run tauri dev

# 或使用 bun
bun tauri dev
```

**注意 (macOS)**: Dev 模式下可能显示 Dock 图标，这是正常的。Build 版本会正确隐藏 Dock 图标。

### 构建应用

```bash
# 构建生产版本
npm run tauri build

# 构建产物位置:
# - Windows: src-tauri/target/release/bundle/msi/
# - macOS: src-tauri/target/release/bundle/dmg/
# - Linux: src-tauri/target/release/bundle/appimage/
```

### macOS 权限设置

首次运行时，需要授予剪切板访问权限：

1. 打开 **系统设置** → **隐私与安全性**
2. 选择 **辅助功能**
3. 点击 🔒 解锁
4. 添加 ClipMan（或运行它的终端/IDE）
5. 勾选启用

## 📁 项目结构

```
ClipMan/
├── src-tauri/              # Rust 后端
│   ├── src/
│   │   ├── main.rs         # 入口点、Tauri 设置、系统托盘、IPC 命令
│   │   ├── clipboard.rs    # 剪切板监控核心（500ms 轮询）
│   │   ├── storage.rs      # SQLite 数据库封装（CRUD + FTS5）
│   │   ├── crypto.rs       # AES-256-GCM 加密模块
│   │   └── settings.rs     # 设置管理（热键、历史上限等）
│   ├── Cargo.toml          # Rust 依赖
│   ├── tauri.conf.json     # Tauri 配置（窗口、托盘、图标）
│   └── build.rs            # 构建脚本（macOS 部署目标）
├── src/                    # Svelte 前端
│   ├── lib/
│   │   ├── components/     # Svelte 组件
│   │   │   ├── SearchBar.svelte        # 搜索栏
│   │   │   ├── ClipboardItem.svelte    # 历史项卡片
│   │   │   └── PermissionCheck.svelte  # macOS 权限检查 UI
│   │   └── stores/
│   │       ├── clipboard.svelte.ts     # 剪切板状态管理（Runes）
│   │       └── router.svelte.ts        # 客户端路由
│   ├── routes/
│   │   ├── +page.svelte                # 主页面（历史列表）
│   │   └── settings/+page.svelte       # 设置页面
│   ├── app.css             # 全局样式（Tailwind）
│   └── main.ts             # 前端入口
├── CLAUDE.md               # 产品需求文档（PRD）
├── CLEANUP_DB.md           # 数据库清理说明
├── ARCHITECTURE.md         # 架构文档（新）
├── package.json            # npm 依赖
├── vite.config.js          # Vite 配置
└── svelte.config.js        # Svelte 配置
```

## 🎯 功能完成度

### ✅ 已完成 (MVP v1.0)

- [x] **剪切板监控** - 文本/图像自动捕获
- [x] **历史记录存储** - SQLite 加密存储
- [x] **全文搜索** - FTS5 高性能搜索
- [x] **置顶功能** - 拖拽排序、独立显示
- [x] **系统托盘集成** - 动态菜单（置顶项 + 最近项）
- [x] **全局热键** - 可自定义快捷键
- [x] **图像缩略图** - 256x256 Lanczos3 缩放
- [x] **端到端加密** - AES-256-GCM
- [x] **设置页面** - 热键配置、历史上限
- [x] **macOS 优化** - 菜单栏模式、权限检查
- [x] **Svelte 5 UI** - 响应式、现代化界面
- [x] **错误恢复** - Poisoned lock 恢复、解密错误跳过
- [x] **Unicode 安全** - 字符边界安全截断

### 🚧 部分完成

- [~] **菜单项复制** - 文本支持 ✅，图片待实现
- [~] **窗口显示** - 基础功能 ✅，需测试清理旧数据后的显示

### ❌ 待实现 (v1.1+)

- [ ] **数据导出** - JSON/CSV 格式
- [ ] **敏感内容过滤** - 密码字段自动排除
- [ ] **自定义主题** - 明/暗模式、颜色主题
- [ ] **文件路径支持** - 复制文件路径
- [ ] **多语言支持** - i18n (英语/中文/德语)
- [ ] **自动更新** - OTA 更新

## 🔑 核心功能实现

### 1. 剪切板监控

使用 `arboard` crate 每 500ms 轮询剪切板变化，支持文本和图像：

```rust
// src-tauri/src/clipboard.rs
pub struct ClipboardMonitor {
    app_handle: AppHandle,
    last_copied_by_us: Arc<Mutex<Option<String>>>,
}

impl ClipboardMonitor {
    pub fn start(&self) {
        thread::spawn(move || {
            loop {
                // 检测文本变化
                if let Ok(text) = clipboard.get_text() {
                    if text != last_text && !should_skip {
                        save_to_storage(&app_handle, item);
                    }
                }
                // 检测图像变化
                if let Ok(image) = clipboard.get_image() {
                    let thumbnail = create_thumbnail(&image_bytes);
                    save_to_storage(&app_handle, item);
                }
                thread::sleep(Duration::from_millis(500));
            }
        });
    }
}
```

### 2. 全文搜索 (SQLite FTS5)

使用 SQLite FTS5 虚拟表实现高性能中文搜索：

```sql
-- 创建 FTS5 虚拟表
CREATE VIRTUAL TABLE clips_fts
USING fts5(id, content_text, content='clips', content_rowid=rowid);

-- 自动同步触发器
CREATE TRIGGER clips_ai AFTER INSERT ON clips BEGIN
    INSERT INTO clips_fts(rowid, id, content_text)
    VALUES (new.rowid, new.id, new.content);
END;
```

### 3. Svelte 5 Runes 状态管理

```typescript
// src/lib/stores/clipboard.svelte.ts
class ClipboardStore {
  items = $state<ClipItem[]>([]);
  searchQuery = $state('');
  isLoading = $state(false);

  // 派生状态 - 自动重新计算
  pinnedItems = $derived(
    this.items
      .filter(item => item.isPinned)
      .sort((a, b) => (a.pinOrder || 0) - (b.pinOrder || 0))
  );

  filteredItems = $derived.by(() => {
    if (!this.searchQuery) return this.items;
    return this.items.filter(item => {
      const text = new TextDecoder().decode(item.content);
      return text.toLowerCase().includes(this.searchQuery.toLowerCase());
    });
  });
}
```

### 4. 动态托盘菜单

使用 Tauri 2.0 新 API 构建动态菜单：

```rust
// src-tauri/src/main.rs
fn build_tray_menu(app: &AppHandle) -> Result<Menu> {
    let storage = safe_lock(&state.storage);

    // 获取置顶项（最多 5 个）
    let pinned_items = storage.get_pinned()?;
    for item in pinned_items.iter().take(5) {
        let preview = truncate_content(&item.content, 50);
        menu_builder = menu_builder.item(&MenuItemBuilder::with_id(
            format!("clip:{}", item.id),
            preview
        ).build(app)?);
    }

    // 获取最近项（最多 10 个）
    let recent_items = storage.get_recent(15)?;
    // ... 构建菜单
}
```

### 5. AES-256-GCM 加密

```rust
// src-tauri/src/crypto.rs
pub struct Crypto {
    key: [u8; 32],
}

impl Crypto {
    pub fn encrypt(&self, data: &[u8]) -> Result<Vec<u8>> {
        let nonce = generate_nonce();
        let encrypted = aead::seal(&self.key, &nonce, data)?;
        // nonce(12) + encrypted_data + tag(16)
        Ok([nonce.to_vec(), encrypted].concat())
    }

    pub fn decrypt(&self, encrypted: &[u8]) -> Result<Vec<u8>> {
        let (nonce, ciphertext) = encrypted.split_at(12);
        aead::open(&self.key, nonce, ciphertext)
    }
}
```

## 🐛 已知问题与解决方案

### 问题 1: macOS Dev 模式显示 Dock 图标

**症状**: 开发模式下显示两个图标（菜单栏 + Dock）

**原因**: Tauri dev 工具限制

**解决方案**:
```bash
# 测试 build 版本
tauri build
open src-tauri/target/release/bundle/macos/ClipMan.app
```

Build 版本会正确设置 `NSApplicationActivationPolicyAccessory`。

### 问题 2: 解密错误

**症状**: 日志显示 `⚠️ Failed to decrypt item xxx`

**原因**: 旧数据库使用不同的加密密钥

**解决方案**:
```bash
# 删除旧数据库和密钥
rm -f ~/Library/Application\ Support/com.clipman.app/clipman.db
rm -f ~/Library/Application\ Support/com.clipman.app/.clipman.key
```

详见 [CLEANUP_DB.md](CLEANUP_DB.md)

### 问题 3: Unicode 字符截断 Panic

**状态**: ✅ 已修复

**修复**: 使用字符迭代器而不是字节索引：
```rust
// 修复前: &text[..50]  ❌
// 修复后: text.chars().take(50).collect()  ✅
```

## 📚 文档

- [CLAUDE.md](CLAUDE.md) - 产品需求文档（PRD）
- [ARCHITECTURE.md](ARCHITECTURE.md) - 架构设计文档
- [CLEANUP_DB.md](CLEANUP_DB.md) - 数据库清理指南
- [DEVELOPMENT.md](DEVELOPMENT.md) - 开发指南

## 🤝 贡献指南

欢迎贡献！请遵循以下步骤：

1. Fork 项目
2. 创建功能分支 (`git checkout -b feature/AmazingFeature`)
3. 提交更改 (`git commit -m 'feat: Add some AmazingFeature'`)
4. 推送到分支 (`git push origin feature/AmazingFeature`)
5. 开启 Pull Request

**提交信息规范**: 使用 [Conventional Commits](https://www.conventionalcommits.org/)
- `feat:` 新功能
- `fix:` Bug 修复
- `docs:` 文档更新
- `refactor:` 代码重构
- `test:` 测试相关

## 📄 许可证

MIT License - 详见 [LICENSE](LICENSE) 文件

## 🙏 致谢

- [Tauri](https://tauri.app/) - 现代桌面应用框架
- [Svelte](https://svelte.dev/) - 响应式 UI 框架
- [rusqlite](https://github.com/rusqlite/rusqlite) - SQLite Rust 绑定
- [arboard](https://github.com/1Password/arboard) - 跨平台剪切板库
- [ring](https://github.com/briansmith/ring) - 加密库

## 📧 联系方式

如有问题或建议，请提交 [Issue](https://github.com/yourusername/clipman/issues)

---

**注**: 本项目使用 2025 年最新技术栈构建，遵循现代化软件工程最佳实践。
