# ClipMan v2.1.0

## New
- **Auto-paste now recovers from a lost Accessibility permission.** On macOS, the permission that lets ClipMan paste for you (Cmd+V) can become invalid — most often after an app update. ClipMan now detects this and guides you to re-grant it (a prompt that can open System Settings, plus a banner in the QuickBar), instead of silently failing to paste. Your copied content still reaches the clipboard, so you can always paste manually.

## Improvements
- **macOS builds are now code-signed**, so the Accessibility permission you grant persists across updates — you no longer have to re-authorize ClipMan every time it updates.

## Upgrading from an older version
- The first launch may show a Gatekeeper “unverified developer” prompt — right-click the app and choose **Open**, or allow it under **System Settings → Privacy & Security**.
- Because older builds were unsigned, you'll need to grant Accessibility permission one more time after this update. From then on it stays granted.

---

# ClipMan v2.1.0 更新日志

## 新功能
- **自动粘贴会在无障碍权限失效时引导你重新授权。** 在 macOS 上，ClipMan 替你粘贴（Cmd+V）所需的“辅助功能”权限可能失效——最常见于应用更新之后。现在 ClipMan 会检测到这种情况并引导你重新授权（弹窗可一键打开系统设置，QuickBar 顶部也会显示提示横幅），而不再静默地粘贴失败。其间复制的内容仍会进入剪贴板，你随时可以手动粘贴。

## 改进
- **macOS 安装包现已代码签名**，因此你授予的“辅助功能”权限会在各次更新间保留——不必每次更新后都重新授权。

## 从旧版本升级
- 首次打开可能出现 Gatekeeper“无法验证开发者”的提示——右键点击应用选择**打开**，或在 **系统设置 → 隐私与安全性** 中允许。
- 由于旧版本未签名，本次更新后需要再授予一次“辅助功能”权限；此后即长期有效。
