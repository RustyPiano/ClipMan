use tauri::AppHandle;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

use crate::settings::Settings;
use crate::window::{ForegroundWindowStore, QuickBarPanel, QuickBarPanelStore};

pub trait ShortcutRegistry {
    fn register(&self, shortcut: &str, panel: QuickBarPanel) -> Result<(), String>;
    fn unregister(&self, shortcut: &str) -> Result<(), String>;
}

pub struct AppShortcutRegistry<'a> {
    pub app: &'a AppHandle,
    pub foreground_store: ForegroundWindowStore,
    pub panel_store: QuickBarPanelStore,
}

impl ShortcutRegistry for AppShortcutRegistry<'_> {
    fn register(&self, shortcut: &str, panel: QuickBarPanel) -> Result<(), String> {
        register_quickbar_shortcut(
            self.app,
            shortcut,
            self.foreground_store.clone(),
            self.panel_store.clone(),
            panel,
        )
    }

    fn unregister(&self, shortcut: &str) -> Result<(), String> {
        self.app
            .global_shortcut()
            .unregister(shortcut)
            .map_err(|e| e.to_string())
    }
}

pub fn register_quickbar_shortcut(
    app: &AppHandle,
    shortcut: &str,
    foreground_store: ForegroundWindowStore,
    panel_store: QuickBarPanelStore,
    panel: QuickBarPanel,
) -> Result<(), String> {
    let app_clone = app.clone();
    let shortcut_display = shortcut.to_string();

    app.global_shortcut()
        .on_shortcut(shortcut, move |_app, _shortcut, event| {
            if !matches!(event.state, ShortcutState::Pressed) {
                return;
            }
            log::info!("Global shortcut triggered: {}", shortcut_display);

            let app_for_action = app_clone.clone();
            let foreground_store = foreground_store.clone();
            let panel_store = panel_store.clone();
            if let Err(e) = crate::window::run_action_on_main_thread(
                &app_clone,
                "show_quickbar_from_shortcut",
                move || {
                    crate::window::toggle_quickbar_with_panel(
                        &app_for_action,
                        &foreground_store,
                        &panel_store,
                        panel,
                    )
                },
            ) {
                log::error!("Failed to show QuickBar: {}", e);
            }
        })
        .map_err(|e| format!("Failed to register shortcut '{}': {}", shortcut, e))
}

pub fn apply_shortcut_changes(
    registry: &impl ShortcutRegistry,
    old_shortcut: &str,
    old_pinned_shortcut: Option<&str>,
    new_shortcut: &str,
    new_pinned_shortcut: Option<&str>,
) -> Result<(), String> {
    let changes: Vec<_> = [
        (
            Some(old_shortcut),
            Some(new_shortcut),
            QuickBarPanel::Recent,
        ),
        (
            old_pinned_shortcut,
            new_pinned_shortcut,
            QuickBarPanel::Pinned,
        ),
    ]
    .into_iter()
    .filter(|(old, new, _)| old != new)
    .collect();

    for &(old, _, panel) in &changes {
        if let Some(old) = old {
            if let Err(e) = registry.unregister(old) {
                log::warn!("Failed to unregister old {panel:?} shortcut '{old}': {e}");
            }
        }
    }

    let result = changes.iter().try_for_each(|&(_, new, panel)| match new {
        Some(new) => registry.register(new, panel),
        None => Ok(()),
    });

    if result.is_err() {
        for &(_, new, _) in &changes {
            if let Some(new) = new {
                let _ = registry.unregister(new);
            }
        }
        for &(old, _, panel) in &changes {
            if let Some(old) = old {
                if let Err(e) = registry.register(old, panel) {
                    log::warn!("Failed to restore old {panel:?} shortcut '{old}': {e}");
                }
            }
        }
    }

    result
}

pub fn disable_quickbar_shortcuts(app: &AppHandle, settings: Settings) -> Result<(), String> {
    let current_shortcut = settings.global_shortcut;

    app.global_shortcut()
        .unregister(current_shortcut.as_str())
        .map_err(|e| format!("Failed to disable shortcut: {}", e))?;

    log::info!(
        "Global shortcut '{}' temporarily disabled",
        current_shortcut
    );

    if let Some(pinned_shortcut) = settings.pinned_shortcut {
        if let Err(e) = app.global_shortcut().unregister(pinned_shortcut.as_str()) {
            log::warn!(
                "Failed to disable pinned shortcut '{}': {}",
                pinned_shortcut,
                e
            );
        } else {
            log::info!("Pinned shortcut '{}' temporarily disabled", pinned_shortcut);
        }
    }

    Ok(())
}

pub fn enable_quickbar_shortcuts(
    app: &AppHandle,
    settings: Settings,
    foreground_store: ForegroundWindowStore,
    panel_store: QuickBarPanelStore,
) -> Result<(), String> {
    let current_shortcut = settings.global_shortcut;

    if !app
        .global_shortcut()
        .is_registered(current_shortcut.as_str())
    {
        register_quickbar_shortcut(
            app,
            current_shortcut.as_str(),
            foreground_store.clone(),
            panel_store.clone(),
            QuickBarPanel::Recent,
        )
        .map_err(|e| format!("Failed to re-enable shortcut: {}", e))?;
    }

    log::info!("Global shortcut '{}' re-enabled", current_shortcut);

    if let Some(pinned_shortcut) = settings.pinned_shortcut {
        if !app
            .global_shortcut()
            .is_registered(pinned_shortcut.as_str())
        {
            register_quickbar_shortcut(
                app,
                pinned_shortcut.as_str(),
                foreground_store,
                panel_store,
                QuickBarPanel::Pinned,
            )
            .map_err(|e| format!("Failed to re-enable pinned shortcut: {}", e))?;
            log::info!("Pinned shortcut '{}' re-enabled", pinned_shortcut);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    #[derive(Default)]
    struct FakeRegistry {
        calls: RefCell<Vec<String>>,
        failing_registrations: Vec<(&'static str, QuickBarPanel)>,
        failing_unregistrations: Vec<&'static str>,
    }

    impl FakeRegistry {
        fn calls(&self) -> Vec<String> {
            self.calls.borrow().clone()
        }
    }

    impl ShortcutRegistry for FakeRegistry {
        fn register(&self, shortcut: &str, panel: QuickBarPanel) -> Result<(), String> {
            self.calls
                .borrow_mut()
                .push(format!("register {shortcut} {panel:?}"));
            if self.failing_registrations.contains(&(shortcut, panel)) {
                return Err(format!("{shortcut} is taken"));
            }
            Ok(())
        }

        fn unregister(&self, shortcut: &str) -> Result<(), String> {
            self.calls
                .borrow_mut()
                .push(format!("unregister {shortcut}"));
            if self.failing_unregistrations.contains(&shortcut) {
                return Err(format!("{shortcut} was not registered"));
            }
            Ok(())
        }
    }

    #[test]
    fn swapping_shortcuts_unregisters_all_old_keys_before_registering_new_ones() {
        let registry = FakeRegistry::default();

        apply_shortcut_changes(&registry, "A", Some("B"), "B", Some("A")).unwrap();

        assert_eq!(
            vec![
                "unregister A",
                "unregister B",
                "register B Recent",
                "register A Pinned",
            ],
            registry.calls()
        );
    }

    #[test]
    fn failed_swap_unregisters_new_keys_then_restores_old_ones() {
        let registry = FakeRegistry {
            failing_registrations: vec![("A", QuickBarPanel::Pinned)],
            ..FakeRegistry::default()
        };

        let error = apply_shortcut_changes(&registry, "A", Some("B"), "B", Some("A")).unwrap_err();

        assert_eq!("A is taken", error);
        assert_eq!(
            vec![
                "unregister A",
                "unregister B",
                "register B Recent",
                "register A Pinned",
                "unregister B",
                "unregister A",
                "register A Recent",
                "register B Pinned",
            ],
            registry.calls()
        );
    }

    #[test]
    fn failed_restore_still_returns_the_original_error() {
        let registry = FakeRegistry {
            failing_registrations: vec![("C", QuickBarPanel::Recent), ("A", QuickBarPanel::Recent)],
            ..FakeRegistry::default()
        };

        let error = apply_shortcut_changes(&registry, "A", None, "C", None).unwrap_err();

        assert_eq!("C is taken", error);
        assert_eq!(
            vec![
                "unregister A",
                "register C Recent",
                "unregister C",
                "register A Recent",
            ],
            registry.calls()
        );
    }

    #[test]
    fn unchanged_shortcuts_are_left_registered() {
        let registry = FakeRegistry::default();

        apply_shortcut_changes(&registry, "A", Some("B"), "A", Some("B")).unwrap();

        assert!(registry.calls().is_empty());
    }

    #[test]
    fn removing_the_pinned_shortcut_only_unregisters_it() {
        let registry = FakeRegistry::default();

        apply_shortcut_changes(&registry, "A", Some("B"), "A", None).unwrap();

        assert_eq!(vec!["unregister B"], registry.calls());
    }

    #[test]
    fn failed_unregister_of_an_old_key_does_not_block_registration() {
        let registry = FakeRegistry {
            failing_unregistrations: vec!["A"],
            ..FakeRegistry::default()
        };

        apply_shortcut_changes(&registry, "A", None, "C", Some("D")).unwrap();

        assert_eq!(
            vec!["unregister A", "register C Recent", "register D Pinned"],
            registry.calls()
        );
    }
}
