# 0023 更名为 ClipLeaf

- 状态：已采纳

## 决定

- 产品名（`productName`）、界面文字、安装包文件名、Linux 包名（`clip-leaf`）和 GitHub 仓库（`RustyPiano/ClipLeaf`）改用 ClipLeaf。
- 以下标识保持原值，因为它们决定已安装用户的数据和授权：bundle identifier `com.clipman.manager`（数据目录、设置、单实例、macOS 辅助功能授权）、数据库文件名 `clipman.db`、macOS 签名证书 `ClipMan Code Signing`（Designated Requirement）、可执行文件名 `clipman`（旧版写入的开机自启项和 Linux `/usr/bin` 路径指向它）。
- Windows MSI 固定 `upgradeCode` 为旧产品名推导出的 `37b0ab03-ac3b-5161-9116-4ffd3268dc29`。Tauri 默认用 `productName` 生成该值，不固定时新 MSI 会与旧版并存安装，不会升级旧版。
- Windows NSIS 安装前通过 `src-tauri/windows/hooks.nsh` 静默卸载旧的 `ClipMan` 安装（保留应用数据），旧版启用了开机自启时为新安装写入同等的 `Run` 项。
- Linux deb 声明 `Conflicts`/`Replaces: clip-man`，rpm 声明 `Obsoletes: clip-man`，更新时替换旧包。
- 自启项以产品名命名（macOS LaunchAgent、Windows `Run` 值、Linux autostart `.desktop`）。设置中启用了自启时，启动时删除名为 `ClipMan` 的旧自启项并以新名称重新登记。
- macOS 自动更新在原路径原地替换应用包，已安装用户的 `/Applications/ClipMan.app` 文件名保持不变，重新下载安装后才是 `ClipLeaf.app`。
