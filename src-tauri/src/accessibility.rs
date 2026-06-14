//! macOS Accessibility (AXIsProcessTrusted) permission helpers.
//!
//! ClipMan simulates Cmd+V to auto-paste. On macOS that requires the
//! Accessibility permission. Crucially, when the permission is missing the
//! `CGEventPost` used by `enigo` fails **silently** — no error is returned — so
//! we cannot detect the problem from the paste result. Instead we proactively
//! query `AXIsProcessTrusted()` before pasting and guide the user to re-grant
//! the permission when it has been lost (commonly after an app update, because
//! the macOS privacy database keys the grant to the code signature).

#[cfg(target_os = "macos")]
mod imp {
    use std::sync::atomic::{AtomicBool, Ordering};

    use tauri::{AppHandle, Manager};
    use tauri_plugin_dialog::{DialogExt, MessageDialogButtons, MessageDialogKind};

    use crate::AppState;

    #[link(name = "ApplicationServices", kind = "framework")]
    extern "C" {
        fn AXIsProcessTrusted() -> bool;
    }

    /// Whether this process currently holds the macOS Accessibility permission.
    pub fn is_trusted() -> bool {
        // Safe: `AXIsProcessTrusted` takes no arguments and returns a Boolean.
        unsafe { AXIsProcessTrusted() }
    }

    /// Open System Settings → Privacy & Security → Accessibility.
    pub fn open_settings() -> Result<(), String> {
        std::process::Command::new("open")
            .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility")
            .spawn()
            .map(|_| ())
            .map_err(|e| format!("Failed to open Accessibility settings: {e}"))
    }

    // Prevents stacking multiple dialogs when several pastes fail in a row.
    static DIALOG_OPEN: AtomicBool = AtomicBool::new(false);

    /// Show a native, app-modal dialog explaining that the Accessibility
    /// permission is required and offering to open System Settings. Safe to call
    /// from any thread — the dialog is dispatched to the main thread.
    pub fn guide_reauthorization(app: &AppHandle) {
        if DIALOG_OPEN.swap(true, Ordering::SeqCst) {
            return; // a dialog is already on screen
        }

        let zh = app
            .state::<AppState>()
            .settings
            .get()
            .locale
            .starts_with("zh");

        let (title, body, open_btn, later_btn) = if zh {
            (
                "需要无障碍权限",
                "ClipMan 无法自动粘贴，因为“辅助功能/无障碍”权限已失效（常见于应用更新后）。\
                 \n\n内容已复制到剪贴板，你仍可手动粘贴。请在系统设置中重新授权 ClipMan 以恢复自动粘贴。",
                "打开系统设置",
                "稍后",
            )
        } else {
            (
                "Accessibility permission needed",
                "ClipMan couldn't auto-paste because its Accessibility permission is no longer \
                 valid (this often happens after an update).\n\nThe content has been copied to \
                 the clipboard, so you can paste manually. Re-grant ClipMan under System Settings \
                 to restore auto-paste.",
                "Open System Settings",
                "Later",
            )
        };

        let app_for_main = app.clone();
        let dispatch = app.run_on_main_thread(move || {
            app_for_main
                .dialog()
                .message(body)
                .title(title)
                .kind(MessageDialogKind::Warning)
                .buttons(MessageDialogButtons::OkCancelCustom(
                    open_btn.to_string(),
                    later_btn.to_string(),
                ))
                .show(move |open_pressed| {
                    DIALOG_OPEN.store(false, Ordering::SeqCst);
                    if open_pressed {
                        if let Err(e) = open_settings() {
                            log::error!("{e}");
                        }
                    }
                });
        });

        if let Err(e) = dispatch {
            DIALOG_OPEN.store(false, Ordering::SeqCst);
            log::error!("Failed to show accessibility dialog: {e}");
        }
    }
}

#[cfg(target_os = "macos")]
pub use imp::{guide_reauthorization, is_trusted, open_settings};

// Non-macOS platforms have no equivalent permission gate for input simulation;
// report trusted and make the helpers no-ops so callers stay platform-agnostic.
#[cfg(not(target_os = "macos"))]
pub fn is_trusted() -> bool {
    true
}

#[cfg(not(target_os = "macos"))]
pub fn open_settings() -> Result<(), String> {
    Ok(())
}

#[cfg(not(target_os = "macos"))]
pub fn guide_reauthorization(_app: &tauri::AppHandle) {}
