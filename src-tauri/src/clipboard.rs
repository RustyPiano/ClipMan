use arboard::{Clipboard, ImageData};
use chrono::Utc;
use clipboard_master::{CallbackResult, ClipboardHandler, Master, Shutdown};
use image::{DynamicImage, ImageBuffer, RgbaImage};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc, Arc, Mutex,
};
use std::thread::JoinHandle;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};
use uuid::Uuid;

use crate::storage::{join_file_paths, ClipItem, ContentType, CopyMarker};

type MonitorReadySender = mpsc::Sender<Result<(), String>>;
const MONITOR_STOP_TIMEOUT: Duration = Duration::from_secs(2);

#[cfg(target_os = "windows")]
#[link(name = "kernel32")]
extern "system" {
    fn GlobalLock(hmem: windows::Win32::Foundation::HANDLE) -> *mut std::ffi::c_void;
    fn GlobalSize(hmem: windows::Win32::Foundation::HANDLE) -> usize;
    fn GlobalUnlock(hmem: windows::Win32::Foundation::HANDLE) -> i32;
}

pub struct ClipboardMonitor {
    app_handle: AppHandle,
    last_copied_by_us: Arc<Mutex<Option<CopyMarker>>>,
    running: Arc<AtomicBool>,
    shutdown: Arc<Mutex<Option<Shutdown>>>,
    handle: Option<JoinHandle<()>>,
}

struct Handler {
    app_handle: AppHandle,
    last_copied_by_us: Arc<Mutex<Option<CopyMarker>>>,
    running: Arc<AtomicBool>,
    last_marker: Option<CopyMarker>,
}

struct ProcessedClipboardImage {
    content_png: Vec<u8>,
    thumbnail_png: Vec<u8>,
    marker: CopyMarker,
}

/// A single representative view of the current clipboard, chosen by priority so
/// one clipboard change yields exactly one record: `Files > Text(+html) > Image`
/// (D1). Finder's file copies also expose a filename string and an icon image;
/// taking the file list first discards that derived noise.
enum ClipboardSnapshot {
    Files(Vec<String>),
    Text { text: String, html: Option<String> },
    Image(ImageData<'static>),
}

/// Reads the clipboard once and returns its single representative format.
///
/// Image decoding copies the full bitmap and is expensive, so it is only
/// attempted when neither files nor text are present (§2.1 laziness).
fn read_clipboard_snapshot(clipboard: &mut Clipboard) -> Option<ClipboardSnapshot> {
    if let Ok(paths) = clipboard.get().file_list() {
        let paths: Vec<String> = paths
            .into_iter()
            .map(|path| path.to_string_lossy().into_owned())
            .filter(|path| !path.is_empty())
            .collect();
        if !paths.is_empty() {
            return Some(ClipboardSnapshot::Files(paths));
        }
    }

    if let Ok(text) = clipboard.get_text() {
        if !text.is_empty() {
            // HTML is an optional companion; a missing or empty value is normal.
            let html = clipboard.get().html().ok().filter(|html| !html.is_empty());
            return Some(ClipboardSnapshot::Text { text, html });
        }
    }

    if let Ok(image) = clipboard.get_image() {
        return Some(ClipboardSnapshot::Image(image));
    }

    None
}

/// Self-copy/dedup marker for a snapshot, hashing only the primary content (D5).
/// Text hashes the plain text (never the html), because after a self-paste the
/// monitor reads back the plain-text alt and must still recognize our write.
fn snapshot_marker(snapshot: &ClipboardSnapshot) -> CopyMarker {
    match snapshot {
        ClipboardSnapshot::Files(paths) => {
            CopyMarker::from_payload(ContentType::Files, join_file_paths(paths).as_bytes())
        }
        ClipboardSnapshot::Text { text, .. } => {
            CopyMarker::from_payload(ContentType::Text, text.as_bytes())
        }
        ClipboardSnapshot::Image(image) => {
            CopyMarker::from_normalized_image_parts(image.width, image.height, image.bytes.as_ref())
        }
    }
}

/// Decides whether a freshly observed clipboard marker should be dispatched to
/// storage, advancing `last_marker` as a side effect.
///
/// Returns false when the marker is unchanged (already recorded) or when it is
/// our own write. Crucially, a self-copy skip still advances `last_marker`, so a
/// later genuine copy of *different* content is never compared against stale
/// state — matching the pre-refactor behavior where `last_text` was updated even
/// on a self-copy skip (§2.2). `is_self_copied` is only evaluated when the
/// marker actually changed, avoiding a needless lock on unchanged clipboards.
fn record_marker_and_decide_dispatch(
    marker: &CopyMarker,
    last_marker: &mut Option<CopyMarker>,
    is_self_copied: impl FnOnce() -> bool,
) -> bool {
    if last_marker.as_ref() == Some(marker) {
        return false;
    }
    *last_marker = Some(marker.clone());
    !is_self_copied()
}

impl ClipboardHandler for Handler {
    fn on_clipboard_change(&mut self) -> CallbackResult {
        if !self.running.load(Ordering::SeqCst) {
            return CallbackResult::Stop;
        }

        let mut clipboard = match Clipboard::new() {
            Ok(cb) => cb,
            Err(e) => {
                log::error!("Failed to create clipboard instance in handler: {}", e);
                return CallbackResult::Next;
            }
        };

        if ClipboardMonitor::should_ignore_current_clipboard(&self.app_handle) {
            log::info!("Skipping clipboard change marked as concealed/transient/autogenerated");
            return CallbackResult::Next;
        }

        ClipboardMonitor::handle_clipboard_event(
            &self.app_handle,
            &self.last_copied_by_us,
            &self.running,
            &mut clipboard,
            &mut self.last_marker,
        );

        CallbackResult::Next
    }

    fn on_clipboard_error(&mut self, error: std::io::Error) -> CallbackResult {
        log::error!("Clipboard error: {}", error);
        CallbackResult::Next
    }
}

impl ClipboardMonitor {
    pub fn new(app_handle: AppHandle, last_copied_by_us: Arc<Mutex<Option<CopyMarker>>>) -> Self {
        Self {
            app_handle,
            last_copied_by_us,
            running: Arc::new(AtomicBool::new(false)),
            shutdown: Arc::new(Mutex::new(None)),
            handle: None,
        }
    }

    pub fn start(&mut self) -> Result<(), String> {
        self.running.store(true, Ordering::SeqCst);
        let app_handle = self.app_handle.clone();
        let last_copied_by_us = self.last_copied_by_us.clone();
        let running = self.running.clone();
        let shutdown_slot = self.shutdown.clone();
        let (ready_sender, ready_receiver) = mpsc::channel();

        // Start event-driven monitoring thread
        let handle = std::thread::spawn(move || {
            log::info!("Clipboard monitoring thread started (event-driven)");

            let handler = Handler {
                app_handle: app_handle.clone(),
                last_copied_by_us: last_copied_by_us.clone(),
                running: running.clone(),
                last_marker: None,
            };

            // Start the clipboard master - this blocks until the application exits
            match Master::new(handler) {
                Ok(mut master) => {
                    // Hand the shutdown channel to the monitor before start()
                    // returns, so stop() can always signal run() before join().
                    *crate::safe_lock(&shutdown_slot) = Some(master.shutdown_channel());
                    let run_returned = Arc::new(AtomicBool::new(false));
                    let run_returned_for_ready = run_returned.clone();
                    let ready_sender_for_event = ready_sender.clone();
                    std::thread::spawn(move || {
                        std::thread::sleep(Duration::from_millis(250));
                        if !run_returned_for_ready.load(Ordering::SeqCst) {
                            let _ = ready_sender_for_event.send(Ok(()));
                        }
                    });

                    let run_result = master.run();
                    run_returned.store(true, Ordering::SeqCst);
                    if let Err(e) = run_result {
                        log::error!("Clipboard master failed: {}", e);
                        log::warn!("Falling back to polling mode...");
                        Self::start_polling(
                            app_handle,
                            last_copied_by_us,
                            running,
                            Some(ready_sender),
                        );
                    }
                }
                Err(e) => {
                    log::error!("Failed to create clipboard master: {}", e);
                    log::warn!("Falling back to polling mode...");
                    Self::start_polling(app_handle, last_copied_by_us, running, Some(ready_sender));
                }
            }
        });

        self.handle = Some(handle);
        match ready_receiver.recv() {
            Ok(Ok(())) => Ok(()),
            Ok(Err(e)) => {
                self.running.store(false, Ordering::SeqCst);
                if let Some(handle) = self.handle.take() {
                    let _ = handle.join();
                }
                Err(e)
            }
            Err(e) => {
                self.running.store(false, Ordering::SeqCst);
                if let Some(handle) = self.handle.take() {
                    let _ = handle.join();
                }
                Err(format!("Clipboard monitor failed to start: {}", e))
            }
        }
    }

    /// Stop the monitor and wait for its thread to exit.
    pub fn stop(mut self) {
        self.running.store(false, Ordering::SeqCst);
        if let Some(shutdown) = crate::safe_lock(&self.shutdown).take() {
            shutdown.signal();
        }
        if let Some(handle) = self.handle.take() {
            join_monitor_thread_with_timeout(handle);
        }
    }

    // Fallback polling implementation
    fn start_polling(
        app_handle: AppHandle,
        last_copied_by_us: Arc<Mutex<Option<CopyMarker>>>,
        running: Arc<AtomicBool>,
        ready_sender: Option<MonitorReadySender>,
    ) {
        use std::thread;

        let mut clipboard = match Clipboard::new() {
            Ok(cb) => cb,
            Err(e) => {
                let message = format!("Failed to create clipboard instance: {}", e);
                log::error!("{}", message);
                running.store(false, Ordering::SeqCst);
                if let Some(ready_sender) = ready_sender {
                    let _ = ready_sender.send(Err(message));
                }
                return;
            }
        };
        if let Some(ready_sender) = ready_sender {
            let _ = ready_sender.send(Ok(()));
        }

        let mut last_marker: Option<CopyMarker> = None;
        let mut last_ignored_concealed = false;

        while running.load(Ordering::SeqCst) {
            if Self::should_ignore_current_clipboard(&app_handle) {
                if !last_ignored_concealed {
                    log::info!(
                        "Skipping clipboard content marked as concealed/transient/autogenerated"
                    );
                }
                last_ignored_concealed = true;
                thread::sleep(Duration::from_millis(500));
                continue;
            }

            last_ignored_concealed = false;

            Self::handle_clipboard_event(
                &app_handle,
                &last_copied_by_us,
                &running,
                &mut clipboard,
                &mut last_marker,
            );

            thread::sleep(Duration::from_millis(500));
        }
    }

    /// Unified entry point shared by the event handler and the polling loop:
    /// read one representative snapshot, dedup against `last_marker`, skip our
    /// own writes, then dispatch a single record for the winning format.
    fn handle_clipboard_event(
        app_handle: &AppHandle,
        last_copied_by_us: &Arc<Mutex<Option<CopyMarker>>>,
        running: &Arc<AtomicBool>,
        clipboard: &mut Clipboard,
        last_marker: &mut Option<CopyMarker>,
    ) {
        // Early-out, alongside the concealed-content check: while capture is
        // paused we must not observe the clipboard at all (SPEC-4 §3), so
        // `last_marker` intentionally does not advance here — the first
        // snapshot taken after resuming is compared against whatever was last
        // recorded before the pause, which is exactly the "resume behaves
        // normally" contract.
        if Self::capture_is_paused(app_handle) {
            return;
        }

        let Some(snapshot) = read_clipboard_snapshot(clipboard) else {
            return;
        };
        let marker = snapshot_marker(&snapshot);

        let dispatch = record_marker_and_decide_dispatch(&marker, last_marker, || {
            Self::is_self_copied(last_copied_by_us, &marker)
        });
        if !dispatch {
            return;
        }

        // Read the frontmost app once, before dispatching to a per-format
        // process_*_change function, instead of each of them calling
        // NSWorkspace independently. This single read is also what
        // process_image_change needs to snapshot *before* its async
        // processing runs, so placing it here (synchronously, pre-dispatch)
        // preserves that ordering for free.
        let source_app = frontmost_app_name();

        // The ignored-apps check runs after record_marker_and_decide_dispatch
        // has already advanced `last_marker`, mirroring the self-copy skip
        // above: an ignored-app skip must not leave `last_marker` stale, or a
        // later genuine copy of different content from a *non*-ignored app
        // would be compared against outdated state (§2.2's rationale applies
        // here too).
        if let Some(app_name) = source_app.as_deref() {
            if Self::is_ignored_app(app_handle, app_name) {
                log::info!("Skipping clipboard change from ignored app: {}", app_name);
                return;
            }
        }

        match snapshot {
            ClipboardSnapshot::Files(paths) => {
                Self::process_files_change(app_handle, paths, source_app)
            }
            ClipboardSnapshot::Text { text, html } => {
                Self::process_text_change(app_handle, &text, html, source_app)
            }
            ClipboardSnapshot::Image(image) => {
                Self::process_image_change(app_handle, running, image, &marker, source_app)
            }
        }
    }

    /// Whether the clipboard monitor is currently paused (SPEC-4 §3).
    fn capture_is_paused(app_handle: &AppHandle) -> bool {
        use crate::AppState;
        app_handle.state::<AppState>().settings.get().capture_paused
    }

    /// Whether `app_name` (the frontmost app captured just before dispatch)
    /// is on the configured ignore list (SPEC-4 §3).
    fn is_ignored_app(app_handle: &AppHandle, app_name: &str) -> bool {
        use crate::AppState;
        let ignored_apps = app_handle.state::<AppState>().settings.get().ignored_apps;
        app_name_matches_ignore_list(app_name, &ignored_apps)
    }

    fn process_text_change(
        app_handle: &AppHandle,
        text: &str,
        html: Option<String>,
        source_app: Option<String>,
    ) {
        use crate::AppState;

        let settings = app_handle.state::<AppState>().settings.get();
        let max_text_bytes = settings.max_text_bytes;

        if !content_within_size_limit(text.len(), max_text_bytes) {
            log::info!(
                "Skipping text clip: {} bytes exceeds max_text_bytes ({})",
                text.len(),
                max_text_bytes
            );
            return;
        }

        if let Some(kind) = secret_skip_reason(text, settings.skip_secrets) {
            log::info!("🔒 Skipping captured secret ({kind})");
            return;
        }

        let html = clamp_html_to_size_limit(html, max_text_bytes);

        log::info!("📋 Text clipboard changed: {} chars", text.len());
        let item = ClipItem {
            id: Uuid::new_v4().to_string(),
            content: text.as_bytes().to_vec(),
            thumbnail: None,
            content_type: ContentType::Text,
            timestamp: Utc::now().timestamp(),
            is_pinned: false,
            pin_order: None,
            label: None,
            group_name: None,
            source_app,
            html,
        };
        Self::save_to_storage(app_handle, item);
    }

    fn process_files_change(
        app_handle: &AppHandle,
        paths: Vec<String>,
        source_app: Option<String>,
    ) {
        use crate::AppState;

        let max_text_bytes = app_handle.state::<AppState>().settings.get().max_text_bytes;
        let content = join_file_paths(&paths).into_bytes();

        if !content_within_size_limit(content.len(), max_text_bytes) {
            log::info!(
                "Skipping files clip: {} bytes exceeds max_text_bytes ({})",
                content.len(),
                max_text_bytes
            );
            return;
        }

        log::info!("📁 Files clipboard changed: {} path(s)", paths.len());
        let item = ClipItem {
            id: Uuid::new_v4().to_string(),
            content,
            thumbnail: None,
            content_type: ContentType::Files,
            timestamp: Utc::now().timestamp(),
            is_pinned: false,
            pin_order: None,
            label: None,
            group_name: None,
            source_app,
            html: None,
        };
        Self::save_to_storage(app_handle, item);
    }

    fn process_image_change(
        app_handle: &AppHandle,
        running: &Arc<AtomicBool>,
        image: ImageData<'static>,
        marker: &CopyMarker,
        source_app: Option<String>,
    ) {
        let width = image.width;
        let height = image.height;
        // The snapshot already owns its RGBA buffer (arboard hands back a
        // `Cow::Owned`), so `into_owned` takes it by move — no 33MB re-copy
        // like the previous `to_vec` (#34).
        let rgba_bytes = image.bytes.into_owned();
        let marker = marker.clone();
        log::info!(
            "Image clipboard changed: {}x{}, {} RGBA bytes",
            width,
            height,
            rgba_bytes.len()
        );

        // The caller (handle_clipboard_event) already captured `source_app`
        // synchronously, before this image processing — which runs on a
        // background task below — has a chance to run. That preserves the
        // original "snapshot before async" guarantee even though the read
        // itself now happens one level up.
        let max_image_dimension = {
            use crate::AppState;
            app_handle
                .state::<AppState>()
                .settings
                .get()
                .max_image_dimension
        };
        let app_handle = app_handle.clone();
        let running = running.clone();
        tauri::async_runtime::spawn(async move {
            let processed = tauri::async_runtime::spawn_blocking(move || {
                Self::process_clipboard_image(
                    width,
                    height,
                    rgba_bytes,
                    marker,
                    max_image_dimension,
                )
            })
            .await;

            match processed {
                Ok(Ok(processed)) => {
                    if !running.load(Ordering::SeqCst) {
                        log::debug!("Skipping processed image save after monitor stopped");
                        return;
                    }

                    let item = ClipItem {
                        id: Uuid::new_v4().to_string(),
                        content: processed.content_png,
                        thumbnail: Some(processed.thumbnail_png),
                        content_type: processed.marker.content_type,
                        timestamp: Utc::now().timestamp(),
                        is_pinned: false,
                        pin_order: None,
                        label: None,
                        group_name: None,
                        source_app,
                        html: None,
                    };
                    Self::save_to_storage(&app_handle, item);
                }
                Ok(Err(e)) => log::error!("Failed to process clipboard image: {}", e),
                Err(e) => log::error!("Image processing task failed: {}", e),
            }
        });
    }

    fn should_ignore_current_clipboard(app_handle: &AppHandle) -> bool {
        use crate::AppState;

        let state = app_handle.state::<AppState>();
        if !state.settings.get().ignore_concealed {
            return false;
        }

        clipboard_has_sensitive_marker()
    }

    fn is_self_copied(
        last_copied_by_us: &Arc<Mutex<Option<CopyMarker>>>,
        expected_marker: &CopyMarker,
    ) -> bool {
        crate::safe_lock(last_copied_by_us).as_ref() == Some(expected_marker)
    }

    fn save_to_storage(app_handle: &AppHandle, item: ClipItem) {
        use crate::storage::{ClipPreviewItem, FrontendClipItem};
        use crate::tray::update_tray_menu;
        use crate::AppState;

        let state = app_handle.state::<AppState>();
        let max_history_items = state.settings.get().max_history_items;

        let result = {
            let storage = crate::safe_lock(&state.storage);

            storage
                .insert(&item, max_history_items)
                .and_then(|existing_id| {
                    if let Some(id) = existing_id {
                        log::debug!("Updated existing item {} timestamp", id);
                        if let Some(existing_item) = storage.get_preview_by_id(&id)? {
                            return Ok(FrontendClipItem::from_preview(existing_item));
                        }

                        log::warn!("Duplicate item {} was not found after timestamp update", id);
                        return Ok(FrontendClipItem::from_preview(
                            ClipPreviewItem::from_clip_item_with_id(&item, id),
                        ));
                    }

                    Ok(FrontendClipItem::from_preview(
                        ClipPreviewItem::from_clip_item(&item),
                    ))
                })
        };

        match result {
            Ok(item_for_emit) => {
                app_handle.emit("clipboard-changed", &item_for_emit).ok();
                log::debug!("Updating tray menu...");
                update_tray_menu(app_handle);
                log::debug!("Clipboard item saved/updated and tray updated");
            }
            Err(e) => {
                log::error!("Failed to save clipboard item: {}", e);
            }
        }
    }

    fn process_clipboard_image(
        width: usize,
        height: usize,
        rgba_bytes: Vec<u8>,
        marker: CopyMarker,
        max_image_dimension: u32,
    ) -> Result<ProcessedClipboardImage, String> {
        const THUMBNAIL_SIZE: u32 = 256;

        let image: RgbaImage = ImageBuffer::from_raw(width as u32, height as u32, rgba_bytes)
            .ok_or_else(|| "Invalid clipboard image buffer dimensions".to_string())?;
        let image = DynamicImage::ImageRgba8(image);
        let image = Self::downscale_if_oversized(image, max_image_dimension);

        let thumbnail = image.resize(
            THUMBNAIL_SIZE,
            THUMBNAIL_SIZE,
            image::imageops::FilterType::Lanczos3,
        );
        let content_png = Self::encode_png(&image)?;
        let thumbnail_png = Self::encode_png(&thumbnail)?;

        log::info!(
            "Processed clipboard image: {}x{} -> {} bytes, thumbnail {} bytes",
            image.width(),
            image.height(),
            content_png.len(),
            thumbnail_png.len()
        );

        Ok(ProcessedClipboardImage {
            content_png,
            thumbnail_png,
            marker,
        })
    }

    fn encode_png(image: &DynamicImage) -> Result<Vec<u8>, String> {
        let mut buffer = Vec::new();
        image
            .write_to(
                &mut std::io::Cursor::new(&mut buffer),
                image::ImageFormat::Png,
            )
            .map_err(|e| format!("Failed to encode PNG: {}", e))?;
        Ok(buffer)
    }

    /// Downsamples an oversized clipboard image so its longest side fits
    /// within `max_dimension` (§5), preserving aspect ratio. `0` is a
    /// deliberate escape hatch that disables downscaling entirely. Images
    /// already within the limit are returned untouched.
    fn downscale_if_oversized(image: DynamicImage, max_dimension: u32) -> DynamicImage {
        if max_dimension == 0 {
            return image;
        }

        let longest_side = image.width().max(image.height());
        if longest_side <= max_dimension {
            return image;
        }

        log::info!(
            "Downsampling oversized clipboard image {}x{} to fit within {}px",
            image.width(),
            image.height(),
            max_dimension
        );
        image.resize(
            max_dimension,
            max_dimension,
            image::imageops::FilterType::Lanczos3,
        )
    }
}

/// Whether Text/Files content of `content_len` bytes is small enough to
/// capture under `max_text_bytes` (§5). Oversized content is skipped
/// entirely rather than truncated, since a truncated path list or partial
/// text is worse than no clip at all.
fn content_within_size_limit(content_len: usize, max_text_bytes: usize) -> bool {
    content_len <= max_text_bytes
}

/// Whether `text` should be skipped because it matches a high-confidence
/// secret pattern (SPEC-4 §2), and if so, the kind for logging. Only Text
/// content is checked — Files/Image never reach this function. Kept pure and
/// separate from `crate::secrets::detect_secret` itself so the `skip_secrets`
/// gate is unit-testable without a running `AppState`.
fn secret_skip_reason(text: &str, skip_secrets: bool) -> Option<&'static str> {
    if !skip_secrets {
        return None;
    }
    crate::secrets::detect_secret(text)
}

/// Whether `app_name` case-insensitively matches (after trimming) any entry
/// in `ignored_apps` (SPEC-4 §3). `settings.rs` already normalizes stored
/// entries, but trimming/lowercasing again here is cheap and keeps this
/// function correct independent of that invariant.
fn app_name_matches_ignore_list(app_name: &str, ignored_apps: &[String]) -> bool {
    let app_name = app_name.trim().to_lowercase();
    ignored_apps
        .iter()
        .any(|ignored| ignored.trim().to_lowercase() == app_name)
}

/// Drops an html companion that exceeds `max_text_bytes` (§5), keeping only
/// the plain text. HTML is optional metadata, so an oversized html on an
/// otherwise-fine text clip degrades to plain text rather than skipping the
/// whole clip.
fn clamp_html_to_size_limit(html: Option<String>, max_text_bytes: usize) -> Option<String> {
    html.filter(|html| {
        let within_limit = content_within_size_limit(html.len(), max_text_bytes);
        if !within_limit {
            log::debug!(
                "Dropping oversized html companion: {} bytes exceeds max_text_bytes ({})",
                html.len(),
                max_text_bytes
            );
        }
        within_limit
    })
}

fn join_monitor_thread_with_timeout(handle: JoinHandle<()>) {
    let (sender, receiver) = mpsc::channel();
    std::thread::spawn(move || {
        let result = handle.join();
        let _ = sender.send(result);
    });

    match receiver.recv_timeout(MONITOR_STOP_TIMEOUT) {
        Ok(Ok(())) => {}
        Ok(Err(_)) => log::warn!("Clipboard monitoring thread panicked while stopping"),
        Err(mpsc::RecvTimeoutError::Timeout) => log::warn!(
            "Timed out waiting for clipboard monitoring thread to stop; continuing shutdown"
        ),
        Err(mpsc::RecvTimeoutError::Disconnected) => {
            log::warn!("Clipboard monitoring thread join waiter disconnected")
        }
    }
}

#[cfg(target_os = "macos")]
fn clipboard_has_sensitive_marker() -> bool {
    use objc2_app_kit::NSPasteboard;

    const IGNORED_PASTEBOARD_TYPES: &[&str] = &[
        "org.nspasteboard.ConcealedType",
        "org.nspasteboard.TransientType",
        "org.nspasteboard.AutoGeneratedType",
    ];

    let pasteboard = NSPasteboard::generalPasteboard();
    let Some(types) = pasteboard.types() else {
        return false;
    };

    for pasteboard_type in types.iter() {
        let type_name = pasteboard_type.to_string();

        if IGNORED_PASTEBOARD_TYPES.contains(&type_name.as_str()) {
            log::info!("Skipping macOS pasteboard marker type: {}", type_name);
            return true;
        }
    }

    false
}

#[cfg(target_os = "windows")]
fn clipboard_has_sensitive_marker() -> bool {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::System::DataExchange::{CloseClipboard, OpenClipboard};

    unsafe {
        if let Err(e) = OpenClipboard(HWND(std::ptr::null_mut())) {
            log::warn!("Failed to open clipboard for privacy marker check: {}", e);
            return false;
        }

        let should_ignore = windows_clipboard_has_exclude_marker()
            || windows_can_include_clipboard_history_is_false();

        if let Err(e) = CloseClipboard() {
            log::warn!(
                "Failed to close clipboard after privacy marker check: {}",
                e
            );
        }

        should_ignore
    }
}

#[cfg(target_os = "windows")]
fn windows_clipboard_has_exclude_marker() -> bool {
    let format = windows_clipboard_format("ExcludeClipboardContentFromMonitorProcessing");
    if windows_clipboard_format_available(format) {
        log::info!("Skipping Windows clipboard history monitor opt-out marker");
        return true;
    }

    false
}

#[cfg(target_os = "windows")]
fn windows_can_include_clipboard_history_is_false() -> bool {
    let format = windows_clipboard_format("CanIncludeInClipboardHistory");
    if format == 0 || !windows_clipboard_format_available(format) {
        return false;
    }

    if windows_clipboard_dword(format) == Some(0) {
        log::info!("Skipping Windows clipboard history include marker set to 0");
        return true;
    }

    false
}

#[cfg(target_os = "windows")]
fn windows_clipboard_format(name: &str) -> u32 {
    use windows::core::PCWSTR;
    use windows::Win32::System::DataExchange::RegisterClipboardFormatW;

    let wide_name: Vec<u16> = name.encode_utf16().chain(std::iter::once(0)).collect();
    unsafe { RegisterClipboardFormatW(PCWSTR(wide_name.as_ptr())) }
}

#[cfg(target_os = "windows")]
fn windows_clipboard_format_available(format: u32) -> bool {
    use windows::Win32::System::DataExchange::IsClipboardFormatAvailable;

    format != 0 && unsafe { IsClipboardFormatAvailable(format).is_ok() }
}

#[cfg(target_os = "windows")]
fn windows_clipboard_dword(format: u32) -> Option<u32> {
    use windows::Win32::System::DataExchange::GetClipboardData;

    unsafe {
        let handle = GetClipboardData(format).ok()?;
        if handle.is_invalid() || GlobalSize(handle) < std::mem::size_of::<u32>() {
            return None;
        }

        let data = GlobalLock(handle);
        if data.is_null() {
            return None;
        }

        let bytes = std::slice::from_raw_parts(data.cast::<u8>(), std::mem::size_of::<u32>());
        let value = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        let _ = GlobalUnlock(handle);
        Some(value)
    }
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
fn clipboard_has_sensitive_marker() -> bool {
    false
}

/// Localized name of the app that was frontmost at capture time — the source
/// the clip was copied from. Returns None when ClipMan itself is frontmost or
/// the name is unavailable.
// ponytail: reads NSWorkspace off the monitor thread, same as window.rs does
// off the command thread; AppKit's frontmostApplication tolerates it.
#[cfg(target_os = "macos")]
fn frontmost_app_name() -> Option<String> {
    use objc2_app_kit::NSWorkspace;

    let front = NSWorkspace::sharedWorkspace().frontmostApplication()?;
    if front.processIdentifier() == std::process::id() as i32 {
        return None;
    }
    front.localizedName().map(|name| name.to_string())
}

#[cfg(not(target_os = "macos"))]
fn frontmost_app_name() -> Option<String> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn processed_clipboard_image_uses_raw_rgba_marker() {
        let rgba_bytes = vec![255, 0, 0, 255];
        let marker = CopyMarker::from_normalized_image_parts(1, 1, &rgba_bytes);

        let processed =
            ClipboardMonitor::process_clipboard_image(1, 1, rgba_bytes, marker.clone(), 0).unwrap();

        assert_eq!(marker, processed.marker);
        assert!(!processed.content_png.is_empty());
        assert!(!processed.thumbnail_png.is_empty());
    }

    #[test]
    fn processed_clipboard_image_preserves_original_content_dimensions() {
        let width = 2050;
        let height = 1;
        let rgba_bytes = vec![255; width * height * 4];
        let marker = CopyMarker::from_normalized_image_parts(width, height, &rgba_bytes);

        // max_image_dimension = 0 disables downscaling, so this only exercises
        // PNG encoding fidelity, independent of the §5 size-limit feature.
        let processed =
            ClipboardMonitor::process_clipboard_image(width, height, rgba_bytes, marker, 0)
                .unwrap();
        let content = image::load_from_memory(&processed.content_png).unwrap();

        assert_eq!(width as u32, content.width());
        assert_eq!(height as u32, content.height());
    }

    #[test]
    fn oversized_image_is_downsampled_to_max_dimension() {
        let width = 2000;
        let height = 1000;
        let rgba_bytes = vec![200u8; width * height * 4];
        let marker = CopyMarker::from_normalized_image_parts(width, height, &rgba_bytes);

        let processed =
            ClipboardMonitor::process_clipboard_image(width, height, rgba_bytes, marker, 500)
                .unwrap();
        let content = image::load_from_memory(&processed.content_png).unwrap();

        assert_eq!(500, content.width());
        assert_eq!(250, content.height());

        // The (unrelated) thumbnail pipeline still runs on the downsampled
        // image and stays within its own 256px bound.
        let thumbnail = image::load_from_memory(&processed.thumbnail_png).unwrap();
        assert!(thumbnail.width() <= 256 && thumbnail.height() <= 256);
    }

    #[test]
    fn image_within_max_dimension_is_not_resized() {
        let width = 100;
        let height = 50;
        let rgba_bytes = vec![5u8; width * height * 4];
        let marker = CopyMarker::from_normalized_image_parts(width, height, &rgba_bytes);

        let processed =
            ClipboardMonitor::process_clipboard_image(width, height, rgba_bytes, marker, 4096)
                .unwrap();
        let content = image::load_from_memory(&processed.content_png).unwrap();

        assert_eq!(width as u32, content.width());
        assert_eq!(height as u32, content.height());
    }

    #[test]
    fn max_image_dimension_zero_disables_downscaling() {
        let width = 3000;
        let height = 1;
        let rgba_bytes = vec![10u8; width * height * 4];
        let marker = CopyMarker::from_normalized_image_parts(width, height, &rgba_bytes);

        let processed =
            ClipboardMonitor::process_clipboard_image(width, height, rgba_bytes, marker, 0)
                .unwrap();
        let content = image::load_from_memory(&processed.content_png).unwrap();

        assert_eq!(width as u32, content.width());
    }

    #[test]
    fn text_and_files_content_size_limit_rejects_only_oversized_content() {
        assert!(content_within_size_limit(100, 100));
        assert!(!content_within_size_limit(101, 100));
    }

    #[test]
    fn oversized_html_is_dropped_but_undersized_html_and_missing_html_are_kept() {
        let big_html = "x".repeat(101);
        assert_eq!(None, clamp_html_to_size_limit(Some(big_html), 100));

        let small_html = "x".repeat(100);
        assert_eq!(
            Some(small_html.clone()),
            clamp_html_to_size_limit(Some(small_html), 100)
        );

        assert_eq!(None, clamp_html_to_size_limit(None, 100));
    }

    #[test]
    fn secret_skip_reason_flags_detected_secrets_only_when_enabled() {
        let secret = "AKIAIOSFODNN7EXAMPLE";

        assert_eq!(
            Some("AWS access key"),
            secret_skip_reason(secret, true),
            "skip_secrets=true must surface the detected kind"
        );
        assert_eq!(
            None,
            secret_skip_reason(secret, false),
            "skip_secrets=false must never intercept, even for a clear secret match"
        );
    }

    #[test]
    fn secret_skip_reason_is_none_for_ordinary_text() {
        assert_eq!(None, secret_skip_reason("just some ordinary text", true));
    }

    #[test]
    fn app_name_matches_ignore_list_is_case_insensitive_and_trims_both_sides() {
        let ignored = vec![" Safari ".to_string(), "1Password".to_string()];

        assert!(app_name_matches_ignore_list("safari", &ignored));
        assert!(app_name_matches_ignore_list("SAFARI", &ignored));
        assert!(app_name_matches_ignore_list(" Safari", &ignored));
        assert!(app_name_matches_ignore_list("1password", &ignored));
    }

    #[test]
    fn app_name_matches_ignore_list_is_false_for_unlisted_or_empty_list() {
        let ignored = vec!["Safari".to_string()];

        assert!(!app_name_matches_ignore_list("Notes", &ignored));
        assert!(!app_name_matches_ignore_list("Safari", &[]));
    }

    #[test]
    fn snapshot_marker_matches_primary_content_and_ignores_html() {
        // Files hash the newline-joined path text.
        let paths = vec!["/a/b.txt".to_string(), "/c/d.png".to_string()];
        assert_eq!(
            CopyMarker::from_payload(ContentType::Files, join_file_paths(&paths).as_bytes()),
            snapshot_marker(&ClipboardSnapshot::Files(paths.clone()))
        );

        // Text hashes only the plain text: identical text with different html
        // produces the same marker (D5).
        let plain = snapshot_marker(&ClipboardSnapshot::Text {
            text: "hello".to_string(),
            html: None,
        });
        let rich = snapshot_marker(&ClipboardSnapshot::Text {
            text: "hello".to_string(),
            html: Some("<b>hello</b>".to_string()),
        });
        assert_eq!(CopyMarker::from_payload(ContentType::Text, b"hello"), plain);
        assert_eq!(plain, rich);

        // Image hashes normalized dimensions + RGBA bytes.
        let bytes = vec![1u8, 2, 3, 4];
        let image = ImageData {
            width: 1,
            height: 1,
            bytes: std::borrow::Cow::Owned(bytes.clone()),
        };
        assert_eq!(
            CopyMarker::from_normalized_image_parts(1, 1, &bytes),
            snapshot_marker(&ClipboardSnapshot::Image(image))
        );
    }

    #[test]
    fn self_copy_skip_still_advances_last_marker() {
        let marker = CopyMarker::from_payload(ContentType::Text, b"hello");
        let mut last_marker = None;

        // The first observation is our own paste: skip dispatch, but still record
        // the marker so the next genuine copy of *different* content is not
        // compared against stale None state (§2.2).
        assert!(!record_marker_and_decide_dispatch(
            &marker,
            &mut last_marker,
            || true
        ));
        assert_eq!(Some(&marker), last_marker.as_ref());

        // Observing the same content again deduplicates without even consulting
        // the self-copy check.
        assert!(!record_marker_and_decide_dispatch(
            &marker,
            &mut last_marker,
            || panic!("unchanged marker must not re-check self-copy")
        ));
        assert_eq!(Some(&marker), last_marker.as_ref());

        // Genuinely new content we did not copy: dispatch and advance.
        let next = CopyMarker::from_payload(ContentType::Text, b"world");
        assert!(record_marker_and_decide_dispatch(
            &next,
            &mut last_marker,
            || false
        ));
        assert_eq!(Some(&next), last_marker.as_ref());
    }
}
