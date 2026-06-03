use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use tauri::AppHandle;
use tauri_plugin_store::StoreExt;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub global_shortcut: String,
    pub auto_paste: bool,
    pub ignore_concealed: bool,
    pub pinned_shortcut: Option<String>,
    pub max_history_items: usize,
    pub tray_text_length: usize,
    pub max_pinned_in_tray: usize,
    pub max_recent_in_tray: usize,
    pub custom_data_path: Option<String>,
    pub enable_autostart: bool,
    pub locale: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            global_shortcut: "CommandOrControl+Shift+V".to_string(),
            auto_paste: true,
            ignore_concealed: true,
            pinned_shortcut: None,
            max_history_items: 100,
            tray_text_length: 50,
            max_pinned_in_tray: 5,
            max_recent_in_tray: 20,
            custom_data_path: None,
            enable_autostart: false,
            locale: "zh-CN".to_string(),
        }
    }
}

pub struct SettingsManager {
    settings: Arc<Mutex<Settings>>,
}

impl Default for SettingsManager {
    fn default() -> Self {
        Self::new()
    }
}

impl SettingsManager {
    pub fn new() -> Self {
        Self {
            settings: Arc::new(Mutex::new(Settings::default())),
        }
    }

    pub fn load(&self, app: &AppHandle) -> Result<(), String> {
        let store = app
            .store("settings.json")
            .map_err(|e| format!("Failed to access store: {}", e))?;
        let mut settings = self.settings.lock().unwrap();

        if let Some(v) = store
            .get("global_shortcut")
            .and_then(|v| v.as_str().map(String::from))
        {
            settings.global_shortcut = v;
        }

        if let Some(v) = store.get("auto_paste").and_then(|v| v.as_bool()) {
            settings.auto_paste = v;
        }

        if let Some(v) = store.get("ignore_concealed").and_then(|v| v.as_bool()) {
            settings.ignore_concealed = v;
        }

        if let Some(v) = store.get("pinned_shortcut") {
            settings.pinned_shortcut = v.as_str().map(String::from);
        }

        if let Some(v) = store.get("max_history_items").and_then(|v| v.as_u64()) {
            settings.max_history_items = v as usize;
        }

        if let Some(v) = store.get("tray_text_length").and_then(|v| v.as_u64()) {
            settings.tray_text_length = v as usize;
        }

        if let Some(v) = store.get("max_pinned_in_tray").and_then(|v| v.as_u64()) {
            settings.max_pinned_in_tray = v as usize;
        }

        if let Some(v) = store.get("max_recent_in_tray").and_then(|v| v.as_u64()) {
            settings.max_recent_in_tray = v as usize;
        }

        if let Some(v) = store
            .get("custom_data_path")
            .and_then(|v| v.as_str().map(String::from))
        {
            settings.custom_data_path = Some(v);
        }

        if let Some(v) = store.get("enable_autostart").and_then(|v| v.as_bool()) {
            settings.enable_autostart = v;
        }

        if let Some(v) = store
            .get("locale")
            .and_then(|v| v.as_str().map(String::from))
        {
            settings.locale = v;
        }

        log::info!("Settings loaded: {:?}", *settings);
        Ok(())
    }

    pub fn save(&self, app: &AppHandle) -> Result<(), String> {
        let store = app
            .store("settings.json")
            .map_err(|e| format!("Failed to access store: {}", e))?;

        let settings = self.settings.lock().unwrap();

        store.set(
            "global_shortcut",
            serde_json::json!(settings.global_shortcut),
        );
        store.set("auto_paste", serde_json::json!(settings.auto_paste));
        store.set(
            "ignore_concealed",
            serde_json::json!(settings.ignore_concealed),
        );
        store.set(
            "pinned_shortcut",
            serde_json::json!(settings.pinned_shortcut),
        );
        store.set(
            "max_history_items",
            serde_json::json!(settings.max_history_items),
        );
        store.set(
            "tray_text_length",
            serde_json::json!(settings.tray_text_length),
        );
        store.set(
            "max_pinned_in_tray",
            serde_json::json!(settings.max_pinned_in_tray),
        );
        store.set(
            "max_recent_in_tray",
            serde_json::json!(settings.max_recent_in_tray),
        );
        store.set(
            "custom_data_path",
            serde_json::json!(settings.custom_data_path),
        );
        store.set(
            "enable_autostart",
            serde_json::json!(settings.enable_autostart),
        );
        store.set("locale", serde_json::json!(settings.locale));

        store
            .save()
            .map_err(|e| format!("Failed to save store: {}", e))?;

        log::info!("Settings saved: {:?}", *settings);
        Ok(())
    }

    pub fn get(&self) -> Settings {
        self.settings.lock().unwrap().clone()
    }

    pub fn set(&self, settings: Settings) {
        *self.settings.lock().unwrap() = settings;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_settings_include_phase0_fields() {
        let settings = Settings::default();

        assert!(settings.auto_paste);
        assert!(settings.ignore_concealed);
        assert_eq!(None, settings.pinned_shortcut);
    }
}
