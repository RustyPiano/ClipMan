// Tauri commands module
use std::sync::mpsc;
use std::time::Duration;

use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};
use tauri_plugin_notification::NotificationExt;

use crate::settings::Settings;
use crate::storage::{ContentType, FrontendClipItem};
use crate::tray::update_tray_menu;
use crate::{migration, safe_lock, AppState};

fn run_window_command_on_main_thread<F>(
    app: &AppHandle,
    command_name: &'static str,
    action: F,
) -> Result<(), String>
where
    F: FnOnce(AppHandle) -> Result<(), String> + Send + 'static,
{
    // Frontend IPC handlers can run off the event-loop thread; macOS window
    // focus/activation touches AppKit and must be dispatched back to main.
    let app_for_task = app.clone();
    let (sender, receiver) = mpsc::channel();

    app.run_on_main_thread(move || {
        let _ = sender.send(action(app_for_task));
    })
    .map_err(|e| format!("Failed to schedule {command_name} on main thread: {e}"))?;

    receiver
        .recv_timeout(Duration::from_secs(5))
        .map_err(|e| format!("Timed out waiting for {command_name} on main thread: {e}"))?
}

#[tauri::command]
pub async fn get_recent_clips(
    state: State<'_, AppState>,
    limit: Option<usize>,
) -> Result<Vec<FrontendClipItem>, String> {
    let storage = state.storage.clone();
    let limit = limit.unwrap_or(100);

    tauri::async_runtime::spawn_blocking(move || {
        let storage = safe_lock(&storage);
        let items = storage.get_recent_clips(limit).map_err(|e| e.to_string())?;
        Ok(items.into_iter().map(FrontendClipItem::from).collect())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn get_pinned_clips(state: State<'_, AppState>) -> Result<Vec<FrontendClipItem>, String> {
    let storage = state.storage.clone();

    tauri::async_runtime::spawn_blocking(move || {
        let storage = safe_lock(&storage);
        let items = storage.get_pinned_clips().map_err(|e| e.to_string())?;
        Ok(items.into_iter().map(FrontendClipItem::from).collect())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn search_clips(
    state: State<'_, AppState>,
    query: String,
) -> Result<Vec<FrontendClipItem>, String> {
    let storage = state.storage.clone();

    tauri::async_runtime::spawn_blocking(move || {
        let storage = safe_lock(&storage);
        let items = storage.search(&query).map_err(|e| e.to_string())?;
        Ok(items.into_iter().map(FrontendClipItem::from).collect())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn toggle_pin(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
    is_pinned: bool,
) -> Result<(), String> {
    let storage = state.storage.clone();

    tauri::async_runtime::spawn_blocking(move || {
        let storage = safe_lock(&storage);
        storage
            .update_pin(&id, is_pinned)
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())??;

    update_tray_menu(&app);
    Ok(())
}

#[tauri::command]
pub async fn delete_clip(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
) -> Result<(), String> {
    let storage = state.storage.clone();

    tauri::async_runtime::spawn_blocking(move || {
        let storage = safe_lock(&storage);
        storage.delete(&id).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())??;

    update_tray_menu(&app);
    Ok(())
}

#[tauri::command]
pub async fn get_settings(state: State<'_, AppState>) -> Result<Settings, String> {
    Ok(state.settings.get())
}

#[tauri::command]
pub async fn check_clipboard_permission() -> Result<String, String> {
    use arboard::Clipboard;

    match Clipboard::new() {
        Ok(mut clipboard) => match clipboard.get_text() {
            Ok(_) => Ok("granted".to_string()),
            Err(e) => Ok(format!("denied: {}", e)),
        },
        Err(e) => Err(format!("Failed to create clipboard: {}", e)),
    }
}

#[tauri::command]
pub async fn clear_non_pinned_history(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    log::info!("Clearing non-pinned clipboard history (user requested)");
    let storage = state.storage.clone();

    tauri::async_runtime::spawn_blocking(move || {
        let storage = safe_lock(&storage);
        storage.clear_non_pinned().map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())??;

    state.icon_cache.clear();
    update_tray_menu(&app);

    // Emit event to notify frontend
    if let Err(e) = app.emit("history-cleared", ()) {
        log::error!("Failed to emit history-cleared event: {}", e);
    }

    Ok(())
}

/// Copy a clip to the system clipboard (used by the tray menu and the in-window
/// Copy button). Reuses the paste module's clipboard writer so there is a single
/// implementation of "touch timestamp + emit + write clipboard".
pub async fn copy_clip_to_clipboard_internal(
    app: &AppHandle,
    clip_id: &str,
    show_notification: bool,
) -> Result<(), String> {
    let state = app.state::<AppState>();
    let item =
        crate::paste::fetch_clip_and_touch_timestamp(app, state.inner(), clip_id.to_string())
            .await?;

    crate::paste::write_clip_to_system_clipboard(&item, state.last_copied_by_us.clone())?;

    if show_notification {
        notify_copied(app, &item.content_type);
    }

    Ok(())
}

#[cfg(not(target_os = "linux"))]
fn notify_copied(app: &AppHandle, content_type: &ContentType) {
    let body = match content_type {
        ContentType::Text => "文本已复制到剪贴板",
        ContentType::Image => "图片已复制到剪贴板",
    };
    let _ = app
        .notification()
        .builder()
        .title("已复制")
        .body(body)
        .show();
}

#[cfg(target_os = "linux")]
fn notify_copied(_app: &AppHandle, _content_type: &ContentType) {}

#[tauri::command]
pub async fn copy_to_system_clipboard(
    app: AppHandle,
    _state: State<'_, AppState>,
    clip_id: String,
) -> Result<(), String> {
    // Use unified function, no notification for window copy
    copy_clip_to_clipboard_internal(&app, &clip_id, false).await
}

// CopyMarker contract:
// - WP-1.B sets hash=SHA256(normalized_clipboard_payload) and content_type when writing clipboard.
// - WP-1.D computes the same normalized hash on clipboard changes and skips matching payloads.
// - The marker is cleared after the existing short self-copy window.
#[tauri::command]
pub async fn paste_clip(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
    mode: String,
) -> Result<(), String> {
    crate::paste::paste_clip(app, state.inner(), id, mode).await
}

pub fn register_quickbar_shortcut(
    app: &AppHandle,
    shortcut: &str,
    foreground_store: crate::window::ForegroundWindowStore,
    panel: crate::window::QuickBarPanel,
) -> Result<(), String> {
    let app_clone = app.clone();
    let shortcut_display = shortcut.to_string();

    app.global_shortcut()
        .on_shortcut(shortcut, move |_app, _shortcut, event| {
            if event.state == ShortcutState::Pressed {
                log::info!("Global shortcut triggered: {}", shortcut_display);
                let result = match panel {
                    crate::window::QuickBarPanel::Recent => {
                        crate::window::show_quickbar(&app_clone, &foreground_store)
                    }
                    crate::window::QuickBarPanel::Pinned => {
                        crate::window::show_quickbar_with_panel(
                            &app_clone,
                            &foreground_store,
                            panel,
                        )
                    }
                };

                if let Err(e) = result {
                    log::error!("Failed to show QuickBar: {}", e);
                }
            }
        })
        .map_err(|e| format!("Failed to register shortcut '{}': {}", shortcut, e))
}

#[tauri::command]
pub async fn set_clip_label(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
    label: Option<String>,
) -> Result<(), String> {
    let storage = state.storage.clone();

    tauri::async_runtime::spawn_blocking(move || {
        let storage = safe_lock(&storage);
        storage
            .set_clip_label(&id, label)
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())??;

    update_tray_menu(&app);
    Ok(())
}

#[tauri::command]
pub async fn reorder_pinned(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
    direction: String,
) -> Result<(), String> {
    let storage = state.storage.clone();

    tauri::async_runtime::spawn_blocking(move || {
        let storage = safe_lock(&storage);
        storage
            .reorder_pinned(&id, direction.as_str())
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())??;

    update_tray_menu(&app);
    Ok(())
}

#[tauri::command]
pub async fn open_settings_window(app: AppHandle) -> Result<(), String> {
    run_window_command_on_main_thread(&app, "open_settings_window", |app| {
        crate::window::open_settings_window(&app)
    })
}

#[tauri::command]
pub async fn hide_quickbar(app: AppHandle) -> Result<(), String> {
    crate::window::hide_quickbar(&app)
}

#[tauri::command]
pub async fn show_quickbar(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    let foreground_store = state.quickbar_foreground_window.clone();

    run_window_command_on_main_thread(&app, "show_quickbar", move |app| {
        crate::window::show_quickbar(&app, &foreground_store)
    })
}

#[tauri::command]
pub async fn check_for_updates(app: AppHandle) -> Result<serde_json::Value, String> {
    use tauri_plugin_updater::UpdaterExt;

    log::info!("Checking for updates...");
    let current_version = app.package_info().version.to_string();

    match app.updater() {
        Ok(updater) => match updater.check().await {
            Ok(update) => {
                if let Some(update_info) = update {
                    let available_version = update_info.version.clone();
                    log::info!(
                        "Update available: {} -> {}",
                        current_version,
                        available_version
                    );

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
        },
        Err(e) => {
            log::error!("Failed to get updater: {}", e);
            Err(format!("Failed to get updater: {}", e))
        }
    }
}

#[tauri::command]
pub async fn install_update(app: AppHandle) -> Result<(), String> {
    use tauri_plugin_updater::UpdaterExt;

    log::info!("Installing update...");

    match app.updater() {
        Ok(updater) => match updater.check().await {
            Ok(update) => {
                if let Some(update_info) = update {
                    log::info!("Downloading and installing update: {}", update_info.version);

                    match update_info
                        .download_and_install(
                            |chunk_length, content_length| {
                                if let Some(total) = content_length {
                                    let progress = (chunk_length as f64 / total as f64) * 100.0;
                                    log::debug!("Download progress: {:.2}%", progress);
                                }
                            },
                            || {
                                log::info!("Download complete, installing...");
                            },
                        )
                        .await
                    {
                        Ok(_) => {
                            log::info!("Update installed successfully. Restarting app...");
                            app.restart();
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
        },
        Err(e) => {
            log::error!("Failed to get updater: {}", e);
            Err(format!("Failed to get updater: {}", e))
        }
    }
}

fn normalize_optional_shortcut(shortcut: Option<String>) -> Option<String> {
    shortcut
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn unregister_shortcut_if_active(app: &AppHandle, shortcut: &str, label: &str) -> bool {
    match app.global_shortcut().unregister(shortcut) {
        Ok(()) => true,
        Err(e) => {
            log::warn!(
                "Failed to unregister {label} shortcut '{}': {}",
                shortcut,
                e
            );
            false
        }
    }
}

fn restore_shortcut(
    app: &AppHandle,
    shortcut: &str,
    foreground_store: crate::window::ForegroundWindowStore,
    panel: crate::window::QuickBarPanel,
    label: &str,
) {
    if let Err(e) = register_quickbar_shortcut(app, shortcut, foreground_store, panel) {
        log::warn!("Failed to restore {label} shortcut '{}': {}", shortcut, e);
    }
}

fn apply_shortcut_changes(
    app: &AppHandle,
    foreground_store: crate::window::ForegroundWindowStore,
    old_shortcut: &str,
    old_pinned_shortcut: Option<&str>,
    new_shortcut: &str,
    new_pinned_shortcut: Option<&str>,
) -> Result<(), String> {
    let shortcut_changed = old_shortcut != new_shortcut;
    let pinned_shortcut_changed = old_pinned_shortcut != new_pinned_shortcut;

    if !shortcut_changed && !pinned_shortcut_changed {
        return Ok(());
    }

    let mut new_main_registered = false;
    let mut old_main_unregistered = false;
    let mut old_pinned_unregistered = false;

    if pinned_shortcut_changed && old_pinned_shortcut == Some(new_shortcut) {
        old_pinned_unregistered = unregister_shortcut_if_active(app, new_shortcut, "old pinned");
    }

    if shortcut_changed {
        if let Err(e) = register_quickbar_shortcut(
            app,
            new_shortcut,
            foreground_store.clone(),
            crate::window::QuickBarPanel::Recent,
        ) {
            if old_pinned_unregistered {
                if let Some(old_pinned_shortcut) = old_pinned_shortcut {
                    restore_shortcut(
                        app,
                        old_pinned_shortcut,
                        foreground_store,
                        crate::window::QuickBarPanel::Pinned,
                        "old pinned",
                    );
                }
            }
            return Err(e);
        }
        new_main_registered = true;
    }

    if shortcut_changed && new_pinned_shortcut == Some(old_shortcut) {
        old_main_unregistered = unregister_shortcut_if_active(app, old_shortcut, "old main");
    }

    if pinned_shortcut_changed {
        if let Some(new_pinned_shortcut) = new_pinned_shortcut {
            if let Err(e) = register_quickbar_shortcut(
                app,
                new_pinned_shortcut,
                foreground_store.clone(),
                crate::window::QuickBarPanel::Pinned,
            ) {
                if new_main_registered {
                    let _ = app.global_shortcut().unregister(new_shortcut);
                }
                if old_main_unregistered {
                    restore_shortcut(
                        app,
                        old_shortcut,
                        foreground_store.clone(),
                        crate::window::QuickBarPanel::Recent,
                        "old main",
                    );
                }
                if old_pinned_unregistered {
                    if let Some(old_pinned_shortcut) = old_pinned_shortcut {
                        restore_shortcut(
                            app,
                            old_pinned_shortcut,
                            foreground_store,
                            crate::window::QuickBarPanel::Pinned,
                            "old pinned",
                        );
                    }
                }
                return Err(e);
            }
        }
    }

    if shortcut_changed && !old_main_unregistered {
        unregister_shortcut_if_active(app, old_shortcut, "old main");
    }

    if pinned_shortcut_changed && !old_pinned_unregistered {
        if let Some(old_pinned_shortcut) = old_pinned_shortcut {
            unregister_shortcut_if_active(app, old_pinned_shortcut, "old pinned");
        }
    }

    Ok(())
}

#[tauri::command]
pub async fn update_settings(
    app: AppHandle,
    state: State<'_, AppState>,
    mut settings: Settings,
) -> Result<(), String> {
    settings.pinned_shortcut = normalize_optional_shortcut(settings.pinned_shortcut.take());
    log::info!("Updating settings: {:?}", settings);

    let old_settings = state.settings.get();
    let old_shortcut = old_settings.global_shortcut;
    let old_pinned_shortcut = old_settings.pinned_shortcut;
    let old_tray_text_length = old_settings.tray_text_length;
    let old_autostart = old_settings.enable_autostart;
    let old_locale = old_settings.locale;
    let new_shortcut = settings.global_shortcut.clone();
    let new_pinned_shortcut = settings.pinned_shortcut.clone();

    if new_pinned_shortcut.as_deref() == Some(new_shortcut.as_str()) {
        return Err("Pinned shortcut cannot match the main global shortcut".to_string());
    }

    let shortcut_changed = old_shortcut != new_shortcut;
    let pinned_shortcut_changed = old_pinned_shortcut != new_pinned_shortcut;
    let tray_text_changed = old_tray_text_length != settings.tray_text_length;
    let autostart_changed = old_autostart != settings.enable_autostart;
    let locale_changed = old_locale != settings.locale;
    let quickbar_foreground_window = state.quickbar_foreground_window.clone();

    // Update autostart if changed
    if autostart_changed {
        use tauri_plugin_autostart::ManagerExt;

        let result = if settings.enable_autostart {
            app.autolaunch().enable()
        } else {
            app.autolaunch().disable()
        };

        if let Err(e) = result {
            log::error!("Failed to update autostart: {}", e);
            return Err(format!("Failed to update autostart: {}", e));
        }

        log::info!(
            "Autostart {} successfully",
            if settings.enable_autostart {
                "enabled"
            } else {
                "disabled"
            }
        );
    }

    if shortcut_changed || pinned_shortcut_changed {
        apply_shortcut_changes(
            &app,
            quickbar_foreground_window,
            old_shortcut.as_str(),
            old_pinned_shortcut.as_deref(),
            new_shortcut.as_str(),
            new_pinned_shortcut.as_deref(),
        )?;
    }

    state.settings.set(settings.clone());
    state.settings.save(&app)?;

    // Rebuild tray menu if text length or locale changed
    if tray_text_changed || locale_changed {
        log::info!("Tray settings changed, rebuilding menu...");
        update_tray_menu(&app);
    }

    Ok(())
}

#[tauri::command]
pub async fn get_current_data_path(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let default_path = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?;

    let settings = state.settings.get();
    let data_dir = migration::get_data_directory(default_path, settings.custom_data_path);

    data_dir
        .to_str()
        .map(|s| s.to_string())
        .ok_or_else(|| "Invalid data path".to_string())
}

#[tauri::command]
pub async fn disable_global_shortcut(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let settings = state.settings.get();
    let current_shortcut = settings.global_shortcut;

    if let Err(e) = app.global_shortcut().unregister(current_shortcut.as_str()) {
        log::warn!(
            "Failed to disable global shortcut '{}': {}",
            current_shortcut,
            e
        );
        return Err(format!("Failed to disable shortcut: {}", e));
    }

    log::info!(
        "Global shortcut '{}' temporarily disabled",
        current_shortcut
    );

    if let Some(pinned_shortcut) = settings.pinned_shortcut {
        if let Err(e) = app.global_shortcut().unregister(pinned_shortcut.as_str()) {
            log::warn!(
                "Failed to disable pinned shortcut '{}': {}",
                pinned_shortcut,
                e
            );
        } else {
            log::info!("Pinned shortcut '{}' temporarily disabled", pinned_shortcut);
        }
    }

    Ok(())
}

#[tauri::command]
pub async fn enable_global_shortcut(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let settings = state.settings.get();
    let current_shortcut = settings.global_shortcut;
    let foreground_store = state.quickbar_foreground_window.clone();

    register_quickbar_shortcut(
        &app,
        current_shortcut.as_str(),
        foreground_store.clone(),
        crate::window::QuickBarPanel::Recent,
    )
    .map_err(|e| format!("Failed to re-enable shortcut: {}", e))?;

    log::info!("Global shortcut '{}' re-enabled", current_shortcut);

    if let Some(pinned_shortcut) = settings.pinned_shortcut {
        if pinned_shortcut != current_shortcut {
            register_quickbar_shortcut(
                &app,
                pinned_shortcut.as_str(),
                foreground_store,
                crate::window::QuickBarPanel::Pinned,
            )
            .map_err(|e| format!("Failed to re-enable pinned shortcut: {}", e))?;
            log::info!("Pinned shortcut '{}' re-enabled", pinned_shortcut);
        }
    }

    Ok(())
}

#[tauri::command]
pub async fn open_folder(path: String) -> Result<(), String> {
    use std::process::Command;

    #[cfg(target_os = "macos")]
    {
        Command::new("open")
            .arg(&path)
            .spawn()
            .map_err(|e| format!("Failed to open folder: {}", e))?;
    }

    #[cfg(target_os = "windows")]
    {
        Command::new("explorer")
            .arg(&path)
            .spawn()
            .map_err(|e| format!("Failed to open folder: {}", e))?;
    }

    #[cfg(target_os = "linux")]
    {
        Command::new("xdg-open")
            .arg(&path)
            .spawn()
            .map_err(|e| format!("Failed to open folder: {}", e))?;
    }

    Ok(())
}

#[tauri::command]
pub async fn migrate_data_location(
    app: AppHandle,
    state: State<'_, AppState>,
    new_path: String,
    delete_old: bool,
) -> Result<(), String> {
    use crate::clipboard::ClipboardMonitor;
    use crate::storage::ClipStorage;

    log::info!(
        "Starting data migration to: {}, delete_old: {}",
        new_path,
        delete_old
    );

    // Stop clipboard monitoring so nothing writes during the copy.
    if let Some(monitor) = safe_lock(&state.monitor).take() {
        monitor.stop();
        log::info!("Clipboard monitoring stopped for migration");
    }

    // Resolve old / new paths.
    let default_path = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?;
    let settings = state.settings.get();
    let old_path = migration::get_data_directory(default_path, settings.custom_data_path.clone());
    let new_path_buf = std::path::PathBuf::from(&new_path);

    // Copy only (deletion happens after we close the old connection).
    migration::migrate_data(&old_path, &new_path_buf, false)?;

    // Persist the new location.
    let mut new_settings = settings.clone();
    new_settings.custom_data_path = Some(new_path.clone());
    state.settings.set(new_settings);
    state
        .settings
        .save(&app)
        .map_err(|e| format!("Failed to save settings: {}", e))?;

    // Reopen storage at the new path so subsequent writes go there immediately.
    let new_db_path = new_path_buf.join("clipman.db");
    let new_db_path = new_db_path
        .to_str()
        .ok_or_else(|| "Invalid new database path".to_string())?;
    let new_storage = ClipStorage::new(new_db_path).map_err(|e| e.to_string())?;
    *safe_lock(&state.storage) = new_storage;

    // Now it is safe to delete the old files; Windows refuses deleting open DBs.
    if delete_old {
        migration::remove_data_files(&old_path);
    }

    log::info!("Data migration completed successfully");

    // Restart clipboard monitoring against the new storage.
    let mut monitor = ClipboardMonitor::new(app.clone(), state.last_copied_by_us.clone());
    monitor.start();
    *safe_lock(&state.monitor) = Some(monitor);
    log::info!("Clipboard monitoring restarted after migration");

    crate::tray::update_tray_menu(&app);
    Ok(())
}
