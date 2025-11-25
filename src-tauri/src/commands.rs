// Tauri commands module
use tauri::{AppHandle, Manager, State, Emitter};
use tauri_plugin_notification::NotificationExt;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

use crate::storage::{FrontendClipItem, ContentType};
use crate::settings::Settings;
use crate::tray::update_tray_menu;
use crate::{AppState, safe_lock, migration};

#[tauri::command]
pub async fn get_clipboard_history(
    state: State<'_, AppState>,
    limit: Option<usize>,
) -> Result<Vec<FrontendClipItem>, String> {
    let storage = state.storage.clone();
    let limit = limit.unwrap_or(100);

    tauri::async_runtime::spawn_blocking(move || {
        let storage = safe_lock(&storage);
        let items = storage.get_recent(limit).map_err(|e| e.to_string())?;
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
        storage.update_pin(&id, is_pinned).map_err(|e| e.to_string())
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
pub async fn get_settings(
    state: State<'_, AppState>,
) -> Result<Settings, String> {
    Ok(state.settings.get())
}

#[tauri::command]
pub async fn check_clipboard_permission() -> Result<String, String> {
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
pub async fn clear_all_history(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    log::info!("Clearing all clipboard history (user requested)");
    let storage = state.storage.clone();

    tauri::async_runtime::spawn_blocking(move || {
        let storage = safe_lock(&storage);
        storage.clear_all().map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())??;

    state.icon_cache.clear();
    update_tray_menu(&app);
    Ok(())
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

/// Copy a clip item to system clipboard (unified function)
pub async fn copy_clip_to_clipboard_internal(
    app: &AppHandle,
    clip_id: &str,
    show_notification: bool,
) -> Result<(), String> {
    use arboard::{Clipboard, ImageData};
    use image::GenericImageView;
    use std::borrow::Cow;
    use chrono::Utc;
    use crate::storage::FrontendClipItem;

    let state = app.state::<AppState>();
    let storage = state.storage.clone();
    let clip_id_for_fetch = clip_id.to_string();

    // Fetch item using get_by_id for efficiency
    let item = tauri::async_runtime::spawn_blocking(move || {
        let storage = safe_lock(&storage);
        storage.get_by_id(&clip_id_for_fetch)
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())?
    .ok_or_else(|| "Clip not found".to_string())?;

    // Update timestamp to move item to top of recent list
    let new_timestamp = Utc::now().timestamp();
    let storage = state.storage.clone();
    let clip_id_for_update = clip_id.to_string();
    tauri::async_runtime::spawn_blocking(move || {
        let storage = safe_lock(&storage);
        storage.update_timestamp(&clip_id_for_update, new_timestamp)
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())?;

    // Emit event to notify frontend about the timestamp update
    let mut updated_item = item.clone();
    updated_item.timestamp = new_timestamp;
    let frontend_item = FrontendClipItem::from(updated_item);
    if let Err(e) = app.emit("clipboard-changed", &frontend_item) {
        log::error!("Failed to emit clipboard-changed event: {}", e);
    }

    // Update tray menu to reflect new order
    update_tray_menu(app);

    // Copy to system clipboard
    let mut clipboard = Clipboard::new().map_err(|e| e.to_string())?;

    match item.content_type {
        ContentType::Text => {
            let text = String::from_utf8_lossy(&item.content).to_string();

            // Mark as self-copied to prevent re-capture
            {
                let mut last_copied = safe_lock(&state.last_copied_by_us);
                *last_copied = Some(text.clone());
            }

            clipboard.set_text(text.clone()).map_err(|e| e.to_string())?;
            log::info!("✅ Copied text to clipboard: {} chars", text.len());

            if show_notification {
                #[cfg(not(target_os = "linux"))]
                let _ = app.notification()
                    .builder()
                    .title("已复制")
                    .body("文本已复制到剪贴板")
                    .show();
            }

            // Clear marker after 2 seconds
            let last_copied_by_us = state.last_copied_by_us.clone();
            std::thread::spawn(move || {
                std::thread::sleep(std::time::Duration::from_secs(2));
                let mut last_copied = safe_lock(&last_copied_by_us);
                *last_copied = None;
                log::debug!("🧹 Cleared self-copy marker");
            });
        }
        ContentType::Image => {
            let img = image::load_from_memory(&item.content)
                .map_err(|e| format!("Failed to decode image: {}", e))?;
            
            let (width, height) = img.dimensions();
            let rgba_bytes = img.to_rgba8().into_raw();

            let image_data = ImageData {
                width: width as usize,
                height: height as usize,
                bytes: Cow::from(rgba_bytes),
            };

            clipboard.set_image(image_data).map_err(|e| e.to_string())?;
            log::info!("✅ Copied image to clipboard ({}x{})", width, height);

            if show_notification {
                #[cfg(not(target_os = "linux"))]
                let _ = app.notification()
                    .builder()
                    .title("已复制")
                    .body("图片已复制到剪贴板")
                    .show();
            }
        }
        ContentType::File => {
            let path = String::from_utf8_lossy(&item.content).to_string();
            clipboard.set_text(path.clone()).map_err(|e| e.to_string())?;
            log::info!("✅ Copied file path to clipboard: {}", path);

            if show_notification {
                #[cfg(not(target_os = "linux"))]
                let _ = app.notification()
                    .builder()
                    .title("已复制")
                    .body("文件路径已复制到剪贴板")
                    .show();
            }
        }
        ContentType::Html | ContentType::Rtf => {
            let text = String::from_utf8_lossy(&item.content).to_string();
            clipboard.set_text(text).map_err(|e| e.to_string())?;
            log::info!("✅ Copied rich text to clipboard as plain text");

            if show_notification {
                #[cfg(not(target_os = "linux"))]
                let _ = app.notification()
                    .builder()
                    .title("已复制")
                    .body("富文本已复制到剪贴板")
                    .show();
            }
        }
    }

    Ok(())
}

#[tauri::command]
pub async fn copy_to_system_clipboard(
    app: AppHandle,
    _state: State<'_, AppState>,
    clip_id: String,
) -> Result<(), String> {
    // Use unified function, no notification for window copy
    copy_clip_to_clipboard_internal(&app, &clip_id, false).await
}

#[tauri::command]
pub async fn check_for_updates(app: AppHandle) -> Result<serde_json::Value, String> {
    use tauri_plugin_updater::UpdaterExt;
    
    log::info!("Checking for updates...");
    let current_version = app.package_info().version.to_string();

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
pub async fn install_update(app: AppHandle) -> Result<(), String> {
    use tauri_plugin_updater::UpdaterExt;
    
    log::info!("Installing update...");

    match app.updater() {
        Ok(updater) => {
            match updater.check().await {
                Ok(update) => {
                    if let Some(update_info) = update {
                        log::info!("Downloading and installing update: {}", update_info.version);

                        match update_info.download_and_install(|chunk_length, content_length| {
                            if let Some(total) = content_length {
                                let progress = (chunk_length as f64 / total as f64) * 100.0;
                                log::debug!("Download progress: {:.2}%", progress);
                            }
                        }, || {
                            log::info!("Download complete, installing...");
                        }).await {
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
            }
        }
        Err(e) => {
            log::error!("Failed to get updater: {}", e);
            Err(format!("Failed to get updater: {}", e))
        }
    }
}

#[tauri::command]
pub async fn update_settings(
    app: AppHandle,
    state: State<'_, AppState>,
    settings: Settings,
) -> Result<(), String> {
    log::info!("Updating settings: {:?}", settings);

    let old_shortcut = state.settings.get().global_shortcut;
    let old_tray_text_length = state.settings.get().tray_text_length;
    let old_autostart = state.settings.get().enable_autostart;
    let old_locale = state.settings.get().locale;
    let new_shortcut = settings.global_shortcut.clone();
    let shortcut_changed = old_shortcut != new_shortcut;
    let tray_text_changed = old_tray_text_length != settings.tray_text_length;
    let autostart_changed = old_autostart != settings.enable_autostart;
    let locale_changed = old_locale != settings.locale;

    state.settings.set(settings.clone());
    state.settings.save(&app)?;

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
        
        log::info!("Autostart {} successfully", 
            if settings.enable_autostart { "enabled" } else { "disabled" });
    }

    // Re-register hotkey if changed
    if shortcut_changed {
        log::info!("Hotkey changed from '{}' to '{}', re-registering...", old_shortcut, new_shortcut);

        if let Err(e) = app.global_shortcut().unregister(old_shortcut.as_str()) {
            log::warn!("Failed to unregister old shortcut '{}': {}", old_shortcut, e);
        }

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
    let default_path = app.path().app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?;
    
    let settings = state.settings.get();
    let data_dir = migration::get_data_directory(
        default_path,
        settings.custom_data_path
    );
    
    data_dir.to_str()
        .map(|s| s.to_string())
        .ok_or_else(|| "Invalid data path".to_string())
}

#[tauri::command]
pub async fn disable_global_shortcut(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let current_shortcut = state.settings.get().global_shortcut;
    
    if let Err(e) = app.global_shortcut().unregister(current_shortcut.as_str()) {
        log::warn!("Failed to disable global shortcut '{}': {}", current_shortcut, e);
        return Err(format!("Failed to disable shortcut: {}", e));
    }
    
    log::info!("Global shortcut '{}' temporarily disabled", current_shortcut);
    Ok(())
}

#[tauri::command]
pub async fn enable_global_shortcut(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let current_shortcut = state.settings.get().global_shortcut;
    let app_clone = app.clone();
    let shortcut_clone = current_shortcut.clone();
    
    app.global_shortcut()
        .on_shortcut(current_shortcut.as_str(), move |_app, _shortcut, event| {
            if event.state == ShortcutState::Pressed {
                log::info!("Global shortcut triggered: {}", shortcut_clone);
                if let Some(window) = app_clone.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
        })
        .map_err(|e| format!("Failed to re-enable shortcut: {}", e))?;
    
    log::info!("Global shortcut '{}' re-enabled", current_shortcut);
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
    
    log::info!("Starting data migration to: {}, delete_old: {}", new_path, delete_old);
    
    // Stop clipboard monitoring during migration
    {
        let mut monitor_guard = state.monitor.lock().unwrap();
        if let Some(monitor) = monitor_guard.take() {
            drop(monitor);
            log::info!("Clipboard monitoring stopped for migration");
        }
    }
    
    // Get current and new paths
    let default_path = app.path().app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?;
    
    let settings = state.settings.get();
    let custom_path = settings.custom_data_path.clone();
    let old_path = migration::get_data_directory(default_path.clone(), custom_path);
    let new_path_buf = std::path::PathBuf::from(&new_path);
    
    // Perform migration
    migration::migrate_data(&old_path, &new_path_buf, delete_old)?;
    
    // Update settings with new path
    let mut new_settings = settings.clone();
    new_settings.custom_data_path = Some(new_path.clone());
    
    state.settings.set(new_settings.clone());
    state.settings.save(&app)
        .map_err(|e| format!("Failed to save settings: {}", e))?;
    
    log::info!("Data migration completed successfully");
    
    // Restart clipboard monitoring
    let monitor = ClipboardMonitor::new(app.clone(), state.last_copied_by_us.clone());
    monitor.start();
    *state.monitor.lock().unwrap() = Some(monitor);
    log::info!("Clipboard monitoring restarted after migration");
    
    Ok(())
}
