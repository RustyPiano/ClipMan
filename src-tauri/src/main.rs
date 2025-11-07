// Prevents additional console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod clipboard;
mod storage;
mod crypto;
mod settings;

use clipboard::ClipboardMonitor;
use storage::{ClipStorage, ClipItem, ContentType};
use crypto::Crypto;
use settings::{Settings, SettingsManager};
use tauri::{
    AppHandle, Manager, State, Emitter,
    menu::{MenuBuilder, MenuItemBuilder},
    tray::{TrayIconBuilder, TrayIconEvent, MouseButton, MouseButtonState},
};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};
use tauri_plugin_updater::UpdaterExt;
use std::sync::{Arc, Mutex};
use std::fs;
use std::path::PathBuf;

#[cfg(target_os = "macos")]
use cocoa::appkit::{NSApp, NSApplication, NSApplicationActivationPolicy};

#[cfg(target_os = "macos")]
fn set_activation_policy() {
    unsafe {
        let app = NSApp();
        app.setActivationPolicy_(NSApplicationActivationPolicy::NSApplicationActivationPolicyAccessory);
    }
    log::info!("macOS activation policy set to Accessory (menu bar only)");
}

// 辅助函数：安全获取 Mutex，即使它是 poisoned 状态
fn safe_lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(|poisoned| {
        log::warn!("⚠️ Recovered from poisoned lock");
        poisoned.into_inner()
    })
}

struct AppState {
    storage: Mutex<ClipStorage>,
    monitor: Mutex<Option<ClipboardMonitor>>,
    #[allow(dead_code)] // crypto is used indirectly via storage
    crypto: Arc<Crypto>,
    settings: Arc<SettingsManager>,
    // Track content we just copied to prevent re-capturing
    last_copied_by_us: Arc<Mutex<Option<String>>>,
}

// 密钥管理：生成或加载加密密钥
fn get_or_create_encryption_key(app_data_dir: &PathBuf) -> Result<[u8; 32], String> {
    let key_path = app_data_dir.join(".clipman.key");

    // 尝试加载现有密钥
    if key_path.exists() {
        log::info!("Loading existing encryption key from {:?}", key_path);
        let key_data = fs::read(&key_path)
            .map_err(|e| format!("Failed to read encryption key: {}", e))?;

        if key_data.len() != 32 {
            return Err("Invalid encryption key file".to_string());
        }

        let mut key = [0u8; 32];
        key.copy_from_slice(&key_data);
        return Ok(key);
    }

    // 生成新密钥
    log::info!("Generating new encryption key at {:?}", key_path);
    use ring::rand::{SecureRandom, SystemRandom};

    let rng = SystemRandom::new();
    let mut key = [0u8; 32];
    rng.fill(&mut key)
        .map_err(|e| format!("Failed to generate key: {:?}", e))?;

    // 保存密钥（使用受限权限）
    fs::write(&key_path, &key)
        .map_err(|e| format!("Failed to save encryption key: {}", e))?;

    // 在 Unix 系统上设置文件权限为 0600（仅所有者可读写）
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&key_path)
            .map_err(|e| format!("Failed to get key file metadata: {}", e))?
            .permissions();
        perms.set_mode(0o600);
        fs::set_permissions(&key_path, perms)
            .map_err(|e| format!("Failed to set key file permissions: {}", e))?;
    }

    log::info!("Encryption key generated and saved successfully");
    Ok(key)
}

// 构建动态托盘菜单
fn build_tray_menu(app: &AppHandle) -> Result<tauri::menu::Menu<tauri::Wry>, tauri::Error> {
    let state = app.state::<AppState>();
    let storage = safe_lock(&state.storage);

    let mut menu_builder = MenuBuilder::new(app);

    // 获取置顶项（最多显示 5 个）
    let pinned_items = storage.get_pinned().unwrap_or_default();
    let pinned_count = pinned_items.len().min(5);

    if pinned_count > 0 {
        // 添加置顶标题
        let pinned_header = MenuItemBuilder::with_id("pinned_header", "📌 置顶项").enabled(false).build(app)?;
        menu_builder = menu_builder.item(&pinned_header);

        // 添加置顶项
        for item in pinned_items.iter().take(5) {
            let preview = truncate_content(&item.content, &item.content_type, 50);
            let menu_item = MenuItemBuilder::with_id(
                format!("clip:{}", item.id),
                preview
            ).build(app)?;
            menu_builder = menu_builder.item(&menu_item);
        }

        // 分隔线
        menu_builder = menu_builder.separator();
    }

    // 获取最近项（最多显示 20 个，排除置顶的）
    let recent_items = storage.get_recent(30).unwrap_or_default();
    let recent_unpinned: Vec<_> = recent_items.iter()
        .filter(|item| !item.is_pinned)
        .take(20)
        .collect();

    if !recent_unpinned.is_empty() {
        // 添加历史标题
        let recent_header = MenuItemBuilder::with_id("recent_header", "🕒 最近复制").enabled(false).build(app)?;
        menu_builder = menu_builder.item(&recent_header);

        // 添加最近项
        for item in recent_unpinned {
            let preview = truncate_content(&item.content, &item.content_type, 50);
            let menu_item = MenuItemBuilder::with_id(
                format!("clip:{}", item.id),
                preview
            ).build(app)?;
            menu_builder = menu_builder.item(&menu_item);
        }
    }

    // 底部分隔线和操作按钮
    menu_builder = menu_builder
        .separator()
        .item(&MenuItemBuilder::with_id("clear_non_pinned", "🗑️ 清除非置顶").build(app)?)
        .item(&MenuItemBuilder::with_id("settings", "⚙️ 设置").build(app)?)
        .item(&MenuItemBuilder::with_id("quit", "退出").build(app)?);

    menu_builder.build()
}

// 截断内容用于菜单显示（安全处理 Unicode 字符边界）
fn truncate_content(content: &[u8], content_type: &ContentType, max_len: usize) -> String {
    match content_type {
        ContentType::Text => {
            let text = String::from_utf8_lossy(content);
            let text = text.replace('\n', " ").replace('\r', "");

            // 安全截断：使用字符迭代器而不是字节索引
            let char_count = text.chars().count();
            if char_count > max_len {
                let truncated: String = text.chars().take(max_len).collect();
                format!("{}...", truncated)
            } else {
                text.to_string()
            }
        }
        ContentType::Image => "🖼️ 图片".to_string(),
        ContentType::File => "📎 文件".to_string(),
    }
}

// 更新托盘菜单
pub fn update_tray_menu(app: &AppHandle) {
    if let Ok(new_menu) = build_tray_menu(app) {
        if let Some(tray) = app.tray_by_id("main") {
            if let Err(e) = tray.set_menu(Some(new_menu)) {
                log::error!("Failed to update tray menu: {}", e);
            } else {
                log::debug!("Tray menu updated successfully");
            }
        }
    }
}

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
async fn search_clips(
    state: State<'_, AppState>,
    query: String,
) -> Result<Vec<ClipItem>, String> {
    let storage = safe_lock(&state.storage);
    storage.search(&query)
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
    storage.update_pin(&id, is_pinned)
        .map_err(|e| e.to_string())?;

    // 释放锁后更新托盘菜单
    drop(storage);
    update_tray_menu(&app);

    Ok(())
}

#[tauri::command]
async fn delete_clip(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
) -> Result<(), String> {
    let storage = safe_lock(&state.storage);
    storage.delete(&id)
        .map_err(|e| e.to_string())?;

    // 释放锁后更新托盘菜单
    drop(storage);
    update_tray_menu(&app);

    Ok(())
}

#[tauri::command]
async fn get_pinned_clips(
    state: State<'_, AppState>,
) -> Result<Vec<ClipItem>, String> {
    let storage = safe_lock(&state.storage);
    storage.get_pinned()
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_settings(
    state: State<'_, AppState>,
) -> Result<Settings, String> {
    Ok(state.settings.get())
}

#[tauri::command]
async fn check_clipboard_permission() -> Result<String, String> {
    use arboard::Clipboard;

    match Clipboard::new() {
        Ok(mut clipboard) => {
            match clipboard.get_text() {
                Ok(_) => Ok("granted".to_string()),
                Err(e) => Ok(format!("denied: {}", e)),
            }
        }
        Err(e) => Err(format!("Failed to create clipboard: {}", e)),
    }
}

#[tauri::command]
async fn clear_all_history(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    log::info!("Clearing all clipboard history (user requested)");
    let storage = safe_lock(&state.storage);
    storage.clear_all().map_err(|e| e.to_string())?;

    // 更新托盘菜单
    drop(storage);
    update_tray_menu(&app);

    Ok(())
}

#[tauri::command]
async fn clear_non_pinned_history(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    log::info!("Clearing non-pinned clipboard history (user requested)");
    let storage = safe_lock(&state.storage);
    storage.clear_non_pinned().map_err(|e| e.to_string())?;

    // 更新托盘菜单
    drop(storage);
    update_tray_menu(&app);

    // 发送事件通知前端更新
    if let Err(e) = app.emit("history-cleared", ()) {
        log::error!("Failed to emit history-cleared event: {}", e);
    }

    Ok(())
}

#[tauri::command]
async fn copy_to_system_clipboard(
    state: State<'_, AppState>,
    clip_id: String,
) -> Result<(), String> {
    use arboard::Clipboard;

    let storage = safe_lock(&state.storage);

    // 从数据库获取完整内容
    let items = storage.get_recent(100).map_err(|e| e.to_string())?;
    let item = items.iter()
        .find(|i| i.id == clip_id)
        .ok_or_else(|| "Clip not found".to_string())?;

    // 复制到系统剪切板
    let mut clipboard = Clipboard::new().map_err(|e| e.to_string())?;

    match item.content_type {
        ContentType::Text => {
            let text = String::from_utf8_lossy(&item.content).to_string();

            // Mark this text as "copied by us" so monitor doesn't re-capture it
            let last_copy = state.last_copied_by_us.clone();
            let mut last_copy_guard = last_copy.lock().unwrap();
            *last_copy_guard = Some(text.clone());
            drop(last_copy_guard);

            clipboard.set_text(&text).map_err(|e| e.to_string())?;

            // Schedule clearing the marker after 2 seconds
            let last_copy_clone = last_copy.clone();
            std::thread::spawn(move || {
                std::thread::sleep(std::time::Duration::from_secs(2));
                let mut guard = last_copy_clone.lock().unwrap();
                *guard = None;
            });

            log::info!("✅ Copied text to clipboard from window (length: {})", text.len());
            Ok(())
        }
        ContentType::Image => {
            // TODO: 实现图片复制（需要 ImageData → arboard 转换）
            log::warn!("Image copy from window not yet implemented");
            Err("图片复制功能尚未实现".to_string())
        }
        ContentType::File => {
            log::warn!("File copy not supported");
            Err("文件复制不支持".to_string())
        }
    }
}

#[tauri::command]
async fn check_for_updates(app: AppHandle) -> Result<serde_json::Value, String> {
    log::info!("Checking for updates...");

    // Get current version from package info
    let current_version = app.package_info().version.to_string();

    // Check for updates using Tauri updater
    match app.updater() {
        Ok(updater) => {
            match updater.check().await {
                Ok(update) => {
                    if let Some(update_info) = update {
                        let available_version = update_info.version.clone();
                        log::info!("Update available: {} -> {}", current_version, available_version);

                        Ok(serde_json::json!({
                            "available": true,
                            "current_version": current_version,
                            "latest_version": available_version,
                            "body": update_info.body,
                            "date": update_info.date.map(|d| d.to_string())
                        }))
                    } else {
                        log::info!("No updates available. Current version: {}", current_version);
                        Ok(serde_json::json!({
                            "available": false,
                            "current_version": current_version
                        }))
                    }
                }
                Err(e) => {
                    log::error!("Failed to check for updates: {}", e);
                    Err(format!("Failed to check for updates: {}", e))
                }
            }
        }
        Err(e) => {
            log::error!("Failed to get updater: {}", e);
            Err(format!("Failed to get updater: {}", e))
        }
    }
}

#[tauri::command]
async fn install_update(app: AppHandle) -> Result<(), String> {
    log::info!("Installing update...");

    match app.updater() {
        Ok(updater) => {
            match updater.check().await {
                Ok(update) => {
                    if let Some(update_info) = update {
                        log::info!("Downloading and installing update: {}", update_info.version);

                        // Download and install the update
                        match update_info.download_and_install(|chunk_length, content_length| {
                            if let Some(total) = content_length {
                                let progress = (chunk_length as f64 / total as f64) * 100.0;
                                log::debug!("Download progress: {:.2}%", progress);
                            }
                        }, || {
                            log::info!("Download complete, installing...");
                        }).await {
                            Ok(_) => {
                                log::info!("Update installed successfully. App will restart.");
                                Ok(())
                            }
                            Err(e) => {
                                log::error!("Failed to download/install update: {}", e);
                                Err(format!("Failed to download/install update: {}", e))
                            }
                        }
                    } else {
                        Err("No update available".to_string())
                    }
                }
                Err(e) => {
                    log::error!("Failed to check for updates: {}", e);
                    Err(format!("Failed to check for updates: {}", e))
                }
            }
        }
        Err(e) => {
            log::error!("Failed to get updater: {}", e);
            Err(format!("Failed to get updater: {}", e))
        }
    }
}

#[tauri::command]
async fn update_settings(
    app: AppHandle,
    state: State<'_, AppState>,
    settings: Settings,
) -> Result<(), String> {
    log::info!("Updating settings: {:?}", settings);

    // 检查热键是否改变
    let old_shortcut = state.settings.get().global_shortcut;
    let new_shortcut = settings.global_shortcut.clone();
    let shortcut_changed = old_shortcut != new_shortcut;

    // 更新设置
    state.settings.set_global_shortcut(settings.global_shortcut.clone());
    state.settings.set_max_history_items(settings.max_history_items);
    state.settings.set_auto_cleanup(settings.auto_cleanup);

    // 保存设置
    state.settings.save(&app)?;

    // 如果热键改变，重新注册
    if shortcut_changed {
        log::info!("Hotkey changed from '{}' to '{}', re-registering...", old_shortcut, new_shortcut);

        // 注销旧热键
        if let Err(e) = app.global_shortcut().unregister(old_shortcut.as_str()) {
            log::warn!("Failed to unregister old shortcut '{}': {}", old_shortcut, e);
        }

        // 注册新热键
        let app_clone = app.clone();
        let new_shortcut_clone = new_shortcut.clone();
        app.global_shortcut()
            .on_shortcut(new_shortcut.as_str(), move |_app, _shortcut, event| {
                if event.state == ShortcutState::Pressed {
                    log::info!("Global shortcut triggered: {}", new_shortcut_clone);
                    if let Some(window) = app_clone.get_webview_window("main") {
                        let _ = window.show();
                        let _ = window.set_focus();
                    }
                }
            })
            .map_err(|e| format!("Failed to register new shortcut '{}': {}", new_shortcut, e))?;

        log::info!("Hotkey successfully updated to '{}'", new_shortcut);
    }

    Ok(())
}

fn main() {
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Debug)
        .init();
    log::info!("ClipMan starting...");

    // macOS 权限检查
    #[cfg(target_os = "macos")]
    {
        use arboard::Clipboard;
        log::info!("Running on macOS - checking clipboard access");

        match Clipboard::new() {
            Ok(mut clipboard) => {
                match clipboard.get_text() {
                    Ok(text) => log::info!("✅ Clipboard access OK, current content: {} chars", text.len()),
                    Err(e) => log::warn!("⚠️ Cannot read clipboard: {}. May need accessibility permission.", e),
                }
            }
            Err(e) => log::error!("❌ Failed to create clipboard instance: {}", e),
        }

        // Set activation policy to Accessory (menu bar only, no Dock icon)
        set_activation_policy();
    }

    tauri::Builder::default()
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(|app| {
            // Initialize storage
            let app_data_dir = app.path().app_data_dir()
                .expect("Failed to get app data directory");

            std::fs::create_dir_all(&app_data_dir)
                .expect("Failed to create app data directory");

            // Initialize encryption
            let encryption_key = get_or_create_encryption_key(&app_data_dir)
                .expect("Failed to initialize encryption key");
            let crypto = Arc::new(Crypto::new(&encryption_key));
            log::info!("Encryption initialized");

            let db_path = app_data_dir.join("clipman.db");
            log::info!("Database path: {:?}", db_path);

            let storage = ClipStorage::new(
                db_path.to_str().unwrap(),
                Some(crypto.clone())
            ).expect("Failed to initialize database");

            // Initialize settings
            let settings_manager = Arc::new(SettingsManager::new());
            if let Err(e) = settings_manager.load(&app.handle()) {
                log::warn!("Failed to load settings, using defaults: {}", e);
            }
            log::info!("Settings initialized");

            let last_copied_by_us = Arc::new(Mutex::new(None));

            let app_state = AppState {
                storage: Mutex::new(storage),
                monitor: Mutex::new(None),
                crypto: crypto.clone(),
                settings: settings_manager.clone(),
                last_copied_by_us: last_copied_by_us.clone(),
            };

            app.manage(app_state);

            // Build initial tray menu
            let menu = build_tray_menu(&app.handle())?;

            // Create system tray with ID
            let tray_id = "main";
            let _tray = TrayIconBuilder::with_id(tray_id)
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .on_menu_event(move |app, event| {
                    let event_id = event.id().as_ref();
                    log::debug!("Menu event: {}", event_id);

                    match event_id {
                        "quit" => {
                            log::info!("Quit menu clicked");
                            app.exit(0);
                        }
                        "clear_non_pinned" => {
                            log::info!("Clear non-pinned menu clicked");
                            let app_clone = app.clone();
                            tauri::async_runtime::spawn(async move {
                                if let Err(e) = clear_non_pinned_history(app_clone.clone(), app_clone.state()).await {
                                    log::error!("Failed to clear non-pinned history: {}", e);
                                }
                            });
                        }
                        "settings" => {
                            log::info!("Settings menu clicked");
                            // TODO: 打开设置窗口
                            if let Some(window) = app.get_webview_window("main") {
                                let _ = window.show();
                                let _ = window.set_focus();
                            }
                        }
                        id if id.starts_with("clip:") => {
                            // 提取剪切板项 ID 并复制内容
                            let clip_id = id.strip_prefix("clip:").unwrap();
                            log::info!("Clip item clicked: {}", clip_id);

                            if let Err(e) = copy_clip_to_clipboard(app, clip_id) {
                                log::error!("Failed to copy clip: {}", e);
                            }
                        }
                        _ => {
                            log::debug!("Unhandled menu event: {}", event_id);
                        }
                    }
                })
                .on_tray_icon_event(|_tray, event| {
                    // 左键点击时手动显示菜单（Tauri 2.0 中菜单会自动显示，这里仅记录日志）
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        log::debug!("Tray left-clicked - menu will show automatically");
                    }
                })
                .build(app)?;

            log::info!("System tray initialized");

            // Setup window close handler to hide instead of quit
            if let Some(window) = app.get_webview_window("main") {
                let window_clone = window.clone();
                window.on_window_event(move |event| {
                    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                        // Prevent window from closing, hide it instead
                        api.prevent_close();
                        let _ = window_clone.hide();
                        log::debug!("Window hidden instead of closed");
                    }
                });
                log::info!("Window close handler registered");
            }

            // Start clipboard monitoring
            let app_handle = app.handle().clone();
            let state: State<AppState> = app_handle.state();

            let monitor = ClipboardMonitor::new(app_handle.clone(), last_copied_by_us.clone());
            monitor.start();

            *safe_lock(&state.monitor) = Some(monitor);

            log::info!("Clipboard monitoring started");

            // Register global shortcuts from settings
            let state: State<AppState> = app_handle.state();
            let current_shortcut = state.settings.get().global_shortcut;

            let app_handle_hotkey = app.handle().clone();
            let shortcut_display = current_shortcut.clone();
            let shortcut_str = current_shortcut.clone();
            app.global_shortcut().on_shortcut(current_shortcut.as_str(), move |_app, _shortcut, event| {
                if event.state == ShortcutState::Pressed {
                    log::info!("Global shortcut triggered: {}", shortcut_display);

                    // Show main window
                    if let Some(window) = app_handle_hotkey.get_webview_window("main") {
                        let _ = window.show();
                        let _ = window.set_focus();
                    }
                }
            }).map_err(|e| {
                log::error!("Failed to register global shortcut '{}': {}", shortcut_str, e);
                e
            })?;

            log::info!("Global shortcuts registered: {}", shortcut_str);

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_clipboard_history,
            search_clips,
            toggle_pin,
            delete_clip,
            get_pinned_clips,
            get_settings,
            update_settings,
            check_clipboard_permission,
            clear_all_history,
            clear_non_pinned_history,
            copy_to_system_clipboard,
            check_for_updates,
            install_update
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

// 复制剪切板项到系统剪切板
fn copy_clip_to_clipboard(app: &AppHandle, clip_id: &str) -> Result<(), String> {
    use arboard::Clipboard;

    let state = app.state::<AppState>();
    let storage = safe_lock(&state.storage);

    // 从数据库获取完整内容
    let items = storage.get_recent(100).map_err(|e| e.to_string())?;
    let item = items.iter()
        .find(|i| i.id == clip_id)
        .ok_or_else(|| "Clip not found".to_string())?;

    // 复制到系统剪切板
    let mut clipboard = Clipboard::new().map_err(|e| e.to_string())?;

    match item.content_type {
        ContentType::Text => {
            let text = String::from_utf8_lossy(&item.content).to_string();

            // Mark this text as "copied by us" so monitor doesn't re-capture it
            {
                let mut last_copied = safe_lock(&state.last_copied_by_us);
                *last_copied = Some(text.clone());
            }

            clipboard.set_text(text.clone()).map_err(|e| e.to_string())?;
            log::info!("✅ Copied text to clipboard: {} chars (marked as self-copy)", text.len());

            // Clear the marker after 2 seconds using std::thread (not tokio)
            let last_copied_by_us = state.last_copied_by_us.clone();
            std::thread::spawn(move || {
                std::thread::sleep(std::time::Duration::from_secs(2));
                let mut last_copied = safe_lock(&last_copied_by_us);
                *last_copied = None;
                log::debug!("🧹 Cleared self-copy marker");
            });
        }
        ContentType::Image => {
            // TODO: 实现图片复制
            log::warn!("Image copy not yet implemented");
            return Err("图片复制功能开发中".to_string());
        }
        ContentType::File => {
            log::warn!("File copy not yet implemented");
            return Err("文件复制功能开发中".to_string());
        }
    }

    Ok(())
}
