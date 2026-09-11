// Tauri commands module
use std::sync::{mpsc, Arc, Mutex};
use std::time::Duration;

use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};
#[cfg(not(target_os = "linux"))]
use tauri_plugin_notification::NotificationExt;

use crate::settings::Settings;
use crate::storage::{ClipStorage, FrontendClipItem};
use crate::tray::update_tray_menu;
use crate::{migration, safe_lock, AppState};

/// Run a blocking storage operation on the blocking thread pool, locking the
/// shared `ClipStorage` for the duration. Collapses the nine near-identical
/// `spawn_blocking` + `safe_lock` + error-flatten blocks the read/write
/// commands used to repeat.
async fn with_storage<T, F>(storage: Arc<Mutex<ClipStorage>>, op: F) -> Result<T, String>
where
    F: FnOnce(&ClipStorage) -> Result<T, String> + Send + 'static,
    T: Send + 'static,
{
    let queued = std::time::Instant::now();
    tauri::async_runtime::spawn_blocking(move || {
        let lock_started = std::time::Instant::now();
        let storage = safe_lock(&storage);
        let operation_started = std::time::Instant::now();
        let result = op(&storage);
        log::debug!(
            "storage: queue {:?}, lock {:?}, operation {:?}",
            lock_started.duration_since(queued),
            operation_started.duration_since(lock_started),
            operation_started.elapsed()
        );
        result
    })
    .await
    .map_err(|e| e.to_string())?
}

type MainThreadAction = Box<dyn FnOnce() -> Result<(), String> + Send>;

fn run_action_on_main_thread(
    app: &AppHandle,
    command_name: &'static str,
    action: MainThreadAction,
) -> Result<(), String> {
    let (sender, receiver) = mpsc::channel();

    app.run_on_main_thread(move || {
        let _ = sender.send(action());
    })
    .map_err(|e| format!("Failed to schedule {command_name} on main thread: {e}"))?;

    receiver
        .recv_timeout(Duration::from_secs(5))
        .map_err(|e| format!("Timed out waiting for {command_name} on main thread: {e}"))?
}

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
    let app_for_action = app.clone();
    run_action_on_main_thread(app, command_name, Box::new(move || action(app_for_action)))
}

fn restart_clipboard_monitor(app: &AppHandle, state: &AppState) -> Result<(), String> {
    let mut monitor =
        crate::clipboard::ClipboardMonitor::new(app.clone(), state.last_copied_by_us.clone());
    match monitor.start() {
        Ok(()) => {
            *safe_lock(&state.monitor) = Some(monitor);
            log::info!("Clipboard monitoring restarted after migration");
            Ok(())
        }
        Err(e) => {
            log::error!(
                "Failed to restart clipboard monitoring after migration: {}",
                e
            );
            Err(e)
        }
    }
}

#[tauri::command]
pub async fn get_recent_clips(
    state: State<'_, AppState>,
    limit: Option<usize>,
    before_timestamp: Option<f64>,
    before_id: Option<String>,
) -> Result<Vec<FrontendClipItem>, String> {
    let limit = limit.unwrap_or(100);

    with_storage(state.storage.clone(), move |storage| {
        // Both cursor parts must be present to page; a missing pair (the old
        // signature, or a first-page request) falls back to the first page.
        let before = match (before_timestamp, before_id.as_deref()) {
            (Some(timestamp), Some(id)) => Some((timestamp, id)),
            _ => None,
        };
        let items = storage
            .get_recent_clip_previews_page(limit, before)
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
            .get_pinned_clip_previews()
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
pub async fn search_clips(
    state: State<'_, AppState>,
    query: String,
) -> Result<Vec<FrontendClipItem>, String> {
    with_storage(state.storage.clone(), move |storage| {
        let items = storage
            .search_clip_previews(&query)
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
pub async fn check_clipboard_permission() -> Result<String, String> {
    use arboard::{Clipboard, Error};

    match Clipboard::new() {
        Ok(mut clipboard) => match clipboard.get_text() {
            Ok(_) => Ok("granted".to_string()),
            Err(Error::ContentNotAvailable) => Ok("granted".to_string()),
            Err(e) => Ok(format!("denied: {}", e)),
        },
        Err(e) => Err(format!("Failed to create clipboard: {}", e)),
    }
}

/// Whether ClipMan currently holds the macOS Accessibility permission needed to
/// simulate Cmd+V. Always `true` on non-macOS platforms.
#[tauri::command]
pub async fn check_accessibility_permission() -> Result<bool, String> {
    Ok(crate::accessibility::is_trusted())
}

/// Open System Settings → Privacy & Security → Accessibility so the user can
/// (re-)grant ClipMan. No-op on non-macOS platforms.
#[tauri::command]
pub async fn open_accessibility_settings() -> Result<(), String> {
    crate::accessibility::open_settings()
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

    // Emit event to notify frontend
    if let Err(e) = app.emit("history-cleared", ()) {
        log::error!("Failed to emit history-cleared event: {}", e);
    }

    Ok(())
}

/// Copy a clip to the system clipboard (used by the tray menu and the in-window
/// Copy button). Reuses the paste module's clipboard writer so there is a single
/// implementation of "write clipboard, then update history".
pub async fn copy_clip_to_clipboard_internal(
    app: &AppHandle,
    clip_id: &str,
    show_notification: bool,
) -> Result<crate::paste::UseOutcome, String> {
    let state = app.state::<AppState>();
    // Copy-only writes follow the paste-format mode so a manual paste into
    // the target app also lands without formatting (takePlain/globalPlain).
    let plain = state.settings.get().takes_plain_text();
    let outcome = crate::paste::use_clips(
        app,
        state.inner(),
        crate::paste::UseRequest {
            ids: vec![clip_id.to_string()],
            mode: "copy".into(),
            plain,
            separator: None,
            hide: false,
        },
    )
    .await?;
    if show_notification {
        notify_copied(app);
    }

    Ok(outcome)
}

#[cfg(not(target_os = "linux"))]
fn notify_copied(app: &AppHandle) {
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
fn notify_copied(_app: &AppHandle) {}

#[tauri::command]
pub async fn copy_to_system_clipboard(
    app: AppHandle,
    clip_id: String,
) -> Result<crate::paste::UseOutcome, String> {
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
    plain: Option<bool>,
) -> Result<crate::paste::UseOutcome, String> {
    // `plain` is optional: absent => the paste-format mode decides (takePlain
    // and globalPlain take text as plain); the frontend sends an explicit flag
    // (Enter follows the mode, ⌥Enter inverts it for one paste).
    crate::paste::use_clips(
        &app,
        state.inner(),
        crate::paste::UseRequest {
            ids: vec![id],
            mode,
            plain: plain.unwrap_or_else(|| state.settings.get().takes_plain_text()),
            separator: None,
            hide: true,
        },
    )
    .await
}

/// Merge several clips (in `ids` order) into a single `separator`-joined text
/// write, then paste per `mode` (task #13 multi-select). Image clips are skipped
/// (v1). The frontend only sends `"\n"` today; the separator is honored verbatim.
#[tauri::command]
pub async fn paste_clips(
    app: AppHandle,
    state: State<'_, AppState>,
    ids: Vec<String>,
    mode: String,
    separator: String,
) -> Result<crate::paste::UseOutcome, String> {
    crate::paste::use_clips(
        &app,
        state.inner(),
        crate::paste::UseRequest {
            ids,
            mode,
            plain: true,
            separator: Some(separator),
            hide: true,
        },
    )
    .await
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
            let app_for_action = app_clone.clone();
            let foreground_store = foreground_store.clone();
            let result = handle_quickbar_shortcut_event(
                event.state,
                &shortcut_display,
                |command_name, action| run_action_on_main_thread(&app_clone, command_name, action),
                move || {
                    crate::window::show_quickbar_with_panel(
                        &app_for_action,
                        &foreground_store,
                        panel,
                    )
                },
            );

            if let Some(Err(e)) = result {
                log::error!("Failed to show QuickBar: {}", e);
            }
        })
        .map_err(|e| format!("Failed to register shortcut '{}': {}", shortcut, e))
}

fn handle_quickbar_shortcut_event<S, A>(
    state: ShortcutState,
    shortcut_display: &str,
    schedule_on_main_thread: S,
    action: A,
) -> Option<Result<(), String>>
where
    S: FnOnce(&'static str, MainThreadAction) -> Result<(), String>,
    A: FnOnce() -> Result<(), String> + Send + 'static,
{
    if !matches!(state, ShortcutState::Pressed) {
        return None;
    }

    log::info!("Global shortcut triggered: {}", shortcut_display);
    Some(schedule_on_main_thread(
        "show_quickbar_from_shortcut",
        Box::new(action),
    ))
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

/// Acquire the updater and check the remote endpoint once. Shared by
/// `check_for_updates` and `install_update` so both surface identical
/// "no updater" / "check failed" errors.
async fn fetch_update(app: &AppHandle) -> Result<Option<tauri_plugin_updater::Update>, String> {
    use tauri_plugin_updater::UpdaterExt;

    let updater = app.updater().map_err(|e| {
        log::error!("Failed to get updater: {}", e);
        format!("Failed to get updater: {}", e)
    })?;

    updater.check().await.map_err(|e| {
        log::error!("Failed to check for updates: {}", e);
        format!("Failed to check for updates: {}", e)
    })
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
        .map_err(|e| {
            log::error!("Failed to download/install update: {}", e);
            format!("Failed to download/install update: {}", e)
        })?;

    log::info!("Update installed successfully. Restarting app...");
    app.restart();
}

fn unregister_shortcut_if_active(app: &AppHandle, shortcut: &str, label: &str) {
    if let Err(e) = app.global_shortcut().unregister(shortcut) {
        log::warn!(
            "Failed to unregister {label} shortcut '{}': {}",
            shortcut,
            e
        );
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

fn shortcut_cleanup_targets<'a>(
    main_changed: bool,
    pinned_changed: bool,
    new_shortcut: &'a str,
    new_pinned_shortcut: Option<&'a str>,
) -> [Option<&'a str>; 2] {
    [
        main_changed.then_some(new_shortcut),
        if pinned_changed {
            new_pinned_shortcut
        } else {
            None
        },
    ]
}

fn apply_shortcut_changes(
    app: &AppHandle,
    foreground_store: crate::window::ForegroundWindowStore,
    old_shortcut: &str,
    old_pinned_shortcut: Option<&str>,
    new_shortcut: &str,
    new_pinned_shortcut: Option<&str>,
) -> Result<(), String> {
    let main_changed = old_shortcut != new_shortcut;
    let pinned_changed = old_pinned_shortcut != new_pinned_shortcut;

    if !main_changed && !pinned_changed {
        return Ok(());
    }

    // Unregister every changed old binding first (frees the keys, so swapping
    // main and pinned needs no special casing), then register the new ones.
    if main_changed {
        unregister_shortcut_if_active(app, old_shortcut, "old main");
    }
    if pinned_changed {
        if let Some(old_pinned) = old_pinned_shortcut {
            unregister_shortcut_if_active(app, old_pinned, "old pinned");
        }
    }

    let result = (|| -> Result<(), String> {
        if main_changed {
            register_quickbar_shortcut(
                app,
                new_shortcut,
                foreground_store.clone(),
                crate::window::QuickBarPanel::Recent,
            )?;
        }
        if pinned_changed {
            if let Some(new_pinned) = new_pinned_shortcut {
                register_quickbar_shortcut(
                    app,
                    new_pinned,
                    foreground_store.clone(),
                    crate::window::QuickBarPanel::Pinned,
                )?;
            }
        }
        Ok(())
    })();

    if result.is_err() {
        // Best effort: drop whatever new bindings landed, restore the old
        // ones. `restore_shortcut` only logs on failure.
        for shortcut in shortcut_cleanup_targets(
            main_changed,
            pinned_changed,
            new_shortcut,
            new_pinned_shortcut,
        )
        .into_iter()
        .flatten()
        {
            let _ = app.global_shortcut().unregister(shortcut);
        }
        if main_changed {
            restore_shortcut(
                app,
                old_shortcut,
                foreground_store.clone(),
                crate::window::QuickBarPanel::Recent,
                "old main",
            );
        }
        if pinned_changed {
            if let Some(old_pinned) = old_pinned_shortcut {
                restore_shortcut(
                    app,
                    old_pinned,
                    foreground_store,
                    crate::window::QuickBarPanel::Pinned,
                    "old pinned",
                );
            }
        }
    }

    result
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
    // An explicit null resets through the same validation and rollback path.
    let mut settings = settings.unwrap_or_default().validate_and_normalize()?;
    let _settings_write_guard = safe_lock(&state.settings_write_lock);
    log::info!("Updating settings: {:?}", settings);

    let old_settings = state.settings.get();
    let history_limit_changed = old_settings.max_history_items != settings.max_history_items;

    // Two fields are owned by other subsystems, not the settings page, so a
    // stale/reset settings object must never write over them here:
    //   * `custom_data_path` is owned exclusively by `migrate_data_location`,
    //     which relocates the database alongside changing the path. A settings
    //     save (or a reset payload sending `null`) that repointed it here would
    //     silently switch the app to a different/empty directory and strand the
    //     existing database. Keep whatever migration last set.
    //   * `capture_paused` is owned exclusively by the tray "Pause Capture"
    //     toggle. A stale settings window saving would otherwise clobber the
    //     tray's current pause state.
    settings.custom_data_path = old_settings.custom_data_path.clone();
    settings.capture_paused = old_settings.capture_paused;
    let old_shortcut = old_settings.global_shortcut;
    let old_pinned_shortcut = old_settings.pinned_shortcut;
    let old_tray_text_length = old_settings.tray_text_length;
    let old_max_pinned_in_tray = old_settings.max_pinned_in_tray;
    let old_max_recent_in_tray = old_settings.max_recent_in_tray;
    let old_autostart = old_settings.enable_autostart;
    let old_locale = old_settings.locale;
    let new_shortcut = settings.global_shortcut.clone();
    let new_pinned_shortcut = settings.pinned_shortcut.clone();

    let shortcut_changed = old_shortcut != new_shortcut;
    let pinned_shortcut_changed = old_pinned_shortcut != new_pinned_shortcut;
    let tray_text_changed = old_tray_text_length != settings.tray_text_length;
    let tray_limits_changed = old_max_pinned_in_tray != settings.max_pinned_in_tray
        || old_max_recent_in_tray != settings.max_recent_in_tray;
    let autostart_changed = old_autostart != settings.enable_autostart;
    let locale_changed = old_locale != settings.locale;
    let quickbar_foreground_window = state.quickbar_foreground_window.clone();

    // Update autostart if changed
    if autostart_changed {
        if let Err(e) = apply_autostart_setting(app, settings.enable_autostart) {
            log::error!("{}", e);
            return Err(e);
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
        if let Err(e) = apply_shortcut_changes(
            app,
            quickbar_foreground_window.clone(),
            old_shortcut.as_str(),
            old_pinned_shortcut.as_deref(),
            new_shortcut.as_str(),
            new_pinned_shortcut.as_deref(),
        ) {
            if autostart_changed {
                if let Err(rollback_error) = apply_autostart_setting(app, old_autostart) {
                    log::warn!(
                        "Failed to roll back autostart after shortcut update failed: {}",
                        rollback_error
                    );
                }
            }
            return Err(e);
        }
    }

    if let Err(e) = state.settings.save_candidate(app, &settings) {
        if shortcut_changed || pinned_shortcut_changed {
            if let Err(rollback_error) = apply_shortcut_changes(
                app,
                quickbar_foreground_window,
                new_shortcut.as_str(),
                new_pinned_shortcut.as_deref(),
                old_shortcut.as_str(),
                old_pinned_shortcut.as_deref(),
            ) {
                log::warn!(
                    "Failed to roll back shortcuts after settings save failed: {}",
                    rollback_error
                );
            }
        }
        if autostart_changed {
            if let Err(rollback_error) = apply_autostart_setting(app, old_autostart) {
                log::warn!(
                    "Failed to roll back autostart after settings save failed: {}",
                    rollback_error
                );
            }
        }
        return Err(e);
    }

    state.settings.set(settings.clone());
    let _ = app.emit("settings-changed", ());

    // Rebuild tray menu if visible tray settings changed.
    if tray_text_changed || tray_limits_changed || locale_changed {
        log::info!("Tray settings changed, rebuilding menu...");
        update_tray_menu(app);
    }

    let warning = if history_limit_changed {
        match safe_lock(&state.storage).enforce_history_limit(settings.max_history_items) {
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

    if !app
        .global_shortcut()
        .is_registered(current_shortcut.as_str())
    {
        register_quickbar_shortcut(
            &app,
            current_shortcut.as_str(),
            foreground_store.clone(),
            crate::window::QuickBarPanel::Recent,
        )
        .map_err(|e| format!("Failed to re-enable shortcut: {}", e))?;
    }

    log::info!("Global shortcut '{}' re-enabled", current_shortcut);

    if let Some(pinned_shortcut) = settings.pinned_shortcut {
        if pinned_shortcut != current_shortcut
            && !app
                .global_shortcut()
                .is_registered(pinned_shortcut.as_str())
        {
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
    use crate::storage::ClipStorage;

    log::info!(
        "Starting data migration to: {}, delete_old: {}",
        new_path,
        delete_old
    );

    let _settings_write_guard = safe_lock(&state.settings_write_lock);
    let settings = state.settings.get();
    let old_path = safe_lock(&state.storage).data_directory().to_path_buf();
    let new_path_buf = std::path::PathBuf::from(&new_path);
    let new_db_path = new_path_buf.join("clipman.db");

    migration::prepare_destination_directory(&old_path, &new_path_buf)?;

    let was_running = safe_lock(&state.monitor).take().map(|m| m.stop()).is_some();
    if was_running {
        log::info!("Clipboard monitoring stopped for migration");
    }
    let migration_result = (|| -> Result<Option<String>, String> {
        let mut storage_guard = safe_lock(&state.storage);
        storage_guard
            .backup_to_path(&new_db_path)
            .map_err(|e| format!("Failed to back up database: {}", e))?;
        let new_storage = ClipStorage::new(&new_db_path).map_err(|e| e.to_string())?;

        let mut new_settings = settings.clone();
        new_settings.custom_data_path = Some(new_path.clone());
        state
            .settings
            .save_candidate(app, &new_settings)
            .map_err(|e| format!("Failed to save settings: {}", e))?;

        *storage_guard = new_storage;
        state.settings.set(new_settings);
        drop(storage_guard);

        // Now it is safe to delete the old files; Windows refuses deleting open DBs.
        let cleanup_warning = delete_old
            .then(|| migration::remove_data_files(&old_path).err())
            .flatten()
            .map(|e| format!("Data migration completed, but failed to remove old data: {e}"));

        if let Some(warning) = &cleanup_warning {
            log::warn!("{warning}");
        } else {
            log::info!("Data migration completed successfully");
        }
        Ok(cleanup_warning)
    })();

    let restart_result = if was_running {
        restart_clipboard_monitor(app, state)
    } else {
        Ok(())
    };
    crate::tray::update_tray_menu(app);

    match (migration_result, restart_result) {
        (Ok(warning), Ok(())) => Ok(warning),
        (Ok(cleanup_warning), Err(e)) => {
            let restart_warning = format!(
                "Data migration completed, but clipboard monitoring failed to restart: {}",
                e
            );
            Ok(Some(
                cleanup_warning
                    .map(|warning| format!("{warning}; additionally: {restart_warning}"))
                    .unwrap_or(restart_warning),
            ))
        }
        (Err(e), Ok(())) => Err(e),
        (Err(migration_error), Err(restart_error)) => Err(format!(
            "{}; additionally clipboard monitoring failed to restart: {}",
            migration_error, restart_error
        )),
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use tauri_plugin_global_shortcut::ShortcutState;

    #[test]
    fn shortcut_cleanup_only_targets_changed_bindings() {
        assert_eq!(
            super::shortcut_cleanup_targets(true, false, "new-main", Some("unchanged-pinned")),
            [Some("new-main"), None]
        );
        assert_eq!(
            super::shortcut_cleanup_targets(false, true, "unchanged-main", Some("new-pinned")),
            [None, Some("new-pinned")]
        );
        assert_eq!(
            super::shortcut_cleanup_targets(true, true, "new-main", Some("new-pinned")),
            [Some("new-main"), Some("new-pinned")]
        );
    }

    #[test]
    fn pressed_quickbar_shortcut_uses_main_thread_scheduler() {
        let scheduled_commands = Arc::new(Mutex::new(Vec::new()));
        let action_ran = Arc::new(Mutex::new(false));

        let scheduled_commands_for_scheduler = scheduled_commands.clone();
        let action_ran_for_action = action_ran.clone();

        let result = super::handle_quickbar_shortcut_event(
            ShortcutState::Pressed,
            "CommandOrControl+Shift+V",
            move |command_name, action| {
                scheduled_commands_for_scheduler
                    .lock()
                    .unwrap()
                    .push(command_name);
                action()
            },
            move || {
                *action_ran_for_action.lock().unwrap() = true;
                Ok(())
            },
        );

        assert!(matches!(result, Some(Ok(()))));
        assert_eq!(
            &*scheduled_commands.lock().unwrap(),
            &["show_quickbar_from_shortcut"]
        );
        assert!(*action_ran.lock().unwrap());
    }

    #[test]
    fn released_quickbar_shortcut_does_not_schedule() {
        let result = super::handle_quickbar_shortcut_event(
            ShortcutState::Released,
            "CommandOrControl+Shift+V",
            |_command_name, _action| panic!("released shortcut should not schedule QuickBar"),
            || {
                panic!("released shortcut should not run QuickBar action");
            },
        );

        assert!(result.is_none());
    }
}
