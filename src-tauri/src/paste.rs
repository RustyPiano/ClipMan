use std::{
    borrow::Cow,
    sync::{mpsc, Arc, Mutex},
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
    storage::{
        join_file_paths, split_file_paths, ClipItem, ContentType, CopyMarker, FrontendClipItem,
    },
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
    plain: bool,
) -> Result<(), String> {
    let mode = PasteMode::try_from(mode.as_str())?;
    let item = fetch_clip_and_touch_timestamp(&app, state, id).await?;
    let auto_paste = state.settings.get().auto_paste;

    write_clip_to_system_clipboard(&item, state.last_copied_by_us.clone(), plain, &app)?;
    hide_quickbar(&app)?;

    if should_simulate_paste(mode, auto_paste) {
        match simulate_paste(&app, state).await? {
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

/// Merge several clips into one clipboard write, then paste per `mode` (task #13).
///
/// Clips are taken in the caller's `ids` order; each is touched (timestamp +
/// emit) through the same helper `paste_clip` uses, so the recent list stays in
/// sync. Text/Files contribute their plain text (Files → newline-joined paths,
/// D3); Image clips have no text form in v1 and are skipped + counted. The
/// merged text is written through the shared self-copy marker + TTL path (D5).
pub async fn paste_clips(
    app: AppHandle,
    state: &AppState,
    ids: Vec<String>,
    mode: String,
    separator: String,
) -> Result<(), String> {
    let mode = PasteMode::try_from(mode.as_str())?;
    if ids.is_empty() {
        return Err("No clips selected for merge paste".to_string());
    }
    let auto_paste = state.settings.get().auto_paste;

    // Touch every selected clip's timestamp in one transaction and fetch each
    // clip's content in selection order (task #13). Unlike paste_clip's
    // single-item path, this does NOT emit per clip or rebuild the tray inside
    // the loop: a merge of N clips used to fire N `clipboard-changed` events and
    // rebuild the tray N times. Now a single batched touch is followed by one
    // emit + one tray rebuild here (#38). This runs before the merge/write (as
    // the per-item touch did), so an all-images selection still surfaces the
    // moved-up clips and refreshes the tray before the early return below.
    let (fetched, touched_preview) = fetch_clips_and_touch_batch(state, &ids).await?;
    if let Err(e) = app.emit("clipboard-changed", &touched_preview) {
        log::error!("Failed to emit clipboard-changed event: {}", e);
    }
    update_tray_menu(&app);

    let (merged, skipped_images) = merge_clip_texts(&fetched, &separator);
    let merged_count = fetched.len() - skipped_images;

    if skipped_images > 0 {
        log::info!("Merge paste skipped {skipped_images} image clip(s) (unsupported in v1)");
    }

    if merged_count == 0 {
        // Every selected clip was an image (no text form in v1): nothing is
        // mergeable. Return an error so the caller surfaces a paste-failure
        // toast instead of the panel silently staying put with no feedback
        // (#14); the clipboard is deliberately left untouched.
        log::warn!("Merge paste had no text/files clips to merge; skipping clipboard write");
        return Err("Merge paste had no text or file clips to merge".to_string());
    }

    write_merged_text_to_system_clipboard(&merged, state.last_copied_by_us.clone())?;
    hide_quickbar(&app)?;

    if should_simulate_paste(mode, auto_paste) {
        match simulate_paste(&app, state).await? {
            PasteSimulation::Pasted => log::info!("Merge-pasted {merged_count} clip(s)"),
            PasteSimulation::CopiedOnly => {
                log::warn!(
                    "Paste simulation unavailable; merged {merged_count} clip(s) copied only"
                );
            }
        }
    } else {
        log::info!("Merged {merged_count} clip(s) to clipboard without paste simulation");
    }

    Ok(())
}

/// Join the plain-text form of clips (in order) with `separator`, skipping and
/// counting Image clips (v1 has no text form for them). Text uses its bytes;
/// Files use their newline-joined path text (D3). Pure so the merge order,
/// image-skip, and separator behaviors are unit-testable without a clipboard.
fn merge_clip_texts(clips: &[(ContentType, Vec<u8>)], separator: &str) -> (String, usize) {
    let mut parts: Vec<String> = Vec::new();
    let mut skipped_images = 0usize;
    for (content_type, content) in clips {
        match content_type {
            ContentType::Image => skipped_images += 1,
            ContentType::Text | ContentType::Files => {
                parts.push(String::from_utf8_lossy(content).into_owned());
            }
        }
    }
    (parts.join(separator), skipped_images)
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

pub async fn fetch_clip_and_touch_timestamp(
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

    let frontend_item =
        FrontendClipItem::from_preview(crate::storage::ClipPreviewItem::from_clip_item(&item));
    if let Err(e) = app.emit("clipboard-changed", &frontend_item) {
        log::error!("Failed to emit clipboard-changed event: {}", e);
    }
    update_tray_menu(app);

    Ok(item)
}

/// Touch every clip in `ids` (same timestamp, one transaction) and return each
/// clip's `(content_type, content)` in selection order, plus a preview of the
/// last touched clip. Image bytes are dropped (the merge skips images, so
/// holding every selected image only to discard it wastes memory — #44); the
/// preview still renders images from their thumbnail. Unlike
/// `fetch_clip_and_touch_timestamp` this emits nothing and does not rebuild the
/// tray — the merge-paste caller does both once for the whole batch (#38).
async fn fetch_clips_and_touch_batch(
    state: &AppState,
    ids: &[String],
) -> Result<(Vec<(ContentType, Vec<u8>)>, FrontendClipItem), String> {
    let storage = state.storage.clone();
    let ids = ids.to_vec();
    let new_timestamp = Utc::now().timestamp();

    tauri::async_runtime::spawn_blocking(move || {
        let storage = safe_lock(&storage);

        let mut items = Vec::with_capacity(ids.len());
        for id in &ids {
            let item = storage
                .get_by_id(id)
                .map_err(|e| e.to_string())?
                .ok_or_else(|| "Clip not found".to_string())?;
            items.push(item);
        }

        storage
            .touch_timestamps(&ids, new_timestamp)
            .map_err(|e| e.to_string())?;

        // Reflect the touch in the returned copies and build the emit preview
        // from the last touched clip (all share `new_timestamp`, so a later full
        // reload surfaces the whole batch at the top; this live event lifts the
        // last one).
        for item in &mut items {
            item.timestamp = new_timestamp;
        }
        let last = items.last().ok_or("No clips fetched for merge paste")?;
        let touched_preview =
            FrontendClipItem::from_preview(crate::storage::ClipPreviewItem::from_clip_item(last));

        let fetched = items
            .into_iter()
            .map(|item| {
                let content = match item.content_type {
                    ContentType::Image => Vec::new(),
                    ContentType::Text | ContentType::Files => item.content,
                };
                (item.content_type, content)
            })
            .collect();

        Ok::<_, String>((fetched, touched_preview))
    })
    .await
    .map_err(|e| e.to_string())?
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
    let marker = CopyMarker::from_payload(ContentType::Files, joined.as_bytes());

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
    let canonical: Vec<String> = paths
        .iter()
        .filter_map(|p| std::fs::canonicalize(p).ok())
        .map(|p| p.to_string_lossy().into_owned())
        .collect();
    if canonical.is_empty() {
        // Every stored path is gone; keep the originals so the text fallback
        // still gives the user something pasteable.
        paths
    } else {
        canonical
    }
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
    use objc2_app_kit::{NSPasteboard, NSPasteboardWriting};
    use objc2_foundation::{NSArray, NSString, NSURL};

    // Pre-flight: trigger the TCC file-access prompt where one exists. The
    // result is deliberately ignored — a denied/unpromptable path (e.g.
    // another app's container, which needs Full Disk Access) is caught by the
    // landed-items check below.
    for path in paths {
        let _ = std::fs::File::open(path);
    }

    let urls: Vec<Retained<ProtocolObject<dyn NSPasteboardWriting>>> = paths
        .iter()
        .map(|path| {
            let url = NSURL::fileURLWithPath(&NSString::from_str(path));
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
    let landed = pasteboard
        .pasteboardItems()
        .map(|items| items.count())
        .unwrap_or(0);
    if landed == 0 {
        return Err("macOS rejected the file URLs (missing file access permission)".to_string());
    }

    Ok(())
}

/// Shown when macOS blocks a file paste: without it the rejected write looks
/// like "pressing Enter did nothing". Non-fatal, fire-and-forget.
fn notify_file_paste_blocked(app: &AppHandle) {
    #[cfg(not(target_os = "linux"))]
    {
        use tauri_plugin_notification::NotificationExt;
        let _ = app
            .notification()
            .builder()
            .title("文件粘贴受限")
            .body(
                "macOS 未授予 ClipMan 访问该文件的权限，已改为复制文件路径文本。\
                 可在 系统设置 → 隐私与安全性 → 完全磁盘访问权限 中启用 ClipMan 后重试。",
            )
            .show();
    }
    #[cfg(target_os = "linux")]
    let _ = app;
}

#[cfg(not(target_os = "macos"))]
fn write_file_list(clipboard: &mut Clipboard, paths: &[String]) -> Result<(), String> {
    let path_bufs: Vec<std::path::PathBuf> = paths.iter().map(std::path::PathBuf::from).collect();
    clipboard
        .set()
        .file_list(&path_bufs)
        .map_err(|e| e.to_string())
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
async fn simulate_paste(app: &AppHandle, state: &AppState) -> Result<PasteSimulation, String> {
    // The body blocks: it waits (up to 5s) on a main-thread round-trip to bring
    // the previous app forward, then sleeps 60ms and posts the Cmd+V CGEvent.
    // Running that on a Tokio worker would stall the async runtime, so hand it
    // to the blocking pool (#48). Only the foreground-window store is needed
    // from `state`, so clone that Arc rather than borrowing `AppState`.
    let app = app.clone();
    let foreground_store = state.quickbar_foreground_window.clone();
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
    }
    // Give the reactivated app a brief moment to become key and accept input.
    thread::sleep(Duration::from_millis(60));

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
async fn simulate_paste(_app: &AppHandle, state: &AppState) -> Result<PasteSimulation, String> {
    crate::window::restore_recorded_foreground_window(&state.quickbar_foreground_window)?;
    send_paste_shortcut(Key::Control).map(|_| PasteSimulation::Pasted)
}

#[cfg(target_os = "linux")]
async fn simulate_paste(_app: &AppHandle, _state: &AppState) -> Result<PasteSimulation, String> {
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
        let (merged, skipped) = merge_clip_texts(&clips, "\n");
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
        let (merged, skipped) = merge_clip_texts(&clips, "\n");
        // Images are dropped from the merge; only text survives, in order.
        assert_eq!(merged, "a\nb");
        assert_eq!(skipped, 2);
    }

    #[test]
    fn merge_uses_the_given_separator_verbatim() {
        let clips = [clip(ContentType::Text, b"a"), clip(ContentType::Text, b"b")];
        assert_eq!(merge_clip_texts(&clips, "\n").0, "a\nb");
        assert_eq!(merge_clip_texts(&clips, "\t").0, "a\tb");
        assert_eq!(merge_clip_texts(&clips, "").0, "ab");
    }

    #[test]
    fn merge_includes_files_paths_as_text() {
        // Files store their absolute paths newline-joined (D3); they merge as
        // that text, indistinguishable from a plain-text clip.
        let clips = [
            clip(ContentType::Files, b"/a/one.txt\n/a/two.txt"),
            clip(ContentType::Text, b"tail"),
        ];
        let (merged, skipped) = merge_clip_texts(&clips, "\n");
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
        let (merged, skipped) = merge_clip_texts(&clips, "\n");
        assert_eq!(merged, "");
        assert_eq!(skipped, 3);
    }
}
