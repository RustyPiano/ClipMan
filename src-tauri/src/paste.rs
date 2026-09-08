use std::{
    borrow::Cow,
    sync::{Arc, Mutex},
    time::Duration,
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
use tauri::Manager;
use tauri::{AppHandle, Emitter};

use crate::{
    safe_lock,
    storage::{
        encode_file_paths, join_file_paths, split_file_paths, ClipItem, ContentType, CopyMarker,
    },
    tray::update_tray_menu,
    AppState,
};

const MAX_MERGE_BYTES: usize = 50_000_000;

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
    pub separator: Option<String>,
    pub hide: bool,
}

/// All entry points share one clipboard operation. Nothing is marked used until
/// the write succeeds; failures after that point must not invite a second paste.
pub async fn use_clips(
    app: &AppHandle,
    state: &AppState,
    request: UseRequest,
) -> Result<UseOutcome, String> {
    let started = std::time::Instant::now();
    let _operation = state
        .clipboard_use_lock
        .try_lock()
        .map_err(|_| "A clipboard operation is already running".to_string())?;
    let mode = PasteMode::try_from(request.mode.as_str())?;
    let simulate = should_simulate_paste(mode, state.settings.get().auto_paste);
    let foreground = Arc::new(Mutex::new(*safe_lock(&state.quickbar_foreground_window)));
    if request.ids.is_empty() || request.ids.len() > 10_000 {
        return Err("Select between 1 and 10000 clips".to_string());
    }
    if request
        .separator
        .as_ref()
        .is_some_and(|value| value.len() > 4096)
    {
        return Err("Merge separator is too long".to_string());
    }
    let storage = state.storage.clone();
    let ids = request.ids;
    let merge = request.separator.is_some();
    let mut items = tauri::async_runtime::spawn_blocking(move || {
        let storage = safe_lock(&storage);
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

    let write_app = app.clone();
    let marker = state.last_copied_by_us.clone();
    let items = tauri::async_runtime::spawn_blocking(move || {
        if let Some(separator) = request.separator {
            // Images have no text representation. Do not count them as used.
            items.retain(|item| item.content_type != ContentType::Image);
            if items.is_empty() {
                return Err("Merge paste had no text or file clips to merge".into());
            }
            let (merged, _) = merge_clip_texts(
                items
                    .iter()
                    .map(|item| (&item.content_type, item.content.as_slice())),
                &separator,
            )?;
            write_merged_text_to_system_clipboard(&merged, marker.clone())?;
        } else {
            if items.len() != 1 {
                return Err("A single copy requires exactly one clip".into());
            }
            write_clip_to_system_clipboard(&items[0], marker.clone(), request.plain, &write_app)?;
        }
        Ok::<_, String>(items)
    })
    .await
    .map_err(|error| error.to_string())??;
    log::debug!("use: clipboard write completed in {:?}", started.elapsed());

    let action = async {
        if request.hide {
            hide_quickbar(app)?;
        }
        if simulate {
            simulate_paste(app, &foreground).await
        } else {
            Ok(PasteSimulation::CopiedOnly)
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
    log::debug!(
        "use: foreground action completed in {:?}",
        started.elapsed()
    );

    // This maintenance runs after hide/focus/paste, once for the whole operation.
    let storage = state.storage.clone();
    let timestamp = crate::storage::current_timestamp();
    let ids = items.iter().map(|item| item.id.clone()).collect::<Vec<_>>();
    drop(items);
    match tauri::async_runtime::spawn_blocking(move || {
        safe_lock(&storage)
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

fn use_outcome(simulate: bool, action: Result<PasteSimulation, String>) -> UseOutcome {
    if let Err(error) = &action {
        log::warn!("Content was copied but could not be pasted: {error}");
    }
    match action {
        Ok(PasteSimulation::Pasted) => UseOutcome::PasteRequested,
        _ if !simulate => UseOutcome::Copied,
        _ => UseOutcome::CopiedOnly,
    }
}

/// Join the plain-text form of clips (in order) with `separator`, skipping and
/// counting Image clips (v1 has no text form for them). Text uses its bytes;
/// Files use their decoded path text (D3). Pure so the merge order,
/// image-skip, and separator behaviors are unit-testable without a clipboard.
fn merge_clip_texts<'a>(
    clips: impl IntoIterator<Item = (&'a ContentType, &'a [u8])>,
    separator: &str,
) -> Result<(String, usize), String> {
    let mut merged = String::new();
    let mut skipped = 0;
    let mut parts = 0;
    for (kind, content) in clips {
        if *kind == ContentType::Image {
            skipped += 1;
            continue;
        }
        let text = String::from_utf8_lossy(content);
        let text = if *kind == ContentType::Files {
            Cow::Owned(join_file_paths(&split_file_paths(&text)))
        } else {
            text
        };
        let separator = if parts == 0 { "" } else { separator };
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
        parts += 1;
    }
    Ok((merged, skipped))
}

/// Write merged plain text to the clipboard using the same self-copy marker +
/// TTL cleanup as every other write (D5). Merge paste never carries html, so
/// this is the plain-text write path only.
fn write_merged_text_to_system_clipboard(
    merged: &str,
    marker_state: Arc<Mutex<Option<CopyMarker>>>,
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

fn should_simulate_paste(mode: PasteMode, auto_paste: bool) -> bool {
    match mode {
        PasteMode::Default | PasteMode::Paste => auto_paste,
        PasteMode::Opposite => !auto_paste,
        PasteMode::Copy => false,
    }
}

pub fn write_clip_to_system_clipboard(
    item: &ClipItem,
    marker_state: Arc<Mutex<Option<CopyMarker>>>,
    plain_text_only: bool,
    app: &AppHandle,
) -> Result<(), String> {
    let mut clipboard = Clipboard::new().map_err(|e| e.to_string())?;
    match &item.content_type {
        ContentType::Text => write_text(&mut clipboard, item, marker_state, plain_text_only)?,
        ContentType::Image => write_image(&mut clipboard, item, marker_state)?,
        ContentType::Files => write_files(&mut clipboard, item, marker_state, app)?,
    };

    Ok(())
}

fn write_text(
    clipboard: &mut Clipboard,
    item: &ClipItem,
    marker_state: Arc<Mutex<Option<CopyMarker>>>,
    plain_text_only: bool,
) -> Result<(), String> {
    let text = String::from_utf8_lossy(&item.content).into_owned();
    // D5: the marker is always the plain-text hash, never the html — the monitor
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
    marker_state: Arc<Mutex<Option<CopyMarker>>>,
    app: &AppHandle,
) -> Result<(), String> {
    let content = String::from_utf8_lossy(&item.content).into_owned();
    let paths = split_file_paths(&content);
    // The self-copy marker must hash exactly the path list the monitor reads
    // back after our write, so resolve the platform's "effective" list first.
    let paths = effective_file_paths(paths);
    let joined = join_file_paths(&paths);
    let marker = CopyMarker::from_payload(ContentType::Files, encode_file_paths(&paths).as_bytes());

    let write_result = write_with_marker(marker_state.clone(), marker, || {
        write_file_list(clipboard, &paths)
    });

    if let Err(file_err) = write_result {
        // Degrade to the path text so the user still gets a usable clipboard,
        // and tell them why the real files didn't make it (macOS quietly
        // rejecting the URLs looks like "paste did nothing" otherwise).
        log::warn!(
            "Failed to write file list for clip {} ({file_err}); falling back to text",
            item.id
        );
        notify_file_paste_blocked(app);
        let text_marker = CopyMarker::from_payload(ContentType::Text, joined.as_bytes());
        write_with_marker(marker_state, text_marker, || {
            clipboard.set_text(joined.as_str()).map_err(|text_err| {
                format!(
                    "Failed to write file list clipboard: {file_err}; text fallback failed: {text_err}"
                )
            })
        })?;
        log::info!(
            "Copied {} file path(s) as text for clip {}",
            paths.len(),
            item.id
        );
        return Ok(());
    }

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

/// Shown when macOS blocks a file paste: without it the rejected write looks
/// like "pressing Enter did nothing". Non-fatal, fire-and-forget.
fn notify_file_paste_blocked(app: &AppHandle) {
    use tauri_plugin_notification::NotificationExt;
    let chinese = app.state::<AppState>().settings.get().locale == "zh-CN";
    let _ = app.notification().builder().title("ClipMan").body(if chinese {
        "部分文件无法写入剪贴板，已改为复制全部路径文本。请检查文件是否存在及访问权限。"
    } else {
        "Some files could not be restored. All paths were copied as text. Check that the files exist and are accessible."
    }).show();
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
    marker_state: Arc<Mutex<Option<CopyMarker>>>,
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

/// The self-copy marker ritual every clipboard write shares (D5): stake the
/// marker before writing, then clear it if the write failed or schedule its TTL
/// expiry if it succeeded. `write` performs the actual clipboard call and owns
/// its own error message. Structuring the invariant here keeps the write_*
/// family from each re-implementing (and potentially skewing) it (#31).
fn write_with_marker(
    marker_state: Arc<Mutex<Option<CopyMarker>>>,
    marker: CopyMarker,
    write: impl FnOnce() -> Result<(), String>,
) -> Result<(), String> {
    set_copy_marker(&marker_state, &marker);
    if let Err(e) = write() {
        clear_marker_if_current(&marker_state, &marker);
        return Err(e);
    }
    schedule_marker_clear(marker_state, marker);
    Ok(())
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
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(COPY_MARKER_TTL).await;
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
async fn simulate_paste(
    app: &AppHandle,
    foreground_store: &crate::window::ForegroundWindowStore,
) -> Result<PasteSimulation, String> {
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
) -> Result<PasteSimulation, String> {
    // Without the Accessibility permission, the CGEvent post that sends Cmd+V
    // fails *silently* — enigo returns Ok but nothing is typed. So we cannot
    // rely on a paste error to detect the problem; check the permission up
    // front. When it is missing (commonly after an update invalidates the
    // grant), guide the user to re-authorize and degrade to copy-only: the clip
    // is already on the clipboard, so they can paste manually.
    if !crate::accessibility::is_trusted() {
        log::warn!("Accessibility permission missing; cannot auto-paste");
        if let Err(e) = app.emit("accessibility-permission-required", ()) {
            log::error!("Failed to emit accessibility-permission-required event: {e}");
        }
        crate::accessibility::guide_reauthorization(app);
        return Ok(PasteSimulation::CopiedOnly);
    }

    // The QuickBar stole keyboard focus while it was open. It is now hidden, so
    // bring the previously frontmost app back to the front before pressing
    // Cmd+V; otherwise the keystroke is delivered to nothing.
    if let Err(e) = restore_recorded_foreground_window_on_main_thread(app, foreground_store) {
        log::warn!("Could not reactivate previous app before paste: {}", e);
        return Ok(PasteSimulation::CopiedOnly);
    }
    // Give the reactivated app a brief moment to become key and accept input.
    thread::sleep(Duration::from_millis(60));

    // A rapid reopen must not receive the pending Cmd+V itself.
    if app
        .get_webview_window(crate::window::QUICKBAR_WINDOW_LABEL)
        .is_some_and(|window| window.is_visible().unwrap_or(true))
    {
        return Ok(PasteSimulation::CopiedOnly);
    }
    send_paste_shortcut(Key::Meta)
        .map(|_| PasteSimulation::Pasted)
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
) -> Result<PasteSimulation, String> {
    crate::window::restore_recorded_foreground_window(foreground_store)?;
    send_paste_shortcut(Key::Control).map(|_| PasteSimulation::Pasted)
}

#[cfg(target_os = "linux")]
async fn simulate_paste(
    _app: &AppHandle,
    _foreground_store: &crate::window::ForegroundWindowStore,
) -> Result<PasteSimulation, String> {
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
        let result = merge_clip_texts([(&ContentType::Files, files.as_bytes())], "\n").unwrap();
        assert_eq!(result.0, "/tmp/a\nb.txt");
        let separator = "x".repeat(MAX_MERGE_BYTES);
        assert!(merge_clip_texts(
            [
                (&ContentType::Text, b"a".as_slice()),
                (&ContentType::Text, b"b".as_slice())
            ],
            &separator
        )
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
        assert_eq!(
            use_outcome(true, Ok(PasteSimulation::Pasted)),
            UseOutcome::PasteRequested
        );
    }

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

    fn merge(clips: &[(ContentType, Vec<u8>)], separator: &str) -> (String, usize) {
        merge_clip_texts(
            clips
                .iter()
                .map(|(kind, content)| (kind, content.as_slice())),
            separator,
        )
        .unwrap()
    }

    fn clip(content_type: ContentType, content: &[u8]) -> (ContentType, Vec<u8>) {
        (content_type, content.to_vec())
    }

    #[test]
    fn merge_preserves_selection_order() {
        let clips = [
            clip(ContentType::Text, b"first"),
            clip(ContentType::Text, b"second"),
            clip(ContentType::Text, b"third"),
        ];
        let (merged, skipped) = merge(&clips, "\n");
        assert_eq!(merged, "first\nsecond\nthird");
        assert_eq!(skipped, 0);
    }

    #[test]
    fn merge_skips_and_counts_image_clips() {
        let clips = [
            clip(ContentType::Text, b"a"),
            clip(ContentType::Image, b"\x89PNG-bytes"),
            clip(ContentType::Text, b"b"),
            clip(ContentType::Image, b"more-png"),
        ];
        let (merged, skipped) = merge(&clips, "\n");
        // Images are dropped from the merge; only text survives, in order.
        assert_eq!(merged, "a\nb");
        assert_eq!(skipped, 2);
    }

    #[test]
    fn merge_uses_the_given_separator_verbatim() {
        let clips = [clip(ContentType::Text, b"a"), clip(ContentType::Text, b"b")];
        assert_eq!(merge(&clips, "\n").0, "a\nb");
        assert_eq!(merge(&clips, "\t").0, "a\tb");
        assert_eq!(merge(&clips, "").0, "ab");
    }

    #[test]
    fn merge_includes_files_paths_as_text() {
        // Legacy newline-separated file records remain readable.
        let clips = [
            clip(ContentType::Files, b"/a/one.txt\n/a/two.txt"),
            clip(ContentType::Text, b"tail"),
        ];
        let (merged, skipped) = merge(&clips, "\n");
        assert_eq!(merged, "/a/one.txt\n/a/two.txt\ntail");
        assert_eq!(skipped, 0);
    }

    #[test]
    fn merge_of_only_images_yields_empty_text_and_full_skip_count() {
        let clips = [
            clip(ContentType::Image, b"one"),
            clip(ContentType::Image, b"two"),
            clip(ContentType::Image, b"three"),
        ];
        let (merged, skipped) = merge(&clips, "\n");
        assert_eq!(merged, "");
        assert_eq!(skipped, 3);
    }
}
