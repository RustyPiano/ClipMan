# 0022 发布与 CI 流水线

- 状态：已采纳

## 决定

- macOS 版本用自签名证书签名。没有 Apple Developer 账号也能让签名要求（Designated Requirement）在各版本间保持不变，用户只需授予一次辅助功能权限。每次发布必须使用同一张证书，否则所有用户更新后都要重新授权。应用没有公证，首次打开仍有一次 Gatekeeper 提示。
- 工作流自己建立 keychain，并把证书信任为代码签名根，不交给 tauri-action 导入：自签名证书默认不受信任，会被 `security find-identity -v -p codesigning` 过滤掉，Tauri bundler 因此报 "failed to resolve signing identity"。tauri-action 不设置 `APPLE_CERTIFICATE`，bundler 从默认 keychain 取得签名身份。
- tauri-action 固定在 `dev` 分支 2026-07-07 的提交 SHA，冻结发布使用的 action 代码。新版本在这条发布流程上验证后再有意升级，最好升级到正式 tag。
- Release 工作流的 concurrency 不取消进行中的构建：重新推送同一标签时让已有构建完成；不同标签的 ref 不同，不会共用一组。
- `releaseName` 只用版本号，保持 Tauri updater 兼容。
- Prepare Release 用 `RELEASE_PAT` 推送：内置 `GITHUB_TOKEN` 推送的标签不会触发 Release 工作流。没有 PAT 时在修改仓库前停止。发布说明必须事先提交，工作流不生成模板，避免发出空说明。
- `cargo test` 和 `cargo clippy` 会构建 Tauri 应用，需要 `dist/` 存在，CI 的 Rust 作业先构建前端。
- macOS 的 Rust 作业不编译 `#[cfg(windows)]` 的剪贴板和窗口 FFI，所以单独在 Windows runner 上运行 clippy 和测试。MSVC 目标无法从 macOS 交叉编译，因为 rusqlite 内置的 SQLite（C 代码）需要 MSVC 工具链。
- 依赖不生成调试信息（`[profile.dev.package."*"] debug = false`），`target/debug` 约减小一半；本 crate 保留完整调试信息，便于断点调试。
- `scripts/release.sh` 用锚定的正则改版本号：package.json 和 tauri.conf.json 只改两格缩进的顶层 `"version"`，Cargo.toml 只改从第 0 列开始的 `version =`，Cargo.lock 只改 clipman 包名的下一行，不碰依赖的版本。README 下载文件名里的版本整体重写，README 和清单已经不一致时也能修正；版本徽章是动态的，不需要修改。原地修改文件的 sed 写法同时兼容 GNU 和 BSD。
- `scripts/check-versions.sh` 只用 grep 和 sed，在 macOS 和 Ubuntu runner 上行为一致。
- `scripts/verify-release.py` 对每个附件只验签一次，默认平台条目和安装格式条目可能指向同一个附件。附件被替换后 ID 会改变，已下载的内容仍要经过验签。
- `tauri-action` 在 `releaseDraft: true` 阶段生成的 `latest.json` 会把平台下载地址写成 GitHub REST API 路径（`https://api.github.com/repos/.../releases/assets/<id>`）。草稿公开后，客户端无 token 请求该地址容易触发 60 次/小时的匿名 API 限流（403）；公开正式版后须将 `latest.json` 中的各平台 `url` 重写为公开直链（`https://github.com/<repo>/releases/download/<tag>/<asset>`）并重新执行 `verify-release.py`。
