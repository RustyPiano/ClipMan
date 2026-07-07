use std::sync::{Arc, Mutex};

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, PhysicalSize, WebviewWindow};

pub const QUICKBAR_WINDOW_LABEL: &str = "main";
pub const SETTINGS_WINDOW_LABEL: &str = "settings";
// Adaptive sizing caps (logical px): comfortable max on large screens, scaled
// down proportionally on smaller ones. The initial window size lives in
// `tauri.conf.json` (820×600); the real size is computed per-monitor in
// `position_quickbar`.
const QUICKBAR_MAX_WIDTH: f64 = 820.0;
const QUICKBAR_MAX_HEIGHT: f64 = 600.0;
pub const QUICKBAR_HIDDEN_EVENT: &str = "quickbar-hidden";

pub type ForegroundWindowStore = Arc<Mutex<Option<ForegroundWindow>>>;

#[derive(Debug, Clone, Copy)]
pub enum QuickBarPanel {
    Recent,
    Pinned,
}

impl QuickBarPanel {
    fn as_str(self) -> &'static str {
        match self {
            QuickBarPanel::Recent => "recent",
            QuickBarPanel::Pinned => "pinned",
        }
    }
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct QuickBarOpenedPayload {
    panel: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ForegroundWindow {
    raw: isize,
}

impl ForegroundWindow {
    pub fn raw(self) -> isize {
        self.raw
    }
}

#[cfg(target_os = "macos")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct MacQuickBarWindowPolicy {
    visible_on_all_workspaces: bool,
    level: objc2_app_kit::NSWindowLevel,
    collection_behavior: objc2_app_kit::NSWindowCollectionBehavior,
    hides_on_deactivate: bool,
}

#[cfg(target_os = "macos")]
fn mac_quickbar_window_policy() -> MacQuickBarWindowPolicy {
    use objc2_app_kit::{NSStatusWindowLevel, NSWindowCollectionBehavior};

    // QuickBar is a non-activating panel; the target app stays active while it is shown.
    MacQuickBarWindowPolicy {
        visible_on_all_workspaces: true,
        level: NSStatusWindowLevel,
        collection_behavior: NSWindowCollectionBehavior::Auxiliary
            | NSWindowCollectionBehavior::CanJoinAllSpaces
            | NSWindowCollectionBehavior::FullScreenAuxiliary
            | NSWindowCollectionBehavior::Transient
            | NSWindowCollectionBehavior::IgnoresCycle,
        hides_on_deactivate: false,
    }
}

pub fn setup_windows(app: &AppHandle) -> Result<(), String> {
    let quickbar = get_window(app, QUICKBAR_WINDOW_LABEL)?;
    setup_quickbar_window(&quickbar)?;
    register_quickbar_events(&quickbar);

    if let Some(settings) = app.get_webview_window(SETTINGS_WINDOW_LABEL) {
        register_settings_events(&settings);
    } else {
        log::warn!("Settings window is not available at setup");
    }

    Ok(())
}

pub fn show_quickbar(
    app: &AppHandle,
    foreground_store: &ForegroundWindowStore,
) -> Result<(), String> {
    show_quickbar_with_panel(app, foreground_store, QuickBarPanel::Recent)
}

pub fn show_quickbar_with_panel(
    app: &AppHandle,
    foreground_store: &ForegroundWindowStore,
    panel: QuickBarPanel,
) -> Result<(), String> {
    let quickbar = get_window(app, QUICKBAR_WINDOW_LABEL)?;

    remember_foreground_window(foreground_store, &quickbar);
    position_quickbar(&quickbar)?;

    quickbar.unminimize().map_err(to_string)?;
    quickbar.show().map_err(to_string)?;
    focus_quickbar(&quickbar)?;
    invalidate_quickbar_shadow(&quickbar);
    app.emit(
        "quickbar-opened",
        QuickBarOpenedPayload {
            panel: panel.as_str(),
        },
    )
    .map_err(to_string)?;

    Ok(())
}

pub fn hide_quickbar(app: &AppHandle) -> Result<(), String> {
    let quickbar = get_window(app, QUICKBAR_WINDOW_LABEL)?;
    hide_quickbar_window(&quickbar)
}

pub fn open_settings_window(app: &AppHandle) -> Result<(), String> {
    let settings = get_window(app, SETTINGS_WINDOW_LABEL)?;

    settings.unminimize().map_err(to_string)?;
    settings.show().map_err(to_string)?;
    focus_settings_window(&settings)?;

    Ok(())
}

#[cfg(windows)]
pub fn recorded_foreground_window(store: &ForegroundWindowStore) -> Option<ForegroundWindow> {
    *crate::safe_lock(store)
}

#[cfg(windows)]
pub fn restore_recorded_foreground_window(store: &ForegroundWindowStore) -> Result<(), String> {
    let target = recorded_foreground_window(store)
        .ok_or_else(|| "No foreground window was recorded before QuickBar opened".to_string())?;
    restore_foreground_window(target)
}

#[cfg(target_os = "macos")]
pub fn restore_recorded_foreground_window(store: &ForegroundWindowStore) -> Result<(), String> {
    use objc2_app_kit::{NSApplicationActivationOptions, NSRunningApplication};

    let target = (*crate::safe_lock(store))
        .ok_or_else(|| "No foreground app was recorded before QuickBar opened".to_string())?;
    let pid = target.raw() as i32;

    let Some(running) = NSRunningApplication::runningApplicationWithProcessIdentifier(pid) else {
        return Err(format!("Recorded app (pid {pid}) is no longer running"));
    };

    if !running.activateWithOptions(NSApplicationActivationOptions::empty()) {
        return Err(format!("Failed to reactivate app (pid {pid})"));
    }

    Ok(())
}

#[cfg(all(not(windows), not(target_os = "macos")))]
#[allow(dead_code)]
pub fn restore_recorded_foreground_window(_store: &ForegroundWindowStore) -> Result<(), String> {
    Ok(())
}

fn setup_quickbar_window(window: &WebviewWindow) -> Result<(), String> {
    window.set_decorations(false).map_err(to_string)?;
    window.set_resizable(false).map_err(to_string)?;
    window.set_always_on_top(true).map_err(to_string)?;
    window.set_skip_taskbar(true).map_err(to_string)?;
    window.set_focusable(true).map_err(to_string)?;

    setup_quickbar_macos(window)?;
    setup_quickbar_windows(window)?;

    Ok(())
}

fn register_quickbar_events(window: &WebviewWindow) {
    let window_for_event = window.clone();
    window.on_window_event(move |event| match event {
        tauri::WindowEvent::CloseRequested { api, .. } => {
            api.prevent_close();
            if let Err(e) = hide_quickbar_window(&window_for_event) {
                log::error!("Failed to hide QuickBar on close: {}", e);
            }
        }
        tauri::WindowEvent::Focused(false) => {
            if let Err(e) = hide_quickbar_window(&window_for_event) {
                log::error!("Failed to hide QuickBar on blur: {}", e);
            }
        }
        _ => {}
    });
}

/// Recompute the native shadow after the window is shown/resized: with a
/// transparent window the shadow is derived from the rendered alpha shape,
/// which can be stale after a size change on another monitor.
#[cfg(target_os = "macos")]
fn invalidate_quickbar_shadow(window: &WebviewWindow) {
    match ns_window(window) {
        Ok(ns_window) => ns_window.invalidateShadow(),
        Err(e) => log::warn!("Failed to invalidate QuickBar shadow: {e}"),
    }
}

#[cfg(not(target_os = "macos"))]
fn invalidate_quickbar_shadow(_window: &WebviewWindow) {}

fn hide_quickbar_window(window: &WebviewWindow) -> Result<(), String> {
    window.hide().map_err(to_string)?;
    window.emit(QUICKBAR_HIDDEN_EVENT, ()).map_err(to_string)?;
    Ok(())
}

fn register_settings_events(window: &WebviewWindow) {
    let window_for_event = window.clone();
    window.on_window_event(move |event| {
        if let tauri::WindowEvent::CloseRequested { api, .. } = event {
            api.prevent_close();
            if let Err(e) = window_for_event.hide() {
                log::error!("Failed to hide settings window on close: {}", e);
            }
        }
    });
}

fn get_window(app: &AppHandle, label: &str) -> Result<WebviewWindow, String> {
    app.get_webview_window(label)
        .ok_or_else(|| format!("Window '{label}' is not available"))
}

#[cfg(target_os = "macos")]
fn ns_window(window: &WebviewWindow) -> Result<&objc2_app_kit::NSWindow, String> {
    let raw = window.ns_window().map_err(to_string)? as *mut objc2_app_kit::NSWindow;
    unsafe {
        raw.as_ref()
            .ok_or_else(|| "macOS NSWindow pointer is null".to_string())
    }
}

fn position_quickbar(window: &WebviewWindow) -> Result<(), String> {
    let Some(monitor) = quickbar_monitor(window) else {
        log::warn!("No monitor detected for QuickBar positioning");
        return Ok(());
    };

    let work_area = monitor.work_area();
    let work_width = work_area.size.width as i32;
    let work_height = work_area.size.height as i32;

    // Size and centering both derive from the *target* monitor's scale (not the
    // window's current one), so mixed-DPI / primary-monitor fallback stays
    // correct. We compute a logical size, then set the exact physical box.
    let scale = monitor.scale_factor().max(0.1);
    let (logical_w, logical_h) =
        quickbar_logical_size(work_width as f64 / scale, work_height as f64 / scale);

    let width = (logical_w * scale).round() as i32;
    let height = (logical_h * scale).round() as i32;

    window
        .set_size(PhysicalSize::new(width.max(1) as u32, height.max(1) as u32))
        .map_err(to_string)?;

    let x = work_area.position.x + ((work_width - width) / 2).max(0);
    let y = work_area.position.y + ((work_height - height) / 3).max(0);

    window
        .set_position(PhysicalPosition::new(x, y))
        .map_err(to_string)
}

/// Pick the monitor the user is actually working on: the one under the cursor,
/// falling back to the window's current monitor, then the primary monitor.
/// `current_monitor()` alone tracks where the *window* last sat, not the active
/// screen, so on multi-monitor setups it would keep misplacing the QuickBar.
fn quickbar_monitor(window: &WebviewWindow) -> Option<tauri::Monitor> {
    if let Ok(cursor) = window.cursor_position() {
        if let Ok(Some(monitor)) = window.monitor_from_point(cursor.x, cursor.y) {
            return Some(monitor);
        }
    }

    window
        .current_monitor()
        .ok()
        .flatten()
        .or_else(|| window.primary_monitor().ok().flatten())
}

/// Compute the QuickBar logical size from a (logical) work-area: scale to a
/// fraction of the screen, capped at a comfortable max. The fraction guarantees
/// the result never exceeds the work area, so no floor is needed.
fn quickbar_logical_size(work_width: f64, work_height: f64) -> (f64, f64) {
    let width = (work_width * 0.92).min(QUICKBAR_MAX_WIDTH);
    let height = (work_height * 0.7).min(QUICKBAR_MAX_HEIGHT);
    (width, height)
}

#[cfg(target_os = "macos")]
fn setup_quickbar_macos(window: &WebviewWindow) -> Result<(), String> {
    use objc2_app_kit::NSWindowStyleMask;

    let ns_window = ns_window(window)?;
    let policy = mac_quickbar_window_policy();

    // macOS-only native shadow: the window server draws it around the opaque
    // rounded panel, outside the window bounds, so it can't be clipped like a
    // CSS box-shadow. Windows keeps `shadow: false` (tauri.conf.json) because
    // DWM shadows follow the rectangular frame, not the panel shape.
    window.set_shadow(true).map_err(to_string)?;

    window
        .set_visible_on_all_workspaces(policy.visible_on_all_workspaces)
        .map_err(to_string)?;
    ns_window.setStyleMask(ns_window.styleMask() | NSWindowStyleMask::NonactivatingPanel);
    ns_window.setLevel(policy.level);
    ns_window.setHidesOnDeactivate(policy.hides_on_deactivate);
    ns_window.setCollectionBehavior(policy.collection_behavior);

    log::info!("Configured QuickBar macOS non-activating panel style fallback");
    Ok(())
}

#[cfg(not(target_os = "macos"))]
fn setup_quickbar_macos(_window: &WebviewWindow) -> Result<(), String> {
    Ok(())
}

#[cfg(windows)]
fn webview_hwnd(window: &WebviewWindow) -> Result<windows::Win32::Foundation::HWND, String> {
    use windows::Win32::Foundation::HWND;

    // Tauri/wry may use a newer `windows` crate; bridge through the raw handle.
    Ok(HWND(
        window.hwnd().map_err(to_string)?.0 as *mut std::ffi::c_void,
    ))
}

#[cfg(windows)]
fn setup_quickbar_windows(window: &WebviewWindow) -> Result<(), String> {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::WindowsAndMessaging::{
        GetWindowLongPtrW, SetWindowLongPtrW, SetWindowPos, GWL_EXSTYLE, SWP_FRAMECHANGED,
        SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER, WS_EX_TOOLWINDOW,
    };

    let hwnd = webview_hwnd(window)?;
    unsafe {
        let style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
        let tool_window_style = style | WS_EX_TOOLWINDOW.0 as isize;
        SetWindowLongPtrW(hwnd, GWL_EXSTYLE, tool_window_style);
        SetWindowPos(
            hwnd,
            HWND::default(),
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_FRAMECHANGED,
        )
        .map_err(to_string)?;
    }

    log::info!("Configured QuickBar Windows tool window style");
    Ok(())
}

#[cfg(not(windows))]
fn setup_quickbar_windows(_window: &WebviewWindow) -> Result<(), String> {
    Ok(())
}

#[cfg(windows)]
fn remember_foreground_window(store: &ForegroundWindowStore, quickbar: &WebviewWindow) {
    use windows::Win32::UI::WindowsAndMessaging::GetForegroundWindow;

    let foreground = unsafe { GetForegroundWindow() };
    if foreground.is_invalid() {
        log::warn!("No valid Windows foreground window to record");
        *crate::safe_lock(store) = None;
        return;
    }

    if let Ok(quickbar_hwnd) = webview_hwnd(quickbar) {
        if foreground == quickbar_hwnd {
            log::debug!("QuickBar is already the foreground window; keeping previous target");
            return;
        }
    }

    *crate::safe_lock(store) = Some(ForegroundWindow {
        raw: foreground.0 as isize,
    });
    log::debug!("Recorded Windows foreground window before QuickBar show");
}

#[cfg(target_os = "macos")]
fn remember_foreground_window(store: &ForegroundWindowStore, _quickbar: &WebviewWindow) {
    use objc2_app_kit::NSWorkspace;

    let workspace = NSWorkspace::sharedWorkspace();
    let Some(front) = workspace.frontmostApplication() else {
        log::warn!("No frontmost macOS application to record");
        return;
    };

    let pid = front.processIdentifier();
    if pid == std::process::id() as i32 {
        log::debug!("ClipMan is already frontmost; keeping previous paste target");
        return;
    }

    *crate::safe_lock(store) = Some(ForegroundWindow { raw: pid as isize });
    log::debug!("Recorded frontmost macOS app pid {pid} before QuickBar show");
}

#[cfg(all(not(windows), not(target_os = "macos")))]
fn remember_foreground_window(_store: &ForegroundWindowStore, _quickbar: &WebviewWindow) {}

#[cfg(windows)]
fn restore_foreground_window(target: ForegroundWindow) -> Result<(), String> {
    use std::ffi::c_void;
    use windows::Win32::Foundation::HWND;
    use windows::Win32::System::Threading::{AttachThreadInput, GetCurrentThreadId};
    use windows::Win32::UI::WindowsAndMessaging::{
        GetForegroundWindow, GetWindowThreadProcessId, IsWindow, SetForegroundWindow,
    };

    let hwnd = HWND(target.raw as *mut c_void);
    if hwnd.is_invalid() {
        return Err("Recorded foreground window handle is invalid".to_string());
    }

    unsafe {
        if !IsWindow(hwnd).as_bool() {
            return Err("Recorded foreground window no longer exists".to_string());
        }

        if SetForegroundWindow(hwnd).as_bool() {
            Ok(())
        } else {
            let current_thread = GetCurrentThreadId();
            let target_thread = GetWindowThreadProcessId(hwnd, None);
            let foreground = GetForegroundWindow();
            let foreground_thread = if foreground.is_invalid() {
                0
            } else {
                GetWindowThreadProcessId(foreground, None)
            };

            let attached_target = target_thread != 0
                && target_thread != current_thread
                && AttachThreadInput(current_thread, target_thread, true).as_bool();
            let attached_foreground = foreground_thread != 0
                && foreground_thread != current_thread
                && foreground_thread != target_thread
                && AttachThreadInput(current_thread, foreground_thread, true).as_bool();

            let restored = SetForegroundWindow(hwnd).as_bool();

            if attached_foreground {
                let _ = AttachThreadInput(current_thread, foreground_thread, false);
            }
            if attached_target {
                let _ = AttachThreadInput(current_thread, target_thread, false);
            }

            if restored {
                Ok(())
            } else {
                Err("SetForegroundWindow failed for recorded foreground window".to_string())
            }
        }
    }
}

#[cfg(target_os = "macos")]
fn focus_quickbar(window: &WebviewWindow) -> Result<(), String> {
    let ns_window = ns_window(window)?;
    ns_window.orderFrontRegardless();
    ns_window.makeKeyAndOrderFront(None);

    window.set_focus().map_err(to_string)
}

#[cfg(not(target_os = "macos"))]
fn focus_quickbar(window: &WebviewWindow) -> Result<(), String> {
    window.set_focus().map_err(to_string)
}

#[cfg(target_os = "macos")]
fn focus_settings_window(window: &WebviewWindow) -> Result<(), String> {
    use objc2::MainThreadMarker;
    use objc2_app_kit::NSApp;

    let mtm = MainThreadMarker::new()
        .ok_or_else(|| "Settings window focus must run on the main thread".to_string())?;
    NSApp(mtm).activate();

    window.set_focus().map_err(to_string)
}

#[cfg(not(target_os = "macos"))]
fn focus_settings_window(window: &WebviewWindow) -> Result<(), String> {
    window.set_focus().map_err(to_string)
}

fn to_string(error: impl std::fmt::Display) -> String {
    error.to_string()
}

#[cfg(test)]
mod size_tests {
    use super::*;

    #[test]
    fn caps_at_max_on_large_screens() {
        let (w, h) = quickbar_logical_size(2560.0, 1440.0);
        assert_eq!(w, QUICKBAR_MAX_WIDTH);
        assert_eq!(h, QUICKBAR_MAX_HEIGHT);
    }

    #[test]
    fn never_exceeds_work_area_on_small_screens() {
        // A tiny work area must not produce a window larger than itself.
        let (work_w, work_h) = (390.0, 320.0);
        let (w, h) = quickbar_logical_size(work_w, work_h);
        assert!(w <= work_w && h <= work_h);
        assert!(w <= QUICKBAR_MAX_WIDTH && h <= QUICKBAR_MAX_HEIGHT);
    }
}

#[cfg(all(test, target_os = "macos"))]
mod tests {
    use objc2_app_kit::{NSStatusWindowLevel, NSWindowCollectionBehavior};

    #[test]
    fn mac_quickbar_policy_keeps_nonactivating_panel_visible_while_app_is_inactive() {
        assert!(!super::mac_quickbar_window_policy().hides_on_deactivate);
    }

    #[test]
    fn mac_quickbar_policy_is_visible_on_all_workspaces() {
        assert!(super::mac_quickbar_window_policy().visible_on_all_workspaces);
    }

    #[test]
    fn mac_quickbar_policy_marks_window_as_fullscreen_auxiliary() {
        let behavior = super::mac_quickbar_window_policy().collection_behavior;

        assert!(behavior.contains(NSWindowCollectionBehavior::Auxiliary));
        assert!(behavior.contains(NSWindowCollectionBehavior::FullScreenAuxiliary));
        assert!(behavior.contains(NSWindowCollectionBehavior::CanJoinAllSpaces));
    }

    #[test]
    fn mac_quickbar_policy_uses_status_window_level() {
        assert_eq!(
            NSStatusWindowLevel,
            super::mac_quickbar_window_policy().level
        );
    }

    #[test]
    fn quickbar_hidden_event_name_matches_frontend_listener() {
        assert_eq!("quickbar-hidden", super::QUICKBAR_HIDDEN_EVENT);
    }
}
