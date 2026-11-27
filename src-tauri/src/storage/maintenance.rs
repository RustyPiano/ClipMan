use rusqlite::{Connection, Result};
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::time::Duration;

use super::{io_to_rusqlite_error, BackupFile, ClipStorage, StorageUsage};

impl ClipStorage {
    pub(super) fn ensure_incremental_auto_vacuum(conn: &Connection) -> Result<()> {
        const INCREMENTAL: i64 = 2;

        let mode: i64 = conn.query_row("PRAGMA auto_vacuum", [], |row| row.get(0))?;
        if mode == INCREMENTAL {
            return Ok(());
        }

        let start = std::time::Instant::now();
        conn.pragma_update(None, "auto_vacuum", INCREMENTAL)?;
        conn.execute("VACUUM", [])?;
        log::info!(
            "Migrated database to incremental auto_vacuum in {:?}",
            start.elapsed()
        );
        Ok(())
    }

    pub(super) fn reclaim_space(&self) {
        let result = self
            .conn
            .prepare("PRAGMA incremental_vacuum")
            .and_then(|mut stmt| {
                let mut rows = stmt.query([])?;
                while rows.next()?.is_some() {}
                Ok(())
            });
        if let Err(error) = result {
            log::warn!("Failed to reclaim database space: {}", error);
        }
    }

    pub fn backup_to_path(&self, destination_db_path: &Path) -> Result<()> {
        backup_connection(&self.conn, destination_db_path)
    }

    pub fn usage(&self) -> Result<StorageUsage> {
        let (total, pinned, images) = self.conn.query_row(
            "SELECT COUNT(*),
                    COALESCE(SUM(is_pinned = 1), 0),
                    COALESCE(SUM(content_type = 'image'), 0)
             FROM clips",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )?;
        let db_path = self.data_dir.join(crate::identity::DATABASE_FILE);
        let mut database_bytes = 0;
        for suffix in ["", "-wal"] {
            match fs::metadata(sqlite_sidecar_path(&db_path, suffix)) {
                Ok(metadata) => database_bytes += metadata.len(),
                Err(error) if error.kind() == ErrorKind::NotFound => {}
                Err(error) => return Err(io_to_rusqlite_error(error)),
            }
        }
        Ok(StorageUsage {
            total,
            pinned,
            images,
            database_bytes,
        })
    }

    pub fn old_backups(&self) -> Result<Vec<BackupFile>> {
        let mut backups = Vec::new();
        for entry in fs::read_dir(&self.data_dir).map_err(io_to_rusqlite_error)? {
            let entry = entry.map_err(io_to_rusqlite_error)?;
            let name = entry.file_name().to_string_lossy().into_owned();
            if is_old_backup_name(&name) {
                let bytes = entry.metadata().map_err(io_to_rusqlite_error)?.len();
                backups.push(BackupFile { name, bytes });
            }
        }
        backups.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(backups)
    }

    pub fn delete_old_backups(&self) -> Result<usize> {
        let backups = self.old_backups()?;
        for backup in &backups {
            fs::remove_file(self.data_dir.join(&backup.name)).map_err(io_to_rusqlite_error)?;
        }
        Ok(backups.len())
    }
}

fn is_old_backup_name(name: &str) -> bool {
    let is_uuid = |value: &str| value.len() == 36 && uuid::Uuid::try_parse(value).is_ok();
    ["clipleaf", "clipman"].into_iter().any(|stem| {
        let pre_v3 = name
            .strip_prefix(&format!("{stem}.pre-v3-"))
            .and_then(|rest| rest.strip_suffix(".db"))
            .is_some_and(is_uuid);
        let corrupt = name
            .strip_prefix(&format!("{stem}.db.corrupt-"))
            .is_some_and(|rest| {
                ["-wal", "-shm", "-journal", ""]
                    .into_iter()
                    .any(|suffix| rest.strip_suffix(suffix).is_some_and(is_uuid))
            });
        pre_v3 || corrupt
    })
}

pub(super) fn backup_connection(conn: &Connection, destination_db_path: &Path) -> Result<()> {
    let result = (|| -> Result<()> {
        let mut destination = Connection::open(destination_db_path)?;
        let backup = rusqlite::backup::Backup::new(conn, &mut destination)?;
        backup.run_to_completion(512, Duration::from_millis(1), None)
    })();

    if result.is_err() {
        if let Err(cleanup_error) = remove_sqlite_database_files(destination_db_path) {
            log::warn!(
                "Failed to clean incomplete backup {}: {}",
                destination_db_path.display(),
                cleanup_error
            );
        }
    }
    result
}

fn sqlite_sidecar_path(path: &Path, suffix: &str) -> PathBuf {
    let mut path = path.as_os_str().to_os_string();
    path.push(suffix);
    PathBuf::from(path)
}

pub(super) fn remove_sqlite_database_files(path: &Path) -> Result<()> {
    for suffix in ["-wal", "-shm", "-journal", ""] {
        match fs::remove_file(sqlite_sidecar_path(path, suffix)) {
            Err(error) if error.kind() != ErrorKind::NotFound => {
                return Err(io_to_rusqlite_error(error));
            }
            _ => {}
        }
    }
    Ok(())
}

fn stage_sqlite_files(path: &Path, staged_db_path: &Path) -> Result<bool> {
    let mut moved_files: Vec<(PathBuf, PathBuf)> = Vec::new();
    for suffix in ["-wal", "-shm", "-journal", ""] {
        let original_path = sqlite_sidecar_path(path, suffix);
        let staged_path = sqlite_sidecar_path(staged_db_path, suffix);
        match fs::rename(&original_path, &staged_path) {
            Ok(()) => moved_files.push((staged_path, original_path)),
            Err(error) if error.kind() == ErrorKind::NotFound => {}
            Err(error) => {
                for (staged_path, original_path) in moved_files.iter().rev() {
                    if let Err(restore_error) = fs::rename(staged_path, original_path) {
                        log::warn!(
                            "Failed to restore moved database file {} to {}: {}",
                            staged_path.display(),
                            original_path.display(),
                            restore_error
                        );
                    }
                }
                return Err(io_to_rusqlite_error(error));
            }
        }
    }
    Ok(moved_files
        .iter()
        .any(|(_, original_path)| original_path == path))
}

pub(crate) fn is_corrupt_database_error(error: &rusqlite::Error) -> bool {
    matches!(
        error.sqlite_error_code(),
        Some(rusqlite::ffi::ErrorCode::DatabaseCorrupt | rusqlite::ffi::ErrorCode::NotADatabase)
    )
}

pub(crate) fn quarantine_corrupt_database(db_path: &Path) -> Result<Option<PathBuf>> {
    let backup_path = sqlite_sidecar_path(db_path, &format!(".corrupt-{}", uuid::Uuid::new_v4()));
    Ok(stage_sqlite_files(db_path, &backup_path)?.then_some(backup_path))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::test_support::*;
    use crate::storage::{ClipItem, ContentType};
    use uuid::Uuid;

    #[test]
    fn usage_counts_items_and_old_backups_can_be_deleted() {
        let root = std::env::temp_dir().join(format!("clipleaf-usage-{}", Uuid::new_v4()));
        fs::create_dir(&root).unwrap();
        let storage = ClipStorage::new(&root.join("clipleaf.db")).unwrap();
        storage
            .insert(&test_item("a", b"a", 1, false, None), 10)
            .unwrap();
        storage
            .insert(&test_item("b", b"b", 2, true, Some(1)), 10)
            .unwrap();
        let usage = storage.usage().unwrap();
        assert_eq!((2, 1, 0), (usage.total, usage.pinned, usage.images));
        assert!(usage.database_bytes > 0);

        let secure_delete: i64 = storage
            .conn
            .query_row("PRAGMA secure_delete", [], |row| row.get(0))
            .unwrap();
        assert_eq!(1, secure_delete);

        let id = "0b5c7c4e-6f1d-4a39-9d55-2b7b1f3a8e10";
        let backups = [
            format!("clipleaf.db.corrupt-{id}"),
            format!("clipleaf.db.corrupt-{id}-wal"),
            format!("clipleaf.pre-v3-{id}.db"),
            format!("clipman.db.corrupt-{id}-shm"),
            format!("clipman.pre-v3-{id}.db"),
        ];
        let kept = [
            "notes.txt".to_string(),
            "report.corrupt-draft.txt".to_string(),
            "clipleaf.pre-v3-1.db".to_string(),
            format!("clipleaf.db.corrupt-{id}.txt"),
            format!("other.db.corrupt-{id}"),
        ];
        for name in backups.iter().chain(&kept) {
            fs::write(root.join(name), b"x").unwrap();
        }
        let names: Vec<_> = storage
            .old_backups()
            .unwrap()
            .into_iter()
            .map(|backup| backup.name)
            .collect();
        assert_eq!(backups.to_vec(), names);
        assert_eq!(backups.len(), storage.delete_old_backups().unwrap());
        assert!(storage.old_backups().unwrap().is_empty());
        for name in &kept {
            assert!(root.join(name).exists(), "{name} was deleted");
        }
        assert!(root.join("clipleaf.db").exists());
        drop(storage);
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn staging_failure_restores_database_and_sidecars() {
        let database_path = temp_db_path("stage_failure_restore");
        let blocked_backup_path = database_path.with_extension("blocked");
        fs::write(&database_path, b"original-db").unwrap();
        for suffix in ["-wal", "-shm", "-journal"] {
            fs::write(
                sqlite_sidecar_path(&database_path, suffix),
                format!("original{suffix}").as_bytes(),
            )
            .unwrap();
        }
        fs::create_dir(&blocked_backup_path).unwrap();

        assert!(stage_sqlite_files(&database_path, &blocked_backup_path).is_err());
        assert_eq!(b"original-db".to_vec(), fs::read(&database_path).unwrap());
        for suffix in ["-wal", "-shm", "-journal"] {
            assert_eq!(
                format!("original{suffix}").into_bytes(),
                fs::read(sqlite_sidecar_path(&database_path, suffix)).unwrap()
            );
            assert!(!sqlite_sidecar_path(&blocked_backup_path, suffix).exists());
        }

        cleanup_db(&database_path);
        fs::remove_dir(&blocked_backup_path).unwrap();
    }

    #[test]
    fn corrupt_database_quarantine_uses_unique_backup_paths() {
        let database_path = temp_db_path("quarantine_unique");
        fs::write(&database_path, b"first").unwrap();
        let first_backup = quarantine_corrupt_database(&database_path)
            .unwrap()
            .unwrap();

        fs::write(&database_path, b"second").unwrap();
        let second_backup = quarantine_corrupt_database(&database_path)
            .unwrap()
            .unwrap();

        assert_ne!(first_backup, second_backup);
        assert_eq!(b"first".to_vec(), fs::read(&first_backup).unwrap());
        assert_eq!(b"second".to_vec(), fs::read(&second_backup).unwrap());

        cleanup_db(&database_path);
        cleanup_db(&first_backup);
        cleanup_db(&second_backup);
    }

    fn database_byte_size(storage: &ClipStorage) -> i64 {
        let page_count: i64 = storage
            .conn
            .query_row("PRAGMA page_count", [], |row| row.get(0))
            .unwrap();
        let page_size: i64 = storage
            .conn
            .query_row("PRAGMA page_size", [], |row| row.get(0))
            .unwrap();
        page_count * page_size
    }

    #[test]
    fn opening_legacy_none_auto_vacuum_database_upgrades_to_incremental() {
        let db_path = temp_db_path("auto_vacuum_legacy");
        {
            let conn = Connection::open(&db_path).unwrap();
            ClipStorage::initialize_schema(&conn).unwrap();
            conn.execute(
                "INSERT INTO clips (id, content, thumbnail, content_hash, content_type, timestamp, is_pinned, pin_order, label)
                 VALUES ('legacy', x'6c6567616379', NULL, 'hash', 'text', 1, 0, NULL, NULL)",
                [],
            )
            .unwrap();

            let mode: i64 = conn
                .query_row("PRAGMA auto_vacuum", [], |row| row.get(0))
                .unwrap();
            assert_eq!(
                0, mode,
                "sanity check: a freshly created db defaults to auto_vacuum=NONE"
            );
        }

        let storage = ClipStorage::new(&db_path).unwrap();

        let mode: i64 = storage
            .conn
            .query_row("PRAGMA auto_vacuum", [], |row| row.get(0))
            .unwrap();
        assert_eq!(2, mode);

        let content: Vec<u8> = storage
            .conn
            .query_row("SELECT content FROM clips WHERE id = 'legacy'", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(b"legacy".to_vec(), content);

        drop(storage);
        cleanup_db(&db_path);
    }

    #[test]
    fn clearing_large_clips_shrinks_database_file_via_incremental_vacuum() {
        let db_path = temp_db_path("reclaim_space");
        let storage = ClipStorage::new(&db_path).unwrap();

        storage
            .insert(
                &ClipItem {
                    content: vec![0xABu8; 200_000],
                    content_type: ContentType::Image,
                    ..test_item("big", b"", 1, false, None)
                },
                1000,
            )
            .unwrap();

        let size_before = database_byte_size(&storage);

        storage.clear_non_pinned().unwrap();

        let size_after = database_byte_size(&storage);

        assert!(
            size_after < size_before,
            "expected database file to shrink after clearing large clips: before={size_before} after={size_after}"
        );

        drop(storage);
        cleanup_db(&db_path);
    }
}
