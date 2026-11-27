use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::Duration;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_updater::{Update, UpdaterExt};

use crate::AppState;

const FIRST_CHECK_DELAY: Duration = Duration::from_secs(30);
const CHECK_INTERVAL: Duration = Duration::from_secs(24 * 60 * 60);
const PROGRESS_STEP_BYTES: u64 = 256 * 1024;

#[derive(Default)]
pub struct UpdateState {
    pending: Mutex<Option<Update>>,
    installing: AtomicBool,
    ready_to_restart: AtomicBool,
}

#[derive(Clone, Serialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct UpdateInfo {
    pub available: bool,
    pub current_version: String,
    pub latest_version: Option<String>,
    pub body: Option<String>,
    pub date: Option<String>,
    pub ready_to_restart: bool,
}

#[derive(Clone, Serialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct UpdateProgress {
    downloaded: u64,
    total: Option<u64>,
}

fn info(app: &AppHandle, update: Option<&Update>) -> UpdateInfo {
    let state = app.state::<UpdateState>();
    UpdateInfo {
        available: update.is_some(),
        current_version: app.package_info().version.to_string(),
        latest_version: update.map(|update| update.version.clone()),
        body: update.and_then(|update| update.body.clone()),
        date: update.and_then(|update| update.date.map(|date| date.to_string())),
        ready_to_restart: state.ready_to_restart.load(Ordering::SeqCst),
    }
}

pub fn is_ready_to_restart(app: &AppHandle) -> bool {
    app.try_state::<UpdateState>()
        .is_some_and(|state| state.ready_to_restart.load(Ordering::SeqCst))
}

fn is_busy(app: &AppHandle) -> bool {
    let state = app.state::<UpdateState>();
    state.installing.load(Ordering::SeqCst) || state.ready_to_restart.load(Ordering::SeqCst)
}

pub fn pending_version(app: &AppHandle) -> Option<String> {
    let state = app.try_state::<UpdateState>()?;
    let pending = state.pending.lock().unwrap();
    pending.as_ref().map(|update| update.version.clone())
}

pub fn status(app: &AppHandle) -> UpdateInfo {
    let state = app.state::<UpdateState>();
    let pending = state.pending.lock().unwrap();
    info(app, pending.as_ref())
}

pub async fn check(app: &AppHandle) -> Result<UpdateInfo, String> {
    if is_busy(app) {
        return Ok(status(app));
    }
    let update = app
        .updater()
        .map_err(|e| format!("Failed to get updater: {e}"))?
        .check()
        .await
        .map_err(|e| format!("Failed to check for updates: {e}"))?;

    let result = info(app, update.as_ref());
    let changed = {
        let state = app.state::<UpdateState>();
        let mut pending = state.pending.lock().unwrap();
        let changed =
            pending.as_ref().map(|update| &update.version) != result.latest_version.as_ref();
        *pending = update;
        changed
    };

    if changed {
        log::info!(
            "Update check: current {}, latest {:?}",
            result.current_version,
            result.latest_version
        );
        crate::tray::update_tray_menu(app);
        let _ = app.emit("update-status", result.clone());
    }
    Ok(result)
}

pub async fn install(app: &AppHandle) -> Result<(), String> {
    let state = app.state::<UpdateState>();
    let update = state
        .pending
        .lock()
        .unwrap()
        .clone()
        .ok_or_else(|| "No update has been found; check for updates first".to_string())?;
    if state.ready_to_restart.load(Ordering::SeqCst) {
        return Err("The update is already installed; restart ClipLeaf to finish".into());
    }
    if state.installing.swap(true, Ordering::SeqCst) {
        return Err("An update is already being installed".into());
    }

    log::info!("Downloading and installing update {}", update.version);
    let mut downloaded: u64 = 0;
    let mut reported: u64 = 0;
    let progress_app = app.clone();
    let result = update
        .download_and_install(
            |chunk_length, total| {
                downloaded += chunk_length as u64;
                if downloaded - reported >= PROGRESS_STEP_BYTES || Some(downloaded) == total {
                    reported = downloaded;
                    let _ =
                        progress_app.emit("update-progress", UpdateProgress { downloaded, total });
                }
            },
            || log::info!("Update download finished"),
        )
        .await
        .map_err(|e| format!("Failed to download or install the update: {e}"));

    if result.is_ok() {
        state.ready_to_restart.store(true, Ordering::SeqCst);
    }
    state.installing.store(false, Ordering::SeqCst);
    result?;
    log::info!("Update {} installed; waiting for restart", update.version);
    crate::tray::update_tray_menu(app);
    let _ = app.emit("update-status", status(app));
    Ok(())
}

pub fn spawn_periodic_checks(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(FIRST_CHECK_DELAY).await;
        loop {
            let enabled = app.state::<AppState>().settings.get().auto_check_updates;
            if enabled && !is_busy(&app) {
                if let Err(e) = check(&app).await {
                    log::warn!("Background update check failed: {e}");
                }
            }
            tokio::time::sleep(CHECK_INTERVAL).await;
        }
    });
}
