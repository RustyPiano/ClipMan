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

use clipboard::ClipboardMonitor;
use commands::{
    check_accessibility_permission, check_clipboard_permission, check_for_updates,
    clear_non_pinned_history, copy_clip_to_clipboard_internal, copy_to_system_clipboard,
    delete_clip, disable_global_shortcut, enable_global_shortcut, get_clip, get_current_data_path,
    get_pinned_clips, get_recent_clips, get_settings, hide_quickbar, install_update,
    migrate_data_location, open_accessibility_settings, open_folder, open_settings_window,
    paste_clip, paste_clips, register_quickbar_shortcut, reorder_pinned, search_clips,
    set_clip_label, show_quickbar, toggle_pin, update_settings,
};
use settings::SettingsManager;
use storage::{ClipStorage, CopyMarker};
use tray::{build_tray_menu, update_tray_menu, TrayIconCache};

use std::path::Path;
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

/// Outcome of a single attempt to make `dir` (and `dir/clipman.db`) usable.
enum StorageAttempt {
    Ready(ClipStorage),
    /// `dir` itself could not be created (or isn't writable).
    DirUnavailable(String),
    /// `dir` exists, but opening/initializing the database inside it failed.
    OpenFailed(String),
}

fn try_open_storage(dir: &Path) -> StorageAttempt {
    log::info!("Using data directory: {:?}", dir);

    if let Err(e) = std::fs::create_dir_all(dir) {
        return StorageAttempt::DirUnavailable(format!(
            "Failed to create data directory {}: {}",
            dir.display(),
            e
        ));
    }

    let db_path = dir.join("clipman.db");
    log::info!("Database path: {:?}", db_path);

    match ClipStorage::new(&db_path) {
        Ok(storage) => StorageAttempt::Ready(storage),
        Err(e) => StorageAttempt::OpenFailed(format!(
            "Failed to open database at {}: {}",
            db_path.display(),
            e
        )),
    }
}

/// Pure startup degradation-chain logic (SPEC-3 §2): try the custom data
/// directory first if one is configured, falling back to `default_dir` when
/// it's unusable; if the database at `default_dir` itself fails to open,
/// assume corruption, quarantine the old files, and rebuild a fresh database
/// in their place.
///
/// Kept free of `AppHandle`/dialogs so it can be exercised directly in unit
/// tests with injected notification closures (see the `tests` module below).
/// The only case this returns `Err` for is `default_dir` being completely
/// unusable (can't even be created), which the caller must treat as fatal.
fn initialize_storage_core(
    default_dir: &Path,
    custom_data_path: Option<String>,
    mut on_custom_dir_fallback: impl FnMut(&str),
    mut on_database_reset: impl FnMut(&Path),
) -> Result<ClipStorage, String> {
    if let Some(custom_path) = custom_data_path {
        let custom_dir =
            migration::get_data_directory(default_dir.to_path_buf(), Some(custom_path));
        match try_open_storage(&custom_dir) {
            StorageAttempt::Ready(storage) => return Ok(storage),
            StorageAttempt::DirUnavailable(e) | StorageAttempt::OpenFailed(e) => {
                log::warn!(
                    "Custom data directory unavailable ({}); falling back to the default directory",
                    e
                );
                on_custom_dir_fallback(&e);
            }
        }
    }

    match try_open_storage(default_dir) {
        StorageAttempt::Ready(storage) => Ok(storage),
        StorageAttempt::DirUnavailable(e) => Err(e),
        StorageAttempt::OpenFailed(open_err) => {
            log::warn!(
                "Default database unavailable, assuming corruption and resetting: {}",
                open_err
            );

            let db_path = default_dir.join("clipman.db");
            match storage::quarantine_corrupt_database(&db_path) {
                Ok(Some(backup_path)) => on_database_reset(&backup_path),
                Ok(None) => {}
                Err(e) => log::error!("Failed to quarantine corrupt database sidecars: {}", e),
            }

            match try_open_storage(default_dir) {
                StorageAttempt::Ready(storage) => Ok(storage),
                StorageAttempt::DirUnavailable(e) | StorageAttempt::OpenFailed(e) => Err(e),
            }
        }
    }
}

/// Resolve and open the on-disk storage for the running app, applying the
/// startup degradation chain above and surfacing non-fatal recoveries
/// (custom-dir fallback, corrupt-db reset) to the user via a modal alert.
/// Returns `Err` only when nothing usable could be initialized at all.
fn resolve_storage(
    app: &tauri::AppHandle,
    custom_data_path: Option<String>,
) -> Result<ClipStorage, String> {
    let default_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to resolve the application data directory: {}", e))?;

    initialize_storage_core(
        &default_dir,
        custom_data_path,
        |error| {
            notify_storage_issue(
                app,
                "数据目录不可用 / Data directory unavailable",
                &format!(
                    "自定义数据目录不可用，本次使用默认目录。\n\
                     Custom data directory unavailable this session; using the default directory instead.\n\n{error}"
                ),
            );
        },
        |backup_path| {
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
        },
    )
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

/// The one allowed non-panic exit path (SPEC-3 §2): storage could not be
/// initialized at all, even after every fallback. Show the reason in a modal
/// alert on a background thread — `setup()` must return first so the event
/// loop starts pumping and the dialog can actually render — then exit once
/// it's dismissed.
fn spawn_fatal_storage_alert(app: &tauri::AppHandle, error: &str) {
    use tauri_plugin_dialog::{DialogExt, MessageDialogButtons, MessageDialogKind};

    let app = app.clone();
    let message = format!(
        "ClipMan 无法初始化数据存储，应用即将退出。\n\
         ClipMan could not initialize its data storage and will exit.\n\n{error}"
    );

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
        // Must be the first plugin: a second launch (double-clicking the app
        // icon while the tray instance is already running — or worse, after
        // replacing the bundle with a new build) must never spawn a competing
        // instance that fights over the global hotkey and clipboard monitor.
        // Surface the QuickBar in the existing instance instead.
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            log::info!("Second app instance launch detected; showing QuickBar");
            if let Some(state) = app.try_state::<AppState>() {
                if let Err(e) = window::show_quickbar(app, &state.quickbar_foreground_window) {
                    log::error!("Failed to show QuickBar for second-instance launch: {e}");
                }
            }
        }))
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

            let settings = settings_manager.get();
            let storage = match resolve_storage(app.handle(), settings.custom_data_path.clone()) {
                Ok(storage) => storage,
                Err(fatal_error) => {
                    // Nothing usable to fall back to (can't resolve/create any
                    // data directory, or even a freshly-reset database won't
                    // open). Release builds run with panic = "abort", so we
                    // must not panic here — tell the user why we're exiting
                    // and stop cleanly instead of crashing silently.
                    log::error!("ClipMan cannot start: {}", fatal_error);
                    // The hidden webviews start invoking commands as soon as
                    // they load, and with no managed AppState those State
                    // extractions panic (= abort in release). Tear the windows
                    // down first so the fatal alert is all that remains.
                    for (label, window) in app.webview_windows() {
                        if let Err(e) = window.destroy() {
                            log::warn!(
                                "Failed to destroy window '{label}' during fatal startup: {e}"
                            );
                        }
                    }
                    spawn_fatal_storage_alert(app.handle(), &fatal_error);
                    return Ok(());
                }
            };

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

            #[cfg(target_os = "macos")]
            let tray_icon =
                tauri::image::Image::new(include_bytes!("../icons/tray-icon.rgba"), 32, 32);
            #[cfg(not(target_os = "macos"))]
            let tray_icon = app.default_window_icon().unwrap().clone();

            let _tray = TrayIconBuilder::with_id("main")
                .icon(tray_icon)
                .icon_as_template(cfg!(target_os = "macos"))
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
                        "pause_capture" => {
                            let state: tauri::State<AppState> = app.state();
                            // Serialize against `update_settings` so a settings-page
                            // save and this tray toggle can't race each other and
                            // clobber one another's change.
                            let _settings_write_guard = safe_lock(&state.settings_write_lock);

                            let mut settings = state.settings.get();
                            settings.capture_paused = !settings.capture_paused;
                            let now_paused = settings.capture_paused;
                            state.settings.set(settings);

                            if let Err(e) = state.settings.save(app) {
                                log::error!("Failed to persist capture_paused toggle: {}", e);
                            }
                            log::info!(
                                "Clipboard capture {} via tray menu",
                                if now_paused { "paused" } else { "resumed" }
                            );

                            update_tray_menu(app);
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

    /// Failure injection #1 (SPEC-3 §2 acceptance): a configured custom data
    /// directory that doesn't exist and can't be created — a plain file in
    /// its ancestry makes `create_dir_all` reliably fail cross-platform.
    #[test]
    fn custom_dir_unusable_falls_back_to_default_and_notifies() {
        let root = temp_root("custom_fallback");
        let default_dir = root.join("default");
        fs::create_dir_all(&default_dir).unwrap();

        let blocker_file = root.join("not_a_dir");
        fs::write(&blocker_file, b"x").unwrap();
        let bad_custom_dir = blocker_file.join("subdir");

        let mut fallback_notified = false;
        let result = initialize_storage_core(
            &default_dir,
            Some(bad_custom_dir.to_string_lossy().into_owned()),
            |_error| fallback_notified = true,
            |_backup| panic!("should not need a database reset in this scenario"),
        );

        let storage = result.expect("should recover using the default directory");
        assert!(storage.get_recent_clips(10).is_ok());
        assert!(
            fallback_notified,
            "expected the custom-dir fallback notification to fire"
        );
        assert!(default_dir.join("clipman.db").exists());

        drop(storage);
        let _ = fs::remove_dir_all(&root);
    }

    /// Failure injection #2 (SPEC-3 §2 acceptance): the default database file
    /// is corrupt (garbage bytes, not a valid sqlite header). Recovery must
    /// quarantine it and rebuild a fresh, usable database in its place.
    #[test]
    fn corrupt_default_database_is_quarantined_and_rebuilt() {
        let root = temp_root("corrupt_db");
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("clipman.db"), b"not a sqlite database").unwrap();

        let mut reset_backup_path = None;
        let result = initialize_storage_core(
            &root,
            None,
            |_error| panic!("no custom dir configured, should not fall back"),
            |backup_path| reset_backup_path = Some(backup_path.to_path_buf()),
        );

        let storage = result.expect("should recover with a freshly rebuilt database");
        assert!(storage.get_recent_clips(10).is_ok());

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

    /// Failure injection #3 (SPEC-3 §2 acceptance): the default data
    /// directory itself cannot be created at all — the only case that must
    /// be reported as fatal rather than recovered from.
    #[test]
    fn default_dir_uncreatable_is_reported_as_fatal_error() {
        let root = temp_root("default_uncreatable");
        fs::create_dir_all(&root).unwrap();

        let blocker_file = root.join("not_a_dir");
        fs::write(&blocker_file, b"x").unwrap();
        let unusable_default_dir = blocker_file.join("data");

        let result = initialize_storage_core(
            &unusable_default_dir,
            None,
            |_error| panic!("no custom dir configured, should not fall back"),
            |_backup| panic!("directory couldn't even be created, nothing to quarantine"),
        );

        assert!(result.is_err());
        let _ = fs::remove_dir_all(&root);
    }

    /// Failure injection #4 (SPEC-3 §2 acceptance): a data directory path
    /// containing non-UTF-8 bytes. Before this change, `main.rs` converted
    /// the db path with `.to_str().unwrap()`, which panics purely in Rust
    /// userspace (independent of any filesystem call) whenever the path
    /// isn't valid UTF-8. `ClipStorage::new` now takes `&Path` directly, so
    /// that unwrap — and its panic — no longer exists anywhere on this path.
    ///
    /// macOS's filesystem itself also rejects illegal byte sequences in
    /// filenames (`create_dir_all` fails with `EILSEQ`), so the meaningful
    /// assertion here is that this returns a graceful `Err` — proving we
    /// reach and handle it via the normal `DirUnavailable` branch — rather
    /// than panicking/aborting before we even get that far.
    #[cfg(unix)]
    #[test]
    fn non_utf8_data_directory_does_not_panic() {
        use std::ffi::OsStr;
        use std::os::unix::ffi::OsStrExt;

        let root = temp_root("non_utf8");
        fs::create_dir_all(&root).unwrap();
        // 0xFF is never a valid standalone UTF-8 byte.
        let non_utf8_name = OsStr::from_bytes(b"fo\xFFo-data");
        let default_dir = root.join(non_utf8_name);

        let result = initialize_storage_core(
            &default_dir,
            None,
            |_error| panic!("no custom dir configured, should not fall back"),
            |_backup| panic!("a fresh directory has no corrupt database to reset"),
        );

        // The important thing is that we got here at all instead of
        // panicking; whether the OS itself accepts the byte sequence is
        // platform-dependent (macOS rejects it with EILSEQ).
        assert!(result.is_err());

        let _ = fs::remove_dir_all(&root);
    }
}
