use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use tauri::AppHandle;
use tauri_plugin_store::StoreExt;

const DEFAULT_LOCALE: &str = "zh-CN";

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
            tray_text_length: 70,
            max_pinned_in_tray: 5,
            max_recent_in_tray: 20,
            custom_data_path: None,
            enable_autostart: false,
            locale: DEFAULT_LOCALE.to_string(),
        }
    }
}

impl Settings {
    pub fn validate_and_normalize(mut self) -> Result<Self, String> {
        self.global_shortcut = self.global_shortcut.trim().to_string();
        if self.global_shortcut.is_empty() {
            return Err("Global shortcut cannot be empty".to_string());
        }

        self.pinned_shortcut = self
            .pinned_shortcut
            .take()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty());

        if self.pinned_shortcut.as_deref() == Some(self.global_shortcut.as_str()) {
            return Err("Pinned shortcut cannot match the main global shortcut".to_string());
        }

        self.max_history_items = self.max_history_items.clamp(1, 10_000);
        self.tray_text_length = self.tray_text_length.clamp(10, 200);
        self.max_pinned_in_tray = self.max_pinned_in_tray.clamp(0, 50);
        self.max_recent_in_tray = self.max_recent_in_tray.clamp(0, 100);

        self.locale = normalize_locale(&self.locale);

        Ok(self)
    }

    pub fn normalize_for_load(mut self) -> Self {
        self.global_shortcut = self.global_shortcut.trim().to_string();
        if self.global_shortcut.is_empty() {
            self.global_shortcut = Settings::default().global_shortcut;
        }

        self.pinned_shortcut = self
            .pinned_shortcut
            .take()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty());

        if self.pinned_shortcut.as_deref() == Some(self.global_shortcut.as_str()) {
            log::warn!("Pinned shortcut matches main shortcut on load; clearing pinned shortcut");
            self.pinned_shortcut = None;
        }

        self.max_history_items = self.max_history_items.clamp(1, 10_000);
        self.tray_text_length = self.tray_text_length.clamp(10, 200);
        self.max_pinned_in_tray = self.max_pinned_in_tray.clamp(0, 50);
        self.max_recent_in_tray = self.max_recent_in_tray.clamp(0, 100);

        self.locale = normalize_locale(&self.locale);

        self
    }
}

fn normalize_locale(locale: &str) -> String {
    match locale.trim() {
        "zh-CN" => "zh-CN".to_string(),
        "en" => "en".to_string(),
        _ => DEFAULT_LOCALE.to_string(),
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
        let mut candidate = Settings::default();

        if let Some(v) = store
            .get("global_shortcut")
            .and_then(|v| v.as_str().map(String::from))
        {
            candidate.global_shortcut = v;
        }

        if let Some(v) = store.get("auto_paste").and_then(|v| v.as_bool()) {
            candidate.auto_paste = v;
        }

        if let Some(v) = store.get("ignore_concealed").and_then(|v| v.as_bool()) {
            candidate.ignore_concealed = v;
        }

        if let Some(v) = store.get("pinned_shortcut") {
            candidate.pinned_shortcut = v.as_str().map(String::from);
        }

        if let Some(v) = store.get("max_history_items").and_then(|v| v.as_u64()) {
            candidate.max_history_items = v as usize;
        }

        if let Some(v) = store.get("tray_text_length").and_then(|v| v.as_u64()) {
            candidate.tray_text_length = v as usize;
        }

        if let Some(v) = store.get("max_pinned_in_tray").and_then(|v| v.as_u64()) {
            candidate.max_pinned_in_tray = v as usize;
        }

        if let Some(v) = store.get("max_recent_in_tray").and_then(|v| v.as_u64()) {
            candidate.max_recent_in_tray = v as usize;
        }

        if let Some(v) = store
            .get("custom_data_path")
            .and_then(|v| v.as_str().map(String::from))
        {
            candidate.custom_data_path = Some(v);
        }

        if let Some(v) = store.get("enable_autostart").and_then(|v| v.as_bool()) {
            candidate.enable_autostart = v;
        }

        if let Some(v) = store
            .get("locale")
            .and_then(|v| v.as_str().map(String::from))
        {
            candidate.locale = v;
        }

        let normalized = candidate.normalize_for_load();
        *self.settings.lock().unwrap() = normalized;

        log::info!("Settings loaded: {:?}", self.get());
        Ok(())
    }

    pub fn save(&self, app: &AppHandle) -> Result<(), String> {
        let settings = self.settings.lock().unwrap();
        Self::save_to_store(app, &settings)?;

        log::info!("Settings saved: {:?}", *settings);
        Ok(())
    }

    pub fn save_candidate(&self, app: &AppHandle, settings: &Settings) -> Result<(), String> {
        Self::save_to_store(app, settings)?;
        log::info!("Settings candidate saved: {:?}", settings);
        Ok(())
    }

    fn save_to_store(app: &AppHandle, settings: &Settings) -> Result<(), String> {
        let store = app
            .store("settings.json")
            .map_err(|e| format!("Failed to access store: {}", e))?;

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

    #[test]
    fn default_tray_text_length_shows_enough_to_tell_similar_clips_apart() {
        // Raised from 50 so look-alike entries (shared prefix/suffix) stay
        // distinguishable in the single-line tray menu. Existing users keep
        // their stored value; only fresh installs and reset use the default.
        assert_eq!(70, Settings::default().tray_text_length);
    }

    #[test]
    fn settings_normalization_clamps_tray_text_length_to_frontend_minimum() {
        let settings = Settings {
            tray_text_length: 0,
            ..Settings::default()
        };

        let normalized = settings.validate_and_normalize().unwrap();

        assert_eq!(10, normalized.tray_text_length);
    }

    #[test]
    fn settings_normalization_rejects_matching_shortcuts() {
        let default_settings = Settings::default();
        let settings = Settings {
            pinned_shortcut: Some(default_settings.global_shortcut.clone()),
            ..default_settings
        };

        let result = settings.validate_and_normalize();

        assert!(result.unwrap_err().contains("cannot match"));
    }

    #[test]
    fn settings_load_normalization_clears_conflicting_pinned_shortcut() {
        let default_settings = Settings::default();
        let settings = Settings {
            pinned_shortcut: Some(default_settings.global_shortcut.clone()),
            custom_data_path: Some("/tmp/clipman-data".to_string()),
            ..default_settings
        };

        let normalized = settings.normalize_for_load();

        assert_eq!(None, normalized.pinned_shortcut);
        assert_eq!(
            Some("/tmp/clipman-data".to_string()),
            normalized.custom_data_path
        );
    }

    #[test]
    fn settings_normalization_trims_supported_locale() {
        let settings = Settings {
            locale: " en ".to_string(),
            ..Settings::default()
        };

        let normalized = settings.validate_and_normalize().unwrap();

        assert_eq!("en", normalized.locale);
    }

    #[test]
    fn settings_load_normalization_resets_unsupported_locale() {
        let settings = Settings {
            locale: "fr-FR".to_string(),
            ..Settings::default()
        };

        let normalized = settings.normalize_for_load();

        assert_eq!(DEFAULT_LOCALE, normalized.locale);
    }
}
