// Prevents additional console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod clipboard;
mod storage;
mod crypto;

use clipboard::ClipboardMonitor;
use storage::{ClipStorage, ClipItem, ContentType};
use crypto::Crypto;
use tauri::{
    AppHandle, Manager, State,
    menu::{MenuBuilder, MenuItemBuilder, PredefinedMenuItem},
    tray::{TrayIconBuilder, TrayIconEvent, MouseButton, MouseButtonState},
};
use std::sync::{Arc, Mutex};
use std::fs;
use std::path::PathBuf;

struct AppState {
    storage: Mutex<ClipStorage>,
    monitor: Mutex<Option<ClipboardMonitor>>,
    crypto: Arc<Crypto>,
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
    let storage = state.storage.lock().unwrap();

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

    // 获取最近项（最多显示 10 个，排除置顶的）
    let recent_items = storage.get_recent(Some(15)).unwrap_or_default();
    let recent_unpinned: Vec<_> = recent_items.iter()
        .filter(|item| !item.is_pinned)
        .take(10)
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
        .item(&MenuItemBuilder::with_id("settings", "⚙️ 设置").build(app)?)
        .item(&MenuItemBuilder::with_id("quit", "退出").build(app)?);

    menu_builder.build()
}

// 截断内容用于菜单显示
fn truncate_content(content: &[u8], content_type: &ContentType, max_len: usize) -> String {
    match content_type {
        ContentType::Text => {
            let text = String::from_utf8_lossy(content);
            let text = text.replace('\n', " ").replace('\r', "");
            if text.len() > max_len {
                format!("{}...", &text[..max_len])
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
    let storage = state.storage.lock().map_err(|e| e.to_string())?;
    storage.get_recent(limit.unwrap_or(100))
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn search_clips(
    state: State<'_, AppState>,
    query: String,
) -> Result<Vec<ClipItem>, String> {
    let storage = state.storage.lock().map_err(|e| e.to_string())?;
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
    let storage = state.storage.lock().map_err(|e| e.to_string())?;
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
    let storage = state.storage.lock().map_err(|e| e.to_string())?;
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
    let storage = state.storage.lock().map_err(|e| e.to_string())?;
    storage.get_pinned()
        .map_err(|e| e.to_string())
}

fn main() {
    env_logger::init();
    log::info!("ClipMan starting...");

    tauri::Builder::default()
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
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

            let app_state = AppState {
                storage: Mutex::new(storage),
                monitor: Mutex::new(None),
                crypto: crypto.clone(),
            };

            app.manage(app_state);

            // Build initial tray menu
            let menu = build_tray_menu(&app.handle())?;

            // Create system tray
            let _tray = TrayIconBuilder::new()
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
                .on_tray_icon_event(|tray, event| {
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
                .id("main") // 设置 ID 以便后续更新菜单
                .build(app)?;

            log::info!("System tray initialized");

            // Start clipboard monitoring
            let app_handle = app.handle().clone();
            let state: State<AppState> = app_handle.state();

            let monitor = ClipboardMonitor::new(app_handle.clone());
            monitor.start();

            *state.monitor.lock().unwrap() = Some(monitor);

            log::info!("Clipboard monitoring started");

            // Register global shortcuts
            use tauri_plugin_global_shortcut::{Code, Modifiers, ShortcutState};

            let app_handle_hotkey = app.handle().clone();
            app.global_shortcut().on_shortcut("CommandOrControl+Shift+V", move |_app, _shortcut, event| {
                if event.state == ShortcutState::Pressed {
                    log::info!("Global shortcut triggered: Ctrl+Shift+V");

                    // Show main window
                    if let Some(window) = app_handle_hotkey.get_webview_window("main") {
                        let _ = window.show();
                        let _ = window.set_focus();
                    }
                }
            }).map_err(|e| {
                log::error!("Failed to register global shortcut: {}", e);
                e
            })?;

            log::info!("Global shortcuts registered: Ctrl+Shift+V");

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_clipboard_history,
            search_clips,
            toggle_pin,
            delete_clip,
            get_pinned_clips
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

// 复制剪切板项到系统剪切板
fn copy_clip_to_clipboard(app: &AppHandle, clip_id: &str) -> Result<(), String> {
    use arboard::Clipboard;

    let state = app.state::<AppState>();
    let storage = state.storage.lock().map_err(|e| e.to_string())?;

    // 从数据库获取完整内容
    let items = storage.get_recent(Some(100)).map_err(|e| e.to_string())?;
    let item = items.iter()
        .find(|i| i.id == clip_id)
        .ok_or_else(|| "Clip not found".to_string())?;

    // 复制到系统剪切板
    let mut clipboard = Clipboard::new().map_err(|e| e.to_string())?;

    match item.content_type {
        ContentType::Text => {
            let text = String::from_utf8_lossy(&item.content);
            clipboard.set_text(text.to_string()).map_err(|e| e.to_string())?;
            log::info!("Copied text to clipboard: {} chars", text.len());
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
