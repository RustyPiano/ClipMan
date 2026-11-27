use rusqlite::{params, Connection, Result};

use super::ClipStorage;

pub(super) const FILES_TEXT_SQL: &str = "CASE WHEN content_type='files'
    THEN CAST(COALESCE((SELECT group_concat(value, char(10)) FROM json_each(CAST(content AS TEXT))), '') AS BLOB) ELSE content END";

fn fts_insert_sql(filter: &str) -> String {
    format!(
        "INSERT INTO clips_fts(rowid, clip_id, search_text, label)
     SELECT rowid, id,
       CASE WHEN content_type IN ('text','files') THEN CAST(({FILES_TEXT_SQL}) AS TEXT) ELSE '' END,
       label
     FROM clips {filter}"
    )
}

impl ClipStorage {
    pub(super) fn initialize_fts(conn: &Connection) -> Result<()> {
        conn.execute(
            "CREATE VIRTUAL TABLE IF NOT EXISTS clips_fts
             USING fts5(clip_id UNINDEXED, search_text, label, tokenize='trigram')",
            [],
        )?;
        conn.execute(
            "INSERT INTO clips_fts(clips_fts, rank) VALUES('secure-delete', 1)",
            [],
        )?;
        Ok(())
    }

    pub(super) fn fts_needs_rebuild(&self) -> Result<bool> {
        self.conn.query_row(
            "SELECT EXISTS(
                SELECT 1 FROM clips c
                LEFT JOIN clips_fts f ON f.rowid = c.rowid AND f.clip_id = c.id
                WHERE f.rowid IS NULL
             ) OR EXISTS(
                SELECT 1 FROM clips_fts f
                LEFT JOIN clips c ON f.rowid = c.rowid AND f.clip_id = c.id
                WHERE c.rowid IS NULL
             )",
            [],
            |row| row.get(0),
        )
    }

    pub(super) fn rebuild_fts_index(&self) -> Result<()> {
        let tx = self.conn.unchecked_transaction()?;
        tx.execute("DELETE FROM clips_fts", [])?;
        tx.execute(&fts_insert_sql(""), [])?;
        tx.commit()
    }

    pub(super) fn sync_fts_for_clip_id_with_conn(conn: &Connection, id: &str) -> Result<()> {
        conn.execute("DELETE FROM clips_fts WHERE clip_id = ?1", params![id])?;
        conn.execute(&fts_insert_sql("WHERE id = ?1"), params![id])?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::test_support::*;

    #[test]
    fn current_database_rebuilds_fts_only_when_incomplete() {
        let db_path = temp_db_path("fts_rebuild_check");
        let first_search_text = |storage: &ClipStorage| -> String {
            storage
                .conn
                .query_row(
                    "SELECT search_text FROM clips_fts WHERE clip_id = 'first'",
                    [],
                    |row| row.get(0),
                )
                .unwrap()
        };

        let storage = ClipStorage::new(&db_path).unwrap();
        storage
            .insert(&test_item("first", b"first", 1, false, None), 100)
            .unwrap();
        storage
            .insert(&test_item("second", b"second", 2, false, None), 100)
            .unwrap();
        storage
            .conn
            .execute(
                "UPDATE clips_fts SET search_text = 'sentinel' WHERE clip_id = 'first'",
                [],
            )
            .unwrap();
        drop(storage);

        let storage = ClipStorage::new(&db_path).unwrap();
        assert_eq!("sentinel", first_search_text(&storage));
        storage
            .conn
            .execute("DELETE FROM clips_fts WHERE clip_id = 'second'", [])
            .unwrap();
        drop(storage);

        let storage = ClipStorage::new(&db_path).unwrap();
        assert_eq!("first", first_search_text(&storage));
        let indexed: i64 = storage
            .conn
            .query_row("SELECT COUNT(*) FROM clips_fts", [], |row| row.get(0))
            .unwrap();
        assert_eq!(2, indexed);
        drop(storage);
        cleanup_db(&db_path);
    }

    #[test]
    fn fts_search_ignores_rows_with_mismatched_clip_id() {
        let db_path = temp_db_path("fts_mismatch_join");
        let storage = ClipStorage::new(&db_path).unwrap();

        storage
            .insert(&test_item("first", b"first content", 1, false, None), 100)
            .unwrap();
        storage
            .insert(&test_item("second", b"second content", 2, false, None), 100)
            .unwrap();
        storage.conn.execute("DELETE FROM clips_fts", []).unwrap();
        let second_rowid: i64 = storage
            .conn
            .query_row("SELECT rowid FROM clips WHERE id = 'second'", [], |row| {
                row.get(0)
            })
            .unwrap();
        storage
            .conn
            .execute(
                "INSERT INTO clips_fts(rowid, clip_id, search_text, label)
                 VALUES (?1, 'first', 'needle', NULL)",
                params![second_rowid],
            )
            .unwrap();

        let preview_search = storage.search_clip_previews("needle").unwrap();

        assert!(preview_search.is_empty());
        drop(storage);
        cleanup_db(&db_path);
    }
}
