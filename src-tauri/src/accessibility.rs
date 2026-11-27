#[cfg(target_os = "macos")]
mod imp {
    use std::sync::atomic::{AtomicBool, Ordering};

    use tauri::{AppHandle, Manager};
    use tauri_plugin_dialog::{DialogExt, MessageDialogButtons, MessageDialogKind};

    use crate::AppState;

    #[link(name = "ApplicationServices", kind = "framework")]
    extern "C" {
        fn AXIsProcessTrusted() -> bool;
        fn AXIsProcessTrustedWithOptions(options: *const std::ffi::c_void) -> bool;
        static kAXTrustedCheckOptionPrompt: *const std::ffi::c_void;
    }

    pub fn is_trusted() -> bool {
        unsafe { AXIsProcessTrusted() }
    }

    pub fn open_settings(app: &AppHandle) -> Result<(), String> {
        use tauri_plugin_opener::OpenerExt;

        app.opener()
            .open_url(
                "x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility",
                None::<&str>,
            )
            .map_err(|e| format!("Failed to open Accessibility settings: {e}"))
    }

    pub fn request_access(app: &AppHandle) -> Result<(), String> {
        use objc2::rc::Retained;
        use objc2_foundation::{NSDictionary, NSNumber, NSString};

        if is_trusted() {
            return Ok(());
        }
        let key = unsafe { &*kAXTrustedCheckOptionPrompt.cast::<NSString>() };
        let prompt = NSNumber::new_bool(true);
        let options = NSDictionary::from_slices(&[key], &[&*prompt]);
        unsafe { AXIsProcessTrustedWithOptions(Retained::as_ptr(&options).cast()) };
        open_settings(app)
    }

    static WATCHING_FOR_ONBOARDING: AtomicBool = AtomicBool::new(false);
    const ONBOARDING_WATCH_LIMIT: std::time::Duration = std::time::Duration::from_secs(180);
    const ONBOARDING_WATCH_INTERVAL: std::time::Duration = std::time::Duration::from_millis(500);

    pub fn show_onboarding_when_trusted(app: &AppHandle) {
        if is_trusted() || WATCHING_FOR_ONBOARDING.swap(true, Ordering::SeqCst) {
            return;
        }
        let app = app.clone();
        std::thread::spawn(move || {
            let deadline = std::time::Instant::now() + ONBOARDING_WATCH_LIMIT;
            while std::time::Instant::now() < deadline && !is_trusted() {
                std::thread::sleep(ONBOARDING_WATCH_INTERVAL);
            }
            WATCHING_FOR_ONBOARDING.store(false, Ordering::SeqCst);
            if !is_trusted() || app.state::<AppState>().settings.get().onboarding_completed {
                return;
            }
            let app_for_main = app.clone();
            let dispatch = app.run_on_main_thread(move || {
                let state = app_for_main.state::<AppState>();
                if let Err(e) = crate::window::show_quickbar(
                    &app_for_main,
                    &state.quickbar_foreground_window,
                    &state.quickbar_panel,
                ) {
                    log::error!("Failed to reopen onboarding after Accessibility was granted: {e}");
                }
            });
            if let Err(e) = dispatch {
                log::error!("Failed to schedule onboarding after Accessibility was granted: {e}");
            }
        });
    }

    static DIALOG_OPEN: AtomicBool = AtomicBool::new(false);

    pub fn guide_reauthorization(app: &AppHandle) {
        if DIALOG_OPEN.swap(true, Ordering::SeqCst) {
            return;
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
                "ClipLeaf 需要“辅助功能”权限才能自动粘贴。内容已复制到剪贴板，可以手动粘贴。\
                 \n\n在系统设置中勾选 ClipLeaf 即可恢复自动粘贴。如果更新后已勾选仍无效，先取消勾选再重新勾选。",
                "打开系统设置",
                "稍后",
            )
        } else {
            (
                "Accessibility permission needed",
                "ClipLeaf needs Accessibility permission to auto-paste. The content has been \
                 copied to the clipboard, so you can paste manually.\n\nTurn on ClipLeaf in \
                 System Settings to restore auto-paste. If it is already on after an update, \
                 turn it off and on again.",
                "Open System Settings",
                "Later",
            )
        };

        let app_for_main = app.clone();
        let app_for_settings = app.clone();
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
                        if let Err(e) = request_access(&app_for_settings) {
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
pub use imp::{guide_reauthorization, is_trusted, request_access, show_onboarding_when_trusted};

#[cfg(not(target_os = "macos"))]
pub fn is_trusted() -> bool {
    true
}

#[cfg(not(target_os = "macos"))]
pub fn request_access(_app: &tauri::AppHandle) -> Result<(), String> {
    Ok(())
}

#[cfg(not(target_os = "macos"))]
pub fn show_onboarding_when_trusted(_app: &tauri::AppHandle) {}
