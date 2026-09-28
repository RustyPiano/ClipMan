// Prevents additional console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod accessibility;
mod clipboard;
mod commands;
mod migration;
mod paste;
mod secrets;
mod settings;
mod storage;
mod tray;
mod window;

use commands::{
    check_accessibility_permission, check_clipboard_permission, check_for_updates,
    clear_non_pinned_history, copy_to_system_clipboard, delete_clip, disable_global_shortcut,
    enable_global_shortcut, get_clip, get_current_data_path, get_pinned_clips, get_recent_clips,
    get_settings, hide_quickbar, install_update, migrate_data_location,
    open_accessibility_settings, open_folder, open_settings_window, paste_clip, paste_clips,
    register_quickbar_shortcut, reorder_pinned, search_clips, set_clip_label, show_quickbar,
    toggle_pin, update_settings,
};
use paste::CopyWrite;
use settings::SettingsManager;
use storage::ClipStorage;
use tray::{build_tray_menu, TrayIconCache, TRAY_ID};

use std::path::Path;
use std::sync::{Arc, Mutex};
use tauri::tray::TrayIconBuilder;
use tauri::Manager;

/// Application state shared across commands
pub struct AppState {
    pub storage: Arc<Mutex<ClipStorage>>,
    pub clipboard_use_lock: tokio::sync::Mutex<()>,
    pub settings: Arc<SettingsManager>,
    pub settings_write_lock: Mutex<()>,
    pub(crate) last_copied_by_us: Arc<Mutex<Option<CopyWrite>>>,
    pub icon_cache: Arc<TrayIconCache>,
    pub quickbar_foreground_window: window::ForegroundWindowStore,
}

/// 一次打开 `dir/clipman.db` 的结果；`is_corrupt` 表示 SQLite 判定数据库已损坏。
enum StorageAttempt {
    Ready(ClipStorage),
    OpenFailed { message: String, is_corrupt: bool },
}

fn try_open_storage(dir: &Path) -> StorageAttempt {
    log::info!("Using data directory: {:?}", dir);

    if let Err(e) = std::fs::create_dir_all(dir) {
        return StorageAttempt::OpenFailed {
            message: format!("Failed to create data directory {}: {}", dir.display(), e),
            is_corrupt: false,
        };
    }

    let db_path = dir.join("clipman.db");
    log::info!("Database path: {:?}", db_path);

    match ClipStorage::new(&db_path) {
        Ok(storage) => StorageAttempt::Ready(storage),
        Err(e) => StorageAttempt::OpenFailed {
            is_corrupt: storage::is_corrupt_database_error(&e),
            message: format!("Failed to open database at {}: {}", db_path.display(), e),
        },
    }
}

/// 打开当前数据目录（设置了自定义目录就用它）的数据库。SQLite 判定损坏时，
/// 把旧文件隔离保存后新建空库；其他失败直接返回错误，由调用方退出应用。
/// 不依赖 `AppHandle`，通知通过闭包注入，便于单元测试。
fn initialize_storage_core(
    default_dir: &Path,
    custom_data_path: Option<String>,
    mut on_database_reset: impl FnMut(&Path),
) -> Result<ClipStorage, String> {
    let dir = migration::get_data_directory(default_dir.to_path_buf(), custom_data_path);
    match try_open_storage(&dir) {
        StorageAttempt::Ready(storage) => Ok(storage),
        StorageAttempt::OpenFailed {
            message,
            is_corrupt: false,
        } => Err(message),
        StorageAttempt::OpenFailed {
            message: open_err,
            is_corrupt: true,
        } => {
            log::warn!(
                "SQLite reported a corrupt database; resetting: {}",
                open_err
            );

            let db_path = dir.join("clipman.db");
            if let Some(backup_path) =
                storage::quarantine_corrupt_database(&db_path).map_err(|e| {
                    format!(
                        "Failed to quarantine corrupt database at {}: {}; original error: {}",
                        db_path.display(),
                        e,
                        open_err
                    )
                })?
            {
                on_database_reset(&backup_path);
            }

            match try_open_storage(&dir) {
                StorageAttempt::Ready(storage) => Ok(storage),
                StorageAttempt::OpenFailed { message, .. } => Err(message),
            }
        }
    }
}

/// 解析数据目录并打开数据库；数据库被重置时用对话框告知用户。
fn resolve_storage(
    app: &tauri::AppHandle,
    custom_data_path: Option<String>,
) -> Result<ClipStorage, String> {
    let default_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to resolve the application data directory: {}", e))?;

    initialize_storage_core(&default_dir, custom_data_path, |backup_path| {
        notify_storage_issue(
            app,
            "历史记录已重置 / History reset",
            &format!(
                "剪贴板历史数据库已损坏，已重置为新的空数据库。旧文件已保留在：\n{}\n\n\
                 The clipboard history database was corrupted and has been reset. \
                 The old file was kept at:\n{}",
                backup_path.display(),
                backup_path.display()
            ),
        );
    })
}

/// Show a non-blocking modal alert. Fire-and-forget from the caller's point
/// of view — the app keeps running (with the already-recovered storage)
/// while the user dismisses it whenever the event loop gets to it. Mirrors
/// the dialog pattern in `accessibility.rs`.
fn notify_storage_issue(app: &tauri::AppHandle, title: &str, message: &str) {
    use tauri_plugin_dialog::{DialogExt, MessageDialogButtons, MessageDialogKind};

    let app_for_dialog = app.clone();
    let title = title.to_string();
    let message = message.to_string();
    let dispatch = app.run_on_main_thread(move || {
        app_for_dialog
            .dialog()
            .message(message)
            .title(title)
            .kind(MessageDialogKind::Warning)
            .buttons(MessageDialogButtons::Ok)
            .show(|_| {});
    });

    if let Err(e) = dispatch {
        log::error!("Failed to show storage alert dialog: {}", e);
    }
}

/// 启动失败时的退出路径（数据存储或剪贴板监听无法初始化）：release 构建是
/// panic = "abort"，直接崩溃用户看不到原因，这里用对话框说明原因再退出。对话框放在
/// 后台线程，因为 `setup()` 必须先返回，事件循环才能开始处理并显示对话框。
fn spawn_fatal_alert(app: &tauri::AppHandle, message: String) {
    use tauri_plugin_dialog::{DialogExt, MessageDialogButtons, MessageDialogKind};

    let app = app.clone();
    std::thread::spawn(move || {
        app.dialog()
            .message(message)
            .title("ClipMan 无法启动 / ClipMan cannot start")
            .kind(MessageDialogKind::Error)
            .buttons(MessageDialogButtons::Ok)
            .blocking_show();
        std::process::exit(1);
    });
}

fn main() {
    // 设置了 RUST_LOG 就按它；否则 debug 构建输出 debug 级别，release 输出 info。
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or(
        if cfg!(debug_assertions) {
            "debug"
        } else {
            "info"
        },
    ))
    .init();
    log::info!("ClipMan starting...");

    tauri::Builder::default()
        // Must be the first plugin: a second launch (double-clicking the app
        // icon while the tray instance is already running — or worse, after
        // replacing the bundle with a new build) must never spawn a competing
        // instance that fights over the global hotkey and clipboard monitor.
        // Surface the QuickBar in the existing instance instead.
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            log::info!("Second app instance launch detected; showing QuickBar");
            // show_quickbar touches AppKit (NSWorkspace / orderFront /
            // invalidateShadow), all main-thread-only, but this callback runs
            // on a tokio worker thread — dispatch the work back to the main
            // thread instead of calling AppKit off-thread.
            let app_for_main = app.clone();
            if let Err(e) = app.run_on_main_thread(move || {
                if let Some(state) = app_for_main.try_state::<AppState>() {
                    if let Err(e) =
                        window::show_quickbar(&app_for_main, &state.quickbar_foreground_window)
                    {
                        log::error!("Failed to show QuickBar for second-instance launch: {e}");
                    }
                }
            }) {
                log::error!("Failed to schedule QuickBar for second-instance launch: {e}");
            }
        }))
        .plugin(tauri_plugin_dialog::init())
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
                log::error!("Failed to load settings; capture paused: {e}");
                notify_storage_issue(app.handle(), "设置读取失败 / Settings could not be loaded",
                    &format!("已暂停采集，原设置文件未被覆盖。请检查设置文件后重启，或确认偏好后保存再恢复采集。\nCapture is paused. The original settings file was not overwritten. Review your preferences before resuming.\n\n{e}"));
            }

            let settings = settings_manager.get();
            let storage = match resolve_storage(app.handle(), settings.custom_data_path.clone()) {
                Ok(storage) => storage,
                Err(fatal_error) => {
                    log::error!("ClipMan cannot start: {}", fatal_error);
                    // 隐藏的 webview 加载后会立即调用命令，而此时没有 AppState，
                    // 取 State 会 panic（release 中即 abort）。先销毁窗口，只留下错误对话框。
                    for (label, window) in app.webview_windows() {
                        if let Err(e) = window.destroy() {
                            log::warn!(
                                "Failed to destroy window '{label}' during fatal startup: {e}"
                            );
                        }
                    }
                    spawn_fatal_alert(
                        app.handle(),
                        format!(
                            "ClipMan 无法初始化数据存储，应用即将退出。\n\
                             ClipMan could not initialize its data storage and will exit.\n\n{fatal_error}"
                        ),
                    );
                    return Ok(());
                }
            };

            let last_copied_by_us = Arc::new(Mutex::new(None));
            let quickbar_foreground_window = Arc::new(Mutex::new(None));

            app.manage(AppState {
                storage: Arc::new(Mutex::new(storage)),
                clipboard_use_lock: tokio::sync::Mutex::new(()),
                settings: settings_manager.clone(),
                settings_write_lock: Mutex::new(()),
                last_copied_by_us: last_copied_by_us.clone(),
                icon_cache: Arc::new(TrayIconCache::new()),
                quickbar_foreground_window: quickbar_foreground_window.clone(),
            });

            if let Err(e) = window::setup_windows(app.handle()) {
                log::error!("Failed to set up QuickBar windows: {}", e);
            }

            // Build tray menu
            let menu = build_tray_menu(app.handle())?;

            #[cfg(target_os = "macos")]
            let tray_icon =
                tauri::image::Image::new(include_bytes!("../icons/tray-icon.rgba"), 32, 32);
            #[cfg(not(target_os = "macos"))]
            let tray_icon = app.default_window_icon().unwrap().clone();

            let _tray = TrayIconBuilder::with_id(TRAY_ID)
                .icon(tray_icon)
                .icon_as_template(cfg!(target_os = "macos"))
                .menu(&menu)
                .on_menu_event(tray::handle_tray_menu_event)
                .build(app)?;

            log::info!("System tray initialized");

            if let Err(e) = clipboard::start_monitor(app.handle().clone(), last_copied_by_us) {
                log::error!("ClipMan cannot start: {e}");
                spawn_fatal_alert(
                    app.handle(),
                    format!(
                        "ClipMan 无法启动剪贴板监听，应用即将退出。\n\
                         ClipMan could not start clipboard monitoring and will exit.\n\n{e}"
                    ),
                );
                return Ok(());
            }
            log::info!("Clipboard monitoring started");

            // Register global shortcuts
            let current_shortcut = settings.global_shortcut;

            if let Err(e) = register_quickbar_shortcut(
                app.handle(),
                current_shortcut.as_str(),
                quickbar_foreground_window.clone(),
                window::QuickBarPanel::Recent,
            ) {
                log::error!("{}", e);
            }

            if let Some(pinned_shortcut) = settings.pinned_shortcut {
                if pinned_shortcut == current_shortcut {
                    log::warn!(
                        "Skipping pinned shortcut '{}' because it matches the main shortcut",
                        pinned_shortcut
                    );
                } else if let Err(e) = register_quickbar_shortcut(
                    app.handle(),
                    pinned_shortcut.as_str(),
                    quickbar_foreground_window,
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
            check_accessibility_permission,
            open_accessibility_settings,
            clear_non_pinned_history,
            copy_to_system_clipboard,
            paste_clip,
            paste_clips,
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

#[cfg(test)]
mod storage_init_tests {
    use super::*;
    use std::fs;
    use uuid::Uuid;

    fn temp_root(name: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!("clipman_main_init_{}_{}", name, Uuid::new_v4()))
    }

    /// 自定义数据目录无法创建时启动失败，不改用默认目录。
    #[test]
    fn unusable_custom_dir_is_fatal() {
        let root = temp_root("custom_unusable");
        let default_dir = root.join("default");
        fs::create_dir_all(&default_dir).unwrap();

        let blocker_file = root.join("not_a_dir");
        fs::write(&blocker_file, b"x").unwrap();
        let bad_custom_dir = blocker_file.join("subdir");

        let result = initialize_storage_core(
            &default_dir,
            Some(bad_custom_dir.to_string_lossy().into_owned()),
            |_backup| panic!("should not need a database reset in this scenario"),
        );

        assert!(result.is_err());
        assert!(!default_dir.join("clipman.db").exists());
        let _ = fs::remove_dir_all(&root);
    }

    /// 损坏的数据库被隔离保存，并在原位置新建可用的数据库。
    #[test]
    fn corrupt_database_is_quarantined_and_rebuilt() {
        let root = temp_root("corrupt_db");
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("clipman.db"), b"not a sqlite database").unwrap();

        let mut reset_backup_path = None;
        let result = initialize_storage_core(&root, None, |backup_path| {
            reset_backup_path = Some(backup_path.to_path_buf())
        });

        let storage = result.expect("should recover with a freshly rebuilt database");
        assert!(storage.get_recent_clip_previews(10).is_ok());

        let backup_path =
            reset_backup_path.expect("expected the corrupt-db reset notification to fire");
        assert!(backup_path.exists());
        assert!(backup_path.to_string_lossy().contains(".corrupt-"));
        assert_eq!(
            b"not a sqlite database".to_vec(),
            fs::read(&backup_path).unwrap()
        );

        drop(storage);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn non_corruption_open_failure_preserves_database_path() {
        let root = temp_root("non_corrupt_open_failure");
        let db_path = root.join("clipman.db");
        fs::create_dir_all(&db_path).unwrap();

        let result = initialize_storage_core(&root, None, |_backup| {
            panic!("non-corruption failures must not reset the database")
        });

        assert!(result.is_err());
        assert!(db_path.is_dir(), "the original path must remain untouched");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn uncreatable_default_dir_is_fatal() {
        let root = temp_root("default_uncreatable");
        fs::create_dir_all(&root).unwrap();

        let blocker_file = root.join("not_a_dir");
        fs::write(&blocker_file, b"x").unwrap();
        let unusable_default_dir = blocker_file.join("data");

        let result = initialize_storage_core(&unusable_default_dir, None, |_backup| {
            panic!("directory couldn't even be created, nothing to quarantine")
        });

        assert!(result.is_err());
        let _ = fs::remove_dir_all(&root);
    }
}
