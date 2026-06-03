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
    Default,
    Opposite,
    Paste,
    Copy,
}

impl TryFrom<&str> for PasteMode {
    type Error = String;

    fn try_from(mode: &str) -> Result<Self, Self::Error> {
        match mode {
            "default" => Ok(Self::Default),
            "opposite" => Ok(Self::Opposite),
            "paste" => Ok(Self::Paste),
            "copy" => Ok(Self::Copy),
            _ => Err(format!(
                "Invalid paste mode '{mode}'. Expected 'default', 'opposite', 'paste', or 'copy'."
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
    let auto_paste = state.settings.get().auto_paste;

    write_clip_to_system_clipboard(&item, state.last_copied_by_us.clone())?;
    hide_quickbar(&app)?;

    if should_simulate_paste(mode, auto_paste) {
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

fn should_simulate_paste(mode: PasteMode, auto_paste: bool) -> bool {
    match mode {
        PasteMode::Default | PasteMode::Paste => auto_paste,
        PasteMode::Opposite => !auto_paste,
        PasteMode::Copy => false,
    }
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
            write_file_path_as_text(&mut clipboard, item, marker_state)?;
        }
        ContentType::Html => {
            write_html(&mut clipboard, item, marker_state)?;
        }
        ContentType::Rtf => {
            write_rtf_as_text(&mut clipboard, item, marker_state)?;
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

fn write_file_path_as_text(
    clipboard: &mut Clipboard,
    item: &ClipItem,
    marker_state: Arc<Mutex<Option<CopyMarker>>>,
) -> Result<(), String> {
    let path = String::from_utf8_lossy(&item.content).into_owned();
    let marker = text_copy_marker(&path);
    set_copy_marker(&marker_state, &marker);

    if let Err(e) = clipboard.set_text(path.clone()) {
        clear_marker_if_current(&marker_state, &marker);
        return Err(format!("Failed to write file path clipboard: {e}"));
    }

    log::warn!(
        "Copied file clip {} as plain path text; native file-list clipboard format is not wired",
        item.id
    );
    schedule_marker_clear(marker_state, marker);
    Ok(())
}

fn write_html(
    clipboard: &mut Clipboard,
    item: &ClipItem,
    marker_state: Arc<Mutex<Option<CopyMarker>>>,
) -> Result<(), String> {
    let html = String::from_utf8_lossy(&item.content).into_owned();
    let marker = text_copy_marker(&html);
    set_copy_marker(&marker_state, &marker);

    if let Err(e) = clipboard.set_html(html.clone(), Some(html.clone())) {
        log::warn!(
            "HTML clipboard write failed for clip {}; falling back to plain text: {}",
            item.id,
            e
        );
        if let Err(e) = clipboard.set_text(html) {
            clear_marker_if_current(&marker_state, &marker);
            return Err(format!("Failed to write HTML fallback clipboard: {e}"));
        }
    } else {
        log::info!(
            "Copied HTML clip {} using native HTML clipboard format",
            item.id
        );
    }

    schedule_marker_clear(marker_state, marker);
    Ok(())
}

fn write_rtf_as_text(
    clipboard: &mut Clipboard,
    item: &ClipItem,
    marker_state: Arc<Mutex<Option<CopyMarker>>>,
) -> Result<(), String> {
    let rtf = String::from_utf8_lossy(&item.content).into_owned();
    let marker = text_copy_marker(&rtf);
    set_copy_marker(&marker_state, &marker);

    if let Err(e) = clipboard.set_text(rtf) {
        clear_marker_if_current(&marker_state, &marker);
        return Err(format!("Failed to write RTF fallback clipboard: {e}"));
    }

    log::warn!(
        "Copied RTF clip {} as plain text; native RTF clipboard format is not available via arboard",
        item.id
    );
    schedule_marker_clear(marker_state, marker);
    Ok(())
}

fn text_copy_marker(text: &str) -> CopyMarker {
    CopyMarker::from_payload(ContentType::Text, text.as_bytes())
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
fn simulate_paste(state: &AppState) -> Result<PasteSimulation, String> {
    // The QuickBar stole keyboard focus while it was open. It is now hidden, so
    // bring the previously frontmost app back to the front before pressing
    // Cmd+V; otherwise the keystroke is delivered to nothing.
    if let Err(e) =
        crate::window::restore_recorded_foreground_window(&state.quickbar_foreground_window)
    {
        log::warn!("Could not reactivate previous app before paste: {}", e);
    }
    // Give the reactivated app a brief moment to become key and accept input.
    thread::sleep(Duration::from_millis(60));

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
        .key(paste_key(), Click)
        .map_err(|e| format!("Failed to click paste key: {e}"));
    let release_result = enigo
        .key(modifier, Release)
        .map_err(|e| format!("Failed to release paste modifier: {e}"));

    click_result?;
    release_result
}

/// The "V" key pressed together with the platform modifier to trigger a paste.
///
/// On macOS we must NOT use `Key::Unicode('v')`: enigo resolves that character
/// to a virtual key code through `TSMGetInputSourceProperty` (Text Input Source
/// Manager). That API asserts it is running on the main dispatch queue and
/// aborts the whole process (EXC_BREAKPOINT) when called from the Tokio worker
/// thread handling the paste command. The raw key code for the physical "V"
/// key (kVK_ANSI_V = 9) bypasses that lookup entirely.
#[cfg(target_os = "macos")]
fn paste_key() -> Key {
    Key::Other(9)
}

#[cfg(not(target_os = "macos"))]
fn paste_key() -> Key {
    Key::Unicode('v')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paste_mode_resolution_uses_backend_auto_paste_setting() {
        assert!(should_simulate_paste(PasteMode::Default, true));
        assert!(!should_simulate_paste(PasteMode::Default, false));
        assert!(should_simulate_paste(PasteMode::Paste, true));
        assert!(!should_simulate_paste(PasteMode::Paste, false));
        assert!(!should_simulate_paste(PasteMode::Copy, true));
        assert!(!should_simulate_paste(PasteMode::Copy, false));
        assert!(!should_simulate_paste(PasteMode::Opposite, true));
        assert!(should_simulate_paste(PasteMode::Opposite, false));
    }

    #[test]
    fn text_like_clipboard_writes_use_text_copy_marker() {
        let marker = text_copy_marker("plain fallback");

        assert_eq!(ContentType::Text, marker.content_type);
        assert_eq!(
            CopyMarker::from_payload(ContentType::Text, b"plain fallback"),
            marker
        );
    }
}
