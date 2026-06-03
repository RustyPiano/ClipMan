use std::{
    borrow::Cow,
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};

use arboard::{Clipboard, ImageData};
use chrono::Utc;
use enigo::{
    Direction::{Click, Press, Release},
    Enigo, Key, Keyboard, Settings as EnigoSettings,
};
use image::GenericImageView;
use tauri::{AppHandle, Emitter};

use crate::{
    safe_lock,
    storage::{ClipItem, ContentType, CopyMarker, FrontendClipItem},
    tray::update_tray_menu,
    AppState,
};

const COPY_MARKER_TTL: Duration = Duration::from_secs(2);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PasteMode {
    Paste,
    Copy,
}

impl TryFrom<&str> for PasteMode {
    type Error = String;

    fn try_from(mode: &str) -> Result<Self, Self::Error> {
        match mode {
            "paste" => Ok(Self::Paste),
            "copy" => Ok(Self::Copy),
            _ => Err(format!(
                "Invalid paste mode '{mode}'. Expected 'paste' or 'copy'."
            )),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
enum PasteSimulation {
    Pasted,
    CopiedOnly,
}

pub async fn paste_clip(
    app: AppHandle,
    state: &AppState,
    id: String,
    mode: String,
) -> Result<(), String> {
    let mode = PasteMode::try_from(mode.as_str())?;
    let item = fetch_clip_and_touch_timestamp(&app, state, id).await?;

    write_clip_to_system_clipboard(&item, state.last_copied_by_us.clone())?;
    hide_quickbar(&app)?;

    if mode == PasteMode::Paste {
        match simulate_paste(state)? {
            PasteSimulation::Pasted => log::info!("Pasted clip {}", item.id),
            PasteSimulation::CopiedOnly => {
                log::warn!(
                    "Paste simulation unavailable; clip {} was copied only",
                    item.id
                );
            }
        }
    } else {
        log::info!("Copied clip {} without paste simulation", item.id);
    }

    Ok(())
}

async fn fetch_clip_and_touch_timestamp(
    app: &AppHandle,
    state: &AppState,
    id: String,
) -> Result<ClipItem, String> {
    let storage = state.storage.clone();
    let new_timestamp = Utc::now().timestamp();
    let id_for_storage = id.clone();

    let item = tauri::async_runtime::spawn_blocking(move || {
        let storage = safe_lock(&storage);
        let mut item = storage
            .get_by_id(&id_for_storage)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "Clip not found".to_string())?;

        storage
            .update_timestamp(&id_for_storage, new_timestamp)
            .map_err(|e| e.to_string())?;
        item.timestamp = new_timestamp;
        Ok::<ClipItem, String>(item)
    })
    .await
    .map_err(|e| e.to_string())??;

    let frontend_item = FrontendClipItem::from(item.clone());
    if let Err(e) = app.emit("clipboard-changed", &frontend_item) {
        log::error!("Failed to emit clipboard-changed event: {}", e);
    }
    update_tray_menu(app);

    Ok(item)
}

fn write_clip_to_system_clipboard(
    item: &ClipItem,
    marker_state: Arc<Mutex<Option<CopyMarker>>>,
) -> Result<(), String> {
    let mut clipboard = Clipboard::new().map_err(|e| e.to_string())?;
    match &item.content_type {
        ContentType::Text => write_text(&mut clipboard, item, marker_state)?,
        ContentType::Image => write_image(&mut clipboard, item, marker_state)?,
        ContentType::File => {
            write_file_path_as_text(&mut clipboard, item)?;
        }
        ContentType::Html => {
            write_html(&mut clipboard, item)?;
        }
        ContentType::Rtf => {
            write_rtf_as_text(&mut clipboard, item)?;
        }
    };

    Ok(())
}

fn write_text(
    clipboard: &mut Clipboard,
    item: &ClipItem,
    marker_state: Arc<Mutex<Option<CopyMarker>>>,
) -> Result<(), String> {
    let text = String::from_utf8_lossy(&item.content).into_owned();
    let marker = CopyMarker::from_payload(ContentType::Text, text.as_bytes());
    set_copy_marker(&marker_state, &marker);

    if let Err(e) = clipboard.set_text(text.clone()) {
        clear_marker_if_current(&marker_state, &marker);
        return Err(format!("Failed to write text clipboard: {e}"));
    }

    log::info!(
        "Copied text clip {} to clipboard: {} chars",
        item.id,
        text.len()
    );
    schedule_marker_clear(marker_state, marker);
    Ok(())
}

fn write_image(
    clipboard: &mut Clipboard,
    item: &ClipItem,
    marker_state: Arc<Mutex<Option<CopyMarker>>>,
) -> Result<(), String> {
    let img = image::load_from_memory(&item.content)
        .map_err(|e| format!("Failed to decode image clip {}: {e}", item.id))?;
    let (width, height) = img.dimensions();
    let rgba_bytes = img.to_rgba8().into_raw();
    let normalized_payload = normalized_image_payload(width as u64, height as u64, &rgba_bytes);
    let marker = CopyMarker::from_payload(ContentType::Image, &normalized_payload);
    set_copy_marker(&marker_state, &marker);

    if let Err(e) = clipboard.set_image(ImageData {
        width: width as usize,
        height: height as usize,
        bytes: Cow::Owned(rgba_bytes),
    }) {
        clear_marker_if_current(&marker_state, &marker);
        return Err(format!("Failed to write image clipboard: {e}"));
    }

    log::info!(
        "Copied image clip {} to clipboard: {}x{}",
        item.id,
        width,
        height
    );
    schedule_marker_clear(marker_state, marker);
    Ok(())
}

fn write_file_path_as_text(clipboard: &mut Clipboard, item: &ClipItem) -> Result<(), String> {
    let path = String::from_utf8_lossy(&item.content).into_owned();
    clipboard
        .set_text(path.clone())
        .map_err(|e| format!("Failed to write file path clipboard: {e}"))?;
    log::warn!(
        "Copied file clip {} as plain path text; native file-list clipboard format is not wired",
        item.id
    );
    Ok(())
}

fn write_html(clipboard: &mut Clipboard, item: &ClipItem) -> Result<(), String> {
    let html = String::from_utf8_lossy(&item.content).into_owned();
    if let Err(e) = clipboard.set_html(html.clone(), Some(html.clone())) {
        log::warn!(
            "HTML clipboard write failed for clip {}; falling back to plain text: {}",
            item.id,
            e
        );
        clipboard
            .set_text(html)
            .map_err(|e| format!("Failed to write HTML fallback clipboard: {e}"))?;
    } else {
        log::info!(
            "Copied HTML clip {} using native HTML clipboard format",
            item.id
        );
    }
    Ok(())
}

fn write_rtf_as_text(clipboard: &mut Clipboard, item: &ClipItem) -> Result<(), String> {
    let rtf = String::from_utf8_lossy(&item.content).into_owned();
    clipboard
        .set_text(rtf)
        .map_err(|e| format!("Failed to write RTF fallback clipboard: {e}"))?;
    log::warn!(
        "Copied RTF clip {} as plain text; native RTF clipboard format is not available via arboard",
        item.id
    );
    Ok(())
}

fn normalized_image_payload(width: u64, height: u64, rgba_bytes: &[u8]) -> Vec<u8> {
    let mut payload = Vec::with_capacity(16 + rgba_bytes.len());
    payload.extend_from_slice(&width.to_le_bytes());
    payload.extend_from_slice(&height.to_le_bytes());
    payload.extend_from_slice(rgba_bytes);
    payload
}

fn set_copy_marker(marker_state: &Arc<Mutex<Option<CopyMarker>>>, marker: &CopyMarker) {
    let mut last_copied = safe_lock(marker_state);
    *last_copied = Some(marker.clone());
}

fn clear_marker_if_current(marker_state: &Arc<Mutex<Option<CopyMarker>>>, marker: &CopyMarker) {
    let mut last_copied = safe_lock(marker_state);
    if last_copied.as_ref() == Some(marker) {
        *last_copied = None;
    }
}

fn schedule_marker_clear(marker_state: Arc<Mutex<Option<CopyMarker>>>, marker: CopyMarker) {
    thread::spawn(move || {
        thread::sleep(COPY_MARKER_TTL);
        let mut last_copied = safe_lock(&marker_state);
        if last_copied.as_ref() == Some(&marker) {
            *last_copied = None;
            log::debug!("Cleared self-copy marker");
        }
    });
}

fn hide_quickbar(app: &AppHandle) -> Result<(), String> {
    crate::window::hide_quickbar(app)
        .map_err(|e| format!("Failed to hide QuickBar before paste: {e}"))
}

#[cfg(target_os = "macos")]
fn simulate_paste(_state: &AppState) -> Result<PasteSimulation, String> {
    send_paste_shortcut(Key::Meta)
        .map(|_| PasteSimulation::Pasted)
        .map_err(|e| format!("accessibility_permission_required_or_input_simulation_failed: {e}"))
}

#[cfg(target_os = "windows")]
fn simulate_paste(state: &AppState) -> Result<PasteSimulation, String> {
    crate::window::restore_recorded_foreground_window(&state.quickbar_foreground_window)?;
    send_paste_shortcut(Key::Control).map(|_| PasteSimulation::Pasted)
}

#[cfg(target_os = "linux")]
fn simulate_paste(_state: &AppState) -> Result<PasteSimulation, String> {
    if std::env::var_os("WAYLAND_DISPLAY").is_some() {
        log::warn!("Wayland detected; degrading paste request to copy-only");
        return Ok(PasteSimulation::CopiedOnly);
    }

    match send_paste_shortcut(Key::Control) {
        Ok(()) => Ok(PasteSimulation::Pasted),
        Err(e) => {
            log::warn!(
                "Linux paste simulation failed; degrading to copy-only: {}",
                e
            );
            Ok(PasteSimulation::CopiedOnly)
        }
    }
}

fn send_paste_shortcut(modifier: Key) -> Result<(), String> {
    let mut enigo = Enigo::new(&EnigoSettings::default())
        .map_err(|e| format!("Failed to initialize input simulation: {e}"))?;

    enigo
        .key(modifier, Press)
        .map_err(|e| format!("Failed to press paste modifier: {e}"))?;
    let click_result = enigo
        .key(Key::Unicode('v'), Click)
        .map_err(|e| format!("Failed to click paste key: {e}"));
    let release_result = enigo
        .key(modifier, Release)
        .map_err(|e| format!("Failed to release paste modifier: {e}"));

    click_result?;
    release_result
}
