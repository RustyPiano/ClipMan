use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use tauri::AppHandle;
use tauri_plugin_store::StoreExt;

const DEFAULT_LOCALE: &str = "zh-CN";
const SETTINGS_KEY: &str = "settings";
const LEGACY_SETTINGS_KEYS: [&str; 16] = [
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
    "max_text_bytes",
    "max_image_dimension",
    "skip_secrets",
    "ignored_apps",
    "capture_paused",
];

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
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
    /// Text/Files clips whose content exceeds this many bytes are skipped
    /// entirely at capture time (§5).
    pub max_text_bytes: usize,
    /// Images whose longest side exceeds this many pixels are downsampled
    /// before being stored (§5). `0` disables downscaling.
    pub max_image_dimension: u32,
    /// When true, Text clips matching a high-confidence secret pattern
    /// (PEM private key, cloud/API token, JWT, ...) are skipped at capture
    /// time instead of being recorded (SPEC-4 §2).
    pub skip_secrets: bool,
    /// App names whose copies are never captured, matched case-insensitively
    /// against the frontmost app at capture time (SPEC-4 §3). Normalized on
    /// every load/save: trimmed, emptied entries dropped, deduplicated, and
    /// capped at 100 entries.
    pub ignored_apps: Vec<String>,
    /// When true, the clipboard monitor observes clipboard changes but
    /// captures nothing at all, regardless of source app or content
    /// (SPEC-4 §3). Toggled from the tray's "Pause Capture" menu item.
    pub capture_paused: bool,
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
    }
}

fn normalize_locale(locale: &str) -> String {
    match locale.trim() {
        "zh-CN" => "zh-CN".to_string(),
        "en" => "en".to_string(),
        _ => DEFAULT_LOCALE.to_string(),
    }
}

/// `0` is a deliberate escape hatch that disables downscaling entirely; any
/// other value is clamped to a sane pixel range (§5).
fn clamp_max_image_dimension(value: u32) -> u32 {
    if value == 0 {
        0
    } else {
        value.clamp(512, 16384)
    }
}

/// Cap on the number of ignored-app entries a user can configure (SPEC-4 §3).
const MAX_IGNORED_APPS: usize = 100;

/// Trims each entry, drops blanks, deduplicates case-insensitively (keeping
/// the first occurrence's original casing so it still displays as typed),
/// and caps the list at `MAX_IGNORED_APPS` (SPEC-4 §3). Case-insensitive
/// dedup matches the matching semantics used at capture time in
/// `clipboard.rs`, so "Safari" and "safari" never coexist as two entries.
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

    if let Some(v) = get("max_text_bytes").and_then(|v| v.as_u64()) {
        candidate.max_text_bytes = v as usize;
    }

    if let Some(v) = get("max_image_dimension").and_then(|v| v.as_u64()) {
        // Saturate to u32::MAX *before* narrowing so a stored value above
        // u32::MAX can't wrap around (e.g. 2^32 -> 0, which would silently
        // disable downscaling). normalize_for_load then clamps to range.
        candidate.max_image_dimension = v.min(u32::MAX as u64) as u32;
    }

    if let Some(v) = get("skip_secrets").and_then(|v| v.as_bool()) {
        candidate.skip_secrets = v;
    }

    if let Some(v) = get("ignored_apps").and_then(|v| v.as_array().cloned()) {
        candidate.ignored_apps = v
            .into_iter()
            .filter_map(|entry| entry.as_str().map(String::from))
            .collect();
    }

    if let Some(v) = get("capture_paused").and_then(|v| v.as_bool()) {
        candidate.capture_paused = v;
    }

    candidate
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
        let candidate = match store.get(SETTINGS_KEY) {
            Some(value) => serde_json::from_value(value)
                .map_err(|e| format!("Failed to parse settings store: {}", e))?,
            None => settings_from_legacy_store(|key| store.get(key)),
        };

        let normalized = candidate.normalize_for_load();
        *crate::safe_lock(&self.settings) = normalized;

        log::info!("Settings loaded: {:?}", self.get());
        Ok(())
    }

    pub fn save(&self, app: &AppHandle) -> Result<(), String> {
        let settings = crate::safe_lock(&self.settings);
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

        let value = serde_json::to_value(settings)
            .map_err(|e| format!("Failed to serialize settings: {}", e))?;
        store.set(SETTINGS_KEY, value);
        for key in LEGACY_SETTINGS_KEYS {
            store.delete(key);
        }

        store
            .save()
            .map_err(|e| format!("Failed to save store: {}", e))?;

        Ok(())
    }

    pub fn get(&self) -> Settings {
        crate::safe_lock(&self.settings).clone()
    }

    pub fn set(&self, settings: Settings) {
        *crate::safe_lock(&self.settings) = settings;
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

    #[test]
    fn default_settings_include_size_limit_fields() {
        let settings = Settings::default();

        assert_eq!(2_000_000, settings.max_text_bytes);
        assert_eq!(4096, settings.max_image_dimension);
    }

    #[test]
    fn default_settings_skip_secrets_is_enabled() {
        assert!(Settings::default().skip_secrets);
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
    fn default_settings_include_app_ignore_and_capture_pause_fields() {
        let settings = Settings::default();

        assert!(settings.ignored_apps.is_empty());
        assert!(!settings.capture_paused);
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
    fn settings_load_normalization_also_normalizes_ignored_apps() {
        // normalize_for_load (the load-time path) must apply the same
        // trim/dedupe/empty-drop rules as validate_and_normalize (the
        // save-time path), so a settings.json hand-edited or written by an
        // older version still round-trips to a clean list.
        let settings = Settings {
            ignored_apps: vec![" Slack ".to_string(), "".to_string(), "slack".to_string()],
            ..Settings::default()
        };

        let normalized = settings.normalize_for_load().ignored_apps;

        assert_eq!(vec!["Slack".to_string()], normalized);
    }

    #[test]
    fn settings_save_and_load_round_trip_preserves_app_ignore_and_capture_pause_fields() {
        // Exercises the same normalization path save() and load() both funnel
        // through, standing in for a full store round trip (§3 acceptance):
        // whatever a candidate carries in survives both normalization entry
        // points unchanged in shape (still deduped/trimmed), not silently
        // reset to defaults.
        let candidate = Settings {
            ignored_apps: vec!["Terminal".to_string(), "1Password".to_string()],
            capture_paused: true,
            ..Settings::default()
        };

        let saved_then_reloaded = candidate
            .clone()
            .validate_and_normalize()
            .unwrap()
            .normalize_for_load();

        assert_eq!(
            vec!["Terminal".to_string(), "1Password".to_string()],
            saved_then_reloaded.ignored_apps
        );
        assert!(saved_then_reloaded.capture_paused);
    }

    #[test]
    fn settings_store_format_loads_new_object_and_legacy_keys() {
        let new_json = serde_json::json!({
            "globalShortcut": " CommandOrControl+Alt+V ",
            "autoPaste": false,
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
            "locale": " en ",
            "max_text_bytes": 123456,
            "max_image_dimension": 2048,
            "skip_secrets": false,
            "ignored_apps": [" Terminal ", "terminal", "Safari"],
            "capture_paused": true
        });

        let new_loaded = serde_json::from_value::<Settings>(new_json)
            .unwrap()
            .normalize_for_load();
        let legacy_loaded =
            settings_from_legacy_store(|key| legacy_json.get(key).cloned()).normalize_for_load();

        for loaded in [new_loaded, legacy_loaded] {
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
            assert_eq!(123456, loaded.max_text_bytes);
            assert_eq!(2048, loaded.max_image_dimension);
            assert!(!loaded.skip_secrets);
            assert_eq!(
                vec!["Terminal".to_string(), "Safari".to_string()],
                loaded.ignored_apps
            );
            assert!(loaded.capture_paused);
        }
    }
}
