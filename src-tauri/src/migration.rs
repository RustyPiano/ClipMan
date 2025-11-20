use std::path::{Path, PathBuf};
use std::fs;

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
    fs::create_dir_all(to)
        .map_err(|e| format!("Failed to create destination directory: {}", e))?;
    
    // Check write permission by creating a test file
    let test_file = to.join(".clipman_test");
    fs::write(&test_file, "test")
        .map_err(|e| format!("Destination directory is not writable: {}", e))?;
    fs::remove_file(&test_file)
        .map_err(|e| format!("Failed to remove test file: {}", e))?;
    
    // Files to migrate
    let files_to_migrate = vec![
        "clipman.db",
        "clipman.db-shm",  // SQLite shared memory (if exists)
        "clipman.db-wal",  // SQLite write-ahead log (if exists)
        ".clipman.key",
    ];
    
    // Copy files
    for filename in &files_to_migrate {
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
    
    // Verify key file exists at destination
    let key_file = to.join(".clipman.key");
    if !key_file.exists() {
        return Err("Migration failed: encryption key not found at destination".to_string());
    }
    
    log::info!("Data migration successful");
    
    // Delete old files if requested
    if delete_old {
        log::info!("Deleting old data files from {:?}", from);
        for filename in &files_to_migrate {
            let old_file = from.join(filename);
            if old_file.exists() {
                fs::remove_file(&old_file)
                    .map_err(|e| format!("Failed to remove old file {}: {}", filename, e))?;
            }
        }
        
        // Try to remove old directory if it's empty
        if let Err(e) = fs::remove_dir(from) {
            log::warn!("Could not remove old directory (may not be empty): {}", e);
        }
    }
    
    Ok(())
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
    use std::env;
    
    #[test]
    fn test_same_path_error() {
        let path = PathBuf::from("/tmp/test");
        let result = migrate_data(&path, &path, false);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("same"));
    }
}
