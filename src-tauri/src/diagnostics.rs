use tauri::plugin::TauriPlugin;
use tauri::{AppHandle, Manager, Wry};
use tauri_plugin_log::{RotationStrategy, Target, TargetKind, TimezoneStrategy};

const MAX_LOG_FILE_BYTES: u128 = 2_000_000;
const KEPT_LOG_FILES: usize = 3;

pub fn log_plugin() -> TauriPlugin<Wry> {
    tauri_plugin_log::Builder::new()
        .clear_targets()
        .targets([
            Target::new(TargetKind::Stdout),
            Target::new(TargetKind::LogDir {
                file_name: Some("clipleaf".into()),
            }),
        ])
        .level(if cfg!(debug_assertions) {
            log::LevelFilter::Debug
        } else {
            log::LevelFilter::Info
        })
        .max_file_size(MAX_LOG_FILE_BYTES)
        .rotation_strategy(RotationStrategy::KeepSome(KEPT_LOG_FILES))
        .timezone_strategy(TimezoneStrategy::UseLocal)
        .build()
}

pub fn install_panic_hook() {
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        log::error!(
            "ClipLeaf panicked: {info}\n{}",
            std::backtrace::Backtrace::force_capture()
        );
        log::logger().flush();
        default_hook(info);
    }));
}

#[cfg(target_os = "macos")]
pub fn os_version() -> String {
    objc2_foundation::NSProcessInfo::processInfo()
        .operatingSystemVersionString()
        .to_string()
}

#[cfg(target_os = "linux")]
pub fn os_version() -> String {
    std::fs::read_to_string("/etc/os-release")
        .ok()
        .and_then(|release| {
            release.lines().find_map(|line| {
                line.strip_prefix("PRETTY_NAME=")
                    .map(|value| value.trim_matches('"').to_string())
            })
        })
        .unwrap_or_default()
}

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
pub fn os_version() -> String {
    String::new()
}

pub fn report(
    app: &AppHandle,
    settings: &crate::settings::Settings,
    usage: &crate::storage::StorageUsage,
) -> Result<String, String> {
    let log_dir = app.path().app_log_dir().map_err(|e| e.to_string())?;
    Ok([
        format!("ClipLeaf {}", app.package_info().version),
        format!(
            "OS: {} {} {}",
            std::env::consts::OS,
            std::env::consts::ARCH,
            os_version()
        ),
        format!("Locale: {}", settings.locale),
        format!(
            "Accessibility trusted: {}",
            crate::accessibility::is_trusted()
        ),
        format!(
            "Items: {} (favorites {}, images {}), database {} bytes",
            usage.total, usage.pinned, usage.images, usage.database_bytes
        ),
        format!(
            "Settings: autoPaste={} pasteFormat={} maxHistoryItems={} capturePaused={} \
             ignoreConcealed={} skipSecrets={} ignoredApps={} autostart={} \
             autoCheckUpdates={} customDataPath={}",
            settings.auto_paste,
            settings.paste_format,
            settings.max_history_items,
            settings.capture_paused,
            settings.ignore_concealed,
            settings.skip_secrets,
            settings.ignored_apps.len(),
            settings.enable_autostart,
            settings.auto_check_updates,
            settings.custom_data_path.is_some()
        ),
        format!("Log folder: {}", log_dir.display()),
    ]
    .join("\n"))
}
