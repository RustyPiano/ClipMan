use tauri::AppHandle;
use tauri_plugin_autostart::ManagerExt;

pub fn migrate_legacy_autostart(app: &AppHandle, reregister: bool) -> Result<(), String> {
    let current_exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let legacy = auto_launch::AutoLaunchBuilder::new()
        .set_app_name("ClipMan")
        .set_app_path(&current_exe.display().to_string())
        .set_use_launch_agent(true)
        .build()
        .map_err(|e| e.to_string())?;
    let legacy_enabled = legacy.is_enabled().map_err(|e| e.to_string())?;
    if legacy_enabled || reregister {
        let autolaunch = app.autolaunch();
        autolaunch.enable().map_err(|e| e.to_string())?;
        if !autolaunch.is_enabled().map_err(|e| e.to_string())? {
            return Err("The ClipLeaf autostart entry was not registered".to_string());
        }
        log::info!(
            "Registered the ClipLeaf autostart entry at {}",
            current_exe.display()
        );
    }
    if legacy_enabled {
        legacy.disable().map_err(|e| e.to_string())?;
    }
    Ok(())
}

pub fn apply_autostart_setting(app: &AppHandle, enable_autostart: bool) -> Result<(), String> {
    let result = if enable_autostart {
        app.autolaunch().enable()
    } else {
        app.autolaunch().disable()
    };

    result.map_err(|e| format!("Failed to update autostart: {}", e))
}
