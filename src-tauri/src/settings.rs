use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use tauri::AppHandle;
use tauri_plugin_store::StoreExt;

const DEFAULT_LOCALE: &str = "zh-CN";
const SETTINGS_KEY: &str = "settings";

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
    #[link(name = "kernel32")]
    extern "system" {
        fn GetUserDefaultLocaleName(buffer: *mut u16, length: i32) -> i32;
    }
    let mut buffer = [0u16; 85];
    // Windows LOCALE_NAME_MAX_LENGTH includes the terminating NUL.
    let length = unsafe { GetUserDefaultLocaleName(buffer.as_mut_ptr(), buffer.len() as i32) };
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

/// v2.1.x 及更早版本在 store 顶层逐项保存的设置键。
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
    /// Paste-format mode; one of [`PASTE_FORMATS`]:
    /// - `original`: paste exactly as copied (rich text included).
    /// - `takePlain`: takes/copies via ClipMan write plain text only; a direct
    ///   system paste keeps the copied formatting.
    /// - `globalPlain`: like `takePlain`, plus every captured rich-text copy is
    ///   immediately flattened on the system clipboard so a direct paste
    ///   anywhere is plain text too.
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
    /// Text/Files clips whose content exceeds this many bytes are skipped
    /// entirely at capture time.
    pub max_text_bytes: usize,
    /// Images whose longest side exceeds this many pixels are downsampled
    /// before being stored. `0` disables downscaling.
    pub max_image_dimension: u32,
    /// When true, Text clips matching a high-confidence secret pattern
    /// (PEM private key, cloud/API token, JWT, ...) are skipped at capture
    /// time instead of being recorded.
    pub skip_secrets: bool,
    /// App names or bundle identifiers whose copies are never captured,
    /// matched case-insensitively against the frontmost app at capture time.
    /// Normalized on every load/save: trimmed, emptied entries dropped,
    /// deduplicated, and capped at 100 entries.
    pub ignored_apps: Vec<String>,
    /// When true, the clipboard monitor observes clipboard changes but
    /// captures nothing at all, regardless of source app or content.
    /// Toggled from the tray's "Pause Capture" menu item.
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
        self.global_shortcut = self.global_shortcut.trim().to_string();
        if self.global_shortcut.is_empty() {
            return Err("Global shortcut cannot be empty".to_string());
        }

        self.normalize_common();

        if self.pinned_shortcut.as_deref() == Some(self.global_shortcut.as_str()) {
            return Err("Pinned shortcut cannot match the main global shortcut".to_string());
        }

        Ok(self)
    }

    pub fn normalize_for_load(mut self) -> Self {
        self.global_shortcut = self.global_shortcut.trim().to_string();
        if self.global_shortcut.is_empty() {
            self.global_shortcut = Settings::default().global_shortcut;
        }

        self.normalize_common();

        if self.pinned_shortcut.as_deref() == Some(self.global_shortcut.as_str()) {
            log::warn!("Pinned shortcut matches main shortcut on load; clearing pinned shortcut");
            self.pinned_shortcut = None;
        }

        self
    }

    fn normalize_common(&mut self) {
        self.pinned_shortcut = self
            .pinned_shortcut
            .take()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty());

        self.max_history_items = self.max_history_items.clamp(1, 10_000);
        self.tray_text_length = self.tray_text_length.clamp(10, 200);
        self.max_pinned_in_tray = self.max_pinned_in_tray.clamp(0, 50);
        self.max_recent_in_tray = self.max_recent_in_tray.clamp(0, 100);
        self.max_text_bytes = self.max_text_bytes.clamp(4096, 50_000_000);
        self.max_image_dimension = clamp_max_image_dimension(self.max_image_dimension);
        self.ignored_apps = normalize_ignored_apps(std::mem::take(&mut self.ignored_apps));

        self.locale = normalize_locale(&self.locale);
        self.paste_format = normalize_paste_format(&self.paste_format);
    }

    /// ClipMan-mediated takes/copies should write plain text only
    /// (`takePlain` and `globalPlain`).
    pub fn takes_plain_text(&self) -> bool {
        matches!(self.paste_format.as_str(), "takePlain" | "globalPlain")
    }

    /// Captured rich-text copies should be flattened on the system clipboard
    /// so a direct paste (without QuickBar) is plain text (`globalPlain` only).
    pub fn strips_rich_text_at_capture(&self) -> bool {
        self.paste_format == "globalPlain"
    }
}

/// IPC values of the paste-format mode; keep in sync with the `Settings`
/// union type in `src/lib/types.ts`.
pub const PASTE_FORMATS: [&str; 3] = ["original", "takePlain", "globalPlain"];

fn normalize_paste_format(value: &str) -> String {
    let trimmed = value.trim();
    if PASTE_FORMATS.contains(&trimmed) {
        trimmed.to_string()
    } else {
        PASTE_FORMATS[0].to_string()
    }
}

fn normalize_locale(locale: &str) -> String {
    match locale.trim() {
        "zh-CN" => "zh-CN".to_string(),
        "en" => "en".to_string(),
        _ => DEFAULT_LOCALE.to_string(),
    }
}

/// `0` disables downscaling (the settings page allows it); any other value is
/// clamped to a sane pixel range.
fn clamp_max_image_dimension(value: u32) -> u32 {
    if value == 0 {
        0
    } else {
        value.clamp(512, 16384)
    }
}

/// Cap on the number of ignored-app entries a user can configure.
const MAX_IGNORED_APPS: usize = 100;

/// Trims each entry, drops blanks, deduplicates case-insensitively (keeping
/// the first occurrence's original casing so it still displays as typed),
/// and caps the list at `MAX_IGNORED_APPS`. Case-insensitive dedup matches
/// the matching semantics used at capture time in `clipboard.rs`, so
/// "Safari" and "safari" never coexist as two entries.
fn normalize_ignored_apps(apps: Vec<String>) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    apps.into_iter()
        .map(|app| app.trim().to_string())
        .filter(|app| !app.is_empty())
        .filter(|app| seen.insert(app.to_lowercase()))
        .take(MAX_IGNORED_APPS)
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

pub struct SettingsManager {
    settings: Mutex<Settings>,
}

impl Default for SettingsManager {
    fn default() -> Self {
        Self::new()
    }
}

impl SettingsManager {
    pub fn new() -> Self {
        Self {
            settings: Mutex::new(Settings::default()),
        }
    }

    pub fn load(&self, app: &AppHandle) -> Result<(), String> {
        let result = (|| {
            let store = app
                .store("settings.json")
                .map_err(|e| format!("Failed to access store: {e}"))?;
            match store.get(SETTINGS_KEY) {
                Some(value) => serde_json::from_value(value)
                    .map_err(|e| format!("Failed to parse settings store: {e}")),
                None => Ok(settings_from_legacy_store(|key| store.get(key))),
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
                // Retain known preferences and stop capture; never save defaults over a bad file.
                self.settings.lock().unwrap().capture_paused = true;
                Err(error)
            }
        }
    }

    /// 把 `settings` 写入 store；内存中的设置由调用方在保存成功后用 `set` 更新。
    pub fn save(&self, app: &AppHandle, settings: &Settings) -> Result<(), String> {
        let store = app
            .store("settings.json")
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
    fn failed_load_keeps_privacy_preferences_and_pauses_capture() {
        let manager = SettingsManager::new();
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
    fn initial_locale_tracks_the_system_language() {
        assert_eq!(locale_for_language("zh-Hant-HK"), "zh-CN");
        assert_eq!(locale_for_language("en-SG"), "en");
        assert_eq!(locale_for_language("ja-JP"), "en");
    }

    #[test]
    fn paste_format_modes_and_unknown_value_coercion() {
        let mut settings = Settings::default();
        assert_eq!("original", settings.paste_format);
        assert!(!settings.takes_plain_text());
        assert!(!settings.strips_rich_text_at_capture());

        settings.paste_format = " takePlain ".to_string();
        let normalized = settings.clone().normalize_for_load();
        assert_eq!("takePlain", normalized.paste_format);
        assert!(normalized.takes_plain_text());
        assert!(!normalized.strips_rich_text_at_capture());

        settings.paste_format = "globalPlain".to_string();
        let normalized = settings.normalize_for_load();
        assert!(normalized.takes_plain_text());
        assert!(normalized.strips_rich_text_at_capture());

        // Unknown values (hand-edited file) fall back to original.
        let coerced = Settings {
            paste_format: "richerezza".to_string(),
            ..Settings::default()
        }
        .normalize_for_load();
        assert_eq!("original", coerced.paste_format);

        // The update path funnels through the same normalization.
        let updated = Settings {
            paste_format: " takePlain ".to_string(),
            ..Settings::default()
        }
        .validate_and_normalize()
        .unwrap();
        assert_eq!("takePlain", updated.paste_format);

        // A persisted payload missing the key deserializes to "" (serde field
        // default) and normalizes to original.
        let missing_key = serde_json::from_value::<Settings>(serde_json::json!({
            "globalShortcut": "CommandOrControl+Shift+V"
        }))
        .unwrap();
        assert_eq!("original", missing_key.normalize_for_load().paste_format);
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

    #[test]
    fn settings_normalization_clamps_max_text_bytes_to_supported_range() {
        let too_small = Settings {
            max_text_bytes: 10,
            ..Settings::default()
        };
        assert_eq!(
            4096,
            too_small.validate_and_normalize().unwrap().max_text_bytes
        );

        let too_large = Settings {
            max_text_bytes: 100_000_000,
            ..Settings::default()
        };
        assert_eq!(
            50_000_000,
            too_large.validate_and_normalize().unwrap().max_text_bytes
        );
    }

    #[test]
    fn settings_normalization_clamps_max_image_dimension_but_allows_zero_to_disable() {
        let disabled = Settings {
            max_image_dimension: 0,
            ..Settings::default()
        };
        assert_eq!(
            0,
            disabled
                .validate_and_normalize()
                .unwrap()
                .max_image_dimension
        );

        let too_small = Settings {
            max_image_dimension: 10,
            ..Settings::default()
        };
        assert_eq!(
            512,
            too_small
                .validate_and_normalize()
                .unwrap()
                .max_image_dimension
        );

        let too_large = Settings {
            max_image_dimension: 100_000,
            ..Settings::default()
        };
        assert_eq!(
            16384,
            too_large
                .validate_and_normalize()
                .unwrap()
                .max_image_dimension
        );
    }

    #[test]
    fn settings_normalization_trims_dedupes_and_drops_empty_ignored_apps() {
        let settings = Settings {
            ignored_apps: vec![
                " 1Password ".to_string(),
                "1password".to_string(), // case-insensitive duplicate of the entry above
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
    fn settings_normalization_caps_ignored_apps_at_one_hundred_entries() {
        let apps: Vec<String> = (0..150).map(|i| format!("App {i}")).collect();
        let settings = Settings {
            ignored_apps: apps,
            ..Settings::default()
        };

        let normalized = settings.validate_and_normalize().unwrap().ignored_apps;

        assert_eq!(100, normalized.len());
        assert_eq!("App 0", normalized[0]);
        assert_eq!("App 99", normalized[99]);
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
            "customDataPath": "/tmp/clipman-data",
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
            "custom_data_path": "/tmp/clipman-data",
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
                Some("/tmp/clipman-data".to_string()),
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
