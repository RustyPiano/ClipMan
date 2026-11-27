use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Mutex;
use tauri::AppHandle;
use tauri_plugin_store::StoreExt;

const DEFAULT_LOCALE: &str = "zh-CN";
const SETTINGS_KEY: &str = "settings";
const SETTINGS_FILE: &str = "settings.json";

fn locale_for_language(language: &str) -> String {
    if language.to_ascii_lowercase().starts_with("zh") {
        "zh-CN"
    } else {
        "en"
    }
    .into()
}

#[cfg(target_os = "macos")]
fn system_locale() -> String {
    let language = objc2_foundation::NSLocale::preferredLanguages()
        .firstObject()
        .map(|value| value.to_string())
        .unwrap_or_default();
    locale_for_language(&language)
}

#[cfg(windows)]
fn system_locale() -> String {
    const LOCALE_NAME_MAX_LENGTH: usize = 85;
    let mut buffer = [0u16; LOCALE_NAME_MAX_LENGTH];
    let length = unsafe { windows::Win32::Globalization::GetUserDefaultLocaleName(&mut buffer) };
    locale_for_language(&String::from_utf16_lossy(
        &buffer[..length.saturating_sub(1) as usize],
    ))
}

#[cfg(not(any(target_os = "macos", windows)))]
fn system_locale() -> String {
    let language = ["LC_ALL", "LC_MESSAGES", "LANG"]
        .iter()
        .find_map(|key| std::env::var(key).ok().filter(|value| !value.is_empty()))
        .unwrap_or_default();
    locale_for_language(&language)
}

const LEGACY_SETTINGS_KEYS: [&str; 11] = [
    "global_shortcut",
    "auto_paste",
    "ignore_concealed",
    "pinned_shortcut",
    "max_history_items",
    "tray_text_length",
    "max_pinned_in_tray",
    "max_recent_in_tray",
    "custom_data_path",
    "enable_autostart",
    "locale",
];

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub global_shortcut: String,
    pub auto_paste: bool,
    pub paste_format: String,
    pub ignore_concealed: bool,
    pub pinned_shortcut: Option<String>,
    pub max_history_items: usize,
    pub tray_text_length: usize,
    pub max_pinned_in_tray: usize,
    pub max_recent_in_tray: usize,
    pub custom_data_path: Option<String>,
    pub enable_autostart: bool,
    pub locale: String,
    pub max_text_bytes: usize,
    pub max_image_dimension: u32,
    pub skip_secrets: bool,
    pub ignored_apps: Vec<String>,
    pub capture_paused: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            global_shortcut: "CommandOrControl+Shift+V".to_string(),
            auto_paste: true,
            paste_format: PASTE_FORMATS[0].to_string(),
            ignore_concealed: true,
            pinned_shortcut: None,
            max_history_items: 100,
            tray_text_length: 70,
            max_pinned_in_tray: 5,
            max_recent_in_tray: 20,
            custom_data_path: None,
            enable_autostart: false,
            locale: system_locale(),
            max_text_bytes: 2_000_000,
            max_image_dimension: 4096,
            skip_secrets: true,
            ignored_apps: Vec::new(),
            capture_paused: false,
        }
    }
}

impl Settings {
    pub fn validate_and_normalize(mut self) -> Result<Self, String> {
        self.normalize_common();
        if self.global_shortcut.is_empty() {
            return Err("Global shortcut cannot be empty".to_string());
        }

        if self.pinned_shortcut.as_deref() == Some(self.global_shortcut.as_str()) {
            return Err("Pinned shortcut cannot match the main global shortcut".to_string());
        }

        for (name, value, min, max) in [
            ("maxHistoryItems", self.max_history_items, 1, 10_000),
            ("trayTextLength", self.tray_text_length, 10, 200),
            ("maxPinnedInTray", self.max_pinned_in_tray, 0, 50),
            ("maxRecentInTray", self.max_recent_in_tray, 0, 100),
            ("maxTextBytes", self.max_text_bytes, 4096, 50_000_000),
        ] {
            if !(min..=max).contains(&value) {
                return Err(format!("{name} must be between {min} and {max}"));
            }
        }
        if self.max_image_dimension != 0 && !(512..=16384).contains(&self.max_image_dimension) {
            return Err("maxImageDimension must be 0 or between 512 and 16384".into());
        }
        if !matches!(self.locale.as_str(), "zh-CN" | "en") {
            return Err("locale must be zh-CN or en".into());
        }
        if !PASTE_FORMATS.contains(&self.paste_format.as_str()) {
            return Err("pasteFormat must be original, takePlain or globalPlain".into());
        }
        if self.ignored_apps.len() > MAX_IGNORED_APPS {
            return Err(format!(
                "ignoredApps cannot exceed {MAX_IGNORED_APPS} entries"
            ));
        }

        Ok(self)
    }

    pub fn normalize_for_load(mut self) -> Self {
        self.normalize_common();
        if self.global_shortcut.is_empty() {
            self.global_shortcut = Settings::default().global_shortcut;
        }

        self.max_history_items = self.max_history_items.clamp(1, 10_000);
        self.tray_text_length = self.tray_text_length.clamp(10, 200);
        self.max_pinned_in_tray = self.max_pinned_in_tray.clamp(0, 50);
        self.max_recent_in_tray = self.max_recent_in_tray.clamp(0, 100);
        self.max_text_bytes = self.max_text_bytes.clamp(4096, 50_000_000);
        if self.max_image_dimension != 0 {
            self.max_image_dimension = self.max_image_dimension.clamp(512, 16384);
        }
        self.ignored_apps.truncate(MAX_IGNORED_APPS);
        if !matches!(self.locale.as_str(), "zh-CN" | "en") {
            self.locale = DEFAULT_LOCALE.to_string();
        }
        if !PASTE_FORMATS.contains(&self.paste_format.as_str()) {
            self.paste_format = PASTE_FORMATS[0].to_string();
        }

        if self.pinned_shortcut.as_deref() == Some(self.global_shortcut.as_str()) {
            log::warn!("Pinned shortcut matches main shortcut on load; clearing pinned shortcut");
            self.pinned_shortcut = None;
        }

        self
    }

    fn normalize_common(&mut self) {
        self.global_shortcut = self.global_shortcut.trim().to_string();
        self.pinned_shortcut = self
            .pinned_shortcut
            .take()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty());

        self.ignored_apps = normalize_ignored_apps(std::mem::take(&mut self.ignored_apps));
        self.locale = self.locale.trim().to_string();
        self.paste_format = self.paste_format.trim().to_string();
    }

    pub fn takes_plain_text(&self) -> bool {
        matches!(self.paste_format.as_str(), "takePlain" | "globalPlain")
    }

    pub fn strips_rich_text_at_capture(&self) -> bool {
        self.paste_format == "globalPlain"
    }
}

pub const PASTE_FORMATS: [&str; 3] = ["original", "takePlain", "globalPlain"];

const MAX_IGNORED_APPS: usize = 100;

fn normalize_ignored_apps(apps: Vec<String>) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    apps.into_iter()
        .map(|app| app.trim().to_string())
        .filter(|app| !app.is_empty())
        .filter(|app| seen.insert(app.to_lowercase()))
        .collect()
}

fn settings_from_legacy_store(mut get: impl FnMut(&str) -> Option<serde_json::Value>) -> Settings {
    let mut candidate = Settings::default();

    if let Some(v) = get("global_shortcut").and_then(|v| v.as_str().map(String::from)) {
        candidate.global_shortcut = v;
    }

    if let Some(v) = get("auto_paste").and_then(|v| v.as_bool()) {
        candidate.auto_paste = v;
    }

    if let Some(v) = get("ignore_concealed").and_then(|v| v.as_bool()) {
        candidate.ignore_concealed = v;
    }

    if let Some(v) = get("pinned_shortcut") {
        candidate.pinned_shortcut = v.as_str().map(String::from);
    }

    if let Some(v) = get("max_history_items").and_then(|v| v.as_u64()) {
        candidate.max_history_items = v as usize;
    }

    if let Some(v) = get("tray_text_length").and_then(|v| v.as_u64()) {
        candidate.tray_text_length = v as usize;
    }

    if let Some(v) = get("max_pinned_in_tray").and_then(|v| v.as_u64()) {
        candidate.max_pinned_in_tray = v as usize;
    }

    if let Some(v) = get("max_recent_in_tray").and_then(|v| v.as_u64()) {
        candidate.max_recent_in_tray = v as usize;
    }

    if let Some(v) = get("custom_data_path").and_then(|v| v.as_str().map(String::from)) {
        candidate.custom_data_path = Some(v);
    }

    if let Some(v) = get("enable_autostart").and_then(|v| v.as_bool()) {
        candidate.enable_autostart = v;
    }

    if let Some(v) = get("locale").and_then(|v| v.as_str().map(String::from)) {
        candidate.locale = v;
    }

    candidate
}

fn read_settings_store(path: &Path) -> Result<HashMap<String, serde_json::Value>, String> {
    match std::fs::read(path) {
        Ok(bytes) => serde_json::from_slice(&bytes)
            .map_err(|e| format!("Failed to parse {}: {e}", path.display())),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(HashMap::new()),
        Err(e) => Err(format!("Failed to read {}: {e}", path.display())),
    }
}

#[derive(Default)]
pub struct SettingsManager {
    settings: Mutex<Settings>,
}

impl SettingsManager {
    pub fn load(&self, app: &AppHandle) -> Result<(), String> {
        let result = (|| {
            let path = tauri_plugin_store::resolve_store_path(app, SETTINGS_FILE)
                .map_err(|e| format!("Failed to resolve the settings file: {e}"))?;
            let store = read_settings_store(&path)?;
            match store.get(SETTINGS_KEY) {
                Some(value) => serde_json::from_value(value.clone())
                    .map_err(|e| format!("Failed to parse settings store: {e}")),
                None => Ok(settings_from_legacy_store(|key| store.get(key).cloned())),
            }
        })();
        self.apply_load_result(result)
    }

    fn apply_load_result(&self, result: Result<Settings, String>) -> Result<(), String> {
        match result {
            Ok(candidate) => {
                self.set(candidate.normalize_for_load());
                Ok(())
            }
            Err(error) => {
                self.settings.lock().unwrap().capture_paused = true;
                Err(error)
            }
        }
    }

    pub fn save(&self, app: &AppHandle, settings: &Settings) -> Result<(), String> {
        let store = app
            .store(SETTINGS_FILE)
            .map_err(|e| format!("Failed to access store: {}", e))?;

        let value = serde_json::to_value(settings)
            .map_err(|e| format!("Failed to serialize settings: {}", e))?;
        store.set(SETTINGS_KEY, value);
        for key in LEGACY_SETTINGS_KEYS {
            store.delete(key);
        }

        store
            .save()
            .map_err(|e| format!("Failed to save store: {}", e))?;

        log::info!("Settings saved: {:?}", settings);
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
    fn settings_file_is_empty_only_when_missing() {
        let root = std::env::temp_dir().join(format!("clipleaf_settings_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&root).unwrap();
        let path = root.join(SETTINGS_FILE);

        assert!(read_settings_store(&path).unwrap().is_empty());

        std::fs::write(&path, b"{\"settings\": {").unwrap();
        assert!(read_settings_store(&path)
            .unwrap_err()
            .contains("Failed to parse"));
        assert_eq!(std::fs::read(&path).unwrap(), b"{\"settings\": {");

        std::fs::remove_file(&path).unwrap();
        std::fs::create_dir(&path).unwrap();
        assert!(read_settings_store(&path)
            .unwrap_err()
            .contains("Failed to read"));
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn failed_load_keeps_privacy_preferences_and_pauses_capture() {
        let manager = SettingsManager::default();
        manager.set(Settings {
            ignored_apps: vec!["Private App".into()],
            ..Settings::default()
        });
        assert!(manager
            .apply_load_result(Err("unreadable settings".into()))
            .is_err());
        assert!(manager.get().capture_paused);
        assert_eq!(manager.get().ignored_apps, ["Private App"]);
        manager.apply_load_result(Ok(Settings::default())).unwrap();
        assert!(!manager.get().capture_paused);
    }

    #[test]
    fn paste_format_modes() {
        for (format, takes_plain, strips_rich_text) in [
            ("original", false, false),
            ("takePlain", true, false),
            ("globalPlain", true, true),
        ] {
            let settings = Settings {
                paste_format: format.into(),
                ..Settings::default()
            };
            assert_eq!(takes_plain, settings.takes_plain_text(), "{format}");
            assert_eq!(
                strips_rich_text,
                settings.strips_rich_text_at_capture(),
                "{format}"
            );
        }

        let missing_key = serde_json::from_value::<Settings>(serde_json::json!({
            "globalShortcut": "CommandOrControl+Shift+V"
        }))
        .unwrap();
        assert_eq!("original", missing_key.paste_format);
    }

    #[test]
    fn saving_rejects_out_of_range_numbers_and_accepts_boundaries() {
        for (name, min, max) in [
            ("maxHistoryItems", 1, 10_000),
            ("trayTextLength", 10, 200),
            ("maxPinnedInTray", 0, 50),
            ("maxRecentInTray", 0, 100),
            ("maxTextBytes", 4096, 50_000_000),
        ] {
            for value in [min, max, max + 1] {
                let mut settings = serde_json::to_value(Settings::default()).unwrap();
                settings[name] = serde_json::json!(value);
                let result = serde_json::from_value::<Settings>(settings)
                    .unwrap()
                    .validate_and_normalize();
                assert_eq!(result.is_ok(), value <= max, "{name}={value}");
            }
            if min > 0 {
                let mut settings = serde_json::to_value(Settings::default()).unwrap();
                settings[name] = serde_json::json!(min - 1);
                assert!(serde_json::from_value::<Settings>(settings)
                    .unwrap()
                    .validate_and_normalize()
                    .unwrap_err()
                    .contains(name));
            }
        }
    }

    #[test]
    fn matching_pinned_shortcut_is_rejected_on_save_and_cleared_on_load() {
        let default_settings = Settings::default();
        let settings = Settings {
            pinned_shortcut: Some(default_settings.global_shortcut.clone()),
            ..default_settings
        };

        assert!(settings
            .clone()
            .validate_and_normalize()
            .unwrap_err()
            .contains("cannot match"));
        assert_eq!(None, settings.normalize_for_load().pinned_shortcut);
    }

    #[test]
    fn locale_and_paste_format_are_trimmed_and_unknown_values_handled() {
        let padded = Settings {
            locale: " en ".into(),
            paste_format: " takePlain ".into(),
            ..Settings::default()
        };
        for normalized in [
            padded.clone().validate_and_normalize().unwrap(),
            padded.normalize_for_load(),
        ] {
            assert_eq!("en", normalized.locale);
            assert_eq!("takePlain", normalized.paste_format);
        }

        let loaded = Settings {
            locale: "fr-FR".into(),
            paste_format: "unknown".into(),
            ..Settings::default()
        }
        .normalize_for_load();
        assert_eq!(DEFAULT_LOCALE, loaded.locale);
        assert_eq!(PASTE_FORMATS[0], loaded.paste_format);

        for settings in [
            Settings {
                locale: "fr-FR".into(),
                ..Settings::default()
            },
            Settings {
                paste_format: "unknown".into(),
                ..Settings::default()
            },
        ] {
            assert!(settings.validate_and_normalize().is_err());
        }
    }

    #[test]
    fn saving_checks_image_dimension_and_allows_zero_to_disable() {
        for value in [0, 1, 511, 512, 16384, 16385] {
            let result = Settings {
                max_image_dimension: value,
                ..Settings::default()
            }
            .validate_and_normalize();
            assert_eq!(result.is_ok(), value == 0 || (512..=16384).contains(&value));
        }
    }

    #[test]
    fn settings_normalization_trims_dedupes_and_drops_empty_ignored_apps() {
        let settings = Settings {
            ignored_apps: vec![
                " 1Password ".to_string(),
                "1password".to_string(),
                "".to_string(),
                "   ".to_string(),
                "Bitwarden".to_string(),
            ],
            ..Settings::default()
        };

        let normalized = settings.validate_and_normalize().unwrap().ignored_apps;

        assert_eq!(
            vec!["1Password".to_string(), "Bitwarden".to_string()],
            normalized
        );
    }

    #[test]
    fn saving_rejects_excess_ignored_apps_and_load_keeps_the_legacy_limit() {
        let apps: Vec<String> = (0..150).map(|i| format!("App {i}")).collect();
        let settings = Settings {
            ignored_apps: apps,
            ..Settings::default()
        };

        assert!(settings
            .clone()
            .validate_and_normalize()
            .unwrap_err()
            .contains("ignoredApps"));
        let normalized = settings.normalize_for_load().ignored_apps;

        assert_eq!(100, normalized.len());
        assert_eq!("App 0", normalized[0]);
        assert_eq!("App 99", normalized[99]);
    }

    #[test]
    fn loading_legacy_numbers_keeps_supported_limits() {
        let settings = Settings {
            max_history_items: 0,
            tray_text_length: 0,
            max_pinned_in_tray: 100,
            max_recent_in_tray: 200,
            max_text_bytes: 0,
            max_image_dimension: 1,
            ..Settings::default()
        }
        .normalize_for_load();
        assert_eq!(settings.max_history_items, 1);
        assert_eq!(settings.tray_text_length, 10);
        assert_eq!(settings.max_pinned_in_tray, 50);
        assert_eq!(settings.max_recent_in_tray, 100);
        assert_eq!(settings.max_text_bytes, 4096);
        assert_eq!(settings.max_image_dimension, 512);
    }

    #[test]
    fn settings_store_format_loads_new_object_and_legacy_keys() {
        let new_json = serde_json::json!({
            "globalShortcut": " CommandOrControl+Alt+V ",
            "autoPaste": false,
            "pasteFormat": "globalPlain",
            "ignoreConcealed": false,
            "pinnedShortcut": " CommandOrControl+Shift+P ",
            "maxHistoryItems": 200,
            "trayTextLength": 80,
            "maxPinnedInTray": 7,
            "maxRecentInTray": 30,
            "customDataPath": "/tmp/clipleaf-data",
            "enableAutostart": true,
            "locale": " en ",
            "maxTextBytes": 123456,
            "maxImageDimension": 2048,
            "skipSecrets": false,
            "ignoredApps": [" Terminal ", "terminal", "Safari"],
            "capturePaused": true
        });
        let legacy_json = serde_json::json!({
            "global_shortcut": " CommandOrControl+Alt+V ",
            "auto_paste": false,
            "ignore_concealed": false,
            "pinned_shortcut": " CommandOrControl+Shift+P ",
            "max_history_items": 200,
            "tray_text_length": 80,
            "max_pinned_in_tray": 7,
            "max_recent_in_tray": 30,
            "custom_data_path": "/tmp/clipleaf-data",
            "enable_autostart": true,
            "locale": " en "
        });

        let new_loaded = serde_json::from_value::<Settings>(new_json)
            .unwrap()
            .normalize_for_load();
        let legacy_loaded =
            settings_from_legacy_store(|key| legacy_json.get(key).cloned()).normalize_for_load();

        for loaded in [&new_loaded, &legacy_loaded] {
            assert_eq!("CommandOrControl+Alt+V", loaded.global_shortcut);
            assert!(!loaded.auto_paste);
            assert!(!loaded.ignore_concealed);
            assert_eq!(
                Some("CommandOrControl+Shift+P".to_string()),
                loaded.pinned_shortcut
            );
            assert_eq!(200, loaded.max_history_items);
            assert_eq!(80, loaded.tray_text_length);
            assert_eq!(7, loaded.max_pinned_in_tray);
            assert_eq!(30, loaded.max_recent_in_tray);
            assert_eq!(
                Some("/tmp/clipleaf-data".to_string()),
                loaded.custom_data_path
            );
            assert!(loaded.enable_autostart);
            assert_eq!("en", loaded.locale);
        }

        assert!(new_loaded.strips_rich_text_at_capture());
        assert_eq!(123456, new_loaded.max_text_bytes);
        assert_eq!(2048, new_loaded.max_image_dimension);
        assert!(!new_loaded.skip_secrets);
        assert_eq!(
            vec!["Terminal".to_string(), "Safari".to_string()],
            new_loaded.ignored_apps
        );
        assert!(new_loaded.capture_paused);
    }
}
