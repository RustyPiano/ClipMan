use std::{
    borrow::Cow,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
#[cfg(target_os = "macos")]
use std::{sync::mpsc, thread};

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
    /// 写入完成的时间；写入进行中（例如等待文件访问授权）为 None，此时标记一直有效。
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
    /// 多条合并为一段换行分隔的纯文本写入。
    pub merge: bool,
    pub hide: bool,
}

/// All entry points share one clipboard operation. Nothing is marked used until
/// the write succeeds; failures after that point must not invite a second paste.
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
    let foreground = Arc::new(Mutex::new(
        *state.quickbar_foreground_window.lock().unwrap(),
    ));
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
    let items = tauri::async_runtime::spawn_blocking(move || {
        if merge {
            // Images have no text representation. Do not count them as used.
            items.retain(|item| item.content_type != ContentType::Image);
            if items.is_empty() {
                return Err("Merge paste had no text or file clips to merge".into());
            }
            let merged = merge_clip_texts(
                items
                    .iter()
                    .map(|item| (&item.content_type, item.content.as_slice())),
            )?;
            write_merged_text_to_system_clipboard(&merged, &marker)?;
        } else {
            if items.len() != 1 {
                return Err("A single copy requires exactly one clip".into());
            }
            write_clip_to_system_clipboard(&items[0], &marker, plain)?;
        }
        Ok::<_, String>(items)
    })
    .await
    .map_err(|error| error.to_string())??;

    let action = async {
        if hide {
            hide_quickbar(app)?;
        }
        if simulate {
            simulate_paste(app, &foreground).await
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
            .title("ClipMan")
            .body(if chinese {
                "已复制，请在目标应用手动粘贴。"
            } else {
                "Copied. Paste manually in the target app."
            })
            .show();
    }

    // This maintenance runs after hide/focus/paste, once for the whole operation.
    let storage = state.storage.clone();
    let timestamp = crate::storage::current_timestamp();
    let ids = items.iter().map(|item| item.id.clone()).collect::<Vec<_>>();
    drop(items);
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

/// 按选择顺序用换行连接各条的纯文本形式：Text 用原文，Files 用解码后的路径文本。
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
        let size = merged
            .len()
            .checked_add(separator.len())
            .and_then(|n| n.checked_add(text.len()))
            .filter(|n| *n <= MAX_MERGE_BYTES)
            .ok_or_else(|| "Merge exceeds the 50 MB content limit".to_string())?;
        merged
            .try_reserve(size - merged.len())
            .map_err(|e| e.to_string())?;
        merged.push_str(separator);
        merged.push_str(&text);
    }
    Ok(merged)
}

/// Write merged plain text to the clipboard using the same self-copy marker +
/// TTL as every other write. Merge paste never carries html, so this is the
/// plain-text write path only.
fn write_merged_text_to_system_clipboard(
    merged: &str,
    marker_state: &Mutex<Option<CopyWrite>>,
) -> Result<(), String> {
    let mut clipboard = Clipboard::new().map_err(|e| e.to_string())?;
    let marker = CopyMarker::from_payload(ContentType::Text, merged.as_bytes());
    write_with_marker(marker_state, marker, || {
        clipboard
            .set_text(merged)
            .map_err(|e| format!("Failed to write merged text clipboard: {e}"))
    })?;

    log::info!(
        "Copied merged clip text to clipboard: {} chars",
        merged.len()
    );
    Ok(())
}

/// Global-plain mode (`globalPlain`): replace the live clipboard's rich text
/// with its plain-text form so a direct paste anywhere (without QuickBar) is
/// unformatted. The marker hashes the plain text only — the same value the
/// monitor just recorded — so the rewrite is treated as our own write and
/// never re-captured.
pub fn strip_rich_text_from_clipboard(
    text: &str,
    marker_state: &Mutex<Option<CopyWrite>>,
) -> Result<(), String> {
    let mut clipboard = Clipboard::new().map_err(|e| e.to_string())?;
    let marker = CopyMarker::from_payload(ContentType::Text, text.as_bytes());
    write_with_marker(marker_state, marker, || {
        clipboard.set_text(text).map_err(|e| e.to_string())
    })?;

    log::info!(
        "Stripped rich text from clipboard (globalPlain): {} chars",
        text.len()
    );
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
    // arboard 未公开其他平台的写入序号，调用方改为核对完整内容。
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
    // The marker is always the plain-text hash, never the html — the monitor
    // reads back the plain-text alt after a self-paste and must still match.
    let marker = CopyMarker::from_payload(ContentType::Text, text.as_bytes());

    let use_html = !plain_text_only && item.html.as_deref().is_some_and(|html| !html.is_empty());
    write_with_marker(marker_state, marker, || {
        if use_html {
            // Place html plus the plain-text alt; ⌥Enter (plain=true) forces text.
            let html = item.html.as_deref().unwrap_or_default();
            clipboard.set().html(html, Some(text.as_str()))
        } else {
            clipboard.set_text(text.as_str())
        }
        .map_err(|e| format!("Failed to write text clipboard: {e}"))
    })?;

    log::info!(
        "Copied text clip {} to clipboard: {} chars (html: {})",
        item.id,
        text.len(),
        use_html
    );
    Ok(())
}

fn write_files(
    clipboard: &mut Clipboard,
    item: &ClipItem,
    marker_state: &Mutex<Option<CopyWrite>>,
) -> Result<(), String> {
    let paths = split_file_paths(&String::from_utf8_lossy(&item.content));
    // The self-copy marker must hash exactly the path list the monitor reads
    // back after our write, so resolve the platform's "effective" list first.
    let paths = effective_file_paths(paths);
    let marker = CopyMarker::from_payload(ContentType::Files, encode_file_paths(&paths).as_bytes());

    write_with_marker(marker_state, marker, || write_file_list(clipboard, &paths)).map_err(
        |e| {
            format!(
                "Failed to write the file list for clip {} ({e}). Check that the files exist and ClipMan can access them.",
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

/// The path list as the clipboard monitor will read it back after our write.
/// macOS writes NSURLs built from the stored paths and `NSURL.path()` returns
/// them unchanged. Other platforms go through arboard, which canonicalizes
/// every path and drops unresolvable ones — mirror that so the marker matches.
#[cfg(target_os = "macos")]
fn effective_file_paths(paths: Vec<String>) -> Vec<String> {
    paths
}

#[cfg(not(target_os = "macos"))]
fn effective_file_paths(paths: Vec<String>) -> Vec<String> {
    paths
        .into_iter()
        .map(|path| {
            std::fs::canonicalize(&path)
                .map(|p| p.to_string_lossy().into_owned())
                .unwrap_or(path)
        })
        .collect()
}

/// Put a file list on the system clipboard.
///
/// macOS writes NSURLs to NSPasteboard directly instead of going through
/// arboard (whose `file_list` canonicalizes every path and drops any it can't
/// `stat`). Two macOS 26 (Tahoe) behaviors shape this function:
///
/// 1. The pasteboard server validates that the writing process can access the
///    file behind each URL; unauthorized items are dropped **silently** —
///    `writeObjects` still returns `true`. Verified empirically: a Desktop
///    file wrote `items=0` while `/Users/Shared` wrote `items=1` and pasted.
/// 2. Opening the file first surfaces the one-time "Files and Folders" TCC
///    prompt (Desktop/Documents/Downloads) and, once granted, the write
///    passes validation. Terminal-launched processes inherit the terminal's
///    grants, which is why this only failed for Finder-launched instances.
#[cfg(target_os = "macos")]
fn write_file_list(_clipboard: &mut Clipboard, paths: &[String]) -> Result<(), String> {
    use objc2::rc::Retained;
    use objc2::runtime::ProtocolObject;
    use objc2_app_kit::{NSPasteboard, NSPasteboardTypeFileURL, NSPasteboardWriting};
    use objc2_foundation::{NSArray, NSString, NSURL};

    // Pre-flight: trigger the TCC file-access prompt where one exists. The
    // result is deliberately ignored — a denied/unpromptable path (e.g.
    // another app's container, which needs Full Disk Access) is caught by the
    // landed-items check below.
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

    // Tahoe drops unauthorized file URLs without reporting an error, so
    // "success" must be confirmed by the items actually being on the board.
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

/// 写入前登记标记，写入期间标记一直有效；成功后记录完成时间，有效期从此开始计算，
/// 失败时清除标记。所有写入都在 `clipboard_use_lock` 内执行，不会有另一次写入替换标记。
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

fn hide_quickbar(app: &AppHandle) -> Result<(), String> {
    crate::window::hide_quickbar(app)
        .map_err(|e| format!("Failed to hide QuickBar before paste: {e}"))
}

#[cfg(target_os = "macos")]
async fn simulate_paste(
    app: &AppHandle,
    foreground_store: &crate::window::ForegroundWindowStore,
) -> Result<(), String> {
    // The body blocks: it waits (up to 5s) on a main-thread round-trip to bring
    // the previous app forward, then sleeps 60ms and posts the Cmd+V CGEvent.
    // Running that on a Tokio worker would stall the async runtime, so hand it
    // to the blocking pool. The target was snapshotted when the operation began.
    let app = app.clone();
    let foreground_store = foreground_store.clone();
    tauri::async_runtime::spawn_blocking(move || simulate_paste_blocking(&app, &foreground_store))
        .await
        .map_err(|e| format!("Paste simulation task failed: {e}"))?
}

#[cfg(target_os = "macos")]
fn simulate_paste_blocking(
    app: &AppHandle,
    foreground_store: &crate::window::ForegroundWindowStore,
) -> Result<(), String> {
    use tauri::Manager;

    // Without the Accessibility permission, the CGEvent post that sends Cmd+V
    // fails *silently* — enigo returns Ok but nothing is typed. So we cannot
    // rely on a paste error to detect the problem; check the permission up
    // front. When it is missing (commonly after an update invalidates the
    // grant), guide the user to re-authorize; the clip is already on the
    // clipboard, so they can paste manually.
    if !crate::accessibility::is_trusted() {
        if let Err(e) = app.emit("accessibility-permission-required", ()) {
            log::error!("Failed to emit accessibility-permission-required event: {e}");
        }
        crate::accessibility::guide_reauthorization(app);
        return Err("Accessibility permission missing; cannot auto-paste".into());
    }

    // The QuickBar stole keyboard focus while it was open. It is now hidden, so
    // bring the previously frontmost app back to the front before pressing
    // Cmd+V; otherwise the keystroke is delivered to nothing.
    restore_recorded_foreground_window_on_main_thread(app, foreground_store)
        .map_err(|e| format!("Could not reactivate previous app before paste: {e}"))?;
    // Give the reactivated app a brief moment to become key and accept input.
    thread::sleep(Duration::from_millis(60));

    // A rapid reopen must not receive the pending Cmd+V itself.
    if app
        .get_webview_window(crate::window::QUICKBAR_WINDOW_LABEL)
        .is_some_and(|window| window.is_visible().unwrap_or(true))
    {
        return Err("QuickBar reopened before the paste was sent".into());
    }
    send_paste_shortcut(Key::Meta)
        .map_err(|e| format!("accessibility_permission_required_or_input_simulation_failed: {e}"))
}

#[cfg(target_os = "macos")]
fn restore_recorded_foreground_window_on_main_thread(
    app: &AppHandle,
    foreground_store: &crate::window::ForegroundWindowStore,
) -> Result<(), String> {
    let foreground_store = foreground_store.clone();
    let (sender, receiver) = mpsc::channel();

    app.run_on_main_thread(move || {
        let _ = sender.send(crate::window::restore_recorded_foreground_window(
            &foreground_store,
        ));
    })
    .map_err(|e| format!("Failed to schedule foreground restore on main thread: {e}"))?;

    receiver
        .recv_timeout(Duration::from_secs(5))
        .map_err(|e| format!("Timed out waiting for foreground restore: {e}"))?
}

#[cfg(target_os = "windows")]
async fn simulate_paste(
    _app: &AppHandle,
    foreground_store: &crate::window::ForegroundWindowStore,
) -> Result<(), String> {
    crate::window::restore_recorded_foreground_window(foreground_store)?;
    send_paste_shortcut(Key::Control)
}

#[cfg(target_os = "linux")]
async fn simulate_paste(
    _app: &AppHandle,
    _foreground_store: &crate::window::ForegroundWindowStore,
) -> Result<(), String> {
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
    fn merge_budget_counts_separators_and_decodes_json_files() {
        let files = encode_file_paths(&["/tmp/a\nb.txt".into()]);
        let merged = merge_clip_texts([(&ContentType::Files, files.as_bytes())]).unwrap();
        assert_eq!(merged, "/tmp/a\nb.txt");
        // Exactly at the limit alone, but the newline separator pushes it over.
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
    fn paste_mode_resolution_uses_backend_auto_paste_setting() {
        assert!(should_simulate_paste(PasteMode::Default, true));
        assert!(!should_simulate_paste(PasteMode::Default, false));
        assert!(!should_simulate_paste(PasteMode::Copy, true));
        assert!(!should_simulate_paste(PasteMode::Copy, false));
        assert!(!should_simulate_paste(PasteMode::Opposite, true));
        assert!(should_simulate_paste(PasteMode::Opposite, false));
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
