use std::sync::{mpsc, Arc, Mutex};
use std::time::Duration;

use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};
#[cfg(not(target_os = "linux"))]
use tauri_plugin_notification::NotificationExt;

use crate::settings::Settings;
use crate::storage::{ClipFilter, ClipStorage, FrontendClipItem};
use crate::tray::update_tray_menu;
use crate::{migration, AppState};

const MAX_BATCH_IDS: usize = 10_000;

async fn with_storage<T, F>(storage: Arc<Mutex<ClipStorage>>, op: F) -> Result<T, String>
where
    F: FnOnce(&ClipStorage) -> Result<T, String> + Send + 'static,
    T: Send + 'static,
{
    tauri::async_runtime::spawn_blocking(move || op(&storage.lock().unwrap()))
        .await
        .map_err(|e| e.to_string())?
}

pub(crate) fn run_action_on_main_thread<F>(
    app: &AppHandle,
    command_name: &'static str,
    action: F,
) -> Result<(), String>
where
    F: FnOnce() -> Result<(), String> + Send + 'static,
{
    let (sender, receiver) = mpsc::channel();

    app.run_on_main_thread(move || {
        let _ = sender.send(action());
    })
    .map_err(|e| format!("Failed to schedule {command_name} on main thread: {e}"))?;

    receiver
        .recv_timeout(Duration::from_secs(5))
        .map_err(|e| format!("Timed out waiting for {command_name} on main thread: {e}"))?
}

#[tauri::command]
pub async fn get_recent_clips(
    state: State<'_, AppState>,
    limit: usize,
    before_timestamp: Option<f64>,
    before_id: Option<String>,
    filter: Option<String>,
) -> Result<Vec<FrontendClipItem>, String> {
    let filter = ClipFilter::from_optional(filter.as_deref())?;
    with_storage(state.storage.clone(), move |storage| {
        let before = before_timestamp.zip(before_id.as_deref());
        let items = storage
            .get_recent_clip_previews_page(limit, before, filter)
            .map_err(|e| e.to_string())?;
        Ok(items
            .into_iter()
            .map(FrontendClipItem::from_preview)
            .collect())
    })
    .await
}

#[tauri::command]
pub async fn get_pinned_clips(state: State<'_, AppState>) -> Result<Vec<FrontendClipItem>, String> {
    with_storage(state.storage.clone(), |storage| {
        let items = storage
            .get_pinned_clip_previews(-1)
            .map_err(|e| e.to_string())?;
        Ok(items
            .into_iter()
            .map(FrontendClipItem::from_preview)
            .collect())
    })
    .await
}

#[tauri::command]
pub async fn get_clip(
    state: State<'_, AppState>,
    id: String,
) -> Result<Option<crate::storage::FrontendClipDetail>, String> {
    with_storage(state.storage.clone(), move |storage| {
        storage.get_detail(&id)
    })
    .await
}

#[tauri::command]
pub async fn get_app_icon(
    state: State<'_, AppState>,
    name: String,
) -> Result<Option<String>, String> {
    with_storage(state.storage.clone(), move |storage| {
        storage.app_icon(&name).map_err(|e| e.to_string())
    })
    .await
}

#[tauri::command]
pub async fn search_clips(
    state: State<'_, AppState>,
    query: String,
    filter: Option<String>,
) -> Result<Vec<FrontendClipItem>, String> {
    let filter = ClipFilter::from_optional(filter.as_deref())?;
    with_storage(state.storage.clone(), move |storage| {
        let items = storage
            .search_clip_previews_filtered(&query, filter)
            .map_err(|e| e.to_string())?;
        Ok(items
            .into_iter()
            .map(FrontendClipItem::from_preview)
            .collect())
    })
    .await
}

#[tauri::command]
pub async fn toggle_pin(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
    is_pinned: bool,
) -> Result<(), String> {
    let limit = state.settings.get().max_history_items;
    with_storage(state.storage.clone(), move |storage| {
        storage
            .update_pin(&id, is_pinned, limit)
            .map_err(|e| e.to_string())
    })
    .await?;

    update_tray_menu(&app);
    Ok(())
}

#[tauri::command]
pub async fn delete_clip(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
) -> Result<(), String> {
    with_storage(state.storage.clone(), move |storage| {
        storage.delete(&id).map_err(|e| e.to_string())
    })
    .await?;

    update_tray_menu(&app);
    Ok(())
}

#[tauri::command]
pub async fn get_settings(state: State<'_, AppState>) -> Result<Settings, String> {
    Ok(state.settings.get())
}

#[tauri::command]
pub async fn check_clipboard_permission() -> Result<(), String> {
    use arboard::{Clipboard, Error};

    let mut clipboard =
        Clipboard::new().map_err(|e| format!("Failed to create clipboard: {}", e))?;
    match clipboard.get_text() {
        Ok(_) | Err(Error::ContentNotAvailable) => Ok(()),
        Err(e) => Err(e.to_string()),
    }
}

#[tauri::command]
pub async fn check_accessibility_permission() -> Result<bool, String> {
    Ok(crate::accessibility::is_trusted())
}

#[tauri::command]
pub async fn open_accessibility_settings(app: AppHandle) -> Result<(), String> {
    crate::accessibility::open_settings(&app)
}

#[tauri::command]
pub async fn clear_non_pinned_history(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    log::info!("Clearing non-pinned clipboard history (user requested)");
    with_storage(state.storage.clone(), |storage| {
        storage.clear_non_pinned().map_err(|e| e.to_string())
    })
    .await?;

    state.icon_cache.clear();
    update_tray_menu(&app);

    if let Err(e) = app.emit("history-cleared", ()) {
        log::error!("Failed to emit history-cleared event: {}", e);
    }

    Ok(())
}

pub async fn copy_clip_to_clipboard_internal(
    app: &AppHandle,
    clip_id: &str,
) -> Result<crate::paste::UseOutcome, String> {
    let state = app.state::<AppState>();
    let plain = state.settings.get().takes_plain_text();
    crate::paste::use_clips(
        app,
        state.inner(),
        crate::paste::UseRequest {
            ids: vec![clip_id.to_string()],
            mode: "copy".into(),
            plain,
            merge: false,
            hide: false,
        },
    )
    .await
}

#[cfg(not(target_os = "linux"))]
pub(crate) fn notify_copied(app: &AppHandle) {
    let chinese = app.state::<AppState>().settings.get().locale == "zh-CN";
    let body = if chinese {
        "内容已复制到剪贴板"
    } else {
        "Content copied to clipboard"
    };
    let _ = app
        .notification()
        .builder()
        .title(if chinese { "已复制" } else { "Copied" })
        .body(body)
        .show();
}

#[cfg(target_os = "linux")]
pub(crate) fn notify_copied(_app: &AppHandle) {}

#[tauri::command]
pub async fn copy_to_system_clipboard(
    app: AppHandle,
    clip_id: String,
) -> Result<crate::paste::UseOutcome, String> {
    copy_clip_to_clipboard_internal(&app, &clip_id).await
}

#[tauri::command]
pub async fn copy_plain_text(state: State<'_, AppState>, text: String) -> Result<(), String> {
    let _operation = state
        .clipboard_use_lock
        .try_lock()
        .map_err(|_| "A clipboard operation is already running".to_string())?;
    let marker = state.last_copied_by_us.clone();
    tauri::async_runtime::spawn_blocking(move || {
        crate::paste::write_plain_text_to_system_clipboard(&text, &marker)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn paste_clip(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
    mode: String,
    plain: bool,
) -> Result<crate::paste::UseOutcome, String> {
    crate::paste::use_clips(
        &app,
        state.inner(),
        crate::paste::UseRequest {
            ids: vec![id],
            mode,
            plain,
            merge: false,
            hide: true,
        },
    )
    .await
}

#[tauri::command]
pub async fn paste_clips(
    app: AppHandle,
    state: State<'_, AppState>,
    ids: Vec<String>,
    mode: String,
) -> Result<crate::paste::UseOutcome, String> {
    crate::paste::use_clips(
        &app,
        state.inner(),
        crate::paste::UseRequest {
            ids,
            mode,
            plain: true,
            merge: true,
            hide: true,
        },
    )
    .await
}

pub fn register_quickbar_shortcut(
    app: &AppHandle,
    shortcut: &str,
    foreground_store: crate::window::ForegroundWindowStore,
    panel_store: crate::window::QuickBarPanelStore,
    panel: crate::window::QuickBarPanel,
) -> Result<(), String> {
    let app_clone = app.clone();
    let shortcut_display = shortcut.to_string();

    app.global_shortcut()
        .on_shortcut(shortcut, move |_app, _shortcut, event| {
            if !matches!(event.state, ShortcutState::Pressed) {
                return;
            }
            log::info!("Global shortcut triggered: {}", shortcut_display);

            let app_for_action = app_clone.clone();
            let foreground_store = foreground_store.clone();
            let panel_store = panel_store.clone();
            if let Err(e) =
                run_action_on_main_thread(&app_clone, "show_quickbar_from_shortcut", move || {
                    crate::window::toggle_quickbar_with_panel(
                        &app_for_action,
                        &foreground_store,
                        &panel_store,
                        panel,
                    )
                })
            {
                log::error!("Failed to show QuickBar: {}", e);
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
    with_storage(state.storage.clone(), move |storage| {
        storage
            .set_clip_label(&id, label)
            .map_err(|e| e.to_string())
    })
    .await?;

    update_tray_menu(&app);
    Ok(())
}

#[tauri::command]
pub async fn update_clip_content(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
    content: String,
) -> Result<(), String> {
    if content.trim().is_empty() {
        return Err("Content cannot be empty".to_string());
    }
    let max_bytes = state.settings.get().max_text_bytes;
    if content.len() > max_bytes {
        return Err(format!(
            "Content exceeds max text limit ({max_bytes} bytes)"
        ));
    }
    let updated = with_storage(state.storage.clone(), move |storage| {
        storage.update_text_content(&id, &content)
    })
    .await?;
    if !updated {
        return Err("Clip not found or cannot be edited".to_string());
    }

    update_tray_menu(&app);
    Ok(())
}

#[tauri::command]
pub async fn create_pinned_clip(
    app: AppHandle,
    state: State<'_, AppState>,
    content: String,
    label: Option<String>,
) -> Result<FrontendClipItem, String> {
    if content.trim().is_empty() {
        return Err("Content cannot be empty".to_string());
    }
    let max_bytes = state.settings.get().max_text_bytes;
    if content.len() > max_bytes {
        return Err(format!(
            "Content exceeds max text limit ({max_bytes} bytes)"
        ));
    }
    let preview = with_storage(state.storage.clone(), move |storage| {
        storage
            .create_pinned_text(&content, label)
            .map_err(|e| e.to_string())
    })
    .await?;

    update_tray_menu(&app);
    Ok(FrontendClipItem::from_preview(preview))
}

#[tauri::command]
pub async fn batch_toggle_pin(
    app: AppHandle,
    state: State<'_, AppState>,
    ids: Vec<String>,
    is_pinned: bool,
) -> Result<usize, String> {
    if ids.len() > MAX_BATCH_IDS {
        return Err(format!(
            "Cannot process more than {MAX_BATCH_IDS} clips at once"
        ));
    }
    let limit = state.settings.get().max_history_items;
    let changed = with_storage(state.storage.clone(), move |storage| {
        storage
            .batch_update_pin(&ids, is_pinned, limit)
            .map_err(|e| e.to_string())
    })
    .await?;

    update_tray_menu(&app);
    Ok(changed)
}

#[tauri::command]
pub async fn batch_delete_clips(
    app: AppHandle,
    state: State<'_, AppState>,
    ids: Vec<String>,
) -> Result<(), String> {
    if ids.len() > MAX_BATCH_IDS {
        return Err(format!(
            "Cannot process more than {MAX_BATCH_IDS} clips at once"
        ));
    }
    with_storage(state.storage.clone(), move |storage| {
        storage.batch_delete(&ids).map_err(|e| e.to_string())
    })
    .await?;

    update_tray_menu(&app);
    Ok(())
}

#[tauri::command]
pub async fn open_url(app: AppHandle, url: String) -> Result<(), String> {
    use tauri_plugin_opener::OpenerExt;

    let trimmed = url.trim();
    if (!trimmed.starts_with("http://") && !trimmed.starts_with("https://"))
        || trimmed.chars().any(char::is_whitespace)
    {
        return Err("Only http:// and https:// URLs can be opened".to_string());
    }
    app.opener()
        .open_url(trimmed, None::<&str>)
        .map_err(|e| format!("Failed to open URL: {e}"))
}

#[tauri::command]
pub async fn reveal_in_folder(app: AppHandle, path: String) -> Result<(), String> {
    use tauri_plugin_opener::OpenerExt;

    let target = std::path::Path::new(&path);
    if !target.exists() {
        return Err("File or folder no longer exists".to_string());
    }
    app.opener()
        .reveal_item_in_dir(target)
        .map_err(|e| format!("Failed to reveal file: {e}"))
}

#[tauri::command]
pub async fn reorder_pinned(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
    direction: String,
) -> Result<(), String> {
    with_storage(state.storage.clone(), move |storage| {
        storage
            .reorder_pinned(&id, direction.as_str())
            .map_err(|e| e.to_string())
    })
    .await?;

    update_tray_menu(&app);
    Ok(())
}

#[tauri::command]
pub async fn open_settings_window(app: AppHandle) -> Result<(), String> {
    let app_for_action = app.clone();
    run_action_on_main_thread(&app, "open_settings_window", move || {
        crate::window::open_settings_window(&app_for_action)
    })
}

#[tauri::command]
pub async fn hide_quickbar(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    let foreground_store = state.quickbar_foreground_window.clone();
    let app_for_action = app.clone();
    run_action_on_main_thread(&app, "hide_quickbar", move || {
        crate::window::hide_quickbar_and_restore(&app_for_action, &foreground_store)
    })
}

#[tauri::command]
pub async fn show_quickbar(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    let foreground_store = state.quickbar_foreground_window.clone();
    let panel_store = state.quickbar_panel.clone();

    let app_for_action = app.clone();
    run_action_on_main_thread(&app, "show_quickbar", move || {
        crate::window::show_quickbar(&app_for_action, &foreground_store, &panel_store)
    })
}

#[tauri::command]
pub async fn set_quickbar_panel(state: State<'_, AppState>, panel: String) -> Result<(), String> {
    let panel = crate::window::QuickBarPanel::try_from(panel.as_str())?;
    crate::window::set_active_quickbar_panel(&state.quickbar_panel, panel);
    Ok(())
}

async fn fetch_update(app: &AppHandle) -> Result<Option<tauri_plugin_updater::Update>, String> {
    use tauri_plugin_updater::UpdaterExt;

    let updater = app
        .updater()
        .map_err(|e| format!("Failed to get updater: {}", e))?;

    updater
        .check()
        .await
        .map_err(|e| format!("Failed to check for updates: {}", e))
}

#[tauri::command]
pub async fn check_for_updates(app: AppHandle) -> Result<serde_json::Value, String> {
    log::info!("Checking for updates...");
    let current_version = app.package_info().version.to_string();

    match fetch_update(&app).await? {
        Some(update_info) => {
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
        }
        None => {
            log::info!("No updates available. Current version: {}", current_version);
            Ok(serde_json::json!({
                "available": false,
                "current_version": current_version
            }))
        }
    }
}

#[tauri::command]
pub async fn install_update(app: AppHandle) -> Result<(), String> {
    log::info!("Installing update...");

    let update_info = fetch_update(&app)
        .await?
        .ok_or_else(|| "No update available".to_string())?;

    log::info!("Downloading and installing update: {}", update_info.version);

    update_info
        .download_and_install(|_, _| {}, || {})
        .await
        .map_err(|e| format!("Failed to download/install update: {}", e))?;

    log::info!("Update installed successfully. Restarting app...");
    app.restart();
}

fn apply_shortcut_changes(
    app: &AppHandle,
    foreground_store: crate::window::ForegroundWindowStore,
    panel_store: crate::window::QuickBarPanelStore,
    old_shortcut: &str,
    old_pinned_shortcut: Option<&str>,
    new_shortcut: &str,
    new_pinned_shortcut: Option<&str>,
) -> Result<(), String> {
    let changes: Vec<_> = [
        (
            Some(old_shortcut),
            Some(new_shortcut),
            crate::window::QuickBarPanel::Recent,
        ),
        (
            old_pinned_shortcut,
            new_pinned_shortcut,
            crate::window::QuickBarPanel::Pinned,
        ),
    ]
    .into_iter()
    .filter(|(old, new, _)| old != new)
    .collect();

    for &(old, _, panel) in &changes {
        if let Some(old) = old {
            if let Err(e) = app.global_shortcut().unregister(old) {
                log::warn!("Failed to unregister old {panel:?} shortcut '{old}': {e}");
            }
        }
    }

    let result = changes.iter().try_for_each(|&(_, new, panel)| match new {
        Some(new) => register_quickbar_shortcut(
            app,
            new,
            foreground_store.clone(),
            panel_store.clone(),
            panel,
        ),
        None => Ok(()),
    });

    if result.is_err() {
        for &(_, new, _) in &changes {
            if let Some(new) = new {
                let _ = app.global_shortcut().unregister(new);
            }
        }
        for &(old, _, panel) in &changes {
            if let Some(old) = old {
                if let Err(e) = register_quickbar_shortcut(
                    app,
                    old,
                    foreground_store.clone(),
                    panel_store.clone(),
                    panel,
                ) {
                    log::warn!("Failed to restore old {panel:?} shortcut '{old}': {e}");
                }
            }
        }
    }

    result
}

pub fn migrate_legacy_autostart(app: &AppHandle, reregister: bool) -> Result<(), String> {
    use tauri_plugin_autostart::ManagerExt;

    let current_exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let legacy = auto_launch::AutoLaunchBuilder::new()
        .set_app_name("ClipMan")
        .set_app_path(&current_exe.display().to_string())
        .set_use_launch_agent(true)
        .build()
        .map_err(|e| e.to_string())?;
    let legacy_enabled = legacy.is_enabled().map_err(|e| e.to_string())?;
    if legacy_enabled {
        legacy.disable().map_err(|e| e.to_string())?;
    }
    if legacy_enabled || reregister {
        app.autolaunch().enable().map_err(|e| e.to_string())?;
        log::info!(
            "Registered the ClipLeaf autostart entry at {}",
            current_exe.display()
        );
    }
    Ok(())
}

fn apply_autostart_setting(app: &AppHandle, enable_autostart: bool) -> Result<(), String> {
    use tauri_plugin_autostart::ManagerExt;

    let result = if enable_autostart {
        app.autolaunch().enable()
    } else {
        app.autolaunch().disable()
    };

    result.map_err(|e| format!("Failed to update autostart: {}", e))
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsUpdateResult {
    settings: Settings,
    warning: Option<String>,
}

#[tauri::command]
pub async fn update_settings(
    app: AppHandle,
    settings: Option<Settings>,
) -> Result<SettingsUpdateResult, String> {
    tauri::async_runtime::spawn_blocking(move || {
        update_settings_blocking(&app, app.state::<AppState>().inner(), settings)
    })
    .await
    .map_err(|e| e.to_string())?
}

fn update_settings_blocking(
    app: &AppHandle,
    state: &AppState,
    settings: Option<Settings>,
) -> Result<SettingsUpdateResult, String> {
    let mut settings = settings.unwrap_or_default().validate_and_normalize()?;
    let _settings_write_guard = state.settings_write_lock.lock().unwrap();
    log::info!("Updating settings: {:?}", settings);

    let old = state.settings.get();

    settings.custom_data_path = old.custom_data_path.clone();
    settings.capture_paused = old.capture_paused;

    let autostart_changed = old.enable_autostart != settings.enable_autostart;
    let foreground_store = state.quickbar_foreground_window.clone();
    let panel_store = state.quickbar_panel.clone();
    let rollback_autostart = |reason: &str| {
        if autostart_changed {
            if let Err(rollback_error) = apply_autostart_setting(app, old.enable_autostart) {
                log::warn!("Failed to roll back autostart after {reason}: {rollback_error}");
            }
        }
    };

    if autostart_changed {
        apply_autostart_setting(app, settings.enable_autostart)?;
        log::info!(
            "Autostart {} successfully",
            if settings.enable_autostart {
                "enabled"
            } else {
                "disabled"
            }
        );
    }

    if let Err(e) = apply_shortcut_changes(
        app,
        foreground_store.clone(),
        panel_store.clone(),
        &old.global_shortcut,
        old.pinned_shortcut.as_deref(),
        &settings.global_shortcut,
        settings.pinned_shortcut.as_deref(),
    ) {
        rollback_autostart("shortcut update failed");
        return Err(e);
    }

    if let Err(e) = state.settings.save(app, &settings) {
        if let Err(rollback_error) = apply_shortcut_changes(
            app,
            foreground_store,
            panel_store,
            &settings.global_shortcut,
            settings.pinned_shortcut.as_deref(),
            &old.global_shortcut,
            old.pinned_shortcut.as_deref(),
        ) {
            log::warn!(
                "Failed to roll back shortcuts after settings save failed: {}",
                rollback_error
            );
        }
        rollback_autostart("settings save failed");
        return Err(e);
    }

    state.settings.set(settings.clone());
    let _ = app.emit("settings-changed", ());

    if old.tray_text_length != settings.tray_text_length
        || old.max_pinned_in_tray != settings.max_pinned_in_tray
        || old.max_recent_in_tray != settings.max_recent_in_tray
        || old.locale != settings.locale
    {
        log::info!("Tray settings changed, rebuilding menu...");
        update_tray_menu(app);
    }

    let warning = if old.max_history_items != settings.max_history_items {
        match state
            .storage
            .lock()
            .unwrap()
            .enforce_history_limit(settings.max_history_items)
        {
            Ok(removed) => {
                if removed > 0 {
                    let _ = app.emit("history-cleared", ());
                }
                None
            }
            Err(error) => Some(format!(
                "Settings saved, but history cleanup failed: {error}"
            )),
        }
    } else {
        None
    };
    Ok(SettingsUpdateResult { settings, warning })
}

#[tauri::command]
pub async fn get_current_data_path(state: State<'_, AppState>) -> Result<String, String> {
    with_storage(state.storage.clone(), |storage| {
        storage
            .data_directory()
            .to_str()
            .map(str::to_owned)
            .ok_or_else(|| "Invalid data path".into())
    })
    .await
}

#[tauri::command]
pub async fn disable_global_shortcut(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let settings = state.settings.get();
    let current_shortcut = settings.global_shortcut;

    app.global_shortcut()
        .unregister(current_shortcut.as_str())
        .map_err(|e| format!("Failed to disable shortcut: {}", e))?;

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
    let panel_store = state.quickbar_panel.clone();

    if !app
        .global_shortcut()
        .is_registered(current_shortcut.as_str())
    {
        register_quickbar_shortcut(
            &app,
            current_shortcut.as_str(),
            foreground_store.clone(),
            panel_store.clone(),
            crate::window::QuickBarPanel::Recent,
        )
        .map_err(|e| format!("Failed to re-enable shortcut: {}", e))?;
    }

    log::info!("Global shortcut '{}' re-enabled", current_shortcut);

    if let Some(pinned_shortcut) = settings.pinned_shortcut {
        if !app
            .global_shortcut()
            .is_registered(pinned_shortcut.as_str())
        {
            register_quickbar_shortcut(
                &app,
                pinned_shortcut.as_str(),
                foreground_store,
                panel_store,
                crate::window::QuickBarPanel::Pinned,
            )
            .map_err(|e| format!("Failed to re-enable pinned shortcut: {}", e))?;
            log::info!("Pinned shortcut '{}' re-enabled", pinned_shortcut);
        }
    }

    Ok(())
}

#[tauri::command]
pub async fn open_folder(app: AppHandle, path: String) -> Result<(), String> {
    use tauri_plugin_opener::OpenerExt;

    app.opener()
        .open_path(&path, None::<&str>)
        .map_err(|e| format!("Failed to open folder: {e}"))
}

#[tauri::command]
pub async fn migrate_data_location(
    app: AppHandle,
    new_path: String,
    delete_old: bool,
) -> Result<Option<String>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        migrate_data_location_blocking(&app, app.state::<AppState>().inner(), new_path, delete_old)
    })
    .await
    .map_err(|e| e.to_string())?
}

fn migrate_data_location_blocking(
    app: &AppHandle,
    state: &AppState,
    new_path: String,
    delete_old: bool,
) -> Result<Option<String>, String> {
    log::info!(
        "Starting data migration to: {}, delete_old: {}",
        new_path,
        delete_old
    );

    let _settings_write_guard = state.settings_write_lock.lock().unwrap();
    let old_path = state.storage.lock().unwrap().data_directory().to_path_buf();
    let new_path_buf = std::path::PathBuf::from(&new_path);
    let new_db_path = new_path_buf.join(crate::identity::DATABASE_FILE);

    migration::prepare_destination_directory(&old_path, &new_path_buf)?;

    {
        let mut storage = state.storage.lock().unwrap();
        storage
            .backup_to_path(&new_db_path)
            .map_err(|e| format!("Failed to back up database: {}", e))?;
        let new_storage = ClipStorage::new(&new_db_path).map_err(|e| e.to_string())?;

        let mut new_settings = state.settings.get();
        new_settings.custom_data_path = Some(new_path.clone());
        state
            .settings
            .save(app, &new_settings)
            .map_err(|e| format!("Failed to save settings: {}", e))?;

        *storage = new_storage;
        state.settings.set(new_settings);
    }

    let warning = delete_old
        .then(|| migration::remove_data_files(&old_path).err())
        .flatten()
        .map(|e| format!("Data migration completed, but failed to remove old data: {e}"));

    if let Some(warning) = &warning {
        log::warn!("{warning}");
    } else {
        log::info!("Data migration completed successfully");
    }
    update_tray_menu(app);
    Ok(warning)
}
