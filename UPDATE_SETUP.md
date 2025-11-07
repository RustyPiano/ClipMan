# ClipMan 更新功能配置指南

## ✅ 功能状态

ClipMan 已完整实现了自动更新功能,包括:

- ✅ 检查 GitHub Releases 获取最新版本
- ✅ 显示版本信息和更新内容
- ✅ 一键下载并安装更新
- ✅ 更新进度提示和错误处理

## 📋 当前配置

### 版本信息
- **当前版本**: 1.0.0
- **更新源**: GitHub Releases API
- **签名验证**: 需要配置公钥

### 已启用的组件

1. **后端 (Rust)**
   - `tauri-plugin-updater` v2.1 已集成
   - `check_for_updates` 命令已实现
   - `install_update` 命令已实现

2. **前端 (Svelte)**
   - 设置页面集成更新检查UI
   - 版本对比显示
   - Release Notes 展示
   - 一键安装按钮

3. **配置文件**
   - `tauri.conf.json` 中 updater 插件已启用
   - 端点配置: `https://api.github.com/repos/{{owner}}/{{repo}}/releases/latest`

## ⚠️ 发布前必须配置

### 1. 生成签名密钥对

为了保证更新的安全性,需要生成密钥对来签名发布包:

```bash
# 安装 Tauri CLI (如果还没有)
cargo install tauri-cli --version "^2.0.0"

# 生成密钥对
tauri signer generate -w ~/.tauri/myapp.key
```

这将生成两个文件:
- **私钥** (`~/.tauri/myapp.key`): 用于签名发布包,**务必保密**
- **公钥** (显示在命令行): 需要配置到 `tauri.conf.json`

### 2. 配置公钥

将生成的公钥复制到 `src-tauri/tauri.conf.json`:

```json
{
  "plugins": {
    "updater": {
      "active": true,
      "pubkey": "YOUR_PUBLIC_KEY_HERE",  // 👈 粘贴公钥
      "endpoints": [
        "https://api.github.com/repos/{{owner}}/{{repo}}/releases/latest"
      ],
      "dialog": false
    }
  }
}
```

### 3. 更新 GitHub 仓库配置

在 `tauri.conf.json` 的 endpoints 中将 `{{owner}}/{{repo}}` 替换为实际的 GitHub 仓库:

```json
"endpoints": [
  "https://api.github.com/repos/yourusername/clipman/releases/latest"
]
```

或者在 `Cargo.toml` 中配置:

```toml
[package]
repository = "https://github.com/yourusername/clipman"
```

### 4. 签名发布包

构建发布版本时,使用私钥签名:

```bash
# 设置私钥路径环境变量
export TAURI_SIGNING_PRIVATE_KEY=~/.tauri/myapp.key
export TAURI_SIGNING_PRIVATE_KEY_PASSWORD=""  # 如果设置了密码

# 构建发布版本
npm run tauri build
```

### 5. 创建 GitHub Release

1. 在 GitHub 仓库创建新的 Release
2. 标签格式: `v1.0.1` (版本号需要递增)
3. 上传构建产物:
   - Windows: `*.msi`, `*.msi.zip`, `*.msi.zip.sig`
   - macOS: `*.app.tar.gz`, `*.app.tar.gz.sig`
   - Linux: `*.AppImage.tar.gz`, `*.AppImage.tar.gz.sig`

**重要**: `.sig` 签名文件必须一起上传!

## 🧪 测试更新功能

### 本地测试

1. 确保当前版本号低于测试版本
2. 在 GitHub 创建一个测试 Release
3. 运行应用并进入设置页面
4. 点击"检查更新"按钮
5. 查看是否检测到新版本
6. 点击"安装更新"测试下载和安装流程

### 测试步骤

```bash
# 1. 构建当前版本
npm run tauri build

# 2. 安装并运行
# Windows: 双击 src-tauri/target/release/bundle/msi/*.msi
# macOS: 打开 src-tauri/target/release/bundle/macos/*.app

# 3. 创建新版本 (修改版本号)
# 编辑 src-tauri/Cargo.toml, package.json, tauri.conf.json
# 将版本改为 1.0.1

# 4. 构建并签名新版本
export TAURI_SIGNING_PRIVATE_KEY=~/.tauri/myapp.key
npm run tauri build

# 5. 在 GitHub 创建 v1.0.1 Release 并上传构建产物

# 6. 在运行的 1.0.0 版本中检查更新
```

## 📝 版本发布流程

### 每次发布新版本时:

1. **更新版本号** (3个文件保持一致):
   ```bash
   # src-tauri/Cargo.toml
   version = "1.0.1"

   # package.json
   "version": "1.0.1"

   # src-tauri/tauri.conf.json
   "version": "1.0.1"
   ```

2. **构建并签名**:
   ```bash
   export TAURI_SIGNING_PRIVATE_KEY=~/.tauri/myapp.key
   npm run tauri build
   ```

3. **创建 GitHub Release**:
   - Tag: `v1.0.1`
   - Title: `ClipMan v1.0.1`
   - Description: 更新内容说明
   - 上传文件:
     - 安装包 (`.msi`, `.app.tar.gz`, `.AppImage.tar.gz`)
     - 压缩包 (`.msi.zip`, `.app.tar.gz`, `.AppImage.tar.gz`)
     - 签名文件 (`.sig`)

4. **验证更新**:
   - 旧版本打开设置
   - 点击检查更新
   - 确认显示新版本信息
   - 测试安装流程

## 🔒 安全注意事项

1. **私钥安全**:
   - ⚠️ **绝对不要** 提交私钥到 Git 仓库
   - ⚠️ **绝对不要** 分享私钥
   - 建议使用 GitHub Actions Secrets 存储私钥

2. **GitHub Actions 自动化** (推荐):

```yaml
# .github/workflows/release.yml
name: Release

on:
  push:
    tags:
      - 'v*'

jobs:
  release:
    runs-on: ${{ matrix.os }}
    strategy:
      matrix:
        os: [ubuntu-latest, macos-latest, windows-latest]

    steps:
      - uses: actions/checkout@v4

      - name: Setup Node.js
        uses: actions/setup-node@v4
        with:
          node-version: '20'

      - name: Setup Rust
        uses: dtolnay/rust-toolchain@stable

      - name: Install dependencies
        run: npm install

      - name: Build and sign
        env:
          TAURI_SIGNING_PRIVATE_KEY: ${{ secrets.TAURI_SIGNING_PRIVATE_KEY }}
          TAURI_SIGNING_PRIVATE_KEY_PASSWORD: ${{ secrets.TAURI_SIGNING_PRIVATE_KEY_PASSWORD }}
        run: npm run tauri build

      - name: Upload Release Assets
        uses: softprops/action-gh-release@v1
        with:
          files: |
            src-tauri/target/release/bundle/**/*.msi
            src-tauri/target/release/bundle/**/*.msi.zip
            src-tauri/target/release/bundle/**/*.msi.zip.sig
            src-tauri/target/release/bundle/**/*.app.tar.gz
            src-tauri/target/release/bundle/**/*.app.tar.gz.sig
            src-tauri/target/release/bundle/**/*.AppImage.tar.gz
            src-tauri/target/release/bundle/**/*.AppImage.tar.gz.sig
```

## 📚 参考文档

- [Tauri Updater 官方文档](https://v2.tauri.app/plugin/updater/)
- [Tauri Signer 工具](https://v2.tauri.app/develop/updater/#signing-updates)
- [GitHub Releases API](https://docs.github.com/en/rest/releases/releases)

## 🐛 常见问题

### Q: 更新检查失败怎么办?
A: 检查:
1. GitHub 仓库是否公开
2. Release 是否已发布 (不是 Draft)
3. 网络连接是否正常
4. API 端点配置是否正确

### Q: 安装更新失败?
A: 检查:
1. 签名文件 (.sig) 是否上传
2. 公钥配置是否正确
3. 构建时是否使用了正确的私钥
4. 磁盘空间是否足够

### Q: 检测不到更新?
A: 确认:
1. 新版本号是否大于当前版本
2. Release tag 格式是否为 `v1.0.1`
3. 清除缓存后重试

## ✅ 完成检查清单

发布前确认:

- [ ] 已生成密钥对
- [ ] 公钥已配置到 `tauri.conf.json`
- [ ] GitHub 仓库信息已更新
- [ ] 版本号已在 3 个文件中更新
- [ ] 私钥已安全保存 (不在 Git 中)
- [ ] 已测试本地构建流程
- [ ] 已测试更新检查功能
- [ ] GitHub Actions 已配置 (可选但推荐)

---

**当前状态**: ✅ 代码实现完成,等待密钥配置和首次发布测试
