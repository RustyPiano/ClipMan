# 清理数据库说明

如果你遇到解密错误或其他数据库问题，可以使用以下方法清理：

## 方法 1: 删除数据库文件（推荐）

```bash
# macOS
rm -f ~/Library/Application\ Support/com.clipman.app/clipman.db
rm -f ~/Library/Application\ Support/com.clipman.app/.clipman.key

# Linux
rm -f ~/.local/share/com.clipman.app/clipman.db
rm -f ~/.local/share/com.clipman.app/.clipman.key

# Windows
del "%APPDATA%\com.clipman.app\clipman.db"
del "%APPDATA%\com.clipman.app\.clipman.key"
```

重新启动应用后，会自动创建新的数据库和加密密钥。

## 方法 2: 使用应用内命令（开发中）

在浏览器开发者控制台中执行：

```javascript
await window.__TAURI__.core.invoke('clear_all_history');
```

这会清除所有历史记录，但保留加密密钥。

## 关于解密错误

如果你看到类似这样的警告：

```
⚠️ Failed to decrypt item xxx: Skipping.
```

这是正常的 - 应用会自动跳过无法解密的旧数据。但为了最佳性能，建议清理数据库。
