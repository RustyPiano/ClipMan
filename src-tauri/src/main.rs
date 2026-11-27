#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod accessibility;
mod clipboard;
mod commands;
mod identity;
mod migration;
mod paste;
mod secrets;
mod settings;
mod storage;
mod tray;
mod window;

use commands::{
    batch_delete_clips, batch_toggle_pin, check_accessibility_permission,
    check_clipboard_permission, check_for_updates, clear_non_pinned_history, copy_plain_text,
    copy_to_system_clipboard, create_pinned_clip, delete_clip, disable_global_shortcut,
    enable_global_shortcut, get_app_icon, get_clip, get_current_data_path, get_pinned_clips,
    get_recent_clips, get_settings, hide_quickbar, install_update, migrate_data_location,
    open_accessibility_settings, open_folder, open_settings_window, open_url, paste_clip,
    paste_clips, register_quickbar_shortcut, reorder_pinned, reveal_in_folder, search_clips,
    set_clip_label, set_quickbar_panel, show_quickbar, toggle_pin, update_clip_content,
    update_settings,
};
use paste::CopyWrite;
use settings::SettingsManager;
use storage::ClipStorage;
use tray::{build_tray_menu, TrayIconCache, TRAY_ID};

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tauri::tray::TrayIconBuilder;
use tauri::Manager;

pub struct AppState {
    pub storage: Arc<Mutex<ClipStorage>>,
    pub clipboard_use_lock: tokio::sync::Mutex<()>,
    pub settings: Arc<SettingsManager>,
    pub settings_write_lock: Mutex<()>,
    pub(crate) last_copied_by_us: Arc<Mutex<Option<CopyWrite>>>,
    pub icon_cache: Arc<TrayIconCache>,
    pub quickbar_foreground_window: window::ForegroundWindowStore,
    pub quickbar_panel: window::QuickBarPanelStore,
}

fn initialize_storage_core(
    default_dir: &Path,
    custom_data_path: Option<String>,
    mut on_database_reset: impl FnMut(&Path),
) -> Result<ClipStorage, String> {
    let dir = custom_data_path
        .map(PathBuf::from)
        .unwrap_or_else(|| default_dir.to_path_buf());
    log::info!("Using data directory: {:?}", dir);
    std::fs::create_dir_all(&dir)
        .map_err(|e| format!("Failed to create data directory {}: {}", dir.display(), e))?;

    identity::rename_legacy_database(&dir)?;
    let db_path = dir.join(identity::DATABASE_FILE);
    log::info!("Database path: {:?}", db_path);
    let open_error =
        |e: rusqlite::Error| format!("Failed to open database at {}: {}", db_path.display(), e);

    match ClipStorage::new(&db_path) {
        Ok(storage) => Ok(storage),
        Err(e) if !storage::is_corrupt_database_error(&e) => Err(open_error(e)),
        Err(e) => {
            let open_err = open_error(e);
            log::warn!(
                "SQLite reported a corrupt database; resetting: {}",
                open_err
            );

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

            ClipStorage::new(&db_path).map_err(open_error)
        }
    }
}

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

fn spawn_fatal_alert(app: &tauri::AppHandle, message: String) {
    use tauri_plugin_dialog::{DialogExt, MessageDialogButtons, MessageDialogKind};

    let app = app.clone();
    std::thread::spawn(move || {
        app.dialog()
            .message(message)
            .title("ClipLeaf 无法启动 / ClipLeaf cannot start")
            .kind(MessageDialogKind::Error)
            .buttons(MessageDialogButtons::Ok)
            .blocking_show();
        std::process::exit(1);
    });
}

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or(
        if cfg!(debug_assertions) {
            "debug"
        } else {
            "info"
        },
    ))
    .init();
    log::info!("ClipLeaf starting...");

    #[cfg(target_os = "macos")]
    match identity::relaunch_from_renamed_bundle() {
        Ok(true) => return,
        Ok(false) => {}
        Err(e) => log::error!("Failed to rename the app bundle to ClipLeaf.app: {e}"),
    }

    let context = tauri::generate_context!();
    let identity_migration = identity::migrate_legacy_identity(&context.config().identifier);

    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            log::info!("Second app instance launch detected; showing QuickBar");
            let app_for_main = app.clone();
            if let Err(e) = app.run_on_main_thread(move || {
                if let Some(state) = app_for_main.try_state::<AppState>() {
                    if let Err(e) = window::show_quickbar(
                        &app_for_main,
                        &state.quickbar_foreground_window,
                        &state.quickbar_panel,
                    ) {
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
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec!["--minimized"]),
        ))
        .setup(move |app| {
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            let identity_migrated = match identity_migration {
                Ok(migrated) => migrated,
                Err(fatal_error) => {
                    log::error!("ClipLeaf cannot migrate ClipMan data: {fatal_error}");
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
                            "ClipLeaf 无法迁移 ClipMan 的数据，应用即将退出。原数据未被删除。\n\
                             ClipLeaf could not migrate the ClipMan data and will exit. The original data was not deleted.\n\n{fatal_error}"
                        ),
                    );
                    return Ok(());
                }
            };

            let settings_manager = Arc::new(SettingsManager::default());
            if let Err(e) = settings_manager.load(app.handle()) {
                log::error!("Failed to load settings; capture paused: {e}");
                notify_storage_issue(app.handle(), "设置读取失败 / Settings could not be loaded",
                    &format!("已暂停采集，原设置文件未被覆盖。请检查设置文件后重启，或确认偏好后保存再恢复采集。\nCapture is paused. The original settings file was not overwritten. Review your preferences before resuming.\n\n{e}"));
            }

            let settings = settings_manager.get();
            let storage = match resolve_storage(app.handle(), settings.custom_data_path.clone()) {
                Ok(storage) => storage,
                Err(fatal_error) => {
                    log::error!("ClipLeaf cannot start: {}", fatal_error);
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
                            "ClipLeaf 无法初始化数据存储，应用即将退出。\n\
                             ClipLeaf could not initialize its data storage and will exit.\n\n{fatal_error}"
                        ),
                    );
                    return Ok(());
                }
            };

            let last_copied_by_us = Arc::new(Mutex::new(None));
            let quickbar_foreground_window = Arc::new(Mutex::new(None));
            let quickbar_panel = Arc::new(Mutex::new(window::QuickBarPanel::Recent));

            app.manage(AppState {
                storage: Arc::new(Mutex::new(storage)),
                clipboard_use_lock: tokio::sync::Mutex::new(()),
                settings: settings_manager.clone(),
                settings_write_lock: Mutex::new(()),
                last_copied_by_us: last_copied_by_us.clone(),
                icon_cache: Arc::new(TrayIconCache::new()),
                quickbar_foreground_window: quickbar_foreground_window.clone(),
                quickbar_panel: quickbar_panel.clone(),
            });

            if let Err(e) = window::setup_windows(app.handle()) {
                log::error!("Failed to set up QuickBar windows: {}", e);
            }

            if settings.enable_autostart {
                if let Err(e) = commands::migrate_legacy_autostart(app.handle(), identity_migrated) {
                    log::error!("Failed to migrate the legacy autostart entry: {e}");
                }
            }

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
                log::error!("ClipLeaf cannot start: {e}");
                spawn_fatal_alert(
                    app.handle(),
                    format!(
                        "ClipLeaf 无法启动剪贴板监听，应用即将退出。\n\
                         ClipLeaf could not start clipboard monitoring and will exit.\n\n{e}"
                    ),
                );
                return Ok(());
            }
            log::info!("Clipboard monitoring started");

            let current_shortcut = settings.global_shortcut;

            if let Err(e) = register_quickbar_shortcut(
                app.handle(),
                current_shortcut.as_str(),
                quickbar_foreground_window.clone(),
                quickbar_panel.clone(),
                window::QuickBarPanel::Recent,
            ) {
                log::error!("{}", e);
            }

            if let Some(pinned_shortcut) = settings.pinned_shortcut {
                if let Err(e) = register_quickbar_shortcut(
                    app.handle(),
                    pinned_shortcut.as_str(),
                    quickbar_foreground_window,
                    quickbar_panel,
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
            get_app_icon,
            get_clip,
            search_clips,
            toggle_pin,
            batch_toggle_pin,
            delete_clip,
            batch_delete_clips,
            get_settings,
            update_settings,
            check_clipboard_permission,
            check_accessibility_permission,
            open_accessibility_settings,
            clear_non_pinned_history,
            copy_plain_text,
            copy_to_system_clipboard,
            paste_clip,
            paste_clips,
            set_clip_label,
            update_clip_content,
            create_pinned_clip,
            reorder_pinned,
            open_settings_window,
            hide_quickbar,
            show_quickbar,
            set_quickbar_panel,
            check_for_updates,
            install_update,
            disable_global_shortcut,
            enable_global_shortcut,
            open_folder,
            open_url,
            reveal_in_folder,
            migrate_data_location,
            get_current_data_path
        ])
        .run(context)
        .expect("error while running tauri application");
}

#[cfg(test)]
mod storage_init_tests {
    use super::*;
    use std::fs;
    use uuid::Uuid;

    fn temp_root(name: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!("clipleaf_main_init_{}_{}", name, Uuid::new_v4()))
    }

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
        assert!(!default_dir.join("clipleaf.db").exists());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn corrupt_database_is_quarantined_and_rebuilt() {
        let root = temp_root("corrupt_db");
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("clipleaf.db"), b"not a sqlite database").unwrap();

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
        let db_path = root.join("clipleaf.db");
        fs::create_dir_all(&db_path).unwrap();

        let result = initialize_storage_core(&root, None, |_backup| {
            panic!("non-corruption failures must not reset the database")
        });

        assert!(result.is_err());
        assert!(db_path.is_dir(), "the original path must remain untouched");
        let _ = fs::remove_dir_all(&root);
    }
}
