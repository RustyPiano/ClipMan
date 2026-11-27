use std::fs;
use std::path::{Path, PathBuf};

pub const LEGACY_IDENTIFIER: &str = "com.clipman.manager";
pub const DATABASE_FILE: &str = "clipleaf.db";
const LEGACY_DATABASE_FILE: &str = "clipman.db";
const DATABASE_SUFFIXES: [&str; 4] = ["", "-shm", "-wal", "-journal"];
const SETTINGS_FILE: &str = "settings.json";

pub fn migrate_legacy_identity(identifier: &str) -> Result<bool, String> {
    let data_root =
        dirs::data_dir().ok_or_else(|| "Failed to resolve the data directory".to_string())?;
    let legacy = data_root.join(LEGACY_IDENTIFIER);
    let current = data_root.join(identifier);
    let migrated = move_dir_contents(&legacy, &current)?;
    if migrated {
        rewrite_custom_data_path(&current.join(SETTINGS_FILE), &legacy, &current)?;
    }
    for root in webview_data_roots() {
        if root == data_root {
            continue;
        }
        let legacy = root.join(LEGACY_IDENTIFIER);
        let current = root.join(identifier);
        if legacy.is_dir() && current.exists() {
            log::warn!(
                "Keeping {} because {} already exists",
                legacy.display(),
                current.display()
            );
            continue;
        }
        if let Err(e) = move_dir_contents(&legacy, &current) {
            log::error!("Failed to move webview data: {e}");
        }
    }
    Ok(migrated)
}

fn webview_data_roots() -> Vec<PathBuf> {
    let mut roots: Vec<PathBuf> = dirs::data_local_dir().into_iter().collect();
    #[cfg(target_os = "macos")]
    roots.extend(dirs::home_dir().map(|home| home.join("Library/WebKit")));
    roots
}

fn move_dir_contents(legacy: &Path, current: &Path) -> Result<bool, String> {
    if !legacy.is_dir() {
        return Ok(false);
    }
    if !current.exists() {
        fs::rename(legacy, current).map_err(|e| {
            format!(
                "Failed to move {} to {}: {e}",
                legacy.display(),
                current.display()
            )
        })?;
        log::info!("Moved {} to {}", legacy.display(), current.display());
        return Ok(true);
    }
    let entries =
        fs::read_dir(legacy).map_err(|e| format!("Failed to read {}: {e}", legacy.display()))?;
    for entry in entries {
        let entry = entry.map_err(|e| format!("Failed to read {}: {e}", legacy.display()))?;
        let target = current.join(entry.file_name());
        if target.exists() {
            return Err(format!(
                "Cannot move {} because {} already exists",
                entry.path().display(),
                target.display()
            ));
        }
        fs::rename(entry.path(), &target).map_err(|e| {
            format!(
                "Failed to move {} to {}: {e}",
                entry.path().display(),
                target.display()
            )
        })?;
    }
    fs::remove_dir(legacy).map_err(|e| format!("Failed to remove {}: {e}", legacy.display()))?;
    log::info!("Merged {} into {}", legacy.display(), current.display());
    Ok(true)
}

fn rewrite_custom_data_path(
    settings_file: &Path,
    legacy: &Path,
    current: &Path,
) -> Result<(), String> {
    if !settings_file.exists() {
        return Ok(());
    }
    let text = fs::read_to_string(settings_file)
        .map_err(|e| format!("Failed to read {}: {e}", settings_file.display()))?;
    let mut store: serde_json::Value = serde_json::from_str(&text)
        .map_err(|e| format!("Failed to parse {}: {e}", settings_file.display()))?;
    let mut changed = false;
    for pointer in ["/settings/customDataPath", "/custom_data_path"] {
        let Some(value) = store.pointer_mut(pointer) else {
            continue;
        };
        let Some(path) = value.as_str().map(PathBuf::from) else {
            continue;
        };
        if let Ok(relative) = path.strip_prefix(legacy) {
            *value = serde_json::Value::String(current.join(relative).display().to_string());
            changed = true;
        }
    }
    if changed {
        let text = serde_json::to_string_pretty(&store)
            .map_err(|e| format!("Failed to serialize {}: {e}", settings_file.display()))?;
        fs::write(settings_file, text)
            .map_err(|e| format!("Failed to write {}: {e}", settings_file.display()))?;
    }
    Ok(())
}

pub fn rename_legacy_database(data_dir: &Path) -> Result<(), String> {
    let legacy = data_dir.join(LEGACY_DATABASE_FILE);
    if !legacy.exists() {
        return Ok(());
    }
    if data_dir.join(DATABASE_FILE).exists() {
        return Err(format!(
            "Both {} and {} exist in {}; keep one of them and restart",
            LEGACY_DATABASE_FILE,
            DATABASE_FILE,
            data_dir.display()
        ));
    }
    for suffix in DATABASE_SUFFIXES {
        let from = data_dir.join(format!("{LEGACY_DATABASE_FILE}{suffix}"));
        if from.exists() {
            let to = data_dir.join(format!("{DATABASE_FILE}{suffix}"));
            fs::rename(&from, &to).map_err(|e| {
                format!(
                    "Failed to rename {} to {}: {e}",
                    from.display(),
                    to.display()
                )
            })?;
        }
    }
    log::info!("Renamed {} to {}", legacy.display(), DATABASE_FILE);
    Ok(())
}

#[cfg(target_os = "macos")]
pub fn relaunch_from_renamed_bundle() -> Result<bool, String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let Some(bundle) = exe.ancestors().nth(3) else {
        return Ok(false);
    };
    if bundle.file_name().and_then(|name| name.to_str()) != Some("ClipMan.app") {
        return Ok(false);
    }
    let renamed = bundle.with_file_name("ClipLeaf.app");
    if renamed.exists() {
        return Ok(false);
    }
    fs::rename(bundle, &renamed).map_err(|e| {
        format!(
            "Failed to rename {} to {}: {e}",
            bundle.display(),
            renamed.display()
        )
    })?;
    let relative_exe = exe.strip_prefix(bundle).map_err(|e| e.to_string())?;
    std::process::Command::new(renamed.join(relative_exe))
        .args(std::env::args_os().skip(1))
        .spawn()
        .map_err(|e| format!("Failed to relaunch {}: {e}", renamed.display()))?;
    log::info!("Renamed {} to {}", bundle.display(), renamed.display());
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_root(name: &str) -> PathBuf {
        let root =
            std::env::temp_dir().join(format!("clipleaf_identity_{name}_{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&root).unwrap();
        root
    }

    #[test]
    fn moves_legacy_dir_and_rewrites_custom_path_inside_it() {
        let root = temp_root("move");
        let legacy = root.join(LEGACY_IDENTIFIER);
        let current = root.join("app.clipleaf.desktop");
        fs::create_dir_all(legacy.join("data")).unwrap();
        fs::write(legacy.join(LEGACY_DATABASE_FILE), b"db").unwrap();
        let custom = legacy.join("data").display().to_string();
        fs::write(
            legacy.join(SETTINGS_FILE),
            serde_json::json!({ "settings": { "customDataPath": custom } }).to_string(),
        )
        .unwrap();

        assert!(move_dir_contents(&legacy, &current).unwrap());
        rewrite_custom_data_path(&current.join(SETTINGS_FILE), &legacy, &current).unwrap();

        assert!(!legacy.exists());
        assert_eq!(fs::read(current.join(LEGACY_DATABASE_FILE)).unwrap(), b"db");
        let store: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(current.join(SETTINGS_FILE)).unwrap())
                .unwrap();
        assert_eq!(
            store["settings"]["customDataPath"],
            current.join("data").display().to_string()
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn merges_into_existing_dir_and_refuses_to_overwrite() {
        let root = temp_root("merge");
        let legacy = root.join(LEGACY_IDENTIFIER);
        let current = root.join("app.clipleaf.desktop");
        fs::create_dir_all(&legacy).unwrap();
        fs::create_dir_all(&current).unwrap();
        fs::write(legacy.join("a"), b"a").unwrap();
        fs::write(current.join("b"), b"b").unwrap();
        assert!(move_dir_contents(&legacy, &current).unwrap());
        assert!(current.join("a").exists() && current.join("b").exists());

        fs::create_dir_all(&legacy).unwrap();
        fs::write(legacy.join("a"), b"other").unwrap();
        let error = move_dir_contents(&legacy, &current).unwrap_err();
        assert!(error.contains("already exists"));
        assert_eq!(fs::read(current.join("a")).unwrap(), b"a");
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn renames_legacy_database_files_and_rejects_two_databases() {
        let root = temp_root("db");
        for suffix in ["", "-wal", "-shm"] {
            fs::write(root.join(format!("{LEGACY_DATABASE_FILE}{suffix}")), suffix).unwrap();
        }
        rename_legacy_database(&root).unwrap();
        assert!(root.join(DATABASE_FILE).exists());
        assert_eq!(
            fs::read_to_string(root.join(format!("{DATABASE_FILE}-wal"))).unwrap(),
            "-wal"
        );
        assert!(!root.join(LEGACY_DATABASE_FILE).exists());

        fs::write(root.join(LEGACY_DATABASE_FILE), b"old").unwrap();
        assert!(rename_legacy_database(&root).unwrap_err().contains("Both"));
        fs::remove_dir_all(root).unwrap();
    }
}
