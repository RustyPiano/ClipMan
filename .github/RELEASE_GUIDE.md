# ClipMan 发布指南

## 发布步骤

### 1. 准备发布

确保所有更改都已提交并推送到 `main`：

```bash
git checkout main && git pull origin main
git status            # 工作区应干净
```

版本号写在四个文件里（`package.json`、`src-tauri/tauri.conf.json`、`src-tauri/Cargo.toml`、`src-tauri/Cargo.lock`）。**不要手动逐个改** —— 用脚本一次性同步，避免漏改导致的版本漂移（CI 的 `versions` 作业和 release 的 `preflight` 作业都会因此失败）。

### 2. 升级版本号 + 创建标签

版本号使用不带前导零的 `X.Y.Z`，例如 `2.3.1`。准备脚本在修改文件前检查格式、版本顺序和已有标签；相同版本在标签尚未创建时可以重复执行。有两条发布路径：

**路径 A —— 本地脚本（需要 Bash 和 Git，无需 Rust/Bun）**

```bash
VERSION=2.3.1                  # 示例：替换为实际版本
git fetch origin --tags
scripts/release.sh "$VERSION"  # 同步四个清单 + README 下载文件名 + 生成 release notes
$EDITOR "release_notes_${VERSION}.md"
git add package.json src-tauri/tauri.conf.json src-tauri/Cargo.toml src-tauri/Cargo.lock \
  README.md README_EN.md "release_notes_${VERSION}.md"
git commit -m "release: v${VERSION}"
git tag "v${VERSION}"
git push --atomic origin HEAD:main "refs/tags/v${VERSION}"
# Release 会检查标签对应的提交，所有检查通过后才开始打包。
```

**路径 B —— GitHub「Prepare Release」工作流（在 Actions 页一键触发）**

1. 先把填好的 `release_notes_<版本号>.md` 提交到 `main`（工作流不会替你生成，避免发出空说明）。
2. Actions → **Prepare Release** → 从 `main` 运行 → 填入不带 `v` 的版本号。流程检查输入后同步版本，仅暂存发布相关文件，在本地创建提交和标签，再一次性推送两者；任一推送被拒绝时，远端两者都保持原状。同一时间只运行一个发布准备任务。
3. **前置条件：** 必须配置 `RELEASE_PAT` secret（具有仓库 `contents: write` 权限的 Personal Access Token）。内置 `GITHUB_TOKEN` 推送标签不会触发下游 Release 工作流；没有 PAT 时，Prepare Release 在修改仓库前停止。
4. 已有标签的版本直接在对应 **Release** 运行中重试。构建或上传失败时使用 **Re-run failed jobs**；最终检查发现附件或更新清单不完整时使用 **Re-run all jobs**，重新生成并验证产物。需要修改源码或配置时使用新版本号。Prepare Release 会拒绝再次修改已有标签的版本。

> README 的版本徽章是动态的（shields `github/v/release`），下载文件名由 `scripts/release.sh` 重写。

### 3. 等待构建完成

1. 访问 GitHub Actions: `https://github.com/RustyPiano/ClipMan/actions`
2. 查看 "Release" workflow 运行状态
3. `preflight` 核对版本、发布说明和上一公开版本的更新公钥。
4. `quality` 调用 CI，前端及 macOS/Linux/Windows Rust 检查并行运行，全部通过后才启动四个打包任务。
5. 等待 `verify-release` 通过。该作业核对 `latest.json` 的版本、平台、附件归属和更新签名。失败时保持草稿，处理原因后重新运行 Release。

构建产物（具体的 updater 压缩包/签名后缀随 Tauri 版本变化，以 Draft Release 为准）：

- **macOS (Apple Silicon / Intel)**: `.dmg`, `.app.tar.gz` 及签名
- **Windows**: `.exe`, `.msi` 及 updater 压缩包/签名
- **Linux**: `.deb`, `.rpm`, `.AppImage` 及签名
- **Updater**: `latest.json`

### 4. 编辑 Release 说明

`Release` 的全部作业通过后：

1. 进入 Releases: `https://github.com/RustyPiano/ClipMan/releases`
2. 找到当前标签对应的 Draft release
3. 点击 "Edit draft"
4. 检查自动填入的 `release_notes_<版本号>.md` 内容是否正确
5. 可选: 添加截图或演示 GIF
6. 取消勾选 "Set as a pre-release" (如果这是正式版本)
7. 点击 "Publish release"

`verify-release` 的通过记录是公开草稿前的必要检查。GitHub 的手动发布按钮不会自动受到工作流结果限制。

### 5. 验证发布

发布后检查:

```bash
# 下载并测试安装包
# macOS
VERSION=2.2.1 # 示例：替换为刚发布的版本
curl -fL "https://github.com/RustyPiano/ClipMan/releases/download/v${VERSION}/ClipMan_${VERSION}_aarch64.dmg" -o ClipMan.dmg

# 打开 dmg，将 ClipMan 拖入 Applications 后验证签名
open ClipMan.dmg
codesign -dv --verbose=2 /Applications/ClipMan.app
codesign --verify --strict /Applications/ClipMan.app

# 验证签名（自签名证书；spctl 因未公证会判为 rejected，属正常现象）
# 应显示 Authority=ClipMan Code Signing，且严格验证成功
```

## Release 说明

创建或更新仓库根目录的 `release_notes_<版本号>.md`，例如 `release_notes_1.10.0.md`。Release workflow 会按 tag 自动读取该文件。

## 注意事项

### 发布检查清单

自动化已覆盖的（由 `scripts/release.sh` + CI 保证，无需手动核对）：

- 四个清单文件版本号一致（CI `versions` + release `preflight` 作业强制）
- README 版本徽章（动态）与下载文件名（脚本重写）
- 缺失 `release_notes_<版本>.md` 会让 release 失败，而非发出空说明
- 标签提交使用与 CI 相同的前端、测试类型及 macOS/Linux/Windows Rust 检查
- 新版本沿用上一公开正式版本的更新公钥
- 四个平台的安装包、更新清单、下载附件及更新签名通过统一校验

仍需人工确认：

- [ ] `release_notes_<版本>.md` 内容写实、准确
- [ ] README 的**功能列表**已随新特性更新（版本号是自动的，特性描述不是）
- [ ] 安装包在目标平台实测通过
- [ ] 已知 bug 已在团队当前的跟踪渠道登记

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

本机缺少更新私钥，而 GitHub Secrets 仍保存原私钥时，继续通过 Actions 发布，并从独立备份恢复本机所需的原私钥。更新私钥、密码和对应公钥需要妥善备份。

原私钥彻底丢失时，新密钥签名的更新无法通过既有客户端中的旧公钥验证，旧用户需要手动安装新版本。常规发布会拒绝与上一公开版本不同的公钥；有意更换密钥需要明确安排升级迁移，不能直接覆盖 Secrets 和配置后继续发布。首次发布没有历史公钥可供比较，最终产物仍须通过当前公钥验签。参见 [Tauri 更新签名说明](https://v2.tauri.app/plugin/updater/#signing-updates)。

### 单独验证发布产物

本机安装 `gh`、Python 3 和 `minisign` 后，可以只读验证已经构建的草稿或公开版本。校验草稿时，`gh` 登录账户需要该仓库的推送权限：

```bash
python3 scripts/verify-release.py --repo RustyPiano/ClipMan --tag v2.3.0
```

本地四份版本清单应与指定标签一致。下载和验签文件保存在已忽略的 `src-tauri/target/release-verification/`，脚本不会修改 Release。仅核对与上一公开版本的公钥是否一致时，增加 `--check-key-only`。

如果只验证代码是否能编译，使用：

```bash
bun run build
cd src-tauri && cargo build
```

### 常见问题

**Q: Workflow 构建失败怎么办?**

A: 检查失败作业的 Actions 日志，并在对应 Release 运行中重新运行失败作业。已有标签保持不变。常见原因：

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

## 发布流程涉及的文件

| 文件                                    | 作用                                                                               |
| --------------------------------------- | ---------------------------------------------------------------------------------- |
| `scripts/release.sh`                    | 一键升级四个清单 + README 文件名 + 生成 release notes 模板（纯 sed，无工具链依赖） |
| `scripts/check-versions.sh`             | 断言四个清单版本一致；可选传入期望版本/标签再断言相等                              |
| `scripts/version-utils.sh`             | 共享版本格式、顺序和已有标签检查                                                  |
| `scripts/verify-release.py`            | 校验历史公钥、平台附件、更新清单及更新包签名                                      |
| `.github/workflows/prepare-release.yml` | 检查输入、同步版本、原子推送提交与标签（需 `RELEASE_PAT`）                         |
| `.github/workflows/ci.yml`              | push/PR 与 Release 共用的前端、发布脚本及 macOS/Linux/Windows Rust 检查           |
| `.github/workflows/release.yml`         | 校验发布输入和公钥、调用 CI、构建签名与草稿、统一验证产物                         |
