# ClipMan 开发指南

本文档提供详细的开发环境配置、代码规范、调试技巧和贡献指南。

## 目录

- [开发环境配置](#开发环境配置)
- [项目结构详解](#项目结构详解)
- [开发工作流](#开发工作流)
- [代码规范](#代码规范)
- [调试技巧](#调试技巧)
- [常见问题](#常见问题)
- [发布流程](#发布流程)

## 开发环境配置

### 必需工具

1. **Rust** (1.82+)
   ```bash
   # 安装 rustup
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

   # 验证安装
   rustc --version
   cargo --version
   ```

2. **Node.js** (18+) 或 **Bun**
   ```bash
   # 使用 nvm 安装 Node.js
   nvm install 18
   nvm use 18

   # 或安装 Bun（推荐，更快）
   curl -fsSL https://bun.sh/install | bash
   ```

3. **Tauri CLI**
   ```bash
   # npm 方式（已包含在 package.json）
   npm install

   # 或全局安装
   cargo install tauri-cli
   ```

### 平台特定要求

#### macOS
```bash
# 安装 Xcode Command Line Tools
xcode-select --install

# 安装 Homebrew（如果未安装）
/bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"
```

#### Windows
```bash
# 安装 Visual Studio Build Tools
# 下载链接: https://visualstudio.microsoft.com/downloads/

# 需要勾选：
# - Desktop development with C++
# - Windows 10 SDK
```

#### Linux (Ubuntu/Debian)
```bash
sudo apt update
sudo apt install libwebkit2gtk-4.0-dev \
    build-essential \
    curl \
    wget \
    file \
    libssl-dev \
    libgtk-3-dev \
    libayatana-appindicator3-dev \
    librsvg2-dev
```

### IDE 配置

#### VS Code (推荐)

**必装扩展**:
```json
{
  "recommendations": [
    "rust-lang.rust-analyzer",
    "tauri-apps.tauri-vscode",
    "svelte.svelte-vscode",
    "bradlc.vscode-tailwindcss"
  ]
}
```

**settings.json 配置**:
```json
{
  "rust-analyzer.checkOnSave.command": "clippy",
  "rust-analyzer.cargo.features": "all",
  "editor.formatOnSave": true,
  "[rust]": {
    "editor.defaultFormatter": "rust-lang.rust-analyzer"
  },
  "[svelte]": {
    "editor.defaultFormatter": "svelte.svelte-vscode"
  }
}
```

## 项目结构详解

```
ClipMan/
├── src-tauri/              # Rust 后端
│   ├── src/
│   │   ├── main.rs         # 460+ 行：入口、托盘、IPC 命令
│   │   ├── clipboard.rs    # 195 行：剪切板监控核心
│   │   ├── storage.rs      # 295 行：数据库封装
│   │   ├── crypto.rs       # 95 行：加密模块
│   │   └── settings.rs     # 80 行：设置管理
│   ├── Cargo.toml          # Rust 依赖配置
│   ├── tauri.conf.json     # Tauri 应用配置
│   ├── build.rs            # 构建脚本
│   └── icons/              # 应用图标
├── src/                    # Svelte 前端
│   ├── lib/
│   │   ├── components/
│   │   │   ├── SearchBar.svelte        # 60 行：搜索组件
│   │   │   ├── ClipboardItem.svelte    # 120 行：历史项卡片
│   │   │   └── PermissionCheck.svelte  # 90 行：权限检查
│   │   └── stores/
│   │       ├── clipboard.svelte.ts     # 135 行：状态管理
│   │       └── router.svelte.ts        # 25 行：路由
│   ├── routes/
│   │   ├── +page.svelte                # 150 行：主页面
│   │   └── settings/+page.svelte       # 180 行：设置页面
│   ├── app.css             # Tailwind 样式
│   └── main.ts             # 前端入口
├── CLAUDE.md               # PRD 文档
├── ARCHITECTURE.md         # 架构文档
├── DEVELOPMENT.md          # 本文档
├── CLEANUP_DB.md           # 数据库清理指南
├── package.json            # npm 依赖
├── vite.config.ts          # Vite 配置
├── svelte.config.js        # Svelte 配置
└── tailwind.config.js      # Tailwind 配置
```

## 开发工作流

### 1. 克隆并初始化

```bash
# 克隆仓库
git clone https://github.com/yourusername/clipman.git
cd clipman

# 安装依赖
npm install  # 或 bun install

# 首次运行（会自动安装 Rust 依赖）
npm run tauri dev
```

### 2. 开发模式

```bash
# 启动热重载开发服务器
npm run tauri dev

# 或使用 bun（更快）
bun tauri dev

# 仅运行前端（调试 UI）
npm run dev
```

**开发模式特点**:
- Rust: 修改后自动重新编译
- Svelte: HMR 热更新
- 日志: 输出到终端
- macOS: 可能显示 Dock 图标（正常）

### 3. 构建发布版本

```bash
# 构建生产版本
npm run tauri build

# 输出位置:
# - macOS: src-tauri/target/release/bundle/dmg/
# - Windows: src-tauri/target/release/bundle/msi/
# - Linux: src-tauri/target/release/bundle/appimage/
```

### 4. 代码检查

```bash
# Rust 代码格式化
cd src-tauri
cargo fmt

# Rust 代码检查
cargo clippy -- -D warnings

# 前端格式化（如果配置了 prettier）
npm run format
```

### 5. 测试

```bash
# Rust 单元测试
cd src-tauri
cargo test

# 前端测试（如果配置了）
npm test
```

## 代码规范

### Rust 代码规范

遵循 [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)

**命名规范**:
```rust
// ✅ 正确
pub struct ClipboardMonitor { }
pub fn start_monitoring() { }
const MAX_ITEMS: usize = 100;

// ❌ 错误
pub struct clipboardMonitor { }
pub fn StartMonitoring() { }
const max_items: usize = 100;
```

**错误处理**:
```rust
// ✅ 正确：使用 Result
pub fn get_item(id: &str) -> Result<ClipItem, String> {
    storage.get(id).map_err(|e| e.to_string())
}

// ❌ 错误：使用 panic
pub fn get_item(id: &str) -> ClipItem {
    storage.get(id).unwrap()  // 危险！
}
```

**文档注释**:
```rust
/// 获取最近的剪切板历史项
///
/// # Arguments
/// * `limit` - 返回的最大项数
///
/// # Returns
/// * `Ok(Vec<ClipItem>)` - 历史项列表
/// * `Err(String)` - 数据库错误
pub fn get_recent(&self, limit: usize) -> Result<Vec<ClipItem>> {
    // ...
}
```

### TypeScript/Svelte 代码规范

**命名规范**:
```typescript
// ✅ 正确
class ClipboardStore { }
function loadHistory() { }
const MAX_ITEMS = 100;

// ❌ 错误
class clipboard_store { }
function LoadHistory() { }
const max_items = 100;
```

**Svelte 5 Runes 最佳实践**:
```typescript
// ✅ 正确：使用 Runes
class ClipboardStore {
  items = $state<ClipItem[]>([]);
  pinnedItems = $derived(this.items.filter(i => i.isPinned));
}

// ❌ 错误：使用旧 API
let items: ClipItem[] = [];
$: pinnedItems = items.filter(i => i.isPinned);
```

**类型安全**:
```typescript
// ✅ 正确：明确类型
async function loadHistory(): Promise<ClipItem[]> {
  return await invoke<ClipItem[]>('get_clipboard_history');
}

// ❌ 错误：any 类型
async function loadHistory(): Promise<any> {
  return await invoke('get_clipboard_history');
}
```

## 调试技巧

### Rust 后端调试

**1. 日志输出**:
```rust
// 不同级别的日志
log::error!("Critical error: {}", e);
log::warn!("Warning: {}", message);
log::info!("Info: operation completed");
log::debug!("Debug: variable = {:?}", var);
```

**2. 查看日志**:
```bash
# 运行时会自动输出到终端
npm run tauri dev

# 调整日志级别
RUST_LOG=debug npm run tauri dev
```

**3. 使用 LLDB (macOS/Linux)**:
```bash
# 编译 debug 版本
cargo build

# 启动调试器
lldb target/debug/clipman

# 设置断点
(lldb) breakpoint set --name main
(lldb) run
```

**4. 使用 VS Code 调试**:

`.vscode/launch.json`:
```json
{
  "version": "0.2.0",
  "configurations": [
    {
      "type": "lldb",
      "request": "launch",
      "name": "Debug Rust",
      "cargo": {
        "args": ["build", "--manifest-path=src-tauri/Cargo.toml"]
      },
      "program": "${workspaceFolder}/src-tauri/target/debug/clipman"
    }
  ]
}
```

### 前端调试

**1. 浏览器开发者工具**:
```bash
# 打开开发工具
npm run tauri dev

# 在窗口中右键 -> Inspect Element
# 或使用快捷键 Cmd+Option+I (macOS) / F12 (Windows/Linux)
```

**2. Console 日志**:
```typescript
console.log('✅ Data loaded:', items);
console.warn('⚠️ Warning:', message);
console.error('❌ Error:', error);
```

**3. Svelte DevTools**:
```bash
# 安装浏览器扩展
# Chrome: https://chrome.google.com/webstore/detail/svelte-devtools
# Firefox: https://addons.mozilla.org/en-US/firefox/addon/svelte-devtools/
```

### IPC 通信调试

**后端日志**:
```rust
#[tauri::command]
async fn get_clipboard_history(limit: usize) -> Result<Vec<ClipItem>, String> {
    log::info!("IPC: get_clipboard_history called with limit={}", limit);
    // ...
}
```

**前端日志**:
```typescript
async loadHistory() {
  console.log('📡 Calling IPC: get_clipboard_history');
  const result = await invoke<ClipItem[]>('get_clipboard_history', {
    limit: 100,
  });
  console.log('📡 IPC result:', result);
}
```

## 常见问题

### 1. 编译错误：找不到 `tauri` crate

**症状**:
```
error: could not find `tauri` in the list of imported crates
```

**解决**:
```bash
cd src-tauri
cargo clean
cargo build
```

### 2. macOS 权限问题

**症状**: 无法读取剪切板

**解决**:
1. 系统设置 → 隐私与安全性 → 辅助功能
2. 添加终端/VS Code
3. 勾选启用

### 3. 前端 HMR 不工作

**症状**: 修改代码后不自动刷新

**解决**:
```bash
# 清除缓存重新启动
rm -rf node_modules/.vite
npm run tauri dev
```

### 4. Windows WebView2 缺失

**症状**: `WebView2 runtime is not installed`

**解决**:
```bash
# 下载并安装 WebView2 Runtime
# https://developer.microsoft.com/en-us/microsoft-edge/webview2/
```

### 5. 数据库锁定错误

**症状**: `database is locked`

**解决**:
```bash
# 关闭所有运行的实例
pkill clipman

# 删除锁文件
rm ~/Library/Application\ Support/com.clipman.app/*.db-shm
rm ~/Library/Application\ Support/com.clipman.app/*.db-wal
```

### 6. Unicode panic

**症状**: `byte index X is not a char boundary`

**原因**: 使用字节索引截断 UTF-8 字符串

**修复**: 使用字符迭代器
```rust
// ❌ 错误
&text[..50]

// ✅ 正确
text.chars().take(50).collect::<String>()
```

## 发布流程

### 1. 版本号更新

更新以下文件中的版本号：

**package.json**:
```json
{
  "version": "1.0.0"
}
```

**src-tauri/Cargo.toml**:
```toml
[package]
version = "1.0.0"
```

**src-tauri/tauri.conf.json**:
```json
{
  "version": "1.0.0"
}
```

### 2. 构建所有平台

```bash
# macOS
npm run tauri build

# Windows (在 Windows 机器上)
npm run tauri build

# Linux (在 Linux 机器上)
npm run tauri build
```

### 3. 测试发布版本

```bash
# macOS
open src-tauri/target/release/bundle/dmg/ClipMan_1.0.0_x64.dmg

# 测试清单:
# - ✅ 启动正常
# - ✅ 剪切板监控工作
# - ✅ 历史记录显示
# - ✅ 置顶功能正常
# - ✅ 搜索功能正常
# - ✅ 托盘菜单正常
# - ✅ 热键工作
# - ✅ 设置保存/加载
```

### 4. 创建 Git Tag

```bash
git tag -a v1.0.0 -m "Release v1.0.0"
git push origin v1.0.0
```

### 5. 创建 GitHub Release

1. 前往 GitHub Releases 页面
2. 点击 "Draft a new release"
3. 选择 tag `v1.0.0`
4. 填写 Release Notes（参考 CHANGELOG.md）
5. 上传构建产物：
   - `ClipMan_1.0.0_x64.dmg` (macOS)
   - `ClipMan_1.0.0_x64.msi` (Windows)
   - `clipman_1.0.0_amd64.AppImage` (Linux)
6. 点击 "Publish release"

### 6. 发布到 Homebrew (macOS)

```bash
# 创建 Homebrew Cask
# 提交 PR 到 homebrew-cask
```

### 7. 更新文档

- 更新 README.md 中的版本号
- 更新 CHANGELOG.md
- 更新下载链接

## 贡献指南

### 提交 Pull Request

1. **Fork 项目**
   ```bash
   # 在 GitHub 上点击 Fork
   git clone https://github.com/YOUR_USERNAME/clipman.git
   cd clipman
   ```

2. **创建功能分支**
   ```bash
   git checkout -b feature/amazing-feature
   ```

3. **编写代码**
   - 遵循代码规范
   - 添加必要的测试
   - 更新文档

4. **提交更改**
   ```bash
   git add .
   git commit -m "feat: Add amazing feature"
   ```

   **提交信息规范** (Conventional Commits):
   - `feat:` 新功能
   - `fix:` Bug 修复
   - `docs:` 文档更新
   - `refactor:` 代码重构
   - `test:` 测试相关
   - `chore:` 构建/工具更新

5. **推送到 GitHub**
   ```bash
   git push origin feature/amazing-feature
   ```

6. **创建 Pull Request**
   - 前往 GitHub 仓库
   - 点击 "New pull request"
   - 填写详细的 PR 描述
   - 等待 Review

### Code Review 检查清单

- [ ] 代码遵循项目规范
- [ ] 所有测试通过
- [ ] 没有引入新的警告
- [ ] 文档已更新
- [ ] 提交信息清晰
- [ ] 没有调试代码（console.log 等）

## 联系方式

- **GitHub Issues**: [提交 Issue](https://github.com/yourusername/clipman/issues)
- **Discussions**: [参与讨论](https://github.com/yourusername/clipman/discussions)

---

Happy Coding! 🚀
