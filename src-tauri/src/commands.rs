use std::sync::{Arc, Mutex};

use tauri::{AppHandle, Emitter, Manager, State};

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
pub async fn open_accessibility_settings(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    crate::accessibility::request_access(&app)?;
    if !state.settings.get().onboarding_completed {
        crate::accessibility::show_onboarding_when_trusted(&app);
    }
    Ok(())
}

#[tauri::command]
pub async fn clear_non_pinned_history(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<Option<String>, String> {
    log::info!("Clearing non-pinned clipboard history (user requested)");
    let wal_truncated = with_storage(state.storage.clone(), |storage| {
        storage.clear_non_pinned().map_err(|e| e.to_string())
    })
    .await?;

    state.icon_cache.clear();
    update_tray_menu(&app);

    if let Err(e) = app.emit("history-cleared", ()) {
        log::error!("Failed to emit history-cleared event: {}", e);
    }

    if wal_truncated {
        return Ok(None);
    }
    log::warn!("History cleared, but another connection kept the WAL from being truncated");
    Ok(Some(
        "History was cleared, but another program is reading the database, so the deleted items \
         remain in its write-ahead log. Close that program and clear history again."
            .to_string(),
    ))
}

#[tauri::command]
pub async fn copy_to_system_clipboard(
    app: AppHandle,
    clip_id: String,
) -> Result<crate::paste::UseOutcome, String> {
    crate::paste::copy_clip_to_clipboard_internal(&app, &clip_id).await
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
pub async fn open_settings_window(app: AppHandle, tab: Option<String>) -> Result<(), String> {
    let app_for_action = app.clone();
    crate::window::run_action_on_main_thread(&app, "open_settings_window", move || {
        crate::window::open_settings_window(&app_for_action, tab.as_deref())
    })
}

#[tauri::command]
pub async fn hide_quickbar(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    let foreground_store = state.quickbar_foreground_window.clone();
    let app_for_action = app.clone();
    crate::window::run_action_on_main_thread(&app, "hide_quickbar", move || {
        crate::window::hide_quickbar_and_restore(&app_for_action, &foreground_store)
    })
}

#[tauri::command]
pub async fn show_quickbar(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    let foreground_store = state.quickbar_foreground_window.clone();
    let panel_store = state.quickbar_panel.clone();

    let app_for_action = app.clone();
    crate::window::run_action_on_main_thread(&app, "show_quickbar", move || {
        crate::window::show_quickbar(&app_for_action, &foreground_store, &panel_store)
    })
}

#[tauri::command]
pub async fn set_quickbar_panel(state: State<'_, AppState>, panel: String) -> Result<(), String> {
    let panel = crate::window::QuickBarPanel::try_from(panel.as_str())?;
    crate::window::set_active_quickbar_panel(&state.quickbar_panel, panel);
    Ok(())
}

#[tauri::command]
pub async fn check_for_updates(app: AppHandle) -> Result<crate::updates::UpdateInfo, String> {
    crate::updates::check(&app).await
}

#[tauri::command]
pub async fn get_update_status(app: AppHandle) -> Result<crate::updates::UpdateInfo, String> {
    Ok(crate::updates::status(&app))
}

#[tauri::command]
pub async fn install_update(app: AppHandle) -> Result<(), String> {
    crate::updates::install(&app).await
}

#[tauri::command]
pub fn restart_app(app: AppHandle) {
    app.restart();
}

#[tauri::command]
pub async fn complete_onboarding(app: AppHandle) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        crate::settings_update::complete_onboarding(&app, app.state::<AppState>().inner())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn update_settings(
    app: AppHandle,
    settings: Option<Settings>,
) -> Result<crate::settings_update::SettingsUpdateResult, String> {
    tauri::async_runtime::spawn_blocking(move || {
        crate::settings_update::update_settings_blocking(
            &app,
            app.state::<AppState>().inner(),
            settings,
        )
    })
    .await
    .map_err(|e| e.to_string())?
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
    crate::shortcuts::disable_quickbar_shortcuts(&app, state.settings.get())
}

#[tauri::command]
pub async fn enable_global_shortcut(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    crate::shortcuts::enable_quickbar_shortcuts(
        &app,
        state.settings.get(),
        state.quickbar_foreground_window.clone(),
        state.quickbar_panel.clone(),
    )
}

#[tauri::command]
pub async fn get_storage_usage(
    state: State<'_, AppState>,
) -> Result<crate::storage::StorageUsage, String> {
    with_storage(state.storage.clone(), |storage| {
        storage.usage().map_err(|e| e.to_string())
    })
    .await
}

#[tauri::command]
pub async fn list_old_backups(
    state: State<'_, AppState>,
) -> Result<Vec<crate::storage::BackupFile>, String> {
    with_storage(state.storage.clone(), |storage| {
        storage.old_backups().map_err(|e| e.to_string())
    })
    .await
}

#[tauri::command]
pub async fn delete_old_backups(state: State<'_, AppState>) -> Result<usize, String> {
    with_storage(state.storage.clone(), |storage| {
        storage.delete_old_backups().map_err(|e| e.to_string())
    })
    .await
}

#[tauri::command]
pub async fn export_favorites(state: State<'_, AppState>, path: String) -> Result<usize, String> {
    with_storage(state.storage.clone(), move |storage| {
        storage.export_pinned_to(std::path::Path::new(&path))
    })
    .await
}

#[derive(serde::Serialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ImportResult {
    imported: usize,
    skipped: usize,
}

#[tauri::command]
pub async fn import_favorites(
    app: AppHandle,
    state: State<'_, AppState>,
    path: String,
) -> Result<ImportResult, String> {
    let (imported, skipped) = with_storage(state.storage.clone(), move |storage| {
        let bytes = std::fs::read(&path).map_err(|e| format!("Failed to read {path}: {e}"))?;
        let file = serde_json::from_slice(&bytes)
            .map_err(|e| format!("{path} is not a ClipLeaf favorites file: {e}"))?;
        storage.import_pinned(file)
    })
    .await?;
    if imported > 0 {
        update_tray_menu(&app);
        let _ = app.emit("clips-used", ());
    }
    Ok(ImportResult { imported, skipped })
}

#[tauri::command]
pub async fn get_diagnostics(app: AppHandle, state: State<'_, AppState>) -> Result<String, String> {
    let usage = with_storage(state.storage.clone(), |storage| {
        storage.usage().map_err(|e| e.to_string())
    })
    .await?;
    crate::diagnostics::report(&app, &state.settings.get(), &usage)
}

#[tauri::command]
pub async fn open_log_folder(app: AppHandle) -> Result<(), String> {
    use tauri_plugin_opener::OpenerExt;

    let log_dir = app.path().app_log_dir().map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&log_dir).map_err(|e| e.to_string())?;
    app.opener()
        .open_path(log_dir.to_string_lossy(), None::<&str>)
        .map_err(|e| format!("Failed to open the log folder: {e}"))
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
        migration::migrate_data_location(
            &app,
            app.state::<AppState>().inner(),
            new_path,
            delete_old,
        )
    })
    .await
    .map_err(|e| e.to_string())?
}
