use rusqlite::{params, Connection, OptionalExtension, Result, Row};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ContentType {
    Text,
    Image,
    File,
    Html,
    Rtf,
}

impl ContentType {
    fn as_db_value(&self) -> &str {
        match self {
            ContentType::Text => "text",
            ContentType::Image => "image",
            ContentType::File => "file",
            ContentType::Html => "html",
            ContentType::Rtf => "rtf",
        }
    }

    fn from_db_value(value: &str) -> Self {
        match value {
            "image" => ContentType::Image,
            "file" => ContentType::File,
            "html" => ContentType::Html,
            "rtf" => ContentType::Rtf,
            _ => ContentType::Text,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CopyMarker {
    pub hash: String,
    pub content_type: ContentType,
}

impl CopyMarker {
    /// The payload must already be normalized by the caller for its clipboard type.
    pub fn from_payload(content_type: ContentType, payload: &[u8]) -> Self {
        Self {
            hash: hash_bytes(payload),
            content_type,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClipItem {
    pub id: String,
    #[serde(with = "serde_bytes")]
    pub content: Vec<u8>,
    #[serde(with = "serde_bytes")]
    pub thumbnail: Option<Vec<u8>>,
    pub content_type: ContentType,
    pub timestamp: i64,
    pub is_pinned: bool,
    pub pin_order: Option<i32>,
    pub label: Option<String>,
    pub group_name: Option<String>,
}

// Frontend-optimized version: converts images to data URLs for zero-cost rendering
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FrontendClipItem {
    pub id: String,
    pub content: String, // Base64 string or data URL
    pub content_type: ContentType,
    pub timestamp: i64,
    pub is_pinned: bool,
    pub pin_order: Option<i32>,
    pub label: Option<String>,
    pub group_name: Option<String>,
}

impl From<ClipItem> for FrontendClipItem {
    fn from(item: ClipItem) -> Self {
        use data_encoding::BASE64;

        let content = match item.content_type {
            ContentType::Image => {
                let image_bytes = item.thumbnail.as_deref().unwrap_or(&item.content);
                format!("data:image/png;base64,{}", BASE64.encode(image_bytes))
            }
            _ => BASE64.encode(&item.content),
        };

        FrontendClipItem {
            id: item.id,
            content,
            content_type: item.content_type,
            timestamp: item.timestamp,
            is_pinned: item.is_pinned,
            pin_order: item.pin_order,
            label: item.label,
            group_name: item.group_name,
        }
    }
}

pub struct ClipStorage {
    conn: Connection,
}

const CLIP_COLUMNS: &str =
    "id, content, thumbnail, content_type, timestamp, is_pinned, pin_order, label, group_name";
const CLIP_COLUMNS_WITH_ALIAS: &str = "c.id, c.content, c.thumbnail, c.content_type, c.timestamp, c.is_pinned, c.pin_order, c.label, c.group_name";

impl ClipStorage {
    pub fn new(db_path: &str) -> Result<Self> {
        let conn = Connection::open(db_path)?;
        conn.pragma_update(None, "journal_mode", "WAL")?;

        Self::initialize_schema(&conn)?;

        let data_dir = data_dir_for_db_path(db_path);
        crate::migration::upgrade_clip_database_to_v1(&conn, &data_dir)
            .map_err(string_to_rusqlite_error)?;

        Self::initialize_fts(&conn)?;

        let storage = Self { conn };
        storage.rebuild_fts_index()?;
        Ok(storage)
    }

    pub fn insert(&self, item: &ClipItem, max_history_items: usize) -> Result<Option<String>> {
        let content_hash = hash_bytes(&item.content);

        let existing_id: Option<String> = self
            .conn
            .query_row(
                "SELECT id FROM clips
                 WHERE content_hash = ?1 AND content_type = ?2
                 ORDER BY timestamp DESC
                 LIMIT 100",
                params![content_hash, item.content_type.as_db_value()],
                |row| row.get(0),
            )
            .optional()?;

        if let Some(id) = existing_id {
            log::debug!(
                "⏭️ Duplicate content detected (hash: {}), updating timestamp",
                &content_hash[..8]
            );
            self.update_timestamp(&id, item.timestamp)?;
            return Ok(Some(id));
        }

        self.conn.execute(
            "INSERT INTO clips (
                id, content, thumbnail, content_hash, content_type, timestamp,
                is_pinned, pin_order, label, group_name
             )
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                item.id,
                item.content,
                item.thumbnail,
                content_hash,
                item.content_type.as_db_value(),
                item.timestamp,
                item.is_pinned as i32,
                item.pin_order,
                item.label,
                item.group_name,
            ],
        )?;

        self.sync_fts_for_clip_id(&item.id)?;
        self.prune_history(max_history_items)?;

        Ok(None)
    }

    pub fn get_recent_clips(&self, limit: usize) -> Result<Vec<ClipItem>> {
        let mut stmt = self.conn.prepare(&format!(
            "SELECT {CLIP_COLUMNS}
             FROM clips
             WHERE is_pinned = 0
             ORDER BY timestamp DESC
             LIMIT ?1"
        ))?;

        let items = stmt.query_map([limit], Self::clip_from_row)?;
        items.collect()
    }

    pub fn get_pinned_clips(&self) -> Result<Vec<ClipItem>> {
        let mut stmt = self.conn.prepare(&format!(
            "SELECT {CLIP_COLUMNS}
             FROM clips
             WHERE is_pinned = 1
             ORDER BY pin_order IS NULL, pin_order ASC, timestamp DESC"
        ))?;

        let items = stmt.query_map([], Self::clip_from_row)?;
        items.collect()
    }

    /// Deprecated compatibility wrapper. New code should call `get_recent_clips`.
    pub fn get_recent(&self, limit: usize) -> Result<Vec<ClipItem>> {
        self.get_recent_clips(limit)
    }

    /// Deprecated compatibility wrapper. New code should call `get_pinned_clips`.
    pub fn get_pinned(&self) -> Result<Vec<ClipItem>> {
        self.get_pinned_clips()
    }

    pub fn search(&self, query: &str) -> Result<Vec<ClipItem>> {
        log::info!("🔍 Searching for: {}", query);

        let query = query.trim();
        if query.is_empty() {
            return self.get_all_for_search();
        }

        if query.chars().count() < 3 {
            return self.search_with_like(query);
        }

        self.search_with_fts(query)
    }

    pub fn update_pin(&self, id: &str, is_pinned: bool) -> Result<()> {
        let pin_order = if is_pinned {
            let max_order: Option<i32> = self
                .conn
                .query_row(
                    "SELECT MAX(pin_order) FROM clips WHERE is_pinned = 1",
                    [],
                    |row| row.get(0),
                )
                .unwrap_or(None);

            Some(max_order.unwrap_or(0) + 1)
        } else {
            None
        };

        self.conn.execute(
            "UPDATE clips SET is_pinned = ?1, pin_order = ?2 WHERE id = ?3",
            params![is_pinned as i32, pin_order, id],
        )?;

        Ok(())
    }

    pub fn set_clip_label(&self, id: &str, label: Option<String>) -> Result<()> {
        self.conn.execute(
            "UPDATE clips SET label = ?1 WHERE id = ?2",
            params![label, id],
        )?;
        self.sync_fts_for_clip_id(id)?;
        Ok(())
    }

    pub fn delete(&self, id: &str) -> Result<()> {
        self.conn
            .execute("DELETE FROM clips_fts WHERE clip_id = ?1", params![id])?;
        self.conn
            .execute("DELETE FROM clips WHERE id = ?1", params![id])?;
        Ok(())
    }

    pub fn clear_all(&self) -> Result<()> {
        log::info!("🗑️ Clearing all clipboard history");
        self.conn.execute("DELETE FROM clips_fts", [])?;
        self.conn.execute("DELETE FROM clips", [])?;
        Ok(())
    }

    pub fn clear_non_pinned(&self) -> Result<()> {
        log::info!("🗑️ Clearing non-pinned clipboard history");
        self.conn.execute(
            "DELETE FROM clips_fts
             WHERE clip_id IN (SELECT id FROM clips WHERE is_pinned = 0)",
            [],
        )?;
        self.conn
            .execute("DELETE FROM clips WHERE is_pinned = 0", [])?;
        Ok(())
    }

    /// Get a single clip item by ID (efficient single-row lookup)
    pub fn get_by_id(&self, id: &str) -> Result<Option<ClipItem>> {
        let mut stmt = self.conn.prepare(&format!(
            "SELECT {CLIP_COLUMNS}
             FROM clips
             WHERE id = ?1"
        ))?;

        stmt.query_row([id], Self::clip_from_row).optional()
    }

    /// Update the timestamp of a clip item (move it to the top of recent list)
    pub fn update_timestamp(&self, id: &str, new_timestamp: i64) -> Result<()> {
        self.conn.execute(
            "UPDATE clips SET timestamp = ?1 WHERE id = ?2",
            params![new_timestamp, id],
        )?;
        log::debug!("📍 Updated timestamp for item {}", id);
        Ok(())
    }

    fn initialize_schema(conn: &Connection) -> Result<()> {
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
                group_name TEXT
            )",
            [],
        )?;

        Self::add_column_if_missing(conn, "content_hash", "TEXT")?;
        Self::add_column_if_missing(conn, "thumbnail", "BLOB")?;
        Self::add_column_if_missing(conn, "label", "TEXT")?;
        Self::add_column_if_missing(conn, "group_name", "TEXT")?;

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

        Ok(())
    }

    fn initialize_fts(conn: &Connection) -> Result<()> {
        conn.execute(
            "CREATE VIRTUAL TABLE IF NOT EXISTS clips_fts
             USING fts5(clip_id UNINDEXED, search_text, label, tokenize='trigram')",
            [],
        )?;
        Ok(())
    }

    fn add_column_if_missing(conn: &Connection, name: &str, column_type: &str) -> Result<()> {
        if !Self::has_column(conn, name)? {
            log::info!("📦 Migrating database: adding {} column", name);
            conn.execute(
                &format!("ALTER TABLE clips ADD COLUMN {name} {column_type}"),
                [],
            )?;
        }

        Ok(())
    }

    fn has_column(conn: &Connection, name: &str) -> Result<bool> {
        let mut stmt = conn.prepare("PRAGMA table_info(clips)")?;
        let columns = stmt.query_map([], |row| row.get::<_, String>(1))?;

        for column in columns {
            if column? == name {
                return Ok(true);
            }
        }

        Ok(false)
    }

    fn clip_from_row(row: &Row<'_>) -> Result<ClipItem> {
        Ok(ClipItem {
            id: row.get(0)?,
            content: row.get(1)?,
            thumbnail: row.get(2)?,
            content_type: ContentType::from_db_value(&row.get::<_, String>(3)?),
            timestamp: row.get(4)?,
            is_pinned: row.get::<_, i32>(5)? != 0,
            pin_order: row.get(6)?,
            label: row.get(7)?,
            group_name: row.get(8)?,
        })
    }

    fn get_all_for_search(&self) -> Result<Vec<ClipItem>> {
        let mut stmt = self.conn.prepare(&format!(
            "SELECT {CLIP_COLUMNS}
             FROM clips
             ORDER BY timestamp DESC
             LIMIT 1000"
        ))?;

        let items = stmt.query_map([], Self::clip_from_row)?;
        items.collect()
    }

    fn search_with_fts(&self, query: &str) -> Result<Vec<ClipItem>> {
        let fts_query = escape_fts_query(query);
        let mut stmt = self.conn.prepare(&format!(
            "SELECT {CLIP_COLUMNS_WITH_ALIAS}
             FROM clips c
             JOIN clips_fts ON clips_fts.rowid = c.rowid
             WHERE clips_fts MATCH ?1
             ORDER BY c.timestamp DESC
             LIMIT 1000"
        ))?;

        let items = stmt.query_map([fts_query], Self::clip_from_row)?;
        items.collect()
    }

    fn search_with_like(&self, query: &str) -> Result<Vec<ClipItem>> {
        let like_query = format!("%{}%", escape_like_query(query));
        let mut stmt = self.conn.prepare(&format!(
            "SELECT {CLIP_COLUMNS}
             FROM clips
             WHERE (
                content_type IN ('text', 'file', 'html', 'rtf')
                AND CAST(content AS TEXT) LIKE ?1 ESCAPE '\\'
             )
             OR COALESCE(label, '') LIKE ?1 ESCAPE '\\'
             ORDER BY timestamp DESC
             LIMIT 1000"
        ))?;

        let items = stmt.query_map([like_query], Self::clip_from_row)?;
        items.collect()
    }

    fn rebuild_fts_index(&self) -> Result<()> {
        self.conn.execute("DELETE FROM clips_fts", [])?;

        let payloads = {
            let mut stmt = self.conn.prepare(
                "SELECT rowid, id, content, content_type, label
                 FROM clips
                 ORDER BY rowid ASC",
            )?;
            let rows = stmt.query_map([], |row| {
                let content_type = ContentType::from_db_value(&row.get::<_, String>(3)?);
                Ok(FtsPayload {
                    rowid: row.get(0)?,
                    clip_id: row.get(1)?,
                    content: row.get(2)?,
                    content_type,
                    label: row.get(4)?,
                })
            })?;
            rows.collect::<Result<Vec<_>>>()?
        };

        for payload in payloads {
            self.insert_fts_payload(&payload)?;
        }

        Ok(())
    }

    fn sync_fts_for_clip_id(&self, id: &str) -> Result<()> {
        self.conn
            .execute("DELETE FROM clips_fts WHERE clip_id = ?1", params![id])?;

        let payload = self
            .conn
            .query_row(
                "SELECT rowid, id, content, content_type, label
                 FROM clips
                 WHERE id = ?1",
                params![id],
                |row| {
                    let content_type = ContentType::from_db_value(&row.get::<_, String>(3)?);
                    Ok(FtsPayload {
                        rowid: row.get(0)?,
                        clip_id: row.get(1)?,
                        content: row.get(2)?,
                        content_type,
                        label: row.get(4)?,
                    })
                },
            )
            .optional()?;

        if let Some(payload) = payload {
            self.insert_fts_payload(&payload)?;
        }

        Ok(())
    }

    fn insert_fts_payload(&self, payload: &FtsPayload) -> Result<()> {
        let search_text = search_text_for_fts(&payload.content, &payload.content_type);
        self.conn.execute(
            "INSERT INTO clips_fts(rowid, clip_id, search_text, label)
             VALUES (?1, ?2, ?3, ?4)",
            params![
                payload.rowid,
                payload.clip_id,
                search_text,
                payload.label,
            ],
        )?;
        Ok(())
    }

    fn prune_history(&self, max_history_items: usize) -> Result<()> {
        let stale_ids = {
            let mut stmt = self.conn.prepare(
                "SELECT id FROM clips
                 WHERE is_pinned = 0
                 ORDER BY timestamp DESC
                 LIMIT -1 OFFSET ?1",
            )?;
            let rows = stmt.query_map([max_history_items], |row| row.get::<_, String>(0))?;
            rows.collect::<Result<Vec<_>>>()?
        };

        for id in stale_ids {
            self.delete(&id)?;
        }

        Ok(())
    }
}

struct FtsPayload {
    rowid: i64,
    clip_id: String,
    content: Vec<u8>,
    content_type: ContentType,
    label: Option<String>,
}

fn hash_bytes(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

fn search_text_for_fts(content: &[u8], content_type: &ContentType) -> String {
    match content_type {
        ContentType::Text | ContentType::File | ContentType::Html | ContentType::Rtf => {
            String::from_utf8_lossy(content).into_owned()
        }
        ContentType::Image => String::new(),
    }
}

fn escape_fts_query(query: &str) -> String {
    format!("\"{}\"", query.replace('"', "\"\""))
}

fn escape_like_query(query: &str) -> String {
    query
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}

fn data_dir_for_db_path(db_path: &str) -> PathBuf {
    Path::new(db_path)
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."))
}

fn string_to_rusqlite_error(error: String) -> rusqlite::Error {
    rusqlite::Error::ToSqlConversionFailure(Box::new(std::io::Error::new(
        std::io::ErrorKind::Other,
        error,
    )))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    use uuid::Uuid;

    fn temp_db_path(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("clipman_{}_{}.db", name, Uuid::new_v4()))
    }

    fn cleanup_db(path: &Path) {
        let _ = fs::remove_file(path);
        let _ = fs::remove_file(format!("{}-shm", path.display()));
        let _ = fs::remove_file(format!("{}-wal", path.display()));
    }

    fn test_item(
        id: &str,
        content: &[u8],
        timestamp: i64,
        is_pinned: bool,
        pin_order: Option<i32>,
    ) -> ClipItem {
        ClipItem {
            id: id.to_string(),
            content: content.to_vec(),
            thumbnail: None,
            content_type: ContentType::Text,
            timestamp,
            is_pinned,
            pin_order,
            label: None,
            group_name: None,
        }
    }

    fn labeled_item(id: &str, content: &[u8], label: &str, timestamp: i64) -> ClipItem {
        ClipItem {
            label: Some(label.to_string()),
            ..test_item(id, content, timestamp, false, None)
        }
    }

    #[test]
    fn new_database_uses_v1_schema_and_wal() {
        let db_path = temp_db_path("schema");
        let storage = ClipStorage::new(db_path.to_str().unwrap()).unwrap();

        let columns: Vec<String> = storage
            .conn
            .prepare("PRAGMA table_info(clips)")
            .unwrap()
            .query_map([], |row| row.get(1))
            .unwrap()
            .collect::<Result<Vec<String>>>()
            .unwrap();

        assert!(columns.contains(&"content_hash".to_string()));
        assert!(columns.contains(&"thumbnail".to_string()));
        assert!(columns.contains(&"label".to_string()));
        assert!(columns.contains(&"group_name".to_string()));

        let journal_mode: String = storage
            .conn
            .query_row("PRAGMA journal_mode", [], |row| row.get(0))
            .unwrap();
        assert_eq!("wal", journal_mode);

        let user_version: i64 = storage
            .conn
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .unwrap();
        assert_eq!(1, user_version);

        drop(storage);
        cleanup_db(&db_path);
    }

    #[test]
    fn insert_stores_plaintext_content() {
        let db_path = temp_db_path("plaintext");
        let storage = ClipStorage::new(db_path.to_str().unwrap()).unwrap();
        storage
            .insert(&test_item("plain", b"Hello, ClipMan!", 1, false, None), 100)
            .unwrap();

        let stored_content: Vec<u8> = storage
            .conn
            .query_row("SELECT content FROM clips WHERE id = 'plain'", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(b"Hello, ClipMan!".to_vec(), stored_content);

        drop(storage);
        cleanup_db(&db_path);
    }

    #[test]
    fn new_database_creates_trigram_fts_table() {
        let db_path = temp_db_path("fts_schema");
        let storage = ClipStorage::new(db_path.to_str().unwrap()).unwrap();

        let table_sql: String = storage
            .conn
            .query_row(
                "SELECT sql FROM sqlite_master WHERE type = 'table' AND name = 'clips_fts'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert!(table_sql.contains("fts5"));
        assert!(table_sql.contains("clip_id UNINDEXED"));
        assert!(table_sql.contains("tokenize='trigram'"));

        drop(storage);
        cleanup_db(&db_path);
    }

    #[test]
    fn fts_index_tracks_insert_label_search_and_delete() {
        let db_path = temp_db_path("fts_sync");
        let storage = ClipStorage::new(db_path.to_str().unwrap()).unwrap();

        storage
            .insert(&labeled_item("labeled", b"deploy command", "work email", 1), 100)
            .unwrap();

        let indexed_label: String = storage
            .conn
            .query_row(
                "SELECT label FROM clips_fts WHERE clip_id = 'labeled'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!("work email", indexed_label);

        let search_ids: Vec<String> = storage
            .search("email")
            .unwrap()
            .into_iter()
            .map(|item| item.id)
            .collect();
        assert_eq!(vec!["labeled"], search_ids);

        storage.delete("labeled").unwrap();
        let fts_count: i64 = storage
            .conn
            .query_row("SELECT COUNT(*) FROM clips_fts", [], |row| row.get(0))
            .unwrap();
        assert_eq!(0, fts_count);

        drop(storage);
        cleanup_db(&db_path);
    }

    #[test]
    fn short_search_query_uses_like_fallback() {
        let db_path = temp_db_path("fts_short_query");
        let storage = ClipStorage::new(db_path.to_str().unwrap()).unwrap();

        storage
            .insert(&test_item("cn", "中文内容".as_bytes(), 1, false, None), 100)
            .unwrap();

        let search_ids: Vec<String> = storage
            .search("中")
            .unwrap()
            .into_iter()
            .map(|item| item.id)
            .collect();
        assert_eq!(vec!["cn"], search_ids);

        drop(storage);
        cleanup_db(&db_path);
    }

    #[test]
    fn recent_clips_exclude_pinned_and_pinned_clips_keep_pin_order() {
        let db_path = temp_db_path("split_queries");
        let storage = ClipStorage::new(db_path.to_str().unwrap()).unwrap();

        storage
            .insert(&test_item("recent-old", b"old", 10, false, None), 100)
            .unwrap();
        storage
            .insert(
                &test_item("pinned-later", b"pin later", 40, true, Some(2)),
                100,
            )
            .unwrap();
        storage
            .insert(&test_item("recent-new", b"new", 30, false, None), 100)
            .unwrap();
        storage
            .insert(
                &test_item("pinned-first", b"pin first", 20, true, Some(1)),
                100,
            )
            .unwrap();

        let recent_ids: Vec<String> = storage
            .get_recent_clips(10)
            .unwrap()
            .into_iter()
            .map(|item| item.id)
            .collect();
        assert_eq!(vec!["recent-new", "recent-old"], recent_ids);

        let pinned_ids: Vec<String> = storage
            .get_pinned_clips()
            .unwrap()
            .into_iter()
            .map(|item| item.id)
            .collect();
        assert_eq!(vec!["pinned-first", "pinned-later"], pinned_ids);

        drop(storage);
        cleanup_db(&db_path);
    }
}
