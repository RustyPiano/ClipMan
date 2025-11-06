# ClipMan 架构设计文档

## 概述

ClipMan 采用经典的 C/S 架构，使用 Rust 构建高性能后端，Svelte 5 构建现代化前端，通过 Tauri 2.0 IPC 通信。

```
┌─────────────────────────────────────────────────────────────┐
│                         ClipMan                              │
├─────────────────────────────────────────────────────────────┤
│                                                               │
│  ┌───────────────────┐         ┌───────────────────────┐   │
│  │   Svelte 5 UI     │◄───IPC─►│   Rust Backend        │   │
│  │                   │         │                       │   │
│  │  • Components     │         │  • Clipboard Monitor  │   │
│  │  • Stores (Runes) │         │  • Storage Layer      │   │
│  │  • Router         │         │  • Crypto Module      │   │
│  └───────────────────┘         │  • Settings Manager   │   │
│                                 └───────────────────────┘   │
│                                           ▲                  │
│                                           │                  │
│                                 ┌─────────▼──────────┐      │
│                                 │  SQLite Database   │      │
│                                 │  (Encrypted FTS5)  │      │
│                                 └────────────────────┘      │
└─────────────────────────────────────────────────────────────┘
```

## 技术选型理由

### 后端：Rust

**为什么选择 Rust？**
1. **内存安全**: 零成本抽象，编译时保证内存安全
2. **高性能**: 接近 C/C++ 的性能，适合剪切板实时监控
3. **并发安全**: Ownership 系统防止数据竞争
4. **生态成熟**: arboard、rusqlite、ring 等库质量高

**关键 crates**:
- `tauri` - 桌面应用框架
- `arboard` - 跨平台剪切板访问
- `rusqlite` - SQLite 绑定
- `ring` - 密码学库
- `tokio` - 异步运行时（Tauri 需要）

### 前端：Svelte 5

**为什么选择 Svelte 5？**
1. **编译时优化**: 无虚拟 DOM，运行时体积小
2. **Runes API**: 更直观的响应式编程（vs React Hooks）
3. **性能**: 比 React/Vue 更快的渲染
4. **学习曲线**: 语法简洁，接近原生 HTML/JS

**Svelte 5 Runes 示例**:
```typescript
// 传统 Svelte (v4)
let count = 0;
$: doubled = count * 2;

// Svelte 5 Runes
let count = $state(0);
let doubled = $derived(count * 2);
```

## 核心模块设计

### 1. Clipboard Monitor (`clipboard.rs`)

**职责**: 实时监控系统剪切板变化

**设计模式**: Observer Pattern

```rust
pub struct ClipboardMonitor {
    app_handle: AppHandle,
    last_copied_by_us: Arc<Mutex<Option<String>>>,
}

impl ClipboardMonitor {
    pub fn start(&self) {
        // 启动独立线程，500ms 轮询
        thread::spawn(move || {
            loop {
                // 1. 检测变化
                // 2. 跳过自己的复制
                // 3. 保存到数据库
                // 4. 发送事件到前端
                thread::sleep(Duration::from_millis(500));
            }
        });
    }
}
```

**关键设计决策**:

1. **为什么轮询而不是事件监听？**
   - arboard 不支持事件监听
   - 轮询开销小（500ms 一次）
   - 跨平台兼容性好

2. **如何避免重复捕获自己的复制？**
   ```rust
   // 使用共享状态标记
   let last_copied_by_us = Arc<Mutex<Option<String>>>;

   // 复制时标记
   *last_copied_by_us.lock() = Some(text.clone());

   // 监控时跳过
   if last_copied == text { skip(); }
   ```

3. **为什么创建缩略图？**
   - 原图太大（几 MB），数据库会膨胀
   - 256x256 缩略图足够预览
   - 使用 Lanczos3 保持质量

### 2. Storage Layer (`storage.rs`)

**职责**: SQLite 数据库 CRUD + FTS5 全文搜索

**设计模式**: Repository Pattern

```rust
pub struct ClipStorage {
    conn: Connection,
    crypto: Option<Arc<Crypto>>,
}

impl ClipStorage {
    // CRUD 操作
    pub fn insert(&self, item: &ClipItem) -> Result<()>;
    pub fn get_recent(&self, limit: usize) -> Result<Vec<ClipItem>>;
    pub fn get_pinned(&self) -> Result<Vec<ClipItem>>;
    pub fn delete(&self, id: &str) -> Result<()>;

    // 搜索
    pub fn search(&self, query: &str) -> Result<Vec<ClipItem>>;

    // 置顶管理
    pub fn update_pin(&self, id: &str, is_pinned: bool) -> Result<()>;
}
```

**数据库 Schema**:

```sql
-- 主表
CREATE TABLE clips (
    id TEXT PRIMARY KEY,
    content BLOB NOT NULL,           -- AES-256 加密
    content_type TEXT NOT NULL,      -- 'text' | 'image' | 'file'
    timestamp INTEGER NOT NULL,
    is_pinned INTEGER DEFAULT 0,
    pin_order INTEGER
);

-- FTS5 虚拟表（全文搜索）
CREATE VIRTUAL TABLE clips_fts
USING fts5(id, content_text, content='clips', content_rowid=rowid);

-- 自动同步触发器
CREATE TRIGGER clips_ai AFTER INSERT ON clips BEGIN
    INSERT INTO clips_fts(rowid, id, content_text)
    VALUES (new.rowid, new.id, CASE
        WHEN new.content_type = 'text' THEN new.content
        ELSE ''
    END);
END;

-- 索引
CREATE INDEX idx_timestamp ON clips(timestamp DESC);
CREATE INDEX idx_pinned ON clips(is_pinned, pin_order);
```

**关键设计决策**:

1. **为什么使用 FTS5？**
   - 支持中文分词（jieba tokenizer）
   - 比 LIKE 查询快 10-100 倍
   - SQLite 内置，无额外依赖

2. **如何处理加密？**
   ```rust
   // 插入时加密
   let content_to_store = if let Some(crypto) = &self.crypto {
       crypto.encrypt(&item.content)?
   } else {
       item.content.clone()
   };

   // 读取时解密
   let content = match self.decrypt_content(encrypted) {
       Ok(c) => c,
       Err(e) => {
           log::warn!("Decrypt failed, skipping");
           Vec::new()  // 返回空，稍后过滤
       }
   };
   ```

3. **如何避免解密错误导致崩溃？**
   - 使用 `filter_map` 过滤无效项
   - 记录警告但不传播错误
   - 让用户可以清理旧数据

### 3. Crypto Module (`crypto.rs`)

**职责**: AES-256-GCM 加密/解密

**算法选择**: AES-256-GCM
- **AES-256**: NIST 标准，量子计算抵抗
- **GCM 模式**: 提供认证（防篡改）
- **96-bit Nonce**: 随机生成，每次加密不同

```rust
pub struct Crypto {
    key: [u8; 32],  // 256-bit key
}

impl Crypto {
    pub fn encrypt(&self, data: &[u8]) -> Result<Vec<u8>> {
        let nonce = generate_nonce();  // 96-bit random
        let mut encrypted = Vec::new();

        // AEAD encryption
        let sealing_key = SealingKey::new(&self.key)?;
        let tag = sealing_key.seal_in_place_separate_tag(
            Nonce::assume_unique_for_key(nonce),
            Aad::empty(),
            &mut encrypted
        )?;

        // Format: nonce(12) + ciphertext + tag(16)
        Ok([nonce.to_vec(), encrypted, tag.to_vec()].concat())
    }

    pub fn decrypt(&self, data: &[u8]) -> Result<Vec<u8>> {
        let (nonce, rest) = data.split_at(12);
        let (ciphertext, tag) = rest.split_at(rest.len() - 16);

        let opening_key = OpeningKey::new(&self.key)?;
        opening_key.open_in_place(
            Nonce::assume_unique_for_key(nonce),
            Aad::empty(),
            ciphertext,
            tag
        )
    }
}
```

**密钥管理**:

```rust
// 首次启动生成，永久保存
fn get_or_create_encryption_key(app_data_dir: &PathBuf) -> Result<[u8; 32]> {
    let key_path = app_data_dir.join(".clipman.key");

    if key_path.exists() {
        // 加载现有密钥
        let key_data = fs::read(&key_path)?;
        Ok(key_data.try_into()?)
    } else {
        // 生成新密钥
        let rng = SystemRandom::new();
        let mut key = [0u8; 32];
        rng.fill(&mut key)?;

        // 保存（权限 0600）
        fs::write(&key_path, &key)?;
        #[cfg(unix)]
        fs::set_permissions(&key_path, Permissions::from_mode(0o600))?;

        Ok(key)
    }
}
```

### 4. Tauri IPC Commands (`main.rs`)

**职责**: 前后端通信桥梁

```rust
#[tauri::command]
async fn get_clipboard_history(
    state: State<'_, AppState>,
    limit: Option<usize>,
) -> Result<Vec<ClipItem>, String> {
    let storage = safe_lock(&state.storage);
    storage.get_recent(limit.unwrap_or(100))
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn toggle_pin(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
    is_pinned: bool,
) -> Result<(), String> {
    let storage = safe_lock(&state.storage);
    storage.update_pin(&id, is_pinned)?;

    // 更新托盘菜单
    drop(storage);
    update_tray_menu(&app);

    Ok(())
}
```

**注册命令**:
```rust
fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            get_clipboard_history,
            search_clips,
            toggle_pin,
            delete_clip,
            get_pinned_clips,
            get_settings,
            update_settings,
            check_clipboard_permission,
            clear_all_history
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

### 5. Frontend State Management (`clipboard.svelte.ts`)

**职责**: 前端状态管理 + 后端通信

**Svelte 5 Runes 模式**:

```typescript
class ClipboardStore {
  // 响应式状态
  items = $state<ClipItem[]>([]);
  searchQuery = $state('');
  isLoading = $state(false);

  // 派生状态（自动重新计算）
  pinnedItems = $derived(
    this.items
      .filter(item => item.isPinned)
      .sort((a, b) => (a.pinOrder || 0) - (b.pinOrder || 0))
  );

  filteredItems = $derived.by(() => {
    if (!this.searchQuery) return this.items;
    return this.items.filter(item => {
      const text = new TextDecoder().decode(item.content);
      return text.toLowerCase().includes(this.searchQuery.toLowerCase());
    });
  });

  async loadHistory() {
    this.isLoading = true;
    try {
      const history = await invoke<ClipItem[]>('get_clipboard_history', {
        limit: 100,
      });
      this.items = history;
    } finally {
      this.isLoading = false;
    }
  }

  async togglePin(id: string) {
    const item = this.items.find(i => i.id === id);
    if (!item) return;

    await invoke('toggle_pin', { id, isPinned: !item.isPinned });
    item.isPinned = !item.isPinned;
    await this.loadHistory();  // 重新加载获取新的 pin_order
  }
}

// 导出单例
export const clipboardStore = new ClipboardStore();
```

**事件监听**:
```typescript
// 监听后端发送的剪切板变化事件
this.unlisten = await listen<ClipItem>('clipboard-changed', (event) => {
  // 添加新项到列表开头
  this.items = [event.payload, ...this.items];
});
```

## 系统托盘设计

**动态菜单构建**:

```rust
fn build_tray_menu(app: &AppHandle) -> Result<Menu> {
    let storage = safe_lock(&state.storage);
    let mut menu_builder = MenuBuilder::new(app);

    // 1. 置顶项区域（最多 5 个）
    let pinned_items = storage.get_pinned()?;
    if !pinned_items.is_empty() {
        menu_builder = menu_builder
            .item(&MenuItemBuilder::with_id("pinned_header", "📌 置顶项")
                .enabled(false).build(app)?)

        for item in pinned_items.iter().take(5) {
            let preview = truncate_content(&item.content, 50);
            menu_builder = menu_builder.item(&MenuItemBuilder::with_id(
                format!("clip:{}", item.id),
                preview
            ).build(app)?);
        }

        menu_builder = menu_builder.separator();
    }

    // 2. 最近项区域（最多 10 个，排除置顶）
    let recent_items = storage.get_recent(15)?;
    let recent_unpinned: Vec<_> = recent_items.iter()
        .filter(|item| !item.is_pinned)
        .take(10)
        .collect();

    if !recent_unpinned.is_empty() {
        menu_builder = menu_builder
            .item(&MenuItemBuilder::with_id("recent_header", "🕒 最近复制")
                .enabled(false).build(app)?);

        for item in recent_unpinned {
            let preview = truncate_content(&item.content, 50);
            menu_builder = menu_builder.item(&MenuItemBuilder::with_id(
                format!("clip:{}", item.id),
                preview
            ).build(app)?);
        }
    }

    // 3. 底部操作
    menu_builder
        .separator()
        .item(&MenuItemBuilder::with_id("settings", "⚙️ 设置").build(app)?)
        .item(&MenuItemBuilder::with_id("quit", "退出").build(app)?)
        .build()
}
```

**点击处理**:
```rust
.on_menu_event(move |app, event| {
    match event.id().as_ref() {
        "quit" => app.exit(0),
        "settings" => {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.set_focus();
            }
        }
        id if id.starts_with("clip:") => {
            let clip_id = id.strip_prefix("clip:").unwrap();
            copy_clip_to_clipboard(app, clip_id);
        }
        _ => {}
    }
})
```

## 错误处理策略

### Poisoned Lock 恢复

**问题**: 线程 panic 导致 Mutex 被污染

**解决方案**:
```rust
fn safe_lock<T>(mutex: &Mutex<T>) -> MutexGuard<T> {
    mutex.lock().unwrap_or_else(|poisoned| {
        log::warn!("⚠️ Recovered from poisoned lock");
        poisoned.into_inner()  // 恢复数据
    })
}
```

### 解密错误处理

**问题**: 旧数据无法解密会导致查询失败

**解决方案**:
```rust
let content = match self.decrypt_content(encrypted) {
    Ok(c) => c,
    Err(e) => {
        log::warn!("⚠️ Failed to decrypt item {}: {:?}. Skipping.", id, e);
        Vec::new()
    }
};

// 过滤空内容
items.filter_map(|item| {
    match item {
        Ok(clip_item) if !clip_item.content.is_empty() => Some(Ok(clip_item)),
        Ok(_) => None,  // 跳过解密失败的项
        Err(e) => Some(Err(e)),
    }
}).collect()
```

### Unicode 安全截断

**问题**: 字节索引截断中文会 panic

**解决方案**:
```rust
// ❌ 错误：字节索引
&text[..50]  // panic if '到' at byte 48-51

// ✅ 正确：字符迭代器
text.chars().take(50).collect::<String>()
```

## 性能优化

1. **数据库索引**:
   ```sql
   CREATE INDEX idx_timestamp ON clips(timestamp DESC);
   CREATE INDEX idx_pinned ON clips(is_pinned, pin_order);
   ```

2. **FTS5 全文搜索**: 比 LIKE 快 10-100 倍

3. **图像缩略图**: 256x256 而不是原图

4. **增量更新**: 只重新加载必要的数据

5. **惰性加载**: 首次只加载 100 条

## 安全性考虑

1. **加密存储**: 所有内容 AES-256-GCM 加密
2. **密钥权限**: Unix 系统设置 0600
3. **本地存储**: 数据不出本地
4. **无网络请求**: 完全离线运行
5. **权限最小化**: 只请求必要权限

## macOS 特殊处理

1. **Activation Policy**:
   ```rust
   #[cfg(target_os = "macos")]
   unsafe {
       let app = NSApp();
       app.setActivationPolicy_(
           NSApplicationActivationPolicyAccessory
       );
   }
   ```

2. **权限检查 UI**: PermissionCheck.svelte 组件

3. **模板图标**: `iconAsTemplate: true`

## 未来优化方向

1. **增量同步**: 避免全量加载
2. **虚拟滚动**: 大量历史项时优化渲染
3. **WebWorker**: 前端搜索卸载到 Worker
4. **Lazy Loading**: 图像按需加载
5. **索引优化**: 复合索引优化查询

## 总结

ClipMan 采用模块化、分层设计：
- **后端**: Rust 保证性能和安全
- **前端**: Svelte 5 提供现代化 UI
- **通信**: Tauri IPC 高效桥接
- **存储**: SQLite + 加密保证数据安全
- **托盘**: 原生菜单提供便捷访问

整体架构清晰、职责分明、易于维护和扩展。
