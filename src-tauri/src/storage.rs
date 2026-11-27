use rusqlite::types::{FromSql, FromSqlError, FromSqlResult, ValueRef};
use rusqlite::{params, Connection, OptionalExtension, Result, Row};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::time::Duration;

pub fn current_timestamp() -> f64 {
    chrono::Utc::now().timestamp_micros() as f64 / 1_000_000.0
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ContentType {
    Text,
    Image,
    Files,
}

impl ContentType {
    fn as_db_value(&self) -> &str {
        match self {
            ContentType::Text => "text",
            ContentType::Image => "image",
            ContentType::Files => "files",
        }
    }
}

impl FromSql for ContentType {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        match value.as_str()? {
            "text" => Ok(ContentType::Text),
            "image" => Ok(ContentType::Image),
            "files" => Ok(ContentType::Files),
            other => Err(FromSqlError::Other(
                format!("unknown content type: {other}").into(),
            )),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CopyMarker {
    pub hash: String,
    pub content_type: ContentType,
}

impl CopyMarker {
    pub fn from_payload(content_type: ContentType, payload: &[u8]) -> Self {
        Self {
            hash: hash_bytes(payload),
            content_type,
        }
    }

    pub fn from_normalized_image_parts(width: usize, height: usize, rgba_bytes: &[u8]) -> Self {
        let mut hasher = Sha256::new();
        hasher.update((width as u64).to_le_bytes());
        hasher.update((height as u64).to_le_bytes());
        hasher.update(rgba_bytes);
        Self {
            hash: format!("{:x}", hasher.finalize()),
            content_type: ContentType::Image,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ClipItem {
    pub id: String,
    pub content: Vec<u8>,
    pub thumbnail: Option<Vec<u8>>,
    pub content_type: ContentType,
    pub timestamp: f64,
    pub is_pinned: bool,
    pub pin_order: Option<i32>,
    pub label: Option<String>,
    pub source_app: Option<String>,
    pub html: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ClipPreviewItem {
    pub id: String,
    pub preview_content: Vec<u8>,
    pub thumbnail: Option<Vec<u8>>,
    pub content_type: ContentType,
    pub timestamp: f64,
    pub is_pinned: bool,
    pub pin_order: Option<i32>,
    pub label: Option<String>,
    pub source_app: Option<String>,
    pub has_html: bool,
    pub content_bytes: usize,
    pub file_count: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FrontendClipItem {
    pub id: String,
    pub content: String,
    pub content_type: ContentType,
    pub timestamp: f64,
    pub is_pinned: bool,
    pub pin_order: Option<i32>,
    pub label: Option<String>,
    pub source_app: Option<String>,
    pub has_html: bool,
    pub content_bytes: usize,
    pub file_count: usize,
}

impl FrontendClipItem {
    pub fn from_preview(item: ClipPreviewItem) -> Self {
        use data_encoding::BASE64;

        let content = match item.content_type {
            ContentType::Image => item
                .thumbnail
                .as_deref()
                .map(png_data_url)
                .unwrap_or_default(),
            ContentType::Text | ContentType::Files => {
                let length = std::str::from_utf8(&item.preview_content)
                    .map_or_else(|error| error.valid_up_to(), |_| item.preview_content.len());
                BASE64.encode(&item.preview_content[..length])
            }
        };

        Self {
            id: item.id,
            content,
            content_type: item.content_type,
            timestamp: item.timestamp,
            is_pinned: item.is_pinned,
            pin_order: item.pin_order,
            label: item.label,
            source_app: item.source_app,
            has_html: item.has_html,
            content_bytes: item.content_bytes,
            file_count: item.file_count,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FrontendClipDetail {
    pub id: String,
    pub content_type: ContentType,
    pub text: String,
    pub image_url: Option<String>,
    pub truncated: bool,
}

pub struct ClipStorage {
    conn: Connection,
    data_dir: PathBuf,
}

const CLIP_COLUMNS: &str =
    "id, content, thumbnail, content_type, timestamp, is_pinned, pin_order, label, source_app, html";
const FILES_TEXT_SQL: &str = "CASE WHEN content_type='files'
    THEN CAST(COALESCE((SELECT group_concat(value, char(10)) FROM json_each(CAST(content AS TEXT))), '') AS BLOB) ELSE content END";
const TEXT_HEAD_PREVIEW: &str = "substr(content, 1, 4096)";
const TEXT_MATCH_EXCERPT: &str = "CAST(substr(CAST(content AS TEXT), max(1, instr(lower(CAST(content AS TEXT)), lower(?2)) - 64), 512) AS BLOB)";

fn fts_insert_sql(filter: &str) -> String {
    format!(
        "INSERT INTO clips_fts(rowid, clip_id, search_text, label)
     SELECT rowid, id,
       CASE WHEN content_type IN ('text','files') THEN CAST(({FILES_TEXT_SQL}) AS TEXT) ELSE '' END,
       label
     FROM clips {filter}"
    )
}

fn preview_columns(text_preview: &str) -> String {
    format!(
        "id,
     CASE WHEN content_type='text' THEN {text_preview}
       WHEN content_type='files' THEN CAST((SELECT json_group_array(substr(value, 1, 4096))
         FROM json_each(CAST(content AS TEXT)) WHERE key < 2) AS BLOB)
       ELSE x'' END AS preview_content,
     thumbnail, content_type, timestamp, is_pinned, pin_order, label, source_app,
     (html IS NOT NULL) AS has_html, length(content) AS content_bytes,
     CASE WHEN content_type='files' THEN json_array_length(CAST(content AS TEXT)) ELSE 0 END AS file_count"
    )
}

impl ClipStorage {
    pub fn new(db_path: &Path) -> Result<Self> {
        let conn = Connection::open(db_path)?;
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
            let backup_path = data_dir.join(format!("clipman.pre-v3-{}.db", uuid::Uuid::new_v4()));
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

    pub fn data_directory(&self) -> &Path {
        &self.data_dir
    }

    pub fn enforce_history_limit(&self, limit: usize) -> Result<usize> {
        let tx = self.conn.unchecked_transaction()?;
        let removed = Self::prune_history_with_conn(&tx, limit)?;
        tx.commit()?;
        if removed > 0 {
            self.reclaim_space();
        }
        Ok(removed)
    }

    fn ensure_incremental_auto_vacuum(conn: &Connection) -> Result<()> {
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

    fn reclaim_space(&self) {
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

    pub fn insert(&self, item: &ClipItem, max_history_items: usize) -> Result<Option<String>> {
        let tx = self.conn.unchecked_transaction()?;
        let result = Self::insert_with_conn(&tx, item, max_history_items)?;
        tx.commit()?;
        self.reclaim_space();
        Ok(result)
    }

    fn insert_with_conn(
        conn: &Connection,
        item: &ClipItem,
        max_history_items: usize,
    ) -> Result<Option<String>> {
        let content_hash = hash_bytes(&item.content);

        let existing_id: Option<String> = conn
            .query_row(
                "SELECT id FROM clips
                 WHERE content_hash = ?1 AND content_type = ?2
                 ORDER BY timestamp DESC
                 LIMIT 1",
                params![content_hash, item.content_type.as_db_value()],
                |row| row.get(0),
            )
            .optional()?;

        if let Some(id) = existing_id {
            log::debug!(
                "Duplicate content detected (hash: {}), updating timestamp",
                &content_hash[..8]
            );
            conn.execute(
                "UPDATE clips
                 SET timestamp = ?1,
                     html = COALESCE(?2, html),
                     source_app = COALESCE(?3, source_app)
                 WHERE id = ?4",
                params![item.timestamp, item.html, item.source_app, id],
            )?;
            Self::prune_history_with_conn(conn, max_history_items)?;
            return Ok(Some(id));
        }

        conn.execute(
            "INSERT INTO clips (
                id, content, thumbnail, content_hash, content_type, timestamp,
                is_pinned, pin_order, label, source_app, html
             )
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
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
                item.source_app,
                item.html,
            ],
        )?;

        Self::sync_fts_for_clip_id_with_conn(conn, &item.id)?;
        Self::prune_history_with_conn(conn, max_history_items)?;

        Ok(None)
    }

    pub fn get_recent_clip_previews(&self, limit: usize) -> Result<Vec<ClipPreviewItem>> {
        self.get_recent_clip_previews_filtered(limit, ClipFilter::All)
    }

    pub fn get_recent_clip_previews_filtered(
        &self,
        limit: usize,
        filter: ClipFilter,
    ) -> Result<Vec<ClipPreviewItem>> {
        let columns = preview_columns(TEXT_HEAD_PREVIEW);
        let filter_sql = filter.sql_clause();
        let mut stmt = self.conn.prepare(&format!(
            "SELECT {columns}
             FROM clips
             WHERE is_pinned = 0 AND ({filter_sql})
             ORDER BY timestamp DESC, id DESC
             LIMIT ?1"
        ))?;

        let items = stmt.query_map([limit], Self::preview_from_row)?;
        items.collect()
    }

    pub fn get_recent_clip_previews_page(
        &self,
        limit: usize,
        before: Option<(f64, &str)>,
        filter: ClipFilter,
    ) -> Result<Vec<ClipPreviewItem>> {
        let Some((before_timestamp, before_id)) = before else {
            return self.get_recent_clip_previews_filtered(limit, filter);
        };

        let columns = preview_columns(TEXT_HEAD_PREVIEW);
        let filter_sql = filter.sql_clause();
        let mut stmt = self.conn.prepare(&format!(
            "SELECT {columns}
             FROM clips
             WHERE is_pinned = 0
               AND ({filter_sql})
               AND (timestamp < ?1 OR (timestamp = ?1 AND id < ?2))
             ORDER BY timestamp DESC, id DESC
             LIMIT ?3"
        ))?;

        let items = stmt.query_map(
            params![before_timestamp, before_id, limit],
            Self::preview_from_row,
        )?;
        items.collect()
    }

    pub fn get_pinned_clip_previews(&self, limit: i64) -> Result<Vec<ClipPreviewItem>> {
        let columns = preview_columns(TEXT_HEAD_PREVIEW);
        let mut stmt = self.conn.prepare(&format!(
            "SELECT {columns}
             FROM clips
             WHERE is_pinned = 1
             ORDER BY pin_order IS NULL, pin_order ASC, timestamp DESC
             LIMIT ?1"
        ))?;

        let items = stmt.query_map([limit], Self::preview_from_row)?;
        items.collect()
    }

    pub fn search_clip_previews(&self, query: &str) -> Result<Vec<ClipPreviewItem>> {
        self.search_clip_previews_filtered(query, ClipFilter::All)
    }

    pub fn search_clip_previews_filtered(
        &self,
        query: &str,
        filter: ClipFilter,
    ) -> Result<Vec<ClipPreviewItem>> {
        log::debug!("Searching previews ({} characters)", query.chars().count());

        let query = query.trim();

        if query.chars().count() < 3 {
            return self.search_previews_with_like(query, filter);
        }

        self.search_previews_with_fts(query, filter)
    }

    pub fn backup_to_path(&self, destination_db_path: &Path) -> Result<()> {
        backup_connection(&self.conn, destination_db_path)
    }

    pub fn update_pin(&self, id: &str, is_pinned: bool, limit: usize) -> Result<()> {
        let tx = self.conn.unchecked_transaction()?;
        let pin_order = if is_pinned {
            let max_order: Option<i32> = tx.query_row(
                "SELECT MAX(pin_order) FROM clips WHERE is_pinned = 1",
                [],
                |row| row.get(0),
            )?;

            Some(max_order.unwrap_or(0) + 1)
        } else {
            None
        };

        tx.execute(
            "UPDATE clips SET is_pinned = ?1, pin_order = ?2 WHERE id = ?3",
            params![is_pinned as i32, pin_order, id],
        )?;
        Self::prune_history_with_conn(&tx, limit)?;
        tx.commit()
    }

    pub fn set_clip_label(&self, id: &str, label: Option<String>) -> Result<()> {
        let tx = self.conn.unchecked_transaction()?;
        tx.execute(
            "UPDATE clips SET label = ?1 WHERE id = ?2",
            params![normalize_label(label), id],
        )?;
        Self::sync_fts_for_clip_id_with_conn(&tx, id)?;
        tx.commit()
    }

    pub fn update_text_content(&self, id: &str, text: &str) -> std::result::Result<bool, String> {
        let bytes = text.as_bytes();
        let content_hash = hash_bytes(bytes);
        let tx = self
            .conn
            .unchecked_transaction()
            .map_err(|e| e.to_string())?;
        let existing: Option<(ContentType, usize)> = tx
            .query_row(
                "SELECT content_type, length(content) FROM clips WHERE id = ?1",
                params![id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()
            .map_err(|e| e.to_string())?;
        let Some((content_type, existing_bytes)) = existing else {
            return Ok(false);
        };
        if content_type != ContentType::Text {
            return Ok(false);
        }
        if existing_bytes > MAX_TEXT_DETAIL_BYTES {
            return Err("Clip exceeds the 1 MiB safe editing limit".to_string());
        }
        let updated = tx
            .execute(
                "UPDATE clips SET content = ?1, content_hash = ?2, html = NULL WHERE id = ?3 AND content_type = 'text'",
                params![bytes, content_hash, id],
            )
            .map_err(|e| e.to_string())?;
        if updated > 0 {
            Self::sync_fts_for_clip_id_with_conn(&tx, id).map_err(|e| e.to_string())?;
        }
        tx.commit().map_err(|e| e.to_string())?;
        Ok(updated > 0)
    }

    pub fn create_pinned_text(&self, text: &str, label: Option<String>) -> Result<ClipPreviewItem> {
        let id = uuid::Uuid::new_v4().to_string();
        let bytes = text.as_bytes();
        let content_hash = hash_bytes(bytes);
        let timestamp = current_timestamp();
        let label = normalize_label(label);
        let tx = self.conn.unchecked_transaction()?;
        let max_order: Option<i32> = tx.query_row(
            "SELECT MAX(pin_order) FROM clips WHERE is_pinned = 1",
            [],
            |row| row.get(0),
        )?;
        let pin_order = max_order.unwrap_or(0) + 1;
        tx.execute(
            "INSERT INTO clips (
                id, content, thumbnail, content_hash, content_type, timestamp,
                is_pinned, pin_order, label, source_app, html
             )
             VALUES (?1, ?2, NULL, ?3, 'text', ?4, 1, ?5, ?6, 'ClipLeaf', NULL)",
            params![id, bytes, content_hash, timestamp, pin_order, label],
        )?;
        Self::sync_fts_for_clip_id_with_conn(&tx, &id)?;
        tx.commit()?;
        self.get_preview_by_id(&id)?
            .ok_or(rusqlite::Error::QueryReturnedNoRows)
    }

    pub fn batch_update_pin(&self, ids: &[String], is_pinned: bool, limit: usize) -> Result<usize> {
        if ids.is_empty() {
            return Ok(0);
        }
        let tx = self.conn.unchecked_transaction()?;
        let mut next_order = if is_pinned {
            let max_order: Option<i32> = tx.query_row(
                "SELECT MAX(pin_order) FROM clips WHERE is_pinned = 1",
                [],
                |row| row.get(0),
            )?;
            max_order.unwrap_or(0) + 1
        } else {
            0
        };
        let mut changed_total = 0usize;
        for id in ids {
            let changed = if is_pinned {
                let updated = tx.execute(
                    "UPDATE clips SET is_pinned = 1, pin_order = ?1 WHERE id = ?2 AND is_pinned = 0",
                    params![next_order, id],
                )?;
                if updated > 0 {
                    next_order += 1;
                }
                updated
            } else {
                tx.execute(
                    "UPDATE clips SET is_pinned = 0, pin_order = NULL WHERE id = ?1 AND is_pinned = 1",
                    params![id],
                )?
            };
            changed_total += changed;
        }
        Self::prune_history_with_conn(&tx, limit)?;
        tx.commit()?;
        Ok(changed_total)
    }

    pub fn batch_delete(&self, ids: &[String]) -> Result<()> {
        if ids.is_empty() {
            return Ok(());
        }
        let tx = self.conn.unchecked_transaction()?;
        for id in ids {
            tx.execute("DELETE FROM clips_fts WHERE clip_id = ?1", params![id])?;
            tx.execute("DELETE FROM clips WHERE id = ?1", params![id])?;
        }
        tx.commit()?;
        self.reclaim_space();
        Ok(())
    }

    pub fn reorder_pinned(&self, id: &str, direction: &str) -> Result<()> {
        let move_up = match direction {
            "up" => true,
            "down" => false,
            _ => return Err(rusqlite::Error::InvalidParameterName(direction.to_string())),
        };

        let tx = self.conn.unchecked_transaction()?;
        let mut pinned_ids = {
            let mut stmt = tx.prepare(
                "SELECT id
                 FROM clips
                 WHERE is_pinned = 1
                 ORDER BY pin_order IS NULL, pin_order ASC, timestamp DESC",
            )?;
            let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
            rows.collect::<Result<Vec<_>>>()?
        };

        let Some(index) = pinned_ids.iter().position(|pinned_id| pinned_id == id) else {
            return Ok(());
        };

        let swap_index = if move_up {
            index.checked_sub(1)
        } else {
            (index + 1 < pinned_ids.len()).then_some(index + 1)
        };
        let Some(swap_index) = swap_index else {
            return Ok(());
        };

        pinned_ids.swap(index, swap_index);

        for (index, pinned_id) in pinned_ids.iter().enumerate() {
            tx.execute(
                "UPDATE clips SET pin_order = ?1 WHERE id = ?2",
                params![(index + 1) as i32, pinned_id],
            )?;
        }

        tx.commit()
    }

    pub fn delete(&self, id: &str) -> Result<()> {
        let tx = self.conn.unchecked_transaction()?;
        tx.execute("DELETE FROM clips_fts WHERE clip_id = ?1", params![id])?;
        tx.execute("DELETE FROM clips WHERE id = ?1", params![id])?;
        tx.commit()?;
        self.reclaim_space();
        Ok(())
    }

    pub fn clear_non_pinned(&self) -> Result<()> {
        log::info!("Clearing non-pinned clipboard history");
        let tx = self.conn.unchecked_transaction()?;
        tx.execute(
            "DELETE FROM clips_fts
             WHERE clip_id IN (SELECT id FROM clips WHERE is_pinned = 0)",
            [],
        )?;
        tx.execute("DELETE FROM clips WHERE is_pinned = 0", [])?;
        tx.commit()?;
        self.reclaim_space();
        Ok(())
    }

    pub fn get_by_id(&self, id: &str) -> Result<Option<ClipItem>> {
        let mut stmt = self.conn.prepare(&format!(
            "SELECT {CLIP_COLUMNS}
             FROM clips
             WHERE id = ?1"
        ))?;

        stmt.query_row([id], Self::clip_from_row).optional()
    }

    pub fn get_detail(&self, id: &str) -> std::result::Result<Option<FrontendClipDetail>, String> {
        const IMAGE_LIMIT: usize = 16 * 1024 * 1024;
        let result = self
            .conn
            .query_row(
                &format!("WITH payload AS (SELECT content_type, {FILES_TEXT_SQL} AS data FROM clips WHERE id=?1)
                 SELECT content_type, length(data), CASE WHEN content_type='image' THEN
                   CASE WHEN length(data)<=?3 THEN data ELSE x'' END ELSE substr(data,1,?2) END FROM payload"),
                params![id, MAX_TEXT_DETAIL_BYTES, IMAGE_LIMIT],
                |row| {
                    Ok((
                        row.get::<_, ContentType>(0)?,
                        row.get::<_, usize>(1)?,
                        row.get::<_, Vec<u8>>(2)?,
                    ))
                },
            )
            .optional()
            .map_err(|e| e.to_string())?;
        let Some((content_type, size, bytes)) = result else {
            return Ok(None);
        };
        let image = content_type == ContentType::Image;
        if image && size > IMAGE_LIMIT {
            return Err("Image exceeds the 16 MiB preview limit".into());
        }
        let truncated = !image && size > bytes.len();
        let end = if truncated {
            std::str::from_utf8(&bytes).map_or_else(|e| e.valid_up_to(), |_| bytes.len())
        } else {
            bytes.len()
        };
        Ok(Some(FrontendClipDetail {
            id: id.into(),
            content_type,
            truncated,
            text: if image {
                String::new()
            } else {
                String::from_utf8_lossy(&bytes[..end]).into_owned()
            },
            image_url: image.then(|| png_data_url(&bytes)),
        }))
    }

    pub fn get_for_merge(&self, id: &str, budget: usize) -> Result<Option<ClipItem>> {
        let size: Option<usize> = self.conn.query_row(
            "SELECT CASE WHEN content_type='image' THEN 0 ELSE length(content) END FROM clips WHERE id=?1", [id], |row| row.get(0)
        ).optional()?;
        if size.is_some_and(|size| size > budget) {
            return Err(string_to_rusqlite_error(
                "Merge exceeds the 50 MB content limit".into(),
            ));
        }
        self.conn
            .prepare(
                "SELECT id, CASE WHEN content_type = 'image' THEN x'' ELSE content END, NULL,
                        content_type, timestamp, is_pinned, pin_order, NULL, NULL, NULL
                 FROM clips WHERE id = ?1",
            )?
            .query_row([id], Self::clip_from_row)
            .optional()
    }

    pub fn get_preview_by_id(&self, id: &str) -> Result<Option<ClipPreviewItem>> {
        let columns = preview_columns(TEXT_HEAD_PREVIEW);
        let mut stmt = self.conn.prepare(&format!(
            "SELECT {columns}
             FROM clips
             WHERE id = ?1"
        ))?;

        stmt.query_row([id], Self::preview_from_row).optional()
    }

    pub fn touch_timestamps(&self, ids: &[String], new_timestamp: f64) -> Result<()> {
        let tx = self.conn.unchecked_transaction()?;
        {
            let mut stmt = tx.prepare("UPDATE clips SET timestamp = ?1 WHERE id = ?2")?;
            for id in ids {
                stmt.execute(params![new_timestamp, id])?;
            }
        }
        tx.commit()?;
        log::debug!("Touched {} item timestamp(s)", ids.len());
        Ok(())
    }

    pub fn save_app_icon(&self, name: &str, png: &[u8]) -> Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO app_icons (name, png) VALUES (?1, ?2)",
            params![name, png],
        )?;
        Ok(())
    }

    pub fn app_icon(&self, name: &str) -> Result<Option<String>> {
        self.conn
            .query_row(
                "SELECT png FROM app_icons WHERE name = ?1",
                params![name],
                |row| row.get::<_, Vec<u8>>(0),
            )
            .optional()
            .map(|png| png.as_deref().map(png_data_url))
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

    fn initialize_fts(conn: &Connection) -> Result<()> {
        conn.execute(
            "CREATE VIRTUAL TABLE IF NOT EXISTS clips_fts
             USING fts5(clip_id UNINDEXED, search_text, label, tokenize='trigram')",
            [],
        )?;
        Ok(())
    }

    fn fts_needs_rebuild(&self) -> Result<bool> {
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

    fn clip_from_row(row: &Row<'_>) -> Result<ClipItem> {
        Ok(ClipItem {
            id: row.get(0)?,
            content: row.get(1)?,
            thumbnail: row.get(2)?,
            content_type: row.get(3)?,
            timestamp: row.get(4)?,
            is_pinned: row.get::<_, i32>(5)? != 0,
            pin_order: row.get(6)?,
            label: row.get(7)?,
            source_app: row.get(8)?,
            html: row.get(9)?,
        })
    }

    fn preview_from_row(row: &Row<'_>) -> Result<ClipPreviewItem> {
        Ok(ClipPreviewItem {
            id: row.get(0)?,
            preview_content: row.get(1)?,
            thumbnail: row.get(2)?,
            content_type: row.get(3)?,
            timestamp: row.get(4)?,
            is_pinned: row.get::<_, i32>(5)? != 0,
            pin_order: row.get(6)?,
            label: row.get(7)?,
            source_app: row.get(8)?,
            has_html: row.get::<_, i32>(9)? != 0,
            content_bytes: row.get(10)?,
            file_count: row.get(11)?,
        })
    }

    fn search_previews_with_fts(
        &self,
        query: &str,
        filter: ClipFilter,
    ) -> Result<Vec<ClipPreviewItem>> {
        let fts_query = escape_fts_query(query);
        let columns = preview_columns(TEXT_MATCH_EXCERPT);
        let filter_sql = filter.sql_clause();
        let mut stmt = self.conn.prepare(&format!(
            "SELECT {columns}
             FROM clips
             WHERE rowid IN (
                SELECT rowid FROM clips WHERE (rowid, id) IN (
                    SELECT rowid, clip_id FROM clips_fts WHERE clips_fts MATCH ?1
                ) AND ({filter_sql}) ORDER BY timestamp DESC, id DESC LIMIT 1001
             )
             ORDER BY timestamp DESC, id DESC"
        ))?;

        let items = stmt.query_map(params![fts_query, query], Self::preview_from_row)?;
        items.collect()
    }

    fn search_previews_with_like(
        &self,
        query: &str,
        filter: ClipFilter,
    ) -> Result<Vec<ClipPreviewItem>> {
        let like_query = format!("%{}%", escape_like_query(query));
        let columns = preview_columns(TEXT_MATCH_EXCERPT);
        let filter_sql = filter.sql_clause();
        let mut stmt = self.conn.prepare(&format!(
            "SELECT {columns}
             FROM clips
             WHERE (
                (
                    content_type IN ('text','files')
                    AND CAST(({FILES_TEXT_SQL}) AS TEXT) LIKE ?1 ESCAPE '\\'
                )
                OR COALESCE(label, '') LIKE ?1 ESCAPE '\\'
             )
             AND ({filter_sql})
             ORDER BY timestamp DESC, id DESC
             LIMIT 1001"
        ))?;

        let items = stmt.query_map(params![like_query, query], Self::preview_from_row)?;
        items.collect()
    }

    fn rebuild_fts_index(&self) -> Result<()> {
        let tx = self.conn.unchecked_transaction()?;
        tx.execute("DELETE FROM clips_fts", [])?;
        tx.execute(&fts_insert_sql(""), [])?;
        tx.commit()
    }

    fn sync_fts_for_clip_id_with_conn(conn: &Connection, id: &str) -> Result<()> {
        conn.execute("DELETE FROM clips_fts WHERE clip_id = ?1", params![id])?;
        conn.execute(&fts_insert_sql("WHERE id = ?1"), params![id])?;
        Ok(())
    }

    fn prune_history_with_conn(conn: &Connection, max_history_items: usize) -> Result<usize> {
        conn.execute(
            "DELETE FROM clips_fts
             WHERE clip_id IN (
                SELECT id FROM clips
                WHERE is_pinned = 0
                ORDER BY timestamp DESC, id DESC
                LIMIT -1 OFFSET ?1
             )",
            params![max_history_items],
        )?;
        let deleted = conn.execute(
            "DELETE FROM clips
             WHERE id IN (
                SELECT id FROM clips
                WHERE is_pinned = 0
                ORDER BY timestamp DESC, id DESC
                LIMIT -1 OFFSET ?1
             )",
            params![max_history_items],
        )?;
        Ok(deleted)
    }
}

pub const MAX_TEXT_DETAIL_BYTES: usize = 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClipFilter {
    All,
    Text,
    Image,
    Files,
    Link,
}

impl ClipFilter {
    pub fn from_optional(value: Option<&str>) -> std::result::Result<Self, String> {
        match value {
            None | Some("all") => Ok(Self::All),
            Some("text") => Ok(Self::Text),
            Some("image") => Ok(Self::Image),
            Some("files") => Ok(Self::Files),
            Some("link") => Ok(Self::Link),
            Some(other) => Err(format!("Invalid clip filter '{other}'")),
        }
    }

    fn sql_clause(self) -> &'static str {
        match self {
            Self::All => "1 = 1",
            Self::Text => "content_type = 'text'",
            Self::Image => "content_type = 'image'",
            Self::Files => "content_type = 'files'",
            Self::Link => {
                "content_type = 'text' \
                 AND (lower(trim(CAST(content AS TEXT), char(9) || char(10) || char(13) || ' ')) LIKE 'http://%' \
                      OR lower(trim(CAST(content AS TEXT), char(9) || char(10) || char(13) || ' ')) LIKE 'https://%') \
                 AND lower(trim(CAST(content AS TEXT), char(9) || char(10) || char(13) || ' ')) NOT IN ('http://', 'https://') \
                 AND trim(CAST(content AS TEXT), char(9) || char(10) || char(13) || ' ') NOT GLOB ('*[' || char(9) || char(10) || char(13) || ' ]*')"
            }
        }
    }
}

pub(crate) fn hash_bytes(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

pub fn encode_file_paths(paths: &[String]) -> String {
    serde_json::to_string(paths).expect("a list of strings is JSON serializable")
}

pub fn join_file_paths(paths: &[String]) -> String {
    paths.join("\n")
}

pub fn split_file_paths(content: &str) -> Vec<String> {
    serde_json::from_str(content)
        .expect("files clips store a JSON array of paths (legacy rows are converted on open)")
}

fn png_data_url(bytes: &[u8]) -> String {
    format!(
        "data:image/png;base64,{}",
        data_encoding::BASE64.encode(bytes)
    )
}

fn normalize_label(label: Option<String>) -> Option<String> {
    label
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
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

fn string_to_rusqlite_error(error: String) -> rusqlite::Error {
    rusqlite::Error::ToSqlConversionFailure(Box::new(std::io::Error::other(error)))
}

fn io_to_rusqlite_error(error: std::io::Error) -> rusqlite::Error {
    rusqlite::Error::ToSqlConversionFailure(Box::new(error))
}

fn backup_connection(conn: &Connection, destination_db_path: &Path) -> Result<()> {
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

fn remove_sqlite_database_files(path: &Path) -> Result<()> {
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
    use std::fs;
    use std::path::PathBuf;
    use uuid::Uuid;

    fn temp_db_path(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("clipman_{}_{}.db", name, Uuid::new_v4()))
    }

    fn cleanup_db(path: &Path) {
        let _ = remove_sqlite_database_files(path);
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
            timestamp: timestamp as f64,
            is_pinned,
            pin_order,
            label: None,
            source_app: None,
            html: None,
        }
    }

    #[test]
    fn history_limit_applies_to_recopies_unpins_and_delayed_images() {
        let root = std::env::temp_dir().join(format!("clipman-limit-{}", Uuid::new_v4()));
        fs::create_dir(&root).unwrap();
        let storage = ClipStorage::new(&root.join("clipman.db")).unwrap();
        for i in 0..3 {
            storage
                .insert(&test_item(&i.to_string(), &[i as u8], i, false, None), 3)
                .unwrap();
        }
        storage
            .insert(&test_item("recopy", &[2], 4, false, None), 1)
            .unwrap();
        assert_eq!(storage.get_recent_clip_previews(10).unwrap().len(), 1);
        storage
            .insert(&test_item("pin", b"pin", 1, true, Some(1)), 1)
            .unwrap();
        storage.update_pin("pin", false, 1).unwrap();
        assert!(storage.get_by_id("pin").unwrap().is_none());
        let old_image = ClipItem {
            content_type: ContentType::Image,
            ..test_item("late", b"image", 0, false, None)
        };
        storage.insert(&old_image, 1).unwrap();
        assert!(storage.get_preview_by_id("late").unwrap().is_none());
        assert_eq!(storage.enforce_history_limit(0).unwrap(), 1);
        assert!(storage.search_clip_previews("").unwrap().is_empty());
        drop(storage);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn v3_upgrade_backs_up_then_converts_files_once_and_rejects_newer_formats() {
        let root = std::env::temp_dir().join(format!("clipman-upgrade-{}", Uuid::new_v4()));
        fs::create_dir(&root).unwrap();
        let path = root.join("clipman.db");
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
                    .starts_with("clipman.pre-v3-")
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
                    .starts_with("clipman.pre-v3-"))
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
    fn json_file_paths_and_bounded_merge_reads_preserve_content() {
        let root = std::env::temp_dir().join(format!("clipman-paths-{}", Uuid::new_v4()));
        fs::create_dir(&root).unwrap();
        let storage = ClipStorage::new(&root.join("clipman.db")).unwrap();
        let paths = vec![
            "中".repeat(4097),
            "/tmp/one\ntwo.txt".to_string(),
            "/tmp/quote\".txt".into(),
            "/tmp/third.txt".into(),
        ];
        let payload = encode_file_paths(&paths);
        assert_eq!(split_file_paths(&payload), paths);
        let item = ClipItem {
            content_type: ContentType::Files,
            ..test_item("files", payload.as_bytes(), 1, false, None)
        };
        storage.insert(&item, 100).unwrap();
        let preview = storage.get_preview_by_id("files").unwrap().unwrap();
        assert_eq!(preview.file_count, 4);
        assert!(!preview.has_html);
        assert_eq!(
            split_file_paths(std::str::from_utf8(&preview.preview_content).unwrap()),
            vec!["中".repeat(4096), paths[1].clone()]
        );
        assert_eq!(
            storage.get_detail("files").unwrap().unwrap().text,
            join_file_paths(&paths)
        );
        assert!(!storage.get_detail("files").unwrap().unwrap().truncated);
        assert_eq!(storage.search_clip_previews("two").unwrap().len(), 1);
        assert_eq!(storage.search_clip_previews("tw").unwrap().len(), 1);
        assert!(storage.get_for_merge("files", 1).is_err());
        assert_eq!(
            storage
                .get_for_merge("files", payload.len())
                .unwrap()
                .unwrap()
                .content,
            payload.as_bytes()
        );
        drop(storage);
        fs::remove_dir_all(root).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn active_data_directory_follows_the_opened_database_symlink() {
        let root = std::env::temp_dir().join(format!("clipman-active-{}", Uuid::new_v4()));
        fs::create_dir(&root).unwrap();
        let actual = root.join("actual");
        fs::create_dir(&actual).unwrap();
        drop(ClipStorage::new(&actual.join("clipman.db")).unwrap());
        let alias = root.join("alias.db");
        std::os::unix::fs::symlink(actual.join("clipman.db"), &alias).unwrap();
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

    #[test]
    fn recent_clips_exclude_pinned_and_pinned_clips_keep_pin_order() {
        let db_path = temp_db_path("split_queries");
        let storage = ClipStorage::new(&db_path).unwrap();

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
            .get_recent_clip_previews(10)
            .unwrap()
            .into_iter()
            .map(|item| item.id)
            .collect();
        assert_eq!(vec!["recent-new", "recent-old"], recent_ids);

        let pinned_ids: Vec<String> = storage
            .get_pinned_clip_previews(-1)
            .unwrap()
            .into_iter()
            .map(|item| item.id)
            .collect();
        assert_eq!(vec!["pinned-first", "pinned-later"], pinned_ids);
        assert_eq!(storage.get_pinned_clip_previews(1).unwrap().len(), 1);

        drop(storage);
        cleanup_db(&db_path);
    }

    #[test]
    fn touch_timestamps_bumps_all_given_ids_in_one_transaction() {
        let db_path = temp_db_path("touch_batch");
        let storage = ClipStorage::new(&db_path).unwrap();

        storage
            .insert(&test_item("a", b"a", 10, false, None), 100)
            .unwrap();
        storage
            .insert(&test_item("b", b"b", 20, false, None), 100)
            .unwrap();
        storage
            .insert(&test_item("c", b"c", 30, false, None), 100)
            .unwrap();

        storage
            .touch_timestamps(
                &["a".to_string(), "b".to_string(), "missing".to_string()],
                99.0,
            )
            .unwrap();

        assert_eq!(storage.get_by_id("a").unwrap().unwrap().timestamp, 99.0);
        assert_eq!(storage.get_by_id("b").unwrap().unwrap().timestamp, 99.0);
        assert_eq!(storage.get_by_id("c").unwrap().unwrap().timestamp, 30.0);

        let recent_ids: Vec<String> = storage
            .get_recent_clip_previews(10)
            .unwrap()
            .into_iter()
            .map(|item| item.id)
            .collect();
        assert_eq!(vec!["b", "a", "c"], recent_ids);

        drop(storage);
        cleanup_db(&db_path);
    }

    #[test]
    fn set_clip_label_trims_empty_labels_and_updates_search_index() {
        let db_path = temp_db_path("label_update");
        let storage = ClipStorage::new(&db_path).unwrap();

        storage
            .insert(&test_item("clip", b"body text", 1, true, Some(1)), 100)
            .unwrap();

        storage
            .set_clip_label("clip", Some("  work email  ".to_string()))
            .unwrap();

        let item = storage.get_by_id("clip").unwrap().unwrap();
        assert_eq!(Some("work email".to_string()), item.label);

        let search_ids: Vec<String> = storage
            .search_clip_previews("email")
            .unwrap()
            .into_iter()
            .map(|item| item.id)
            .collect();
        assert_eq!(vec!["clip"], search_ids);

        storage
            .set_clip_label("clip", Some("   ".to_string()))
            .unwrap();

        let item = storage.get_by_id("clip").unwrap().unwrap();
        assert_eq!(None, item.label);

        storage.delete("clip").unwrap();
        let fts_count: i64 = storage
            .conn
            .query_row("SELECT COUNT(*) FROM clips_fts", [], |row| row.get(0))
            .unwrap();
        assert_eq!(0, fts_count);

        drop(storage);
        cleanup_db(&db_path);
    }

    #[test]
    fn reorder_pinned_swaps_adjacent_items_and_renumbers_slots() {
        let db_path = temp_db_path("reorder_pinned");
        let storage = ClipStorage::new(&db_path).unwrap();

        storage
            .insert(&test_item("first", b"first", 30, true, Some(10)), 100)
            .unwrap();
        storage
            .insert(&test_item("second", b"second", 20, true, Some(20)), 100)
            .unwrap();
        storage
            .insert(&test_item("third", b"third", 10, true, Some(30)), 100)
            .unwrap();

        storage.reorder_pinned("second", "up").unwrap();

        let pinned: Vec<(String, Option<i32>)> = storage
            .get_pinned_clip_previews(-1)
            .unwrap()
            .into_iter()
            .map(|item| (item.id, item.pin_order))
            .collect();
        assert_eq!(
            vec![
                ("second".to_string(), Some(1)),
                ("first".to_string(), Some(2)),
                ("third".to_string(), Some(3)),
            ],
            pinned
        );

        storage.reorder_pinned("second", "down").unwrap();

        let pinned_ids: Vec<String> = storage
            .get_pinned_clip_previews(-1)
            .unwrap()
            .into_iter()
            .map(|item| item.id)
            .collect();
        assert_eq!(vec!["first", "second", "third"], pinned_ids);

        drop(storage);
        cleanup_db(&db_path);
    }

    #[test]
    fn prune_history_removes_stale_clips_and_fts_rows() {
        let db_path = temp_db_path("prune_atomicity");
        let storage = ClipStorage::new(&db_path).unwrap();

        for index in 0..5 {
            let item = test_item(
                &format!("clip-{index}"),
                format!("clip content {index}").as_bytes(),
                index,
                false,
                None,
            );
            storage.insert(&item, 3).unwrap();
        }

        let clip_count: i64 = storage
            .conn
            .query_row(
                "SELECT COUNT(*) FROM clips WHERE is_pinned = 0",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let orphan_fts_count: i64 = storage
            .conn
            .query_row(
                "SELECT COUNT(*)
                 FROM clips_fts
                 LEFT JOIN clips ON clips_fts.clip_id = clips.id
                 WHERE clips.id IS NULL",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(3, clip_count);
        assert_eq!(0, orphan_fts_count);
        drop(storage);
        cleanup_db(&db_path);
    }

    #[test]
    fn clear_non_pinned_removes_matching_fts_rows() {
        let db_path = temp_db_path("clear_non_pinned_atomicity");
        let storage = ClipStorage::new(&db_path).unwrap();

        storage
            .insert(&test_item("recent", b"recent text", 1, false, None), 100)
            .unwrap();
        storage
            .insert(&test_item("pinned", b"pinned text", 2, true, Some(1)), 100)
            .unwrap();
        storage.clear_non_pinned().unwrap();

        let remaining_clip_ids: Vec<String> = storage
            .conn
            .prepare("SELECT clip_id FROM clips_fts ORDER BY clip_id")
            .unwrap()
            .query_map([], |row| row.get::<_, String>(0))
            .unwrap()
            .collect::<Result<Vec<_>>>()
            .unwrap();

        assert_eq!(vec!["pinned".to_string()], remaining_clip_ids);
        drop(storage);
        cleanup_db(&db_path);
    }

    #[test]
    fn fractional_seconds_preserve_capture_recopy_and_paste_order_with_legacy_rows() {
        let path = temp_db_path("subsecond_order");
        let storage = ClipStorage::new(&path).unwrap();
        storage
            .insert(&test_item("legacy", b"legacy", 100, false, None), 100)
            .unwrap();
        let mut first = test_item("z-first", b"first", 100, false, None);
        first.timestamp = 100.1;
        let mut next = test_item("a-next", b"next", 100, false, None);
        next.timestamp = 100.2;
        storage.insert(&first, 100).unwrap();
        storage.insert(&next, 100).unwrap();
        let page = storage
            .get_recent_clip_previews_page(1, None, ClipFilter::All)
            .unwrap();
        assert_eq!(page[0].id, "a-next");
        let rest = storage
            .get_recent_clip_previews_page(
                10,
                Some((page[0].timestamp, &page[0].id)),
                ClipFilter::All,
            )
            .unwrap();
        assert_eq!(
            rest.iter().map(|i| i.id.as_str()).collect::<Vec<_>>(),
            ["z-first", "legacy"]
        );
        first.timestamp = 100.3;
        storage.insert(&first, 100).unwrap();
        assert_eq!(
            storage.get_recent_clip_previews(1).unwrap()[0].id,
            "z-first"
        );
        storage.touch_timestamps(&[next.id], 100.4).unwrap();
        assert_eq!(storage.get_recent_clip_previews(1).unwrap()[0].id, "a-next");
        drop(storage);
        let reopened = ClipStorage::new(&path).unwrap();
        assert_eq!(
            reopened.get_by_id("legacy").unwrap().unwrap().timestamp,
            100.0
        );
        assert_eq!(
            reopened.get_by_id("a-next").unwrap().unwrap().timestamp,
            100.4
        );
        drop(reopened);
        cleanup_db(&path);
    }

    #[test]
    fn keyset_pagination_covers_duplicate_timestamps_without_loss_or_repeat() {
        let db_path = temp_db_path("keyset_duplicate_timestamps");
        let storage = ClipStorage::new(&db_path).unwrap();

        for id in ["a", "b", "c"] {
            storage
                .insert(&test_item(id, id.as_bytes(), 5, false, None), 100)
                .unwrap();
        }
        storage
            .insert(&test_item("older", b"older", 1, false, None), 100)
            .unwrap();
        storage
            .insert(&test_item("newer", b"newer", 9, false, None), 100)
            .unwrap();

        let page_size = 2;
        let mut collected: Vec<String> = Vec::new();
        let mut cursor: Option<(f64, String)> = None;
        loop {
            let before = cursor.as_ref().map(|(ts, id)| (*ts, id.as_str()));
            let page = storage
                .get_recent_clip_previews_page(page_size, before, ClipFilter::All)
                .unwrap();
            let Some(last) = page.last() else {
                break;
            };
            let short_page = page.len() < page_size;
            cursor = Some((last.timestamp, last.id.clone()));
            collected.extend(page.into_iter().map(|item| item.id));
            if short_page {
                break;
            }
        }

        assert_eq!(vec!["newer", "c", "b", "a", "older"], collected);

        drop(storage);
        cleanup_db(&db_path);
    }

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

    #[test]
    fn recent_clip_previews_do_not_return_full_payloads() {
        let db_path = temp_db_path("recent_previews");
        let storage = ClipStorage::new(&db_path).unwrap();

        let long_text = vec![b'a'; 5000];
        storage
            .insert(
                &ClipItem {
                    content: long_text.clone(),
                    ..test_item("text", b"", 1, false, None)
                },
                100,
            )
            .unwrap();
        storage
            .insert(
                &ClipItem {
                    id: "image".to_string(),
                    content: b"full image payload".to_vec(),
                    thumbnail: Some(b"thumbnail".to_vec()),
                    content_type: ContentType::Image,
                    timestamp: 2.0,
                    is_pinned: false,
                    pin_order: None,
                    label: None,
                    source_app: None,
                    html: None,
                },
                100,
            )
            .unwrap();

        let previews = storage.get_recent_clip_previews(10).unwrap();
        let text_preview = previews.iter().find(|item| item.id == "text").unwrap();
        let image_preview = previews.iter().find(|item| item.id == "image").unwrap();

        assert_eq!(4096, text_preview.preview_content.len());
        assert_eq!(&long_text[..4096], text_preview.preview_content.as_slice());
        assert!(image_preview.preview_content.is_empty());
        assert_eq!(Some(b"thumbnail".to_vec()), image_preview.thumbnail);

        drop(storage);
        cleanup_db(&db_path);
    }

    #[test]
    #[ignore]
    fn search_latency_baseline() {
        let path = temp_db_path("search_latency");
        let storage = ClipStorage::new(&path).unwrap();
        let mut previous = 0;
        for total in [100, 1000, 10_000] {
            let tx = storage.conn.unchecked_transaction().unwrap();
            for count in previous + 1..=total {
                let mut item = test_item(&format!("clip-{count}"), b"", count, false, None);
                item.content_type = match count % 10 {
                    0 => ContentType::Image,
                    1 => ContentType::Files,
                    _ => ContentType::Text,
                };
                item.content = match item.content_type {
                    ContentType::Image => vec![0; 1024],
                    ContentType::Files => {
                        encode_file_paths(&[format!("/tmp/report-{count}.txt")]).into_bytes()
                    }
                    ContentType::Text => format!(
                        "{}needle-{count}中文",
                        "x".repeat(if count % 20 == 2 { 65536 } else { 256 })
                    )
                    .into_bytes(),
                };
                ClipStorage::insert_with_conn(&tx, &item, 10_000).unwrap();
            }
            tx.commit().unwrap();
            previous = total;
            let count = total;
            for query in ["中", "zz", "needle-99", "missing", "needle"] {
                let mut samples = Vec::new();
                for _ in 0..20 {
                    let start = std::time::Instant::now();
                    let result = storage.search_clip_previews(query).unwrap();
                    assert!(result.len() <= 1001);
                    samples.push(start.elapsed().as_secs_f64() * 1000.0);
                }
                samples.sort_by(f64::total_cmp);
                println!(
                    "rows={count} query={query:?} p50={:.2}ms p95={:.2}ms",
                    samples[10], samples[18]
                );
            }
        }
        drop(storage);
        cleanup_db(&path);
    }

    #[test]
    fn merge_read_omits_original_images_but_keeps_text() {
        let path = temp_db_path("merge_read");
        let storage = ClipStorage::new(&path).unwrap();
        let image = ClipItem {
            content_type: ContentType::Image,
            ..test_item("image", &vec![7; 1_000_000], 1, false, None)
        };
        storage.insert(&image, 100).unwrap();
        storage
            .insert(&test_item("text", b"complete text", 2, false, None), 100)
            .unwrap();
        assert!(storage
            .get_for_merge("image", 100)
            .unwrap()
            .unwrap()
            .content
            .is_empty());
        assert_eq!(
            storage.get_for_merge("text", 100).unwrap().unwrap().content,
            b"complete text"
        );
        drop(storage);
        cleanup_db(&path);
    }

    #[test]
    fn preview_encoding_does_not_split_utf8() {
        let db_path = temp_db_path("preview_utf8");
        let storage = ClipStorage::new(&db_path).unwrap();
        let item = test_item("unicode", "中".repeat(2000).as_bytes(), 1, false, None);
        storage.insert(&item, 100).unwrap();
        let preview =
            FrontendClipItem::from_preview(storage.get_preview_by_id("unicode").unwrap().unwrap());
        let bytes = data_encoding::BASE64
            .decode(preview.content.as_bytes())
            .unwrap();
        assert!(std::str::from_utf8(&bytes).is_ok());
        assert_eq!(bytes.len(), 4095);
        assert_eq!(preview.content_bytes, 6000);
        drop(storage);
        cleanup_db(&db_path);
    }

    #[test]
    fn search_returns_excerpt_around_a_late_match() {
        let path = temp_db_path("late_excerpt");
        let storage = ClipStorage::new(&path).unwrap();
        let content = format!("{}needle中", "a".repeat(5000));
        storage
            .insert(&test_item("late", content.as_bytes(), 1, false, None), 100)
            .unwrap();
        for query in ["needle", "中"] {
            let result = storage.search_clip_previews(query).unwrap();
            assert!(String::from_utf8_lossy(&result[0].preview_content).contains(query));
            assert!(result[0].preview_content.len() <= 2048);
            assert_eq!(result[0].content_bytes, content.len());
        }
        drop(storage);
        cleanup_db(&path);
    }

    #[test]
    fn detail_bounds_payloads_without_changing_stored_content() {
        let path = temp_db_path("detail_bounds");
        let storage = ClipStorage::new(&path).unwrap();
        let text = "中".repeat(400_000);
        storage
            .insert(&test_item("text", text.as_bytes(), 1, false, None), 100)
            .unwrap();
        let detail = storage.get_detail("text").unwrap().unwrap();
        assert!(detail.truncated);
        assert_eq!(detail.text.len(), 1024 * 1024 - 1);
        assert!(text.starts_with(&detail.text));
        assert_eq!(
            storage.get_by_id("text").unwrap().unwrap().content,
            text.as_bytes()
        );
        storage
            .insert(&test_item("small", b"complete", 2, false, None), 100)
            .unwrap();
        assert!(!storage.get_detail("small").unwrap().unwrap().truncated);
        assert_eq!(
            storage.get_detail("small").unwrap().unwrap().text,
            "complete"
        );
        let image = ClipItem {
            content_type: ContentType::Image,
            ..test_item("image", b"image bytes", 3, false, None)
        };
        storage.insert(&image, 100).unwrap();
        assert!(storage
            .get_detail("image")
            .unwrap()
            .unwrap()
            .image_url
            .is_some());
        storage
            .conn
            .execute(
                "UPDATE clips SET content = zeroblob(?1) WHERE id = 'image'",
                [16 * 1024 * 1024 + 1],
            )
            .unwrap();
        assert!(storage.get_detail("image").unwrap_err().contains("16 MiB"));
        assert!(storage.get_detail("missing").unwrap().is_none());
        drop(storage);
        cleanup_db(&path);
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

    #[test]
    fn image_marker_includes_dimensions_and_bytes() {
        let bytes = [255, 0, 0, 255];
        let marker_a = CopyMarker::from_normalized_image_parts(1, 1, &bytes);
        let marker_b = CopyMarker::from_normalized_image_parts(2, 1, &bytes);
        let marker_c = CopyMarker::from_normalized_image_parts(1, 1, &[0, 0, 0, 255]);

        assert_ne!(marker_a, marker_b);
        assert_ne!(marker_a, marker_c);
    }

    #[test]
    fn duplicate_insert_preserves_metadata_and_coalesces_html() {
        let db_path = temp_db_path("duplicate_metadata");
        let storage = ClipStorage::new(&db_path).unwrap();

        storage
            .insert(
                &ClipItem {
                    html: Some("<b>hello</b>".to_string()),
                    source_app: Some("Browser".to_string()),
                    label: Some("favorite".to_string()),
                    ..test_item("rich", b"hello", 1, true, Some(2))
                },
                100,
            )
            .unwrap();
        let stored = storage.get_by_id("rich").unwrap().unwrap();
        assert_eq!(Some("<b>hello</b>".to_string()), stored.html);
        assert_eq!(Some("Browser".to_string()), stored.source_app);
        assert!(storage.get_preview_by_id("rich").unwrap().unwrap().has_html);

        let dup_id = storage
            .insert(&test_item("plain-dup", b"hello", 2, false, None), 100)
            .unwrap();
        assert_eq!(Some("rich".to_string()), dup_id);
        assert!(storage.get_by_id("plain-dup").unwrap().is_none());
        let after_plain = storage.get_by_id("rich").unwrap().unwrap();
        assert_eq!(Some("<b>hello</b>".to_string()), after_plain.html);
        assert_eq!(Some("Browser".to_string()), after_plain.source_app);
        assert_eq!(2.0, after_plain.timestamp);
        assert!(after_plain.is_pinned);
        assert_eq!(Some(2), after_plain.pin_order);
        assert_eq!(Some("favorite".to_string()), after_plain.label);

        storage
            .insert(
                &ClipItem {
                    html: Some("<i>hello</i>".to_string()),
                    source_app: Some("Editor".to_string()),
                    ..test_item("rich-again", b"hello", 3, false, None)
                },
                100,
            )
            .unwrap();
        let after_rich = storage.get_by_id("rich").unwrap().unwrap();
        assert_eq!(Some("<i>hello</i>".to_string()), after_rich.html);
        assert_eq!(Some("Editor".to_string()), after_rich.source_app);

        drop(storage);
        cleanup_db(&db_path);
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

    #[test]
    fn update_text_content_and_create_pinned_text_sync_fts() {
        let db_path = temp_db_path("edit_and_create_pinned");
        let storage = ClipStorage::new(&db_path).unwrap();

        let created = storage
            .create_pinned_text("initial prompt template", Some("Prompt".into()))
            .unwrap();
        assert!(created.is_pinned);
        assert_eq!(created.label.as_deref(), Some("Prompt"));
        assert_eq!(storage.search_clip_previews("template").unwrap().len(), 1);

        let updated = storage
            .update_text_content(&created.id, "updated reusable snippet")
            .unwrap();
        assert!(updated);
        assert!(storage.search_clip_previews("template").unwrap().is_empty());
        assert_eq!(storage.search_clip_previews("reusable").unwrap().len(), 1);

        let detail = storage.get_detail(&created.id).unwrap().unwrap();
        assert_eq!(detail.text, "updated reusable snippet");

        drop(storage);
        cleanup_db(&db_path);
    }

    #[test]
    fn batch_pin_and_batch_delete_are_atomic() {
        let db_path = temp_db_path("batch_ops");
        let storage = ClipStorage::new(&db_path).unwrap();

        storage
            .insert(&test_item("a", b"alpha", 1, false, None), 100)
            .unwrap();
        storage
            .insert(&test_item("b", b"beta", 2, false, None), 100)
            .unwrap();
        storage
            .insert(&test_item("c", b"gamma", 3, false, None), 100)
            .unwrap();

        storage
            .batch_update_pin(&["a".into(), "b".into()], true, 100)
            .unwrap();
        let pinned = storage.get_pinned_clip_previews(-1).unwrap();
        assert_eq!(pinned.len(), 2);

        storage.batch_delete(&["a".into(), "c".into()]).unwrap();
        assert!(storage.get_by_id("a").unwrap().is_none());
        assert!(storage.get_by_id("b").unwrap().is_some());
        assert!(storage.get_by_id("c").unwrap().is_none());

        drop(storage);
        cleanup_db(&db_path);
    }

    #[test]
    fn update_text_content_rejects_non_text_and_oversized_clips() {
        let db_path = temp_db_path("edit_guards");
        let storage = ClipStorage::new(&db_path).unwrap();

        let img = ClipItem {
            content_type: ContentType::Image,
            ..test_item("img-1", b"png-bytes", 1, false, None)
        };
        storage.insert(&img, 100).unwrap();
        assert!(!storage.update_text_content("img-1", "text").unwrap());

        let oversized = vec![b'a'; MAX_TEXT_DETAIL_BYTES + 16];
        storage
            .insert(&test_item("big-1", &oversized, 2, false, None), 100)
            .unwrap();
        let err = storage
            .update_text_content("big-1", "truncated draft")
            .unwrap_err();
        assert!(err.contains("1 MiB"));
        assert_eq!(
            storage.get_by_id("big-1").unwrap().unwrap().content.len(),
            MAX_TEXT_DETAIL_BYTES + 16
        );

        drop(storage);
        cleanup_db(&db_path);
    }

    #[test]
    fn batch_pin_preserves_existing_order_and_unpin_enforces_history_limit() {
        let db_path = temp_db_path("batch_pin_order_and_limit");
        let storage = ClipStorage::new(&db_path).unwrap();

        storage
            .insert(&test_item("p1", b"one", 1, true, Some(1)), 100)
            .unwrap();
        storage
            .insert(&test_item("p2", b"two", 2, true, Some(2)), 100)
            .unwrap();
        storage
            .insert(&test_item("r1", b"three", 3, false, None), 100)
            .unwrap();
        storage
            .insert(&test_item("r2", b"four", 4, false, None), 100)
            .unwrap();

        let changed = storage
            .batch_update_pin(&["p1".into(), "r1".into()], true, 100)
            .unwrap();
        assert_eq!(changed, 1);
        let pinned_ids: Vec<_> = storage
            .get_pinned_clip_previews(-1)
            .unwrap()
            .into_iter()
            .map(|c| c.id)
            .collect();
        assert_eq!(pinned_ids, vec!["p1", "p2", "r1"]);

        let unpinned = storage
            .batch_update_pin(&["p1".into(), "p2".into(), "r1".into()], false, 2)
            .unwrap();
        assert_eq!(unpinned, 3);
        let recent_ids: Vec<_> = storage
            .get_recent_clip_previews(10)
            .unwrap()
            .into_iter()
            .map(|c| c.id)
            .collect();
        assert_eq!(recent_ids, vec!["r2", "r1"]);

        drop(storage);
        cleanup_db(&db_path);
    }

    #[test]
    fn sql_filters_recent_pagination_and_search_by_category() {
        let db_path = temp_db_path("sql_filters");
        let storage = ClipStorage::new(&db_path).unwrap();

        storage
            .insert(&test_item("t1", b"plain note alpha", 1, false, None), 100)
            .unwrap();
        storage
            .insert(
                &test_item(
                    "url1",
                    b"  https://example.com/alpha?x=1&y=2\n",
                    2,
                    false,
                    None,
                ),
                100,
            )
            .unwrap();
        storage
            .insert(
                &test_item(
                    "url-multi",
                    b"https://example.com extra words",
                    3,
                    false,
                    None,
                ),
                100,
            )
            .unwrap();
        storage
            .insert(&test_item("url-bare", b"https://", 4, false, None), 100)
            .unwrap();
        storage
            .insert(
                &test_item("url2", b"HTTPS://CLIPMAN.TOP/alpha", 5, false, None),
                100,
            )
            .unwrap();
        storage
            .insert(
                &ClipItem {
                    content_type: ContentType::Image,
                    label: Some("alpha shot".into()),
                    ..test_item("img1", b"png", 6, false, None)
                },
                100,
            )
            .unwrap();
        storage
            .insert(
                &ClipItem {
                    content_type: ContentType::Files,
                    ..test_item(
                        "file1",
                        encode_file_paths(&["/tmp/alpha.pdf".into()]).as_bytes(),
                        7,
                        false,
                        None,
                    )
                },
                100,
            )
            .unwrap();

        let page1 = storage
            .get_recent_clip_previews_page(1, None, ClipFilter::Link)
            .unwrap();
        assert_eq!(page1.len(), 1);
        assert_eq!(page1[0].id, "url2");
        let page2 = storage
            .get_recent_clip_previews_page(
                10,
                Some((page1[0].timestamp, &page1[0].id)),
                ClipFilter::Link,
            )
            .unwrap();
        assert_eq!(
            page2.iter().map(|i| i.id.as_str()).collect::<Vec<_>>(),
            vec!["url1"]
        );

        assert_eq!(
            storage
                .get_recent_clip_previews_filtered(10, ClipFilter::Image)
                .unwrap()
                .iter()
                .map(|i| i.id.as_str())
                .collect::<Vec<_>>(),
            vec!["img1"]
        );
        assert_eq!(
            storage
                .get_recent_clip_previews_filtered(10, ClipFilter::Files)
                .unwrap()
                .iter()
                .map(|i| i.id.as_str())
                .collect::<Vec<_>>(),
            vec!["file1"]
        );

        let fts_links = storage
            .search_clip_previews_filtered("alpha", ClipFilter::Link)
            .unwrap();
        assert_eq!(
            fts_links.iter().map(|i| i.id.as_str()).collect::<Vec<_>>(),
            vec!["url2", "url1"]
        );
        let like_images = storage
            .search_clip_previews_filtered("al", ClipFilter::Image)
            .unwrap();
        assert_eq!(
            like_images
                .iter()
                .map(|i| i.id.as_str())
                .collect::<Vec<_>>(),
            vec!["img1"]
        );

        drop(storage);
        cleanup_db(&db_path);
    }
}
