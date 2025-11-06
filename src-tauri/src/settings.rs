use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use tauri_plugin_store::StoreExt;
use tauri::{AppHandle, Manager};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub global_shortcut: String,
    pub max_history_items: usize,
    pub auto_cleanup: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            global_shortcut: "CommandOrControl+Shift+V".to_string(),
            max_history_items: 100,
            auto_cleanup: true,
        }
    }
}

pub struct SettingsManager {
    settings: Arc<Mutex<Settings>>,
}

impl SettingsManager {
    pub fn new() -> Self {
        Self {
            settings: Arc::new(Mutex::new(Settings::default())),
        }
    }

    pub fn load(&self, app: &AppHandle) -> Result<(), String> {
        let store = app.store("settings.json")
            .map_err(|e| format!("Failed to access store: {}", e))?;

        // 尝试加载设置
        if let Some(shortcut) = store.get("global_shortcut") {
            if let Some(s) = shortcut.as_str() {
                self.settings.lock().unwrap().global_shortcut = s.to_string();
            }
        }

        if let Some(max_items) = store.get("max_history_items") {
            if let Some(n) = max_items.as_u64() {
                self.settings.lock().unwrap().max_history_items = n as usize;
            }
        }

        if let Some(auto_cleanup) = store.get("auto_cleanup") {
            if let Some(b) = auto_cleanup.as_bool() {
                self.settings.lock().unwrap().auto_cleanup = b;
            }
        }

        log::info!("Settings loaded: {:?}", self.settings.lock().unwrap());
        Ok(())
    }

    pub fn save(&self, app: &AppHandle) -> Result<(), String> {
        let store = app.store("settings.json")
            .map_err(|e| format!("Failed to access store: {}", e))?;

        let settings = self.settings.lock().unwrap();

        store.set("global_shortcut", serde_json::json!(settings.global_shortcut));
        store.set("max_history_items", serde_json::json!(settings.max_history_items));
        store.set("auto_cleanup", serde_json::json!(settings.auto_cleanup));

        store.save().map_err(|e| format!("Failed to save store: {}", e))?;

        log::info!("Settings saved: {:?}", *settings);
        Ok(())
    }

    pub fn get(&self) -> Settings {
        self.settings.lock().unwrap().clone()
    }

    pub fn set_global_shortcut(&self, shortcut: String) {
        self.settings.lock().unwrap().global_shortcut = shortcut;
    }

    pub fn set_max_history_items(&self, max_items: usize) {
        self.settings.lock().unwrap().max_history_items = max_items;
    }

    pub fn set_auto_cleanup(&self, auto_cleanup: bool) {
        self.settings.lock().unwrap().auto_cleanup = auto_cleanup;
    }
}
