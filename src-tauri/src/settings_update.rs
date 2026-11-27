use tauri::{AppHandle, Emitter};

use crate::settings::Settings;
use crate::shortcuts::{apply_shortcut_changes, AppShortcutRegistry};
use crate::tray::update_tray_menu;
use crate::AppState;

#[derive(serde::Serialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SettingsUpdateResult {
    settings: Settings,
    warning: Option<String>,
}

pub trait SettingsEffects {
    fn apply_autostart(&self, enable: bool) -> Result<(), String>;
    fn apply_shortcuts(&self, from: &Settings, to: &Settings) -> Result<(), String>;
    fn save(&self, settings: &Settings) -> Result<(), String>;
}

struct AppSettingsEffects<'a> {
    app: &'a AppHandle,
    state: &'a AppState,
}

impl SettingsEffects for AppSettingsEffects<'_> {
    fn apply_autostart(&self, enable: bool) -> Result<(), String> {
        crate::autostart::apply_autostart_setting(self.app, enable)
    }

    fn apply_shortcuts(&self, from: &Settings, to: &Settings) -> Result<(), String> {
        let registry = AppShortcutRegistry {
            app: self.app,
            foreground_store: self.state.quickbar_foreground_window.clone(),
            panel_store: self.state.quickbar_panel.clone(),
        };
        apply_shortcut_changes(
            &registry,
            &from.global_shortcut,
            from.pinned_shortcut.as_deref(),
            &to.global_shortcut,
            to.pinned_shortcut.as_deref(),
        )
    }

    fn save(&self, settings: &Settings) -> Result<(), String> {
        self.state.settings.save(self.app, settings)
    }
}

pub fn apply_settings_update(
    old: &Settings,
    mut settings: Settings,
    effects: &impl SettingsEffects,
) -> Result<Settings, String> {
    settings.custom_data_path = old.custom_data_path.clone();
    settings.capture_paused = old.capture_paused;
    settings.onboarding_completed = old.onboarding_completed;

    let autostart_changed = old.enable_autostart != settings.enable_autostart;
    let rollback_autostart = |reason: &str| {
        if autostart_changed {
            if let Err(rollback_error) = effects.apply_autostart(old.enable_autostart) {
                log::warn!("Failed to roll back autostart after {reason}: {rollback_error}");
            }
        }
    };

    if autostart_changed {
        effects.apply_autostart(settings.enable_autostart)?;
        log::info!(
            "Autostart {} successfully",
            if settings.enable_autostart {
                "enabled"
            } else {
                "disabled"
            }
        );
    }

    if let Err(e) = effects.apply_shortcuts(old, &settings) {
        rollback_autostart("shortcut update failed");
        return Err(e);
    }

    if let Err(e) = effects.save(&settings) {
        if let Err(rollback_error) = effects.apply_shortcuts(&settings, old) {
            log::warn!(
                "Failed to roll back shortcuts after settings save failed: {}",
                rollback_error
            );
        }
        rollback_autostart("settings save failed");
        return Err(e);
    }

    Ok(settings)
}

pub fn update_settings_blocking(
    app: &AppHandle,
    state: &AppState,
    settings: Option<Settings>,
) -> Result<SettingsUpdateResult, String> {
    let settings = settings.unwrap_or_default().validate_and_normalize()?;
    let _settings_write_guard = state.settings_write_lock.lock().unwrap();
    log::info!("Updating settings: {:?}", settings);

    let old = state.settings.get();
    let settings = apply_settings_update(&old, settings, &AppSettingsEffects { app, state })?;

    state.settings.set(settings.clone());
    let _ = app.emit("settings-changed", ());

    if old.tray_text_length != settings.tray_text_length
        || old.max_pinned_in_tray != settings.max_pinned_in_tray
        || old.max_recent_in_tray != settings.max_recent_in_tray
        || old.locale != settings.locale
    {
        log::info!("Tray settings changed, rebuilding menu...");
        update_tray_menu(app);
    }

    let warning = if old.max_history_items != settings.max_history_items {
        match state
            .storage
            .lock()
            .unwrap()
            .enforce_history_limit(settings.max_history_items)
        {
            Ok(removed) => {
                if removed > 0 {
                    let _ = app.emit("history-cleared", ());
                }
                None
            }
            Err(error) => Some(format!(
                "Settings saved, but history cleanup failed: {error}"
            )),
        }
    } else {
        None
    };
    Ok(SettingsUpdateResult { settings, warning })
}

pub fn complete_onboarding(app: &AppHandle, state: &AppState) -> Result<(), String> {
    let _settings_write_guard = state.settings_write_lock.lock().unwrap();
    let mut settings = state.settings.get();
    if settings.onboarding_completed {
        return Ok(());
    }
    settings.onboarding_completed = true;
    state.settings.save(app, &settings)?;
    state.settings.set(settings);
    let _ = app.emit("settings-changed", ());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    #[derive(Default)]
    struct FakeEffects {
        calls: RefCell<Vec<String>>,
        saved: RefCell<Option<Settings>>,
        failing_autostart: Option<bool>,
        failing_shortcut_target: Option<&'static str>,
        fail_save: bool,
    }

    impl FakeEffects {
        fn calls(&self) -> Vec<String> {
            self.calls.borrow().clone()
        }
    }

    impl SettingsEffects for FakeEffects {
        fn apply_autostart(&self, enable: bool) -> Result<(), String> {
            self.calls.borrow_mut().push(format!("autostart {enable}"));
            if self.failing_autostart == Some(enable) {
                return Err("autostart failed".to_string());
            }
            Ok(())
        }

        fn apply_shortcuts(&self, from: &Settings, to: &Settings) -> Result<(), String> {
            self.calls.borrow_mut().push(format!(
                "shortcuts {} -> {}",
                from.global_shortcut, to.global_shortcut
            ));
            if self.failing_shortcut_target == Some(to.global_shortcut.as_str()) {
                return Err(format!("{} is taken", to.global_shortcut));
            }
            Ok(())
        }

        fn save(&self, settings: &Settings) -> Result<(), String> {
            self.calls.borrow_mut().push("save".to_string());
            if self.fail_save {
                return Err("disk full".to_string());
            }
            *self.saved.borrow_mut() = Some(settings.clone());
            Ok(())
        }
    }

    fn settings(global_shortcut: &str, enable_autostart: bool) -> Settings {
        Settings {
            global_shortcut: global_shortcut.to_string(),
            enable_autostart,
            ..Settings::default()
        }
    }

    #[test]
    fn update_keeps_data_path_capture_pause_and_onboarding_state() {
        let old = Settings {
            custom_data_path: Some("/data/clipleaf".to_string()),
            capture_paused: true,
            onboarding_completed: false,
            ..settings("A", false)
        };
        let requested = Settings {
            custom_data_path: None,
            capture_paused: false,
            onboarding_completed: true,
            max_history_items: 250,
            ..settings("A", false)
        };
        let effects = FakeEffects::default();

        let updated = apply_settings_update(&old, requested, &effects).unwrap();

        for result in [updated, effects.saved.borrow().clone().unwrap()] {
            assert_eq!(Some("/data/clipleaf".to_string()), result.custom_data_path);
            assert!(result.capture_paused);
            assert!(!result.onboarding_completed);
            assert_eq!(250, result.max_history_items);
        }
        assert_eq!(vec!["shortcuts A -> A", "save"], effects.calls());
    }

    #[test]
    fn failed_save_rolls_back_shortcuts_before_autostart() {
        let effects = FakeEffects {
            fail_save: true,
            ..FakeEffects::default()
        };

        let error = apply_settings_update(&settings("A", false), settings("B", true), &effects)
            .unwrap_err();

        assert_eq!("disk full", error);
        assert_eq!(
            vec![
                "autostart true",
                "shortcuts A -> B",
                "save",
                "shortcuts B -> A",
                "autostart false",
            ],
            effects.calls()
        );
    }

    #[test]
    fn failed_shortcut_update_rolls_back_autostart_without_saving() {
        let effects = FakeEffects {
            failing_shortcut_target: Some("B"),
            ..FakeEffects::default()
        };

        let error = apply_settings_update(&settings("A", false), settings("B", true), &effects)
            .unwrap_err();

        assert_eq!("B is taken", error);
        assert_eq!(
            vec!["autostart true", "shortcuts A -> B", "autostart false"],
            effects.calls()
        );
    }

    #[test]
    fn failed_autostart_change_stops_before_shortcuts() {
        let effects = FakeEffects {
            failing_autostart: Some(true),
            ..FakeEffects::default()
        };

        let error = apply_settings_update(&settings("A", false), settings("B", true), &effects)
            .unwrap_err();

        assert_eq!("autostart failed", error);
        assert_eq!(vec!["autostart true"], effects.calls());
    }

    #[test]
    fn unchanged_autostart_is_not_touched_during_rollback() {
        let effects = FakeEffects {
            fail_save: true,
            ..FakeEffects::default()
        };

        apply_settings_update(&settings("A", true), settings("B", true), &effects).unwrap_err();

        assert_eq!(
            vec!["shortcuts A -> B", "save", "shortcuts B -> A"],
            effects.calls()
        );
    }

    #[test]
    fn failed_rollback_still_returns_the_save_error() {
        let effects = FakeEffects {
            fail_save: true,
            failing_shortcut_target: Some("A"),
            failing_autostart: Some(false),
            ..FakeEffects::default()
        };

        let error = apply_settings_update(&settings("A", false), settings("B", true), &effects)
            .unwrap_err();

        assert_eq!("disk full", error);
        assert_eq!(
            vec![
                "autostart true",
                "shortcuts A -> B",
                "save",
                "shortcuts B -> A",
                "autostart false",
            ],
            effects.calls()
        );
    }
}
