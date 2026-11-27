use std::sync::{Arc, Mutex};

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, PhysicalSize, WebviewWindow};

pub const QUICKBAR_WINDOW_LABEL: &str = "main";
pub const SETTINGS_WINDOW_LABEL: &str = "settings";
const QUICKBAR_MAX_WIDTH: f64 = 820.0;
const QUICKBAR_MAX_HEIGHT: f64 = 600.0;
pub const QUICKBAR_HIDDEN_EVENT: &str = "quickbar-hidden";

pub type ForegroundWindowStore = Arc<Mutex<Option<isize>>>;
pub type QuickBarPanelStore = Arc<Mutex<QuickBarPanel>>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum QuickBarPanel {
    Recent,
    Pinned,
}

impl TryFrom<&str> for QuickBarPanel {
    type Error = String;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "recent" => Ok(Self::Recent),
            "pinned" => Ok(Self::Pinned),
            _ => Err(format!("Invalid QuickBar panel '{value}'")),
        }
    }
}

pub fn setup_windows(app: &AppHandle) -> Result<(), String> {
    let quickbar = get_window(app, QUICKBAR_WINDOW_LABEL)?;
    setup_quickbar_macos(&quickbar)?;
    setup_quickbar_windows(&quickbar)?;
    register_quickbar_events(&quickbar);

    register_settings_events(&get_window(app, SETTINGS_WINDOW_LABEL)?);

    Ok(())
}

pub fn show_quickbar(
    app: &AppHandle,
    foreground_store: &ForegroundWindowStore,
    panel_store: &QuickBarPanelStore,
) -> Result<(), String> {
    show_quickbar_with_panel(app, foreground_store, panel_store, QuickBarPanel::Recent)
}

pub fn set_active_quickbar_panel(panel_store: &QuickBarPanelStore, panel: QuickBarPanel) {
    *panel_store.lock().unwrap() = panel;
}

pub fn toggle_quickbar_with_panel(
    app: &AppHandle,
    foreground_store: &ForegroundWindowStore,
    panel_store: &QuickBarPanelStore,
    panel: QuickBarPanel,
) -> Result<(), String> {
    let quickbar = get_window(app, QUICKBAR_WINDOW_LABEL)?;
    let visible = quickbar.is_visible().map_err(to_string)?;
    let focused = quickbar.is_focused().map_err(to_string)?;
    if visible && focused {
        if *panel_store.lock().unwrap() == panel {
            return hide_quickbar_and_restore(app, foreground_store);
        }
        *panel_store.lock().unwrap() = panel;
        return app
            .emit("quickbar-opened", serde_json::json!({ "panel": panel }))
            .map_err(to_string);
    }
    show_quickbar_with_panel(app, foreground_store, panel_store, panel)
}

pub fn show_quickbar_with_panel(
    app: &AppHandle,
    foreground_store: &ForegroundWindowStore,
    panel_store: &QuickBarPanelStore,
    panel: QuickBarPanel,
) -> Result<(), String> {
    let quickbar = get_window(app, QUICKBAR_WINDOW_LABEL)?;

    remember_foreground_window(foreground_store, &quickbar);
    let geometry_changed = position_quickbar(&quickbar)?;

    if quickbar.is_minimized().map_err(to_string)? {
        quickbar.unminimize().map_err(to_string)?;
    }
    quickbar.show().map_err(to_string)?;
    focus_quickbar(&quickbar)?;
    if geometry_changed {
        invalidate_quickbar_shadow(&quickbar);
    }
    *panel_store.lock().unwrap() = panel;
    app.emit("quickbar-opened", serde_json::json!({ "panel": panel }))
        .map_err(to_string)
}

pub fn hide_quickbar(app: &AppHandle) -> Result<(), String> {
    let quickbar = get_window(app, QUICKBAR_WINDOW_LABEL)?;
    hide_quickbar_window(&quickbar)
}

pub fn hide_quickbar_and_restore(
    app: &AppHandle,
    foreground_store: &ForegroundWindowStore,
) -> Result<(), String> {
    let quickbar = get_window(app, QUICKBAR_WINDOW_LABEL)?;
    hide_quickbar_window(&quickbar)?;
    #[cfg(target_os = "macos")]
    {
        let settings_visible = match app.get_webview_window(SETTINGS_WINDOW_LABEL) {
            Some(window) => window.is_visible().map_err(to_string)?,
            None => false,
        };
        if !settings_visible {
            let target = *foreground_store.lock().unwrap();
            if let Err(error) = restore_recorded_foreground_window(target) {
                log::debug!("Did not restore previous app after QuickBar hide: {error}");
            }
        }
    }
    #[cfg(not(target_os = "macos"))]
    let _ = foreground_store;
    Ok(())
}

pub fn open_settings_window(app: &AppHandle) -> Result<(), String> {
    let settings = get_window(app, SETTINGS_WINDOW_LABEL)?;

    settings.unminimize().map_err(to_string)?;
    settings.show().map_err(to_string)?;
    focus_settings_window(&settings)?;

    Ok(())
}

#[cfg(windows)]
pub fn restore_recorded_foreground_window(target: Option<isize>) -> Result<(), String> {
    let target = target
        .ok_or_else(|| "No foreground window was recorded before QuickBar opened".to_string())?;
    restore_foreground_window(target)
}

#[cfg(target_os = "macos")]
pub fn restore_recorded_foreground_window(target: Option<isize>) -> Result<(), String> {
    use objc2_app_kit::{NSApplicationActivationOptions, NSRunningApplication};

    let target = target
        .ok_or_else(|| "No foreground app was recorded before QuickBar opened".to_string())?;
    let pid = target as i32;

    let Some(running) = NSRunningApplication::runningApplicationWithProcessIdentifier(pid) else {
        return Err(format!("Recorded app (pid {pid}) is no longer running"));
    };

    if !running.activateWithOptions(NSApplicationActivationOptions::empty()) {
        return Err(format!("Failed to reactivate app (pid {pid})"));
    }

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
            let _ = window_for_event.emit("settings-hidden", ());
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

fn position_quickbar(window: &WebviewWindow) -> Result<bool, String> {
    let Some(monitor) = quickbar_monitor(window) else {
        log::warn!("No monitor detected for QuickBar positioning");
        return Ok(false);
    };

    let work_area = monitor.work_area();
    let work_width = work_area.size.width as i32;
    let work_height = work_area.size.height as i32;

    let scale = monitor.scale_factor().max(0.1);
    let logical_w = (work_width as f64 / scale * 0.92).min(QUICKBAR_MAX_WIDTH);
    let logical_h = (work_height as f64 / scale * 0.7).min(QUICKBAR_MAX_HEIGHT);

    let width = (logical_w * scale).round() as i32;
    let height = (logical_h * scale).round() as i32;

    let size = PhysicalSize::new(width.max(1) as u32, height.max(1) as u32);
    let resized = window.inner_size().ok() != Some(size);
    if resized {
        window.set_size(size).map_err(to_string)?;
    }

    let x = work_area.position.x + ((work_width - width) / 2).max(0);
    let y = work_area.position.y + ((work_height - height) / 3).max(0);

    let position = PhysicalPosition::new(x, y);
    let moved = window.outer_position().ok() != Some(position);
    if moved {
        window.set_position(position).map_err(to_string)?;
    }
    Ok(resized || moved)
}

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

#[cfg(target_os = "macos")]
fn setup_quickbar_macos(window: &WebviewWindow) -> Result<(), String> {
    use objc2_app_kit::{NSFloatingWindowLevel, NSWindowCollectionBehavior};

    let ns_window = ns_window(window)?;

    window.set_shadow(true).map_err(to_string)?;

    window
        .set_visible_on_all_workspaces(true)
        .map_err(to_string)?;
    ns_window.setLevel(NSFloatingWindowLevel);
    ns_window.setHidesOnDeactivate(false);
    ns_window.setCollectionBehavior(
        NSWindowCollectionBehavior::CanJoinAllSpaces
            | NSWindowCollectionBehavior::FullScreenAuxiliary
            | NSWindowCollectionBehavior::Transient
            | NSWindowCollectionBehavior::IgnoresCycle,
    );

    log::info!("Configured QuickBar macOS floating window");
    Ok(())
}

#[cfg(not(target_os = "macos"))]
fn setup_quickbar_macos(_window: &WebviewWindow) -> Result<(), String> {
    Ok(())
}

#[cfg(windows)]
fn webview_hwnd(window: &WebviewWindow) -> Result<windows::Win32::Foundation::HWND, String> {
    use windows::Win32::Foundation::HWND;

    Ok(HWND(window.hwnd().map_err(to_string)?.0))
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
        *store.lock().unwrap() = None;
        return;
    }

    if let Ok(quickbar_hwnd) = webview_hwnd(quickbar) {
        if foreground == quickbar_hwnd {
            log::debug!("QuickBar is already the foreground window; keeping previous target");
            return;
        }
    }

    *store.lock().unwrap() = Some(foreground.0 as isize);
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

    *store.lock().unwrap() = Some(pid as isize);
    log::debug!("Recorded frontmost macOS app pid {pid} before QuickBar show");
}

#[cfg(all(not(windows), not(target_os = "macos")))]
fn remember_foreground_window(_store: &ForegroundWindowStore, _quickbar: &WebviewWindow) {}

#[cfg(windows)]
fn restore_foreground_window(target: isize) -> Result<(), String> {
    use std::ffi::c_void;
    use windows::Win32::Foundation::HWND;
    use windows::Win32::System::Threading::{AttachThreadInput, GetCurrentThreadId};
    use windows::Win32::UI::WindowsAndMessaging::{
        GetForegroundWindow, GetWindowThreadProcessId, IsWindow, SetForegroundWindow,
    };

    let hwnd = HWND(target as *mut c_void);

    unsafe {
        if !IsWindow(hwnd).as_bool() {
            return Err("Recorded foreground window no longer exists".to_string());
        }

        if SetForegroundWindow(hwnd).as_bool() {
            return Ok(());
        }

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

#[cfg(target_os = "macos")]
pub fn frontmost_app_pid() -> Option<i32> {
    use objc2_app_kit::NSWorkspace;

    NSWorkspace::sharedWorkspace()
        .frontmostApplication()
        .map(|app| app.processIdentifier())
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

fn focus_settings_window(window: &WebviewWindow) -> Result<(), String> {
    window.set_focus().map_err(to_string)
}

fn to_string(error: impl std::fmt::Display) -> String {
    error.to_string()
}
