// Prevents additional console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod clipboard;
mod storage;
mod crypto;

use clipboard::ClipboardMonitor;
use storage::{ClipStorage, ClipItem};
use tauri::{
    Manager, State,
    menu::{MenuBuilder, MenuItemBuilder},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
};
use std::sync::Mutex;

struct AppState {
    storage: Mutex<ClipStorage>,
    monitor: Mutex<Option<ClipboardMonitor>>,
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
    state: State<'_, AppState>,
    id: String,
    is_pinned: bool,
) -> Result<(), String> {
    let storage = state.storage.lock().map_err(|e| e.to_string())?;
    storage.update_pin(&id, is_pinned)
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn delete_clip(
    state: State<'_, AppState>,
    id: String,
) -> Result<(), String> {
    let storage = state.storage.lock().map_err(|e| e.to_string())?;
    storage.delete(&id)
        .map_err(|e| e.to_string())
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

    tauri::Builder::default()
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .setup(|app| {
            // Initialize storage
            let app_data_dir = app.path().app_data_dir()
                .expect("Failed to get app data directory");

            std::fs::create_dir_all(&app_data_dir)
                .expect("Failed to create app data directory");

            let db_path = app_data_dir.join("clipman.db");

            let storage = ClipStorage::new(db_path.to_str().unwrap())
                .expect("Failed to initialize database");

            let app_state = AppState {
                storage: Mutex::new(storage),
                monitor: Mutex::new(None),
            };

            app.manage(app_state);

            // Create system tray menu
            let quit_item = MenuItemBuilder::with_id("quit", "退出")
                .build(app)?;
            let show_item = MenuItemBuilder::with_id("show", "显示历史")
                .build(app)?;
            let pinned_item = MenuItemBuilder::with_id("pinned", "置顶列表")
                .build(app)?;

            let menu = MenuBuilder::new(app)
                .items(&[&show_item, &pinned_item, &quit_item])
                .build()?;

            // Create system tray
            let _tray = TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .on_menu_event(move |app, event| match event.id().as_ref() {
                    "quit" => {
                        app.exit(0);
                    }
                    "show" => {
                        if let Some(window) = app.get_webview_window("main") {
                            window.show().unwrap_or_else(|e| log::error!("Failed to show window: {}", e));
                            window.set_focus().unwrap_or_else(|e| log::error!("Failed to focus window: {}", e));
                        }
                    }
                    "pinned" => {
                        // TODO: Show pinned window
                        if let Some(window) = app.get_webview_window("main") {
                            window.show().unwrap_or_else(|e| log::error!("Failed to show window: {}", e));
                            window.set_focus().unwrap_or_else(|e| log::error!("Failed to focus window: {}", e));
                        }
                    }
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        let app = tray.app_handle();
                        if let Some(window) = app.get_webview_window("main") {
                            window.unminimize().unwrap_or_else(|e| log::error!("Failed to unminimize: {}", e));
                            window.show().unwrap_or_else(|e| log::error!("Failed to show: {}", e));
                            window.set_focus().unwrap_or_else(|e| log::error!("Failed to focus: {}", e));
                        }
                    }
                })
                .build(app)?;

            // Start clipboard monitoring
            let app_handle = app.handle().clone();
            let state: State<AppState> = app_handle.state();

            let monitor = ClipboardMonitor::new(app_handle.clone());
            monitor.start();

            *state.monitor.lock().unwrap() = Some(monitor);

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
