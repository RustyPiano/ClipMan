# 0023 更名为 ClipLeaf

- 状态：已采纳（v2.5.1 改产品名与仓库，v2.5.2 改其余标识）

## 决定

- 产品名、界面文字、安装包文件名、GitHub 仓库（`RustyPiano/ClipLeaf`）、bundle identifier（`app.clipleaf.desktop`）、可执行文件名（`clipleaf`）、Linux 包名（`clip-leaf`）、数据库文件名（`clipleaf.db`）和 macOS 签名证书（`ClipLeaf Code Signing`）全部使用 ClipLeaf。
- identifier 变化后数据目录随之变化。`src-tauri/src/identity.rs` 在 Tauri 创建窗口之前，把旧 identifier `com.clipman.manager` 下的应用数据目录、本地数据目录（Windows WebView2 数据）和 macOS `~/Library/WebKit` 目录移到新 identifier 下；应用数据目录在目标已存在时逐项合并，同名项冲突则报错退出、不覆盖任何文件；WebView 数据目录只保存主题等界面状态，目标已存在或移动失败时记录日志并保留旧目录，不阻止启动（macOS 的 WebKit 和系统偏好在应用启动时就可能按新 identifier 建好目录）。`settings.json` 中位于旧目录内的自定义数据路径同步改写。打开数据库前把 `clipman.db` 及其 `-wal`/`-shm`/`-journal` 改名为 `clipleaf.db`，两者同时存在时报错。
- macOS 辅助功能授权绑定 identifier 和签名证书，identifier 变更后无论如何都要重新授权一次，因此证书在同一版本一起更换。新证书与旧证书参数相同（RSA 2048、10 年、Code Signing EKU），生成参数和文件在仓库根目录被忽略的 `ClipLeaf-signing/`。
- macOS 应用在 `ClipMan.app` 中启动时，先把应用包改名为同目录下的 `ClipLeaf.app`，从新路径重新启动自身后退出。改名失败（例如目录无写权限）只记录日志，不影响使用。
- 设置启用了开机自启时，完成数据迁移的首次启动按当前可执行文件路径重新登记自启项，并删除名为 `ClipMan` 的旧自启项。
- Windows MSI 固定 `upgradeCode` 为旧产品名推导出的 `37b0ab03-ac3b-5161-9116-4ffd3268dc29`，否则新 MSI 会与旧版并存。Windows NSIS 安装前通过 `src-tauri/windows/hooks.nsh` 静默卸载登记在 `Software\clipman\ClipMan` 和 `Software\clipman\ClipLeaf` 下的旧安装（保留应用数据），旧版启用了开机自启时为新安装写入 `Run` 项。
- Linux deb 声明 `Conflicts`/`Replaces: clip-man`，rpm 声明 `Obsoletes: clip-man`。两种包都保留 `/usr/bin/clipman` 转发脚本，旧版写入的自启项和更新后的重启仍能启动应用。
- 旧版本遗留的历史名称只在迁移代码中出现：`com.clipman.manager`、`clipman.db`、`.clipman.key`（v1 之前的加密密钥）、自启项名称 `ClipMan` 和 NSIS 注册表路径。
