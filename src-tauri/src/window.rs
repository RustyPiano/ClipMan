use std::sync::{Arc, Mutex};

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, WebviewWindow};

pub const QUICKBAR_WINDOW_LABEL: &str = "main";
pub const SETTINGS_WINDOW_LABEL: &str = "settings";
pub const QUICKBAR_WIDTH: u32 = 560;
pub const QUICKBAR_HEIGHT: u32 = 420;

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
    quickbar.hide().map_err(to_string)
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
    use cocoa::base::{id, nil, BOOL, YES};
    use objc::{class, msg_send, sel, sel_impl};

    let target = (*crate::safe_lock(store))
        .ok_or_else(|| "No foreground app was recorded before QuickBar opened".to_string())?;
    let pid = target.raw() as i32;

    unsafe {
        let running: id = msg_send![
            class!(NSRunningApplication),
            runningApplicationWithProcessIdentifier: pid
        ];
        if running == nil {
            return Err(format!("Recorded app (pid {pid}) is no longer running"));
        }

        // NSApplicationActivateIgnoringOtherApps = 1 << 1
        let activated: BOOL = msg_send![running, activateWithOptions: 1u64 << 1];
        if activated != YES {
            return Err(format!("Failed to reactivate app (pid {pid})"));
        }
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
            if let Err(e) = window_for_event.hide() {
                log::error!("Failed to hide QuickBar on close: {}", e);
            }
        }
        tauri::WindowEvent::Focused(false) => {
            if let Err(e) = window_for_event.hide() {
                log::error!("Failed to hide QuickBar on blur: {}", e);
            }
        }
        _ => {}
    });
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

fn position_quickbar(window: &WebviewWindow) -> Result<(), String> {
    let monitor = window
        .current_monitor()
        .map_err(to_string)?
        .or(window.primary_monitor().map_err(to_string)?);

    let Some(monitor) = monitor else {
        log::warn!("No monitor detected for QuickBar positioning");
        return Ok(());
    };

    let work_area = monitor.work_area();
    let size = window
        .inner_size()
        .unwrap_or_else(|_| (QUICKBAR_WIDTH, QUICKBAR_HEIGHT).into());

    let width = size.width as i32;
    let height = size.height as i32;
    let work_width = work_area.size.width as i32;
    let work_height = work_area.size.height as i32;

    let x = work_area.position.x + ((work_width - width) / 2).max(0);
    let y = work_area.position.y + ((work_height - height) / 3).max(0);

    window
        .set_position(PhysicalPosition::new(x, y))
        .map_err(to_string)
}

#[cfg(target_os = "macos")]
#[allow(unexpected_cfgs)]
fn setup_quickbar_macos(window: &WebviewWindow) -> Result<(), String> {
    use cocoa::appkit::{NSWindow, NSWindowCollectionBehavior};
    use cocoa::base::{id, YES};
    use cocoa::foundation::NSUInteger;
    use objc::{msg_send, sel, sel_impl};

    const NS_FLOATING_WINDOW_LEVEL: i64 = 3;
    const NS_WINDOW_STYLE_MASK_NONACTIVATING_PANEL: NSUInteger = 1 << 7;

    let ns_window = window.ns_window().map_err(to_string)? as id;
    unsafe {
        let style_mask: NSUInteger = msg_send![ns_window, styleMask];
        let _: () = msg_send![
            ns_window,
            setStyleMask: style_mask | NS_WINDOW_STYLE_MASK_NONACTIVATING_PANEL
        ];
        ns_window.setLevel_(NS_FLOATING_WINDOW_LEVEL);
        ns_window.setHidesOnDeactivate_(YES);
        ns_window.setCollectionBehavior_(
            NSWindowCollectionBehavior::NSWindowCollectionBehaviorCanJoinAllSpaces
                | NSWindowCollectionBehavior::NSWindowCollectionBehaviorFullScreenAuxiliary
                | NSWindowCollectionBehavior::NSWindowCollectionBehaviorTransient
                | NSWindowCollectionBehavior::NSWindowCollectionBehaviorIgnoresCycle,
        );
    }

    log::info!("Configured QuickBar macOS non-activating panel style fallback");
    Ok(())
}

#[cfg(not(target_os = "macos"))]
fn setup_quickbar_macos(_window: &WebviewWindow) -> Result<(), String> {
    Ok(())
}

#[cfg(windows)]
fn setup_quickbar_windows(window: &WebviewWindow) -> Result<(), String> {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::WindowsAndMessaging::{
        GetWindowLongPtrW, SetWindowLongPtrW, SetWindowPos, GWL_EXSTYLE, SWP_FRAMECHANGED,
        SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER, WS_EX_TOOLWINDOW,
    };

    let hwnd = window.hwnd().map_err(to_string)?;
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

    if let Ok(quickbar_hwnd) = quickbar.hwnd() {
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
    use cocoa::base::{id, nil};
    use objc::{class, msg_send, sel, sel_impl};

    unsafe {
        let workspace: id = msg_send![class!(NSWorkspace), sharedWorkspace];
        let front: id = msg_send![workspace, frontmostApplication];
        if front == nil {
            log::warn!("No frontmost macOS application to record");
            return;
        }

        let pid: i32 = msg_send![front, processIdentifier];
        if pid == std::process::id() as i32 {
            log::debug!("ClipMan is already frontmost; keeping previous paste target");
            return;
        }

        *crate::safe_lock(store) = Some(ForegroundWindow {
            raw: pid as isize,
        });
        log::debug!("Recorded frontmost macOS app pid {pid} before QuickBar show");
    }
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
    use cocoa::appkit::NSWindow;
    use cocoa::base::{id, nil};

    let ns_window = window.ns_window().map_err(to_string)? as id;
    unsafe {
        ns_window.orderFrontRegardless();
        ns_window.makeKeyAndOrderFront_(nil);
    }

    window.set_focus().map_err(to_string)
}

#[cfg(not(target_os = "macos"))]
fn focus_quickbar(window: &WebviewWindow) -> Result<(), String> {
    window.set_focus().map_err(to_string)
}

#[cfg(target_os = "macos")]
fn focus_settings_window(window: &WebviewWindow) -> Result<(), String> {
    use cocoa::appkit::{NSApp, NSApplication};
    use cocoa::base::YES;

    unsafe {
        NSApp().activateIgnoringOtherApps_(YES);
    }

    window.set_focus().map_err(to_string)
}

#[cfg(not(target_os = "macos"))]
fn focus_settings_window(window: &WebviewWindow) -> Result<(), String> {
    window.set_focus().map_err(to_string)
}

fn to_string(error: impl std::fmt::Display) -> String {
    error.to_string()
}
