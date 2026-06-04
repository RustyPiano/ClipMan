// Prevents additional console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod clipboard;
mod commands;
mod migration;
mod paste;
mod settings;
mod storage;
mod tray;
mod window;

use clipboard::ClipboardMonitor;
use commands::{
    check_clipboard_permission, check_for_updates, clear_non_pinned_history,
    copy_clip_to_clipboard_internal, copy_to_system_clipboard, delete_clip,
    disable_global_shortcut, enable_global_shortcut, get_clip, get_current_data_path,
    get_pinned_clips, get_recent_clips, get_settings, hide_quickbar, install_update,
    migrate_data_location, open_folder, open_settings_window, paste_clip,
    register_quickbar_shortcut, reorder_pinned, search_clips, set_clip_label, show_quickbar,
    toggle_pin, update_settings,
};
use settings::SettingsManager;
use storage::{ClipStorage, CopyMarker};
use tray::{build_tray_menu, TrayIconCache};

use std::sync::{Arc, Mutex};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::Manager;

/// Helper function: safely acquire Mutex even if poisoned
pub fn safe_lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(|poisoned| {
        log::warn!("⚠️ Recovered from poisoned lock");
        poisoned.into_inner()
    })
}

/// Application state shared across commands
pub struct AppState {
    pub storage: Arc<Mutex<ClipStorage>>,
    pub monitor: Mutex<Option<ClipboardMonitor>>,
    pub settings: Arc<SettingsManager>,
    pub settings_write_lock: Mutex<()>,
    pub last_copied_by_us: Arc<Mutex<Option<CopyMarker>>>,
    pub icon_cache: Arc<TrayIconCache>,
    pub quickbar_foreground_window: window::ForegroundWindowStore,
}

fn main() {
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Debug)
        .init();
    log::info!("ClipMan starting...");

    #[cfg(target_os = "macos")]
    {
        use arboard::Clipboard;
        log::info!("Running on macOS - checking clipboard access");

        match Clipboard::new() {
            Ok(mut clipboard) => match clipboard.get_text() {
                Ok(text) => log::info!(
                    "✅ Clipboard access OK, current content: {} chars",
                    text.len()
                ),
                Err(e) => log::warn!(
                    "⚠️ Cannot read clipboard: {}. May need accessibility permission.",
                    e
                ),
            },
            Err(e) => log::error!("❌ Failed to create clipboard instance: {}", e),
        }
    }

    tauri::Builder::default()
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec!["--minimized"]),
        ))
        .setup(|app| {
            // Hide the Dock icon: run as a menu-bar (Accessory) app. This must
            // happen here, after Tauri/tao has created the NSApplication —
            // doing it earlier (before the event loop) gets reset to Regular.
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            // Initialize settings first
            let settings_manager = Arc::new(SettingsManager::new());
            if let Err(e) = settings_manager.load(app.handle()) {
                log::warn!("Failed to load settings, using defaults: {}", e);
            }

            let default_app_data_dir = app
                .path()
                .app_data_dir()
                .expect("Failed to get app data directory");

            let settings = settings_manager.get();
            let data_dir = migration::get_data_directory(
                default_app_data_dir.clone(),
                settings.custom_data_path.clone(),
            );

            log::info!("Using data directory: {:?}", data_dir);

            std::fs::create_dir_all(&data_dir).expect("Failed to create data directory");

            let db_path = data_dir.join("clipman.db");
            log::info!("Database path: {:?}", db_path);

            let storage =
                ClipStorage::new(db_path.to_str().unwrap()).expect("Failed to initialize database");

            let last_copied_by_us = Arc::new(Mutex::new(None));
            let icon_cache = Arc::new(TrayIconCache::new());
            let quickbar_foreground_window = Arc::new(Mutex::new(None));

            let app_state = AppState {
                storage: Arc::new(Mutex::new(storage)),
                monitor: Mutex::new(None),
                settings: settings_manager.clone(),
                settings_write_lock: Mutex::new(()),
                last_copied_by_us: last_copied_by_us.clone(),
                icon_cache: icon_cache.clone(),
                quickbar_foreground_window: quickbar_foreground_window.clone(),
            };

            app.manage(app_state);

            if let Err(e) = window::setup_windows(app.handle()) {
                log::error!("Failed to set up QuickBar windows: {}", e);
            }

            // Build tray menu
            let menu = build_tray_menu(app.handle())?;

            let _tray = TrayIconBuilder::with_id("main")
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
                                if let Err(e) =
                                    clear_non_pinned_history(app_clone.clone(), app_clone.state())
                                        .await
                                {
                                    log::error!("Failed to clear non-pinned history: {}", e);
                                }
                            });
                        }
                        "settings" => {
                            log::info!("Settings menu clicked");
                            if let Err(e) = window::open_settings_window(app) {
                                log::error!("Failed to open settings window: {}", e);
                            }
                        }
                        id if id.starts_with("clip:") => {
                            let clip_id = id.strip_prefix("clip:").unwrap().to_string();
                            log::info!("Clip item clicked: {}", clip_id);

                            let app_clone = app.clone();
                            tauri::async_runtime::spawn(async move {
                                // Use unified copy function with notification
                                if let Err(e) =
                                    copy_clip_to_clipboard_internal(&app_clone, &clip_id, true)
                                        .await
                                {
                                    log::error!("Failed to copy clip: {}", e);
                                }
                            });
                        }
                        _ => {
                            log::debug!("Unhandled menu event: {}", event_id);
                        }
                    }
                })
                .on_tray_icon_event(|_tray, event| {
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

            // Start clipboard monitoring
            let app_handle = app.handle().clone();
            let state: tauri::State<AppState> = app_handle.state();

            let mut monitor = ClipboardMonitor::new(app_handle.clone(), last_copied_by_us.clone());
            match monitor.start() {
                Ok(()) => {
                    *safe_lock(&state.monitor) = Some(monitor);
                    log::info!("Clipboard monitoring started");
                }
                Err(e) => log::error!("Failed to start clipboard monitoring: {}", e),
            }

            // Register global shortcuts
            let state: tauri::State<AppState> = app_handle.state();
            let settings = state.settings.get();
            let current_shortcut = settings.global_shortcut;
            let pinned_shortcut = settings.pinned_shortcut;

            let main_shortcut_registered = register_quickbar_shortcut(
                app.handle(),
                current_shortcut.as_str(),
                quickbar_foreground_window.clone(),
                window::QuickBarPanel::Recent,
            );

            if let Err(e) = main_shortcut_registered {
                log::error!("{}", e);
            }

            if let Some(pinned_shortcut) = pinned_shortcut {
                if pinned_shortcut == current_shortcut {
                    log::warn!(
                        "Skipping pinned shortcut '{}' because it matches the main shortcut",
                        pinned_shortcut
                    );
                } else if let Err(e) = register_quickbar_shortcut(
                    app.handle(),
                    pinned_shortcut.as_str(),
                    quickbar_foreground_window.clone(),
                    window::QuickBarPanel::Pinned,
                ) {
                    log::warn!("{}", e);
                } else {
                    log::info!("Pinned shortcut registered: {}", pinned_shortcut);
                }
            }

            log::info!("Global shortcuts registered: {}", current_shortcut);

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_recent_clips,
            get_pinned_clips,
            get_clip,
            search_clips,
            toggle_pin,
            delete_clip,
            get_settings,
            update_settings,
            check_clipboard_permission,
            clear_non_pinned_history,
            copy_to_system_clipboard,
            paste_clip,
            set_clip_label,
            reorder_pinned,
            open_settings_window,
            hide_quickbar,
            show_quickbar,
            check_for_updates,
            install_update,
            disable_global_shortcut,
            enable_global_shortcut,
            open_folder,
            migrate_data_location,
            get_current_data_path
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
