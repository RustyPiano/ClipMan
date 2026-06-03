use std::fs;
use std::path::{Path, PathBuf};

const DATA_FILES: [&str; 3] = ["clipman.db", "clipman.db-shm", "clipman.db-wal"];

/// Upgrade the clipboard database to the v1 plaintext schema marker.
pub fn upgrade_clip_database_to_v1(
    conn: &rusqlite::Connection,
    data_dir: &Path,
) -> Result<(), String> {
    let user_version: i64 = conn
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .map_err(|e| format!("Failed to read database user_version: {}", e))?;

    if user_version >= 1 {
        return Ok(());
    }

    let legacy_key = data_dir.join(".clipman.key");
    if legacy_key.exists() {
        log::warn!("Legacy encrypted database detected; resetting clipboard history for v1 plaintext storage");
        reset_clips_table(conn)?;
        fs::remove_file(&legacy_key)
            .map_err(|e| format!("Failed to remove legacy encryption key: {}", e))?;
    }

    conn.pragma_update(None, "user_version", 1)
        .map_err(|e| format!("Failed to set database user_version: {}", e))?;

    Ok(())
}

fn reset_clips_table(conn: &rusqlite::Connection) -> Result<(), String> {
    conn.execute("DELETE FROM clips", [])
        .map_err(|e| format!("Failed to reset clipboard history: {}", e))?;
    Ok(())
}

/// Migrate ClipMan data from one directory to another
pub fn migrate_data(from: &Path, to: &Path, delete_old: bool) -> Result<(), String> {
    log::info!("Starting data migration from {:?} to {:?}", from, to);

    // Validate paths
    if !from.exists() {
        return Err(format!("Source directory does not exist: {:?}", from));
    }

    if from == to {
        return Err("Source and destination are the same".to_string());
    }

    // Create destination directory
    fs::create_dir_all(to).map_err(|e| format!("Failed to create destination directory: {}", e))?;

    // Check write permission by creating a test file
    let test_file = to.join(".clipman_test");
    fs::write(&test_file, "test")
        .map_err(|e| format!("Destination directory is not writable: {}", e))?;
    fs::remove_file(&test_file).map_err(|e| format!("Failed to remove test file: {}", e))?;

    // Copy files
    for filename in DATA_FILES {
        let source_file = from.join(filename);
        if source_file.exists() {
            let dest_file = to.join(filename);

            log::info!("Copying {:?} to {:?}", source_file, dest_file);

            fs::copy(&source_file, &dest_file)
                .map_err(|e| format!("Failed to copy {}: {}", filename, e))?;

            // Verify file size matches
            let source_size = fs::metadata(&source_file)
                .map_err(|e| format!("Failed to get source metadata: {}", e))?
                .len();
            let dest_size = fs::metadata(&dest_file)
                .map_err(|e| format!("Failed to get dest metadata: {}", e))?
                .len();

            if source_size != dest_size {
                return Err(format!(
                    "File size mismatch for {}: source {} bytes, dest {} bytes",
                    filename, source_size, dest_size
                ));
            }
        }
    }

    // Verify at least the database or key file exists at destination
    // Verify database file was migrated successfully
    let db_file = to.join("clipman.db");
    if !db_file.exists() {
        return Err("Migration failed: database file not found at destination".to_string());
    }

    log::info!("Data migration successful");

    // Delete old files if requested
    if delete_old {
        log::info!("Deleting old data files from {:?}", from);
        remove_data_files(from);
    }

    Ok(())
}

/// Remove ClipMan data files (and the directory if it becomes empty).
pub fn remove_data_files(dir: &Path) {
    for filename in DATA_FILES {
        let path = dir.join(filename);
        if path.exists() {
            if let Err(e) = fs::remove_file(&path) {
                log::warn!("Failed to remove old file {}: {}", filename, e);
            }
        }
    }
    if let Err(e) = fs::remove_dir(dir) {
        log::warn!("Could not remove old directory (may not be empty): {}", e);
    }
}

/// Get the effective data directory based on settings
pub fn get_data_directory(app_data_dir: PathBuf, custom_path: Option<String>) -> PathBuf {
    if let Some(path) = custom_path {
        PathBuf::from(path)
    } else {
        app_data_dir
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_same_path_error() {
        // Create a temporary directory that exists
        let test_dir = std::env::temp_dir().join("clipman_test");
        std::fs::create_dir_all(&test_dir).unwrap();

        let result = migrate_data(&test_dir, &test_dir, false);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("same"));

        // Cleanup
        let _ = std::fs::remove_dir_all(&test_dir);
    }

    #[test]
    fn upgrade_database_to_v1_removes_legacy_key_and_resets_encrypted_rows() {
        let test_dir =
            std::env::temp_dir().join(format!("clipman_migration_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&test_dir).unwrap();
        let db_path = test_dir.join("clipman.db");
        let conn = rusqlite::Connection::open(&db_path).unwrap();
        conn.execute(
            "CREATE TABLE clips (
                id TEXT PRIMARY KEY,
                content BLOB NOT NULL,
                content_type TEXT NOT NULL,
                timestamp INTEGER NOT NULL,
                is_pinned INTEGER DEFAULT 0,
                pin_order INTEGER
            )",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO clips (id, content, content_type, timestamp, is_pinned, pin_order)
             VALUES ('legacy', x'001122', 'text', 1, 0, NULL)",
            [],
        )
        .unwrap();
        std::fs::write(test_dir.join(".clipman.key"), [7u8; 32]).unwrap();

        upgrade_clip_database_to_v1(&conn, &test_dir).unwrap();

        let row_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM clips", [], |row| row.get(0))
            .unwrap();
        assert_eq!(0, row_count);
        assert!(!test_dir.join(".clipman.key").exists());

        let user_version: i64 = conn
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .unwrap();
        assert_eq!(1, user_version);

        drop(conn);
        let _ = std::fs::remove_dir_all(&test_dir);
    }
}
