use rusqlite::{Connection, Result};
use std::fs;
use std::path::Path;

use super::maintenance::backup_connection;
use super::{io_to_rusqlite_error, string_to_rusqlite_error, ClipStorage};

impl ClipStorage {
    pub fn new(db_path: &Path) -> Result<Self> {
        let conn = Connection::open(db_path)?;
        conn.pragma_update(None, "secure_delete", "ON")?;
        let data_dir = fs::canonicalize(db_path)
            .map_err(io_to_rusqlite_error)?
            .parent()
            .unwrap()
            .to_path_buf();
        let version: i64 = conn.query_row("PRAGMA user_version", [], |row| row.get(0))?;
        if version > crate::migration::CURRENT_DB_USER_VERSION {
            return Err(string_to_rusqlite_error(format!(
                "Database format {version} is newer than this app supports"
            )));
        }
        let existing: bool = conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='clips')",
            [],
            |row| row.get(0),
        )?;
        if existing && version < crate::migration::CURRENT_DB_USER_VERSION {
            let backup_path = data_dir.join(format!("clipleaf.pre-v3-{}.db", uuid::Uuid::new_v4()));
            backup_connection(&conn, &backup_path)?;
            let legacy_key = data_dir.join(".clipman.key");
            if legacy_key.exists() {
                fs::copy(&legacy_key, backup_path.with_extension("key"))
                    .map_err(io_to_rusqlite_error)?;
            }
            log::info!(
                "Saved pre-upgrade database backup at {}",
                backup_path.display()
            );
        }
        conn.pragma_update(None, "journal_mode", "WAL")?;

        Self::initialize_schema(&conn)?;

        let needs_fts_rebuild =
            crate::migration::upgrade_clip_database_to_current(&conn, &data_dir)
                .map_err(string_to_rusqlite_error)?;

        Self::initialize_fts(&conn)?;
        Self::ensure_incremental_auto_vacuum(&conn)?;

        let storage = Self { conn, data_dir };
        if needs_fts_rebuild || storage.fts_needs_rebuild()? {
            storage.rebuild_fts_index()?;
        }
        if version < crate::migration::CURRENT_DB_USER_VERSION {
            crate::migration::mark_clip_database_current(&storage.conn)
                .map_err(string_to_rusqlite_error)?;
        }
        Ok(storage)
    }

    pub(super) fn initialize_schema(conn: &Connection) -> Result<()> {
        conn.execute(
            "CREATE TABLE IF NOT EXISTS clips (
                id TEXT PRIMARY KEY,
                content BLOB NOT NULL,
                thumbnail BLOB,
                content_hash TEXT,
                content_type TEXT NOT NULL,
                timestamp INTEGER NOT NULL,
                is_pinned INTEGER DEFAULT 0,
                pin_order INTEGER,
                label TEXT,
                source_app TEXT,
                html TEXT
            )",
            [],
        )?;

        Self::add_column_if_missing(conn, "content_hash", "TEXT")?;
        Self::add_column_if_missing(conn, "thumbnail", "BLOB")?;
        Self::add_column_if_missing(conn, "label", "TEXT")?;
        Self::add_column_if_missing(conn, "source_app", "TEXT")?;
        Self::add_column_if_missing(conn, "html", "TEXT")?;
        conn.execute(
            "CREATE TABLE IF NOT EXISTS app_icons (name TEXT PRIMARY KEY, png BLOB NOT NULL)",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_timestamp ON clips(timestamp DESC)",
            [],
        )?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_pinned ON clips(is_pinned, pin_order)",
            [],
        )?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_content_hash ON clips(content_hash, content_type)",
            [],
        )?;
        conn.execute("DROP INDEX IF EXISTS idx_recent_unpinned_timestamp", [])?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_recent_unpinned_ts_id
             ON clips(is_pinned, timestamp DESC, id DESC)",
            [],
        )?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_pinned_order_stable
             ON clips(is_pinned, (pin_order IS NULL), pin_order ASC, timestamp DESC)",
            [],
        )?;

        Ok(())
    }

    fn add_column_if_missing(conn: &Connection, name: &str, column_type: &str) -> Result<()> {
        let exists: bool = conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM pragma_table_info('clips') WHERE name = ?1)",
            [name],
            |row| row.get(0),
        )?;
        if !exists {
            log::info!("Migrating database: adding {} column", name);
            conn.execute(
                &format!("ALTER TABLE clips ADD COLUMN {name} {column_type}"),
                [],
            )?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::test_support::*;
    use crate::storage::{encode_file_paths, join_file_paths};
    use uuid::Uuid;

    #[test]
    fn v3_upgrade_backs_up_then_converts_files_once_and_rejects_newer_formats() {
        let root = std::env::temp_dir().join(format!("clipleaf-upgrade-{}", Uuid::new_v4()));
        fs::create_dir(&root).unwrap();
        let path = root.join("clipleaf.db");
        {
            let conn = Connection::open(&path).unwrap();
            ClipStorage::initialize_schema(&conn).unwrap();
            conn.execute("INSERT INTO clips(id,content,content_type,timestamp,is_pinned) VALUES('legacy',?1,'files',1,1)", [b"/tmp/first.txt\n/tmp/second.txt".as_slice()]).unwrap();
            conn.pragma_update(None, "user_version", 2).unwrap();
        }
        let storage = ClipStorage::new(&path).unwrap();
        let backups = fs::read_dir(&root)
            .unwrap()
            .filter_map(|e| {
                let p = e.unwrap().path();
                p.file_name()
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .starts_with("clipleaf.pre-v3-")
                    .then_some(p)
            })
            .collect::<Vec<_>>();
        assert_eq!(backups.len(), 1);
        let backup = Connection::open(&backups[0]).unwrap();
        assert_eq!(
            backup
                .query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0))
                .unwrap(),
            2
        );
        let old: Vec<u8> = backup
            .query_row("SELECT content FROM clips WHERE id='legacy'", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(old, b"/tmp/first.txt\n/tmp/second.txt");
        let paths = vec!["/tmp/first.txt".to_string(), "/tmp/second.txt".to_string()];
        assert_eq!(
            storage.get_by_id("legacy").unwrap().unwrap().content,
            encode_file_paths(&paths).as_bytes()
        );
        assert_eq!(
            storage.get_detail("legacy").unwrap().unwrap().text,
            join_file_paths(&paths)
        );
        drop(backup);
        drop(storage);
        Connection::open(&path)
            .unwrap()
            .execute("INSERT INTO clips(id,content,content_type,timestamp,is_pinned) VALUES('downgraded',?1,'files',2,0)", [b"/tmp/a.txt\n\n/tmp/b.txt\n".as_slice()])
            .unwrap();
        let reopened = ClipStorage::new(&path).unwrap();
        assert_eq!(
            reopened.get_by_id("downgraded").unwrap().unwrap().content,
            encode_file_paths(&["/tmp/a.txt".into(), "/tmp/b.txt".into()]).as_bytes()
        );
        assert_eq!(reopened.search_clip_previews("b.txt").unwrap().len(), 1);
        assert_eq!(
            fs::read_dir(&root)
                .unwrap()
                .filter(|e| e
                    .as_ref()
                    .unwrap()
                    .file_name()
                    .to_string_lossy()
                    .starts_with("clipleaf.pre-v3-"))
                .count(),
            1
        );
        reopened
            .conn
            .pragma_update(None, "user_version", 99)
            .unwrap();
        drop(reopened);
        assert!(ClipStorage::new(&path).is_err());
        assert_eq!(
            Connection::open(&path)
                .unwrap()
                .query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0))
                .unwrap(),
            99
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn active_data_directory_follows_the_opened_database_symlink() {
        let root = std::env::temp_dir().join(format!("clipleaf-active-{}", Uuid::new_v4()));
        fs::create_dir(&root).unwrap();
        let actual = root.join("actual");
        fs::create_dir(&actual).unwrap();
        drop(ClipStorage::new(&actual.join("clipleaf.db")).unwrap());
        let alias = root.join("alias.db");
        std::os::unix::fs::symlink(actual.join("clipleaf.db"), &alias).unwrap();
        let storage = ClipStorage::new(&alias).unwrap();
        assert_eq!(storage.data_directory(), fs::canonicalize(&actual).unwrap());
        drop(storage);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn new_database_uses_wal_current_version_and_incremental_vacuum() {
        let db_path = temp_db_path("schema");
        let storage = ClipStorage::new(&db_path).unwrap();

        let journal_mode: String = storage
            .conn
            .query_row("PRAGMA journal_mode", [], |row| row.get(0))
            .unwrap();
        assert_eq!("wal", journal_mode);

        let user_version: i64 = storage
            .conn
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .unwrap();
        assert_eq!(crate::migration::CURRENT_DB_USER_VERSION, user_version);

        let auto_vacuum: i64 = storage
            .conn
            .query_row("PRAGMA auto_vacuum", [], |row| row.get(0))
            .unwrap();
        assert_eq!(2, auto_vacuum);

        drop(storage);
        cleanup_db(&db_path);
    }
}
