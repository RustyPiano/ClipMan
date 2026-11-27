#[cfg(target_os = "macos")]
use std::thread;
use std::{
    borrow::Cow,
    sync::Mutex,
    time::{Duration, Instant},
};

use arboard::{Clipboard, ImageData};
use enigo::{
    Direction::{Click, Press, Release},
    Enigo, Key, Keyboard, Settings as EnigoSettings,
};
use image::GenericImageView;
use serde::Serialize;
use tauri::{AppHandle, Emitter};

use crate::{
    storage::{
        encode_file_paths, join_file_paths, split_file_paths, ClipItem, ContentType, CopyMarker,
    },
    tray::update_tray_menu,
    AppState,
};

const MAX_MERGE_BYTES: usize = 50_000_000;

const COPY_MARKER_TTL: Duration = Duration::from_secs(2);

pub(crate) struct CopyWrite {
    marker: CopyMarker,
    written_at: Option<Instant>,
}

impl CopyWrite {
    pub(crate) fn matches(&self, marker: &CopyMarker) -> bool {
        self.marker == *marker
            && self
                .written_at
                .is_none_or(|written_at| written_at.elapsed() < COPY_MARKER_TTL)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PasteMode {
    Default,
    Opposite,
    Copy,
}

impl TryFrom<&str> for PasteMode {
    type Error = String;

    fn try_from(mode: &str) -> Result<Self, Self::Error> {
        match mode {
            "default" => Ok(Self::Default),
            "opposite" => Ok(Self::Opposite),
            "copy" => Ok(Self::Copy),
            _ => Err(format!(
                "Invalid paste mode '{mode}'. Expected 'default', 'opposite', or 'copy'."
            )),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum UseOutcome {
    Copied,
    PasteRequested,
    CopiedOnly,
}

pub struct UseRequest {
    pub ids: Vec<String>,
    pub mode: String,
    pub plain: bool,
    pub merge: bool,
    pub hide: bool,
}

pub async fn use_clips(
    app: &AppHandle,
    state: &AppState,
    request: UseRequest,
) -> Result<UseOutcome, String> {
    let UseRequest {
        ids,
        mode,
        plain,
        merge,
        hide,
    } = request;
    let _operation = state
        .clipboard_use_lock
        .try_lock()
        .map_err(|_| "A clipboard operation is already running".to_string())?;
    let mode = PasteMode::try_from(mode.as_str())?;
    let simulate = should_simulate_paste(mode, state.settings.get().auto_paste);
    let foreground = *state.quickbar_foreground_window.lock().unwrap();
    if ids.is_empty() || ids.len() > 10_000 {
        return Err("Select between 1 and 10000 clips".to_string());
    }
    let storage = state.storage.clone();
    let mut items = tauri::async_runtime::spawn_blocking(move || {
        let storage = storage.lock().unwrap();
        let mut remaining = MAX_MERGE_BYTES;
        ids.iter()
            .map(|id| {
                let item = (if merge {
                    storage.get_for_merge(id, remaining)
                } else {
                    storage.get_by_id(id)
                })
                .map_err(|e| e.to_string())?
                .ok_or_else(|| "Clip not found".to_string())?;
                if merge {
                    remaining -= item.content.len();
                }
                Ok::<_, String>(item)
            })
            .collect::<Result<Vec<_>, _>>()
    })
    .await
    .map_err(|e| e.to_string())??;

    let marker = state.last_copied_by_us.clone();
    let ids = tauri::async_runtime::spawn_blocking(move || {
        if merge {
            items.retain(|item| item.content_type != ContentType::Image);
            if items.is_empty() {
                return Err("Merge paste had no text or file clips to merge".into());
            }
            let merged = merge_clip_texts(
                items
                    .iter()
                    .map(|item| (&item.content_type, item.content.as_slice())),
            )?;
            write_plain_text_to_system_clipboard(&merged, &marker)?;
        } else {
            if items.len() != 1 {
                return Err("A single copy requires exactly one clip".into());
            }
            write_clip_to_system_clipboard(&items[0], &marker, plain)?;
        }
        Ok::<_, String>(items.into_iter().map(|item| item.id).collect::<Vec<_>>())
    })
    .await
    .map_err(|error| error.to_string())??;

    let foreground_store = state.quickbar_foreground_window.clone();
    let action = async {
        if hide && !simulate {
            let app_for_hide = app.clone();
            let store_for_hide = foreground_store.clone();
            crate::commands::run_action_on_main_thread(
                app,
                "hide_quickbar_and_restore",
                move || crate::window::hide_quickbar_and_restore(&app_for_hide, &store_for_hide),
            )
            .map_err(|e| format!("Failed to hide QuickBar after copy: {e}"))?;
            return Ok(());
        }
        if hide {
            let app_for_hide = app.clone();
            crate::commands::run_action_on_main_thread(app, "hide_quickbar", move || {
                crate::window::hide_quickbar(&app_for_hide)
            })
            .map_err(|e| format!("Failed to hide QuickBar before paste: {e}"))?;
        }
        if simulate {
            let result = simulate_paste(app, foreground).await;
            if result.is_err() && hide && crate::accessibility::is_trusted() {
                let app_for_restore = app.clone();
                let store_for_restore = foreground_store.clone();
                if let Err(restore_error) = crate::commands::run_action_on_main_thread(
                    app,
                    "restore_after_paste_failure",
                    move || {
                        crate::window::hide_quickbar_and_restore(
                            &app_for_restore,
                            &store_for_restore,
                        )
                    },
                ) {
                    log::debug!(
                        "Could not restore previous app after paste failure: {restore_error}"
                    );
                }
            }
            result
        } else {
            Ok(())
        }
    }
    .await;
    let outcome = use_outcome(simulate, action);
    if outcome == UseOutcome::CopiedOnly {
        use tauri_plugin_notification::NotificationExt;
        let chinese = state.settings.get().locale == "zh-CN";
        let _ = app
            .notification()
            .builder()
            .title("ClipLeaf")
            .body(if chinese {
                "已复制，请在目标应用手动粘贴。"
            } else {
                "Copied. Paste manually in the target app."
            })
            .show();
    }

    let storage = state.storage.clone();
    let timestamp = crate::storage::current_timestamp();
    match tauri::async_runtime::spawn_blocking(move || {
        storage
            .lock()
            .unwrap()
            .touch_timestamps(&ids, timestamp)
            .map_err(|e| e.to_string())
    })
    .await
    {
        Ok(Ok(())) => {
            let _ = app.emit("clips-used", ());
        }
        result => log::warn!("Copied content, but failed to update recent history: {result:?}"),
    }
    update_tray_menu(app);
    Ok(outcome)
}

fn use_outcome(simulate: bool, action: Result<(), String>) -> UseOutcome {
    if let Err(error) = &action {
        log::warn!("Content was copied but could not be pasted: {error}");
    }
    match (simulate, action) {
        (false, _) => UseOutcome::Copied,
        (true, Ok(())) => UseOutcome::PasteRequested,
        (true, Err(_)) => UseOutcome::CopiedOnly,
    }
}

fn merge_clip_texts<'a>(
    clips: impl IntoIterator<Item = (&'a ContentType, &'a [u8])>,
) -> Result<String, String> {
    let mut merged = String::new();
    for (index, (kind, content)) in clips.into_iter().enumerate() {
        let text = String::from_utf8_lossy(content);
        let text = if *kind == ContentType::Files {
            Cow::Owned(join_file_paths(&split_file_paths(&text)))
        } else {
            text
        };
        let separator = if index == 0 { "" } else { "\n" };
        if merged.len() + separator.len() + text.len() > MAX_MERGE_BYTES {
            return Err("Merge exceeds the 50 MB content limit".to_string());
        }
        merged.push_str(separator);
        merged.push_str(&text);
    }
    Ok(merged)
}

pub fn write_plain_text_to_system_clipboard(
    text: &str,
    marker_state: &Mutex<Option<CopyWrite>>,
) -> Result<(), String> {
    let mut clipboard = Clipboard::new().map_err(|e| e.to_string())?;
    let marker = CopyMarker::from_payload(ContentType::Text, text.as_bytes());
    write_with_marker(marker_state, marker, || {
        clipboard
            .set_text(text)
            .map_err(|e| format!("Failed to write plain text clipboard: {e}"))
    })?;

    log::info!("Copied plain text to clipboard: {} bytes", text.len());
    Ok(())
}

#[cfg(target_os = "macos")]
pub(crate) fn clipboard_sequence() -> Option<u64> {
    Some(objc2_app_kit::NSPasteboard::generalPasteboard().changeCount() as u64)
}

#[cfg(target_os = "windows")]
pub(crate) fn clipboard_sequence() -> Option<u64> {
    let sequence = unsafe { windows::Win32::System::DataExchange::GetClipboardSequenceNumber() };
    (sequence != 0).then_some(u64::from(sequence))
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
pub(crate) fn clipboard_sequence() -> Option<u64> {
    None
}

fn should_simulate_paste(mode: PasteMode, auto_paste: bool) -> bool {
    match mode {
        PasteMode::Default => auto_paste,
        PasteMode::Opposite => !auto_paste,
        PasteMode::Copy => false,
    }
}

fn write_clip_to_system_clipboard(
    item: &ClipItem,
    marker_state: &Mutex<Option<CopyWrite>>,
    plain_text_only: bool,
) -> Result<(), String> {
    let mut clipboard = Clipboard::new().map_err(|e| e.to_string())?;
    match &item.content_type {
        ContentType::Text => write_text(&mut clipboard, item, marker_state, plain_text_only),
        ContentType::Image => write_image(&mut clipboard, item, marker_state),
        ContentType::Files => write_files(&mut clipboard, item, marker_state),
    }
}

fn write_text(
    clipboard: &mut Clipboard,
    item: &ClipItem,
    marker_state: &Mutex<Option<CopyWrite>>,
    plain_text_only: bool,
) -> Result<(), String> {
    let text = String::from_utf8_lossy(&item.content).into_owned();
    let marker = CopyMarker::from_payload(ContentType::Text, text.as_bytes());

    let html = item
        .html
        .as_deref()
        .filter(|html| !plain_text_only && !html.is_empty());
    write_with_marker(marker_state, marker, || {
        match html {
            Some(html) => clipboard.set().html(html, Some(text.as_str())),
            None => clipboard.set_text(text.as_str()),
        }
        .map_err(|e| format!("Failed to write text clipboard: {e}"))
    })?;

    log::info!(
        "Copied text clip {} to clipboard: {} chars (html: {})",
        item.id,
        text.len(),
        html.is_some()
    );
    Ok(())
}

fn write_files(
    clipboard: &mut Clipboard,
    item: &ClipItem,
    marker_state: &Mutex<Option<CopyWrite>>,
) -> Result<(), String> {
    let paths = split_file_paths(&String::from_utf8_lossy(&item.content));
    let paths = effective_file_paths(paths)?;
    let marker = CopyMarker::from_payload(ContentType::Files, encode_file_paths(&paths).as_bytes());

    write_with_marker(marker_state, marker, || write_file_list(clipboard, &paths)).map_err(
        |e| {
            format!(
                "Failed to write the file list for clip {} ({e}). Check that the files exist and ClipLeaf can access them.",
                item.id
            )
        },
    )?;

    log::info!(
        "Copied {} file(s) to clipboard for clip {}",
        paths.len(),
        item.id
    );
    Ok(())
}

#[cfg(target_os = "macos")]
fn effective_file_paths(paths: Vec<String>) -> Result<Vec<String>, String> {
    Ok(paths)
}

#[cfg(not(target_os = "macos"))]
fn effective_file_paths(paths: Vec<String>) -> Result<Vec<String>, String> {
    paths
        .iter()
        .map(|path| {
            std::fs::canonicalize(path)
                .map(|p| p.to_string_lossy().into_owned())
                .map_err(|e| format!("Failed to resolve file path {path}: {e}"))
        })
        .collect()
}

#[cfg(target_os = "macos")]
fn write_file_list(_clipboard: &mut Clipboard, paths: &[String]) -> Result<(), String> {
    use objc2::rc::Retained;
    use objc2::runtime::ProtocolObject;
    use objc2_app_kit::{NSPasteboard, NSPasteboardTypeFileURL, NSPasteboardWriting};
    use objc2_foundation::{NSArray, NSString, NSURL};

    for path in paths {
        let _ = std::fs::File::open(path);
    }

    let mut expected = Vec::new();
    let urls: Vec<Retained<ProtocolObject<dyn NSPasteboardWriting>>> = paths
        .iter()
        .map(|path| {
            let url = NSURL::fileURLWithPath(&NSString::from_str(path));
            expected.push(
                url.absoluteString()
                    .map(|s| s.to_string())
                    .unwrap_or_default(),
            );
            ProtocolObject::from_retained(url)
        })
        .collect();
    if urls.is_empty() {
        return Err("No file paths to write".to_string());
    }

    let objects = NSArray::from_retained_slice(&urls);
    let pasteboard = NSPasteboard::generalPasteboard();
    pasteboard.clearContents();
    if !pasteboard.writeObjects(&objects) {
        return Err("NSPasteboard writeObjects returned false".to_string());
    }

    let actual = pasteboard
        .pasteboardItems()
        .map(|items| {
            items
                .iter()
                .filter_map(|item| {
                    item.stringForType(unsafe { NSPasteboardTypeFileURL })
                        .map(|url| url.to_string())
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    verify_file_list(&expected, &actual)
}

fn verify_file_list(expected: &[String], actual: &[String]) -> Result<(), String> {
    let mut expected = expected.iter().collect::<Vec<_>>();
    let mut actual = actual.iter().collect::<Vec<_>>();
    expected.sort();
    actual.sort();
    if !expected.is_empty() && expected == actual {
        Ok(())
    } else {
        Err("The clipboard did not retain every selected file".into())
    }
}

#[cfg(not(target_os = "macos"))]
fn write_file_list(clipboard: &mut Clipboard, paths: &[String]) -> Result<(), String> {
    let path_bufs: Vec<std::path::PathBuf> = paths.iter().map(std::path::PathBuf::from).collect();
    clipboard
        .set()
        .file_list(&path_bufs)
        .map_err(|e| e.to_string())?;
    let actual = clipboard
        .get()
        .file_list()
        .map_err(|e| e.to_string())?
        .into_iter()
        .map(|p| p.to_string_lossy().into_owned())
        .collect::<Vec<_>>();
    verify_file_list(paths, &actual)
}

fn write_image(
    clipboard: &mut Clipboard,
    item: &ClipItem,
    marker_state: &Mutex<Option<CopyWrite>>,
) -> Result<(), String> {
    let img = image::load_from_memory(&item.content)
        .map_err(|e| format!("Failed to decode image clip {}: {e}", item.id))?;
    let (width, height) = img.dimensions();
    let rgba_bytes = img.to_rgba8().into_raw();
    let marker =
        CopyMarker::from_normalized_image_parts(width as usize, height as usize, &rgba_bytes);

    write_with_marker(marker_state, marker, || {
        clipboard
            .set_image(ImageData {
                width: width as usize,
                height: height as usize,
                bytes: Cow::Owned(rgba_bytes),
            })
            .map_err(|e| format!("Failed to write image clipboard: {e}"))
    })?;

    log::info!(
        "Copied image clip {} to clipboard: {}x{}",
        item.id,
        width,
        height
    );
    Ok(())
}

fn write_with_marker(
    marker_state: &Mutex<Option<CopyWrite>>,
    marker: CopyMarker,
    write: impl FnOnce() -> Result<(), String>,
) -> Result<(), String> {
    *marker_state.lock().unwrap() = Some(CopyWrite {
        marker: marker.clone(),
        written_at: None,
    });
    let result = write();
    *marker_state.lock().unwrap() = result.is_ok().then(|| CopyWrite {
        marker,
        written_at: Some(Instant::now()),
    });
    result
}

#[cfg(target_os = "macos")]
async fn simulate_paste(app: &AppHandle, foreground: Option<isize>) -> Result<(), String> {
    let app = app.clone();
    tauri::async_runtime::spawn_blocking(move || simulate_paste_blocking(&app, foreground))
        .await
        .map_err(|e| format!("Paste simulation task failed: {e}"))?
}

#[cfg(target_os = "macos")]
fn simulate_paste_blocking(app: &AppHandle, foreground: Option<isize>) -> Result<(), String> {
    use tauri::Manager;

    if !crate::accessibility::is_trusted() {
        if let Err(e) = app.emit("accessibility-permission-required", ()) {
            log::error!("Failed to emit accessibility-permission-required event: {e}");
        }
        crate::accessibility::guide_reauthorization(app);
        return Err("Accessibility permission missing; cannot auto-paste".into());
    }

    crate::commands::run_action_on_main_thread(app, "foreground restore", move || {
        crate::window::restore_recorded_foreground_window(foreground)
    })
    .map_err(|e| format!("Could not reactivate previous app before paste: {e}"))?;

    if let Some(expected_pid) = foreground.map(|pid| pid as i32) {
        let deadline = Instant::now() + Duration::from_millis(500);
        loop {
            if crate::window::frontmost_app_pid() == Some(expected_pid) {
                break;
            }
            if Instant::now() >= deadline {
                return Err(format!(
                    "Timed out waiting for previous app (pid {expected_pid}) to become frontmost"
                ));
            }
            thread::sleep(Duration::from_millis(10));
        }
    }
    thread::sleep(Duration::from_millis(20));

    if app
        .get_webview_window(crate::window::QUICKBAR_WINDOW_LABEL)
        .is_some_and(|window| window.is_visible().unwrap_or(true))
    {
        return Err("QuickBar reopened before the paste was sent".into());
    }
    send_paste_shortcut(Key::Meta)
        .map_err(|e| format!("accessibility_permission_required_or_input_simulation_failed: {e}"))
}

#[cfg(target_os = "windows")]
async fn simulate_paste(_app: &AppHandle, foreground: Option<isize>) -> Result<(), String> {
    crate::window::restore_recorded_foreground_window(foreground)?;
    send_paste_shortcut(Key::Control)
}

#[cfg(target_os = "linux")]
async fn simulate_paste(_app: &AppHandle, _foreground: Option<isize>) -> Result<(), String> {
    if std::env::var_os("WAYLAND_DISPLAY").is_some() {
        return Err("Wayland does not support simulated paste; copied only".into());
    }
    send_paste_shortcut(Key::Control)
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
    fn copy_marker_is_valid_while_writing_and_expires_after_ttl() {
        let marker = CopyMarker::from_payload(ContentType::Text, b"hello");
        let state = Mutex::new(None);
        write_with_marker(&state, marker.clone(), || {
            let current = state.lock().unwrap();
            let current = current.as_ref().unwrap();
            assert!(current.written_at.is_none());
            assert!(current.matches(&marker));
            Ok(())
        })
        .unwrap();

        let mut current = state.lock().unwrap();
        let current = current.as_mut().unwrap();
        assert!(current.written_at.is_some());
        assert!(current.matches(&marker));
        assert!(!current.matches(&CopyMarker::from_payload(ContentType::Text, b"other")));
        current.written_at = Some(Instant::now() - COPY_MARKER_TTL);
        assert!(!current.matches(&marker));
    }

    #[test]
    fn failed_write_clears_marker() {
        let marker = CopyMarker::from_payload(ContentType::Text, b"hello");
        let state = Mutex::new(None);
        assert!(write_with_marker(&state, marker, || Err("write failed".into())).is_err());
        assert!(state.lock().unwrap().is_none());
    }

    #[test]
    fn file_verification_rejects_partial_or_different_lists() {
        let expected = vec!["file:///a".into(), "file:///b".into()];
        assert!(verify_file_list(&expected, &expected).is_ok());
        assert!(verify_file_list(&expected, &["file:///b".into(), "file:///a".into()]).is_ok());
        assert!(verify_file_list(&expected, &["file:///a".into()]).is_err());
        assert!(verify_file_list(&expected, &["file:///a".into(), "file:///c".into()]).is_err());
        assert!(verify_file_list(&[], &[]).is_err());
    }

    #[test]
    fn merge_budget_counts_separators() {
        let full = vec![b'a'; MAX_MERGE_BYTES - 1];
        assert!(merge_clip_texts([
            (&ContentType::Text, full.as_slice()),
            (&ContentType::Text, b"b".as_slice())
        ])
        .is_err());
    }

    #[test]
    fn failures_after_clipboard_write_report_copy_success() {
        assert_eq!(
            use_outcome(true, Err("focus failed".into())),
            UseOutcome::CopiedOnly
        );
        assert_eq!(
            use_outcome(false, Err("hide failed".into())),
            UseOutcome::Copied
        );
        assert_eq!(use_outcome(true, Ok(())), UseOutcome::PasteRequested);
    }

    #[test]
    fn merge_preserves_selection_order_and_includes_file_paths() {
        let files = encode_file_paths(&["/a/one.txt".into(), "/a/two.txt".into()]);
        let merged = merge_clip_texts([
            (&ContentType::Text, b"first".as_slice()),
            (&ContentType::Files, files.as_bytes()),
            (&ContentType::Text, b"tail".as_slice()),
        ])
        .unwrap();
        assert_eq!(merged, "first\n/a/one.txt\n/a/two.txt\ntail");
    }
}
