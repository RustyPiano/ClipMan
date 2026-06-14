# ClipMan 发布指南

## 📋 发布步骤

### 1. 准备发布

确保所有更改都已提交并推送到 GitHub:

```bash
# 检查状态
git status

# 确保在 main 分支
git checkout main
git pull origin main
```

### 2. 创建版本标签

使用语义化版本号 (Semantic Versioning):

```bash
# 格式: v主版本号.次版本号.修订号
# 例如: v1.0.0, v1.1.0, v1.0.1

# 创建标签
git tag -a v1.0.0 -m "Release v1.0.0 - 首个正式版本"

# 推送标签到 GitHub (这会触发 workflow)
git push origin v1.0.0
```

### 3. 等待构建完成

1. 访问 GitHub Actions: `https://github.com/RustyPiano/ClipMan/actions`
2. 查看 "Release" workflow 运行状态
3. 等待所有平台构建完成 (约 10-20 分钟)

构建产物:
- **macOS (Apple Silicon)**: `.dmg`, `.app.tar.gz`
- **macOS (Intel)**: `.dmg`, `.app.tar.gz`
- **Windows**: `.msi`, `.msi.zip`
- **Linux**: `.deb`, `.AppImage`, `.AppImage.tar.gz`

### 4. 编辑 Release 说明

构建完成后:

1. 进入 Releases: `https://github.com/RustyPiano/ClipMan/releases`
2. 找到自动创建的 Draft release (v1.0.0)
3. 点击 "Edit draft"
4. 检查自动填入的 `release_notes_<版本号>.md` 内容是否正确
5. 可选: 添加截图或演示 GIF
6. 取消勾选 "Set as a pre-release" (如果这是正式版本)
7. 点击 "Publish release"

### 5. 验证发布

发布后检查:

```bash
# 下载并测试安装包
# macOS
curl -L https://github.com/RustyPiano/ClipMan/releases/download/v1.0.0/ClipMan_1.0.0_aarch64.dmg -o ClipMan.dmg

# 验证签名（自签名证书；spctl 因未公证会判为 rejected，属正常现象）
codesign -dv --verbose=2 ClipMan.app    # 应显示 Authority=ClipMan Code Signing
codesign --verify --strict ClipMan.app  # 应输出 "satisfies its Designated Requirement"

# 测试安装
open ClipMan.dmg
```

## 🔧 版本号规则

遵循语义化版本 (SemVer):

- **主版本号 (Major)**: 不兼容的 API 改动
  - 例: `v1.0.0` → `v2.0.0`

- **次版本号 (Minor)**: 向后兼容的功能新增
  - 例: `v1.0.0` → `v1.1.0`

- **修订号 (Patch)**: 向后兼容的问题修正
  - 例: `v1.0.0` → `v1.0.1`

示例:
```bash
# Bug 修复
git tag -a v1.0.1 -m "Fix: 修复搜索功能问题"

# 新功能
git tag -a v1.1.0 -m "Feature: 添加图片复制支持"

# 重大更新
git tag -a v2.0.0 -m "Breaking: 升级到 Tauri 3.0"
```

## 📝 Release 说明模板

### 简短版 (GitHub Release)

创建或更新仓库根目录的 `release_notes_<版本号>.md`，例如 `release_notes_1.10.0.md`。Release workflow 会按 tag 自动读取该文件。

### 详细版 (博客/公告)

参考 `.github/RELEASE_TEMPLATE.md`

## ⚠️ 注意事项

### 首次发布检查清单

- [ ] README.md 中的安装链接已更新
- [ ] 项目截图/GIF 已添加
- [ ] LICENSE 文件存在
- [ ] CHANGELOG.md 已更新
- [ ] 所有已知 bug 已在 Issues 中标记
- [ ] 文档中的示例代码已测试
- [ ] 安装包在所有平台上测试通过

### 本地发布构建

`bun tauri build` 会生成 updater artifact。因为 `src-tauri/tauri.conf.json` 配置了 updater 公钥，本地完整发布构建需要设置：

```bash
export TAURI_SIGNING_PRIVATE_KEY="..."
export TAURI_SIGNING_PRIVATE_KEY_PASSWORD="..."
bun tauri build
```

GitHub Actions secrets 已配置时，GitHub 只能列出 secret 名称，不能读回 secret 值。可用下面命令确认仓库里是否存在对应 secret：

```bash
gh secret list --repo RustyPiano/ClipMan
```

如果本机没有环境变量，也找不到当初生成的私钥，只能重新生成一组 updater signing key：

```bash
bun tauri signer generate --write-keys ~/.tauri/clipman.key
```

把新的 private key 写入 GitHub secrets，把新的 public key 同步更新到 `src-tauri/tauri.conf.json` 的 updater `pubkey`。

如果只验证代码是否能编译，使用：

```bash
bun run build
cd src-tauri && cargo build
```

### 常见问题

**Q: Workflow 构建失败怎么办?**

A: 检查 Actions 日志,常见原因:
- Rust 依赖问题: 更新 `Cargo.toml`
- Node/Bun 依赖: 运行 `bun install`
- 平台特定问题: 检查对应平台的构建日志
- Updater 签名问题: 确认 `TAURI_SIGNING_PRIVATE_KEY` 和 `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` 已配置为 GitHub Secrets

**Q: 如何删除错误的 Release?**

A:
```bash
# 删除远程标签
git push --delete origin v1.0.0

# 删除本地标签
git tag -d v1.0.0

# 在 GitHub 上手动删除 Release
```

**Q: 如何配置代码签名?**

A:
- **macOS**: 已配置。Release 构建用**自签名证书**签名（无需 Apple Developer 账号）。目的是让 app 的签名要求（Designated Requirement）在各版本间保持稳定——用户只需授予一次辅助功能权限，更新后也不会失效（ad-hoc 签名每次构建哈希都变，会反复要求重新授权）。涉及：
  - GitHub Secrets：`APPLE_CERTIFICATE`（`.p12` 的 base64）、`APPLE_CERTIFICATE_PASSWORD`；`release.yml` 中写死 `APPLE_SIGNING_IDENTITY: 'ClipMan Code Signing'`。
  - 证书与私钥保存在仓库之外（本机 `~/ClipMan-signing/`），**必须永久复用同一张**；一旦更换，所有用户在下次更新后都要重新授权辅助功能。务必备份该目录。
  - **未做公证（notarization）**：用户首次打开仍会遇到 Gatekeeper“无法验证开发者”提示（右键打开 / “仍要打开”一次即可）。要彻底消除该提示，需付费的 Apple Developer ID + 公证。
- **Windows**: 需要 Code Signing 证书（未配置）。
- **Linux**: 通常不需要。

参考: https://tauri.app/distribute/

## 🚀 自动化发布 (可选)

使用 GitHub Actions 自动发布:

```bash
# 创建 Release Drafter 配置
.github/release-drafter.yml

# 自动生成更新日志
.github/workflows/update-changelog.yml
```

## 📊 发布后

1. **更新文档**: 确保 README 和文档中的链接指向最新版本
2. **社交媒体**: 在 Twitter, Reddit, Hacker News 等平台宣传
3. **收集反馈**: 关注 Issues 和 Discussions
4. **规划下一版本**: 根据反馈制定 roadmap

## 🔗 相关资源

- [Tauri 发布指南](https://tauri.app/v1/guides/distribution/)
- [GitHub Releases 文档](https://docs.github.com/en/repositories/releasing-projects-on-github)
- [语义化版本规范](https://semver.org/lang/zh-CN/)
