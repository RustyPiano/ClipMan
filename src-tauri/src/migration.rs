use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};

const DATA_FILES: [&str; 3] = ["clipman.db", "clipman.db-shm", "clipman.db-wal"];
pub const CURRENT_DB_USER_VERSION: i64 = 2;
const THUMBNAIL_SIZE: u32 = 256;
const BACKFILL_BATCH_SIZE: i64 = 100;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DatabaseUpgrade {
    pub rebuilt_plaintext_storage: bool,
    pub needs_fts_rebuild: bool,
}

/// Upgrade the clipboard database to the current plaintext schema marker.
pub fn upgrade_clip_database_to_current(
    conn: &rusqlite::Connection,
    data_dir: &Path,
) -> Result<DatabaseUpgrade, String> {
    let mut user_version: i64 = conn
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .map_err(|e| format!("Failed to read database user_version: {}", e))?;
    let mut upgrade = DatabaseUpgrade {
        rebuilt_plaintext_storage: false,
        needs_fts_rebuild: false,
    };

    if user_version < 1 {
        upgrade_to_v1_plaintext(conn, data_dir)?;
        upgrade.rebuilt_plaintext_storage = true;
        upgrade.needs_fts_rebuild = true;
        user_version = 1;
    }

    if user_version < 2 {
        backfill_v2_search_columns(conn)?;
        upgrade.needs_fts_rebuild = true;
    }

    Ok(upgrade)
}

pub fn mark_clip_database_current(conn: &rusqlite::Connection) -> Result<(), String> {
    conn.pragma_update(None, "user_version", CURRENT_DB_USER_VERSION)
        .map_err(|e| format!("Failed to set database user_version: {}", e))
}

fn upgrade_to_v1_plaintext(conn: &rusqlite::Connection, data_dir: &Path) -> Result<(), String> {
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

/// Callers must ensure the current schema columns already exist
/// (`ClipStorage::initialize_schema` runs before this in `ClipStorage::new`);
/// this only fills in values for rows written by older versions.
fn backfill_v2_search_columns(conn: &rusqlite::Connection) -> Result<(), String> {
    backfill_content_hashes(conn)?;
    backfill_image_thumbnails(conn)?;
    Ok(())
}

fn backfill_content_hashes(conn: &rusqlite::Connection) -> Result<(), String> {
    loop {
        let ids = {
            let mut stmt = conn
                .prepare(
                    "SELECT id
                     FROM clips
                     WHERE content_hash IS NULL OR content_hash = ''
                     ORDER BY rowid ASC
                     LIMIT ?1",
                )
                .map_err(|e| format!("Failed to prepare content hash backfill: {}", e))?;
            let rows = stmt
                .query_map([BACKFILL_BATCH_SIZE], |row| row.get::<_, String>(0))
                .map_err(|e| format!("Failed to read rows for content hash backfill: {}", e))?;
            rows.collect::<Result<Vec<_>, _>>()
                .map_err(|e| format!("Failed to collect ids for content hash backfill: {}", e))?
        };

        if ids.is_empty() {
            break;
        }

        let tx = conn
            .unchecked_transaction()
            .map_err(|e| format!("Failed to start content hash backfill transaction: {}", e))?;
        for id in ids {
            let content: Vec<u8> = tx
                .query_row(
                    "SELECT content FROM clips WHERE id = ?1",
                    rusqlite::params![&id],
                    |row| row.get(0),
                )
                .map_err(|e| format!("Failed to read content for hash backfill: {}", e))?;
            tx.execute(
                "UPDATE clips SET content_hash = ?1 WHERE id = ?2",
                rusqlite::params![hash_bytes(&content), &id],
            )
            .map_err(|e| format!("Failed to backfill content hash: {}", e))?;
        }
        tx.commit()
            .map_err(|e| format!("Failed to commit content hash backfill: {}", e))?;
    }

    Ok(())
}

fn backfill_image_thumbnails(conn: &rusqlite::Connection) -> Result<(), String> {
    loop {
        let ids = {
            let mut stmt = conn
                .prepare(
                    "SELECT id
                     FROM clips
                     WHERE content_type = 'image' AND thumbnail IS NULL
                     ORDER BY rowid ASC
                     LIMIT ?1",
                )
                .map_err(|e| format!("Failed to prepare thumbnail backfill: {}", e))?;
            let rows = stmt
                .query_map([BACKFILL_BATCH_SIZE], |row| row.get::<_, String>(0))
                .map_err(|e| format!("Failed to read rows for thumbnail backfill: {}", e))?;
            rows.collect::<Result<Vec<_>, _>>()
                .map_err(|e| format!("Failed to collect ids for thumbnail backfill: {}", e))?
        };

        if ids.is_empty() {
            break;
        }

        let tx = conn
            .unchecked_transaction()
            .map_err(|e| format!("Failed to start thumbnail backfill transaction: {}", e))?;
        for id in ids {
            let content: Vec<u8> = tx
                .query_row(
                    "SELECT content FROM clips WHERE id = ?1",
                    rusqlite::params![&id],
                    |row| row.get(0),
                )
                .map_err(|e| format!("Failed to read content for thumbnail backfill: {}", e))?;
            let Some(thumbnail) = thumbnail_png(&content) else {
                log::warn!(
                    "Skipping thumbnail backfill for unreadable image clip {}",
                    id
                );
                tx.execute(
                    "UPDATE clips SET thumbnail = x'' WHERE id = ?1",
                    rusqlite::params![&id],
                )
                .map_err(|e| format!("Failed to mark unreadable image thumbnail: {}", e))?;
                continue;
            };
            tx.execute(
                "UPDATE clips SET thumbnail = ?1 WHERE id = ?2",
                rusqlite::params![thumbnail, &id],
            )
            .map_err(|e| format!("Failed to backfill image thumbnail: {}", e))?;
        }
        tx.commit()
            .map_err(|e| format!("Failed to commit thumbnail backfill: {}", e))?;
    }

    Ok(())
}

fn thumbnail_png(image_bytes: &[u8]) -> Option<Vec<u8>> {
    let image = image::load_from_memory(image_bytes).ok()?;
    let thumbnail = image.thumbnail(THUMBNAIL_SIZE, THUMBNAIL_SIZE);
    let mut cursor = Cursor::new(Vec::new());
    thumbnail
        .write_to(&mut cursor, image::ImageFormat::Png)
        .ok()?;
    Some(cursor.into_inner())
}

fn hash_bytes(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};

    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

fn reset_clips_table(conn: &rusqlite::Connection) -> Result<(), String> {
    conn.execute("DELETE FROM clips", [])
        .map_err(|e| format!("Failed to reset clipboard history: {}", e))?;
    Ok(())
}

pub fn prepare_destination_directory(from: &Path, to: &Path) -> Result<(), String> {
    if !from.exists() {
        return Err(format!("Source directory does not exist: {:?}", from));
    }

    fs::create_dir_all(to).map_err(|e| format!("Failed to create destination directory: {}", e))?;

    if paths_refer_to_same_location(from, to)? {
        return Err("Source and destination are the same".to_string());
    }

    // Use a unique name + exclusive create so the probe can never overwrite a
    // pre-existing user file in the destination directory: a fixed name written
    // with `fs::write` would clobber (and then delete) an unrelated file that
    // happened to share the name.
    let test_file = to.join(format!(".clipman_test_{}", uuid::Uuid::new_v4()));
    fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&test_file)
        .map_err(|e| format!("Destination directory is not writable: {}", e))?;
    fs::remove_file(&test_file).map_err(|e| format!("Failed to remove test file: {}", e))?;
    Ok(())
}

pub fn paths_refer_to_same_location(left: &Path, right: &Path) -> Result<bool, String> {
    let left = fs::canonicalize(left)
        .map_err(|e| format!("Failed to canonicalize source directory: {}", e))?;
    let right = fs::canonicalize(right)
        .map_err(|e| format!("Failed to canonicalize destination directory: {}", e))?;
    Ok(left == right)
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
    use image::{ImageBuffer, ImageFormat, Rgba};
    use std::io::Cursor;

    #[test]
    fn prepare_destination_rejects_same_path() {
        let test_dir =
            std::env::temp_dir().join(format!("clipman_prepare_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&test_dir).unwrap();

        let result = prepare_destination_directory(&test_dir, &test_dir);

        assert!(result.unwrap_err().contains("same"));
        let _ = std::fs::remove_dir_all(&test_dir);
    }

    #[cfg(unix)]
    #[test]
    fn prepare_destination_rejects_symlink_to_source() {
        let test_root =
            std::env::temp_dir().join(format!("clipman_prepare_symlink_{}", uuid::Uuid::new_v4()));
        let source = test_root.join("source");
        let link = test_root.join("link");
        std::fs::create_dir_all(&source).unwrap();
        std::os::unix::fs::symlink(&source, &link).unwrap();

        let result = prepare_destination_directory(&source, &link);

        assert!(result.unwrap_err().contains("same"));
        let _ = std::fs::remove_dir_all(&test_root);
    }

    #[test]
    fn upgrade_database_to_current_removes_legacy_key_and_resets_encrypted_rows() {
        let test_dir =
            std::env::temp_dir().join(format!("clipman_migration_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&test_dir).unwrap();
        let db_path = test_dir.join("clipman.db");
        let conn = rusqlite::Connection::open(&db_path).unwrap();
        conn.execute(
            "CREATE TABLE clips (
                id TEXT PRIMARY KEY,
                content BLOB NOT NULL,
                thumbnail BLOB,
                content_hash TEXT,
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

        let upgrade = upgrade_clip_database_to_current(&conn, &test_dir).unwrap();

        let row_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM clips", [], |row| row.get(0))
            .unwrap();
        assert_eq!(0, row_count);
        assert!(!test_dir.join(".clipman.key").exists());
        assert!(upgrade.rebuilt_plaintext_storage);
        assert!(upgrade.needs_fts_rebuild);

        let user_version: i64 = conn
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .unwrap();
        assert_eq!(1, user_version);

        mark_clip_database_current(&conn).unwrap();
        let user_version: i64 = conn
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .unwrap();
        assert_eq!(CURRENT_DB_USER_VERSION, user_version);

        drop(conn);
        let _ = std::fs::remove_dir_all(&test_dir);
    }

    #[test]
    fn upgrade_database_to_current_backfills_v2_search_columns() {
        let test_dir =
            std::env::temp_dir().join(format!("clipman_migration_v2_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&test_dir).unwrap();
        let db_path = test_dir.join("clipman.db");
        let conn = rusqlite::Connection::open(&db_path).unwrap();
        conn.execute(
            "CREATE TABLE clips (
                id TEXT PRIMARY KEY,
                content BLOB NOT NULL,
                thumbnail BLOB,
                content_hash TEXT,
                content_type TEXT NOT NULL,
                timestamp INTEGER NOT NULL,
                is_pinned INTEGER DEFAULT 0,
                pin_order INTEGER,
                label TEXT,
                group_name TEXT
            )",
            [],
        )
        .unwrap();

        let mut image_bytes = Cursor::new(Vec::new());
        let image = ImageBuffer::<Rgba<u8>, Vec<u8>>::from_pixel(1, 1, Rgba([255, 0, 0, 255]));
        image.write_to(&mut image_bytes, ImageFormat::Png).unwrap();
        let image_bytes = image_bytes.into_inner();

        conn.execute(
            "INSERT INTO clips (id, content, thumbnail, content_hash, content_type, timestamp, is_pinned, pin_order, label, group_name)
             VALUES ('text', x'68656c6c6f', NULL, NULL, 'text', 1, 0, NULL, NULL, NULL)",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO clips (id, content, thumbnail, content_hash, content_type, timestamp, is_pinned, pin_order, label, group_name)
             VALUES ('image', ?1, NULL, NULL, 'image', 2, 0, NULL, NULL, NULL)",
            [image_bytes],
        )
        .unwrap();
        conn.pragma_update(None, "user_version", 1).unwrap();

        let upgrade = upgrade_clip_database_to_current(&conn, &test_dir).unwrap();

        let missing_hashes: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM clips WHERE content_hash IS NULL OR content_hash = ''",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let image_thumbnail: Vec<u8> = conn
            .query_row(
                "SELECT thumbnail FROM clips WHERE id = 'image'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let user_version: i64 = conn
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .unwrap();

        assert_eq!(0, missing_hashes);
        assert!(!image_thumbnail.is_empty());
        assert_eq!(1, user_version);
        assert!(!upgrade.rebuilt_plaintext_storage);
        assert!(upgrade.needs_fts_rebuild);

        mark_clip_database_current(&conn).unwrap();
        let user_version: i64 = conn
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .unwrap();
        assert_eq!(CURRENT_DB_USER_VERSION, user_version);

        drop(conn);
        let _ = std::fs::remove_dir_all(&test_dir);
    }
}
