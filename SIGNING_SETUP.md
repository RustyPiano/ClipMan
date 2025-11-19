# 🔐 ClipMan 签名配置指南

本文档说明如何为 ClipMan 配置更新包签名，确保用户安全更新。

## ✅ 已完成的配置

### 1. 密钥对生成 ✅
```bash
✅ 私钥位置: ~/.tauri/clipman.key
✅ 公钥位置: ~/.tauri/clipman.key.pub
✅ 公钥已添加到: src-tauri/tauri.conf.json
✅ GitHub 仓库地址已更新: Kiaana/ClipMan
```

### 2. GitHub Actions 配置 ✅
```yaml
✅ 已添加签名环境变量引用
✅ 已改为自动发布 (releaseDraft: false)
```

## 🔧 需要手动完成的步骤

### 步骤 1: 读取私钥内容

在终端运行以下命令，复制输出内容：

```bash
cat ~/.tauri/clipman.key
```

**输出示例**:
```
untrusted comment: <comment>
<base64 encoded key data>
```

⚠️ **重要**: 复制**完整内容**，包括第一行注释。

### 步骤 2: 添加 GitHub Secrets

1. 访问 GitHub 仓库设置页面：
   ```
   https://github.com/Kiaana/ClipMan/settings/secrets/actions
   ```

2. 点击 **"New repository secret"** 按钮

3. 添加第一个 Secret:
   - **Name**: `TAURI_SIGNING_PRIVATE_KEY`
   - **Value**: 粘贴步骤 1 中复制的私钥内容
   - 点击 **"Add secret"**

4. 添加第二个 Secret:
   - **Name**: `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`
   - **Value**: 你在生成密钥时设置的密码
   - 点击 **"Add secret"**

### 步骤 3: 验证配置

完成上述步骤后，验证配置是否正确：

```bash
# 1. 查看 tauri.conf.json 中的 pubkey
cat src-tauri/tauri.conf.json | grep -A 5 "updater"

# 应该看到:
# "updater": {
#   "active": true,
#   "pubkey": "dW50cnVzdGVkIGNvbW1lbnQ6...", ← 不为空
#   ...
# }

# 2. 确认 GitHub endpoint 正确
# 应该看到:
#   "endpoints": [
#     "https://api.github.com/repos/Kiaana/ClipMan/releases/latest"
#   ]
```

## 🚀 测试更新流程

### 本地测试

1. **创建测试 tag**:
   ```bash
   git tag v1.0.1
   git push origin v1.0.1
   ```

2. **观察 GitHub Actions**:
   - 访问: https://github.com/Kiaana/ClipMan/actions
   - 查看是否触发了 "Release" workflow
   - 等待构建完成（约 10-15 分钟）

3. **检查 Release**:
   - 访问: https://github.com/Kiaana/ClipMan/releases
   - 应该看到 `v1.0.1` release（自动发布，非草稿）
   - 下载的包应该包含 `.sig` 签名文件

4. **测试应用内更新**:
   - 将 `tauri.conf.json` 和 `Cargo.toml` 中的版本改为 `1.0.0`
   - 运行应用: `npm run tauri dev`
   - 打开设置页面 → 点击"检查更新"
   - 应该检测到 `v1.0.1` 可用

### 生产发布流程

```bash
# 1. 更新版本号
# 编辑以下文件中的 version 字段:
# - src-tauri/Cargo.toml
# - src-tauri/tauri.conf.json
# - package.json

# 2. 提交更改
git add .
git commit -m "chore: bump version to 1.x.x"

# 3. 创建 tag
git tag v1.x.x

# 4. 推送到 GitHub
git push origin main
git push origin v1.x.x

# 5. GitHub Actions 会自动:
#    - 构建所有平台的安装包
#    - 使用私钥签名
#    - 创建 GitHub Release
#    - 上传签名后的安装包
```

## 🔒 安全提示

### ✅ 正确的做法

- ✅ 私钥仅存储在本地和 GitHub Secrets 中
- ✅ 绝不将私钥提交到 Git 仓库
- ✅ 定期备份私钥（加密存储）
- ✅ 使用强密码保护私钥

### ❌ 禁止的操作

- ❌ 不要将 `~/.tauri/clipman.key` 添加到版本控制
- ❌ 不要在公共场合分享私钥或密码
- ❌ 不要在日志中打印私钥内容
- ❌ 不要使用空密码保护私钥

## 📋 检查清单

在首次发布前，确认以下项目：

- [ ] ✅ 私钥和公钥已生成
- [ ] ✅ 公钥已添加到 `tauri.conf.json`
- [ ] ✅ GitHub endpoint 已更新为实际仓库地址
- [ ] ⚠️ `TAURI_SIGNING_PRIVATE_KEY` Secret 已添加到 GitHub
- [ ] ⚠️ `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` Secret 已添加到 GitHub
- [ ] ⬜ 已测试更新流程（创建测试 tag）
- [ ] ⬜ 本地应用成功检测到测试更新
- [ ] ⬜ 签名验证通过（无签名错误）

## 🆘 故障排查

### 问题 1: GitHub Actions 构建失败 - "signing key not found"

**原因**: GitHub Secrets 未正确配置

**解决方案**:
1. 检查 Secret 名称是否完全匹配（区分大小写）:
   - `TAURI_SIGNING_PRIVATE_KEY`
   - `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`
2. 确认私钥内容完整（包括首行注释）
3. 重新添加 Secrets

### 问题 2: 应用检查更新失败 - "signature verification failed"

**原因**: 公钥/私钥不匹配

**解决方案**:
1. 确认 `tauri.conf.json` 中的 `pubkey` 与 `~/.tauri/clipman.key.pub` 内容一致
2. 重新生成密钥对（需要重新签名所有历史版本）

### 问题 3: Release 创建成功但未生成 `.sig` 文件

**原因**: GitHub Secrets 未正确配置或工作流配置错误

**解决方案**:
1. 检查 `.github/workflows/release.yml` 中的 `env` 部分
2. 确认包含:
   ```yaml
   env:
     TAURI_SIGNING_PRIVATE_KEY: ${{ secrets.TAURI_SIGNING_PRIVATE_KEY }}
     TAURI_SIGNING_PRIVATE_KEY_PASSWORD: ${{ secrets.TAURI_SIGNING_PRIVATE_KEY_PASSWORD }}
   ```

## 📚 相关文档

- [Tauri 签名官方文档](https://tauri.app/v1/guides/distribution/sign-windows/)
- [GitHub Actions Secrets 使用指南](https://docs.github.com/en/actions/security-guides/encrypted-secrets)
- [ClipMan 更新功能说明](UPDATES.md)

## 🔄 密钥轮换

如果需要更换密钥（例如密钥泄露）:

1. **生成新密钥对**:
   ```bash
   npm run tauri signer generate -- -w ~/.tauri/clipman-new.key
   ```

2. **更新配置**:
   - 更新 `tauri.conf.json` 中的 `pubkey`
   - 更新 GitHub Secrets

3. **发布过渡版本**:
   - 使用**旧密钥**签名一个过渡版本
   - 该版本内置**新公钥**

4. **后续版本**:
   - 使用新密钥签名所有后续版本

⚠️ **警告**: 直接更换密钥会导致旧版本用户无法更新，必须按上述流程操作。

---

**配置完成后，别忘了提交更改到 Git！**

```bash
git add src-tauri/tauri.conf.json .github/workflows/release.yml
git commit -m "feat: 配置应用更新签名"
git push origin integrate-update-check
```
