use rusqlite::{params, Connection, OptionalExtension, Result, Row};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
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

    fn from_db_value(value: &str) -> Self {
        match value {
            "image" => ContentType::Image,
            "files" => ContentType::Files,
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
    /// App that was frontmost when the clip was captured (the copy source).
    pub source_app: Option<String>,
    /// Optional HTML companion to a Text clip's plain-text `content` (D2).
    pub html: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClipPreviewItem {
    pub id: String,
    pub preview_content: Vec<u8>,
    pub thumbnail: Option<Vec<u8>>,
    pub content_type: ContentType,
    pub timestamp: i64,
    pub is_pinned: bool,
    pub pin_order: Option<i32>,
    pub label: Option<String>,
    pub group_name: Option<String>,
    pub source_app: Option<String>,
    pub has_html: bool,
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
    pub source_app: Option<String>,
    pub has_html: bool,
}

impl From<ClipItem> for FrontendClipItem {
    fn from(item: ClipItem) -> Self {
        use data_encoding::BASE64;

        // Files, like Text, ship their path text as base64; only images render
        // as data URLs from their thumbnail/content bytes.
        let content = match item.content_type {
            ContentType::Image => {
                let image_bytes = item.thumbnail.as_deref().unwrap_or(&item.content);
                format!("data:image/png;base64,{}", BASE64.encode(image_bytes))
            }
            _ => BASE64.encode(&item.content),
        };
        let has_html = item.html.is_some();

        FrontendClipItem {
            id: item.id,
            content,
            content_type: item.content_type,
            timestamp: item.timestamp,
            is_pinned: item.is_pinned,
            pin_order: item.pin_order,
            label: item.label,
            group_name: item.group_name,
            source_app: item.source_app,
            has_html,
        }
    }
}

impl ClipPreviewItem {
    pub fn from_clip_item(item: &ClipItem) -> Self {
        Self::from_clip_item_with_id(item, item.id.clone())
    }

    pub fn from_clip_item_with_id(item: &ClipItem, id: String) -> Self {
        let preview_content = match item.content_type {
            ContentType::Text | ContentType::Files => item
                .content
                .iter()
                .take(TEXT_PREVIEW_BYTES)
                .copied()
                .collect(),
            ContentType::Image => Vec::new(),
        };

        Self {
            id,
            preview_content,
            thumbnail: item.thumbnail.clone(),
            content_type: item.content_type.clone(),
            timestamp: item.timestamp,
            is_pinned: item.is_pinned,
            pin_order: item.pin_order,
            label: item.label.clone(),
            group_name: item.group_name.clone(),
            source_app: item.source_app.clone(),
            has_html: item.html.is_some(),
        }
    }
}

impl FrontendClipItem {
    pub fn from_preview(item: ClipPreviewItem) -> Self {
        use data_encoding::BASE64;

        let content = match item.content_type {
            ContentType::Image => item
                .thumbnail
                .as_deref()
                .map(|bytes| format!("data:image/png;base64,{}", BASE64.encode(bytes)))
                .unwrap_or_default(),
            ContentType::Text | ContentType::Files => BASE64.encode(&item.preview_content),
        };

        Self {
            id: item.id,
            content,
            content_type: item.content_type,
            timestamp: item.timestamp,
            is_pinned: item.is_pinned,
            pin_order: item.pin_order,
            label: item.label,
            group_name: item.group_name,
            source_app: item.source_app,
            has_html: item.has_html,
        }
    }

    /// Full-fidelity text mapping for the QuickBar preview pane. Text and Files
    /// return their complete payload (Files so the pane can show the full path
    /// list); images are intentionally rejected here: the UI reuses thumbnails
    /// for images so full image payloads do not cross the JSON IPC boundary.
    pub fn from_full_text(item: ClipItem) -> Option<Self> {
        use data_encoding::BASE64;

        if !matches!(item.content_type, ContentType::Text | ContentType::Files) {
            return None;
        }

        let content = BASE64.encode(&item.content);
        let has_html = item.html.is_some();

        Some(Self {
            id: item.id,
            content,
            content_type: item.content_type,
            timestamp: item.timestamp,
            is_pinned: item.is_pinned,
            pin_order: item.pin_order,
            label: item.label,
            group_name: item.group_name,
            source_app: item.source_app,
            has_html,
        })
    }
}

pub struct ClipStorage {
    conn: Connection,
}

const CLIP_COLUMNS: &str =
    "id, content, thumbnail, content_type, timestamp, is_pinned, pin_order, label, group_name, source_app, html";
const CLIP_COLUMNS_WITH_ALIAS: &str = "c.id, c.content, c.thumbnail, c.content_type, c.timestamp, c.is_pinned, c.pin_order, c.label, c.group_name, c.source_app, c.html";
const CLIP_PREVIEW_COLUMNS: &str = "id,
     CASE WHEN content_type IN ('text','files') THEN substr(content, 1, 4096) ELSE x'' END AS preview_content,
     thumbnail,
     content_type, timestamp, is_pinned, pin_order, label, group_name, source_app,
     (html IS NOT NULL) AS has_html";
const CLIP_PREVIEW_COLUMNS_WITH_ALIAS: &str =
    "c.id,
     CASE WHEN c.content_type IN ('text','files') THEN substr(c.content, 1, 4096) ELSE x'' END AS preview_content,
     c.thumbnail,
     c.content_type, c.timestamp, c.is_pinned, c.pin_order, c.label, c.group_name, c.source_app,
     (c.html IS NOT NULL) AS has_html";
const FTS_REBUILD_BATCH_SIZE: i64 = 100;
const TEXT_PREVIEW_BYTES: usize = 4096;

impl ClipStorage {
    pub fn new(db_path: &Path) -> Result<Self> {
        let conn = Connection::open(db_path)?;
        conn.pragma_update(None, "journal_mode", "WAL")?;

        Self::initialize_schema(&conn)?;

        let data_dir = data_dir_for_db_path(db_path);
        let upgrade = crate::migration::upgrade_clip_database_to_current(&conn, &data_dir)
            .map_err(string_to_rusqlite_error)?;

        Self::initialize_fts(&conn)?;
        Self::ensure_incremental_auto_vacuum(&conn)?;

        let storage = Self { conn };
        if upgrade.needs_fts_rebuild || storage.fts_needs_rebuild()? {
            storage.rebuild_fts_index()?;
            if upgrade.needs_fts_rebuild {
                crate::migration::mark_clip_database_current(&storage.conn)
                    .map_err(string_to_rusqlite_error)?;
            }
        }
        Ok(storage)
    }

    /// One-time migration to incremental auto_vacuum (§4). `auto_vacuum`
    /// only takes effect after a `VACUUM`, so a database created before this
    /// change (or by an older ClipMan version) pays a one-time full rewrite
    /// here; every later open sees `auto_vacuum = INCREMENTAL` already and
    /// skips straight past this check.
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
            "🧹 Migrated database to incremental auto_vacuum in {:?}",
            start.elapsed()
        );
        Ok(())
    }

    /// Best-effort reclaim of pages freed by a delete/prune (§4). SQLite
    /// forbids running `incremental_vacuum` inside a transaction, so every
    /// caller must invoke this only *after* its own transaction has
    /// committed. Failures are only logged: reclaiming disk space must never
    /// turn an otherwise-successful delete/prune into a caller-visible error.
    fn reclaim_space(&self) {
        // `PRAGMA incremental_vacuum` yields a row per freed batch, so it must
        // be stepped as a query and drained; `execute()` bails with "Execute
        // returned results" after the first freed page and leaves the rest of
        // the freelist unreclaimed.
        let result = self
            .conn
            .prepare("PRAGMA incremental_vacuum")
            .and_then(|mut stmt| {
                let mut rows = stmt.query([])?;
                while rows.next()?.is_some() {}
                Ok(())
            });
        if let Err(error) = result {
            log::warn!("⚠️ Failed to reclaim database space: {}", error);
        }
    }

    pub fn insert(&self, item: &ClipItem, max_history_items: usize) -> Result<Option<String>> {
        let tx = self.conn.unchecked_transaction()?;
        let result = Self::insert_with_conn(&tx, item, max_history_items)?;
        tx.commit()?;
        // insert_with_conn may have pruned stale history above max_history_items;
        // reclaim those freed pages now that the transaction is closed (§4).
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
                "⏭️ Duplicate content detected (hash: {}), updating timestamp",
                &content_hash[..8]
            );
            Self::refresh_duplicate_with_conn(conn, &id, item.timestamp, item.html.as_deref())?;
            return Ok(Some(id));
        }

        conn.execute(
            "INSERT INTO clips (
                id, content, thumbnail, content_hash, content_type, timestamp,
                is_pinned, pin_order, label, group_name, source_app, html
             )
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
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
                item.source_app,
                item.html,
            ],
        )?;

        Self::sync_fts_for_clip_id_with_conn(conn, &item.id)?;
        Self::prune_history_with_conn(conn, max_history_items)?;

        Ok(None)
    }

    pub fn get_recent_clips(&self, limit: usize) -> Result<Vec<ClipItem>> {
        let mut stmt = self.conn.prepare(&format!(
            "SELECT {CLIP_COLUMNS}
             FROM clips
             WHERE is_pinned = 0
             ORDER BY timestamp DESC, id DESC
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

    pub fn get_recent_clip_previews(&self, limit: usize) -> Result<Vec<ClipPreviewItem>> {
        let mut stmt = self.conn.prepare(&format!(
            "SELECT {CLIP_PREVIEW_COLUMNS}
             FROM clips
             WHERE is_pinned = 0
             ORDER BY timestamp DESC, id DESC
             LIMIT ?1"
        ))?;

        let items = stmt.query_map([limit], Self::preview_from_row)?;
        items.collect()
    }

    /// Keyset-paginated recent previews (§1). `before` is the `(timestamp, id)`
    /// cursor of the last row the caller already holds; `None` returns the
    /// first page. The strict `id` tiebreak keeps paging stable even when many
    /// rows share a timestamp, where a timestamp-only cursor would drop rows
    /// straddling a page boundary or repeat them on the next page.
    pub fn get_recent_clip_previews_page(
        &self,
        limit: usize,
        before: Option<(i64, &str)>,
    ) -> Result<Vec<ClipPreviewItem>> {
        let Some((before_timestamp, before_id)) = before else {
            return self.get_recent_clip_previews(limit);
        };

        let mut stmt = self.conn.prepare(&format!(
            "SELECT {CLIP_PREVIEW_COLUMNS}
             FROM clips
             WHERE is_pinned = 0
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

    pub fn get_pinned_clip_previews(&self) -> Result<Vec<ClipPreviewItem>> {
        let mut stmt = self.conn.prepare(&format!(
            "SELECT {CLIP_PREVIEW_COLUMNS}
             FROM clips
             WHERE is_pinned = 1
             ORDER BY pin_order IS NULL, pin_order ASC, timestamp DESC"
        ))?;

        let items = stmt.query_map([], Self::preview_from_row)?;
        items.collect()
    }

    pub fn get_pinned_clip_previews_with_limit(
        &self,
        limit: usize,
    ) -> Result<Vec<ClipPreviewItem>> {
        let mut stmt = self.conn.prepare(&format!(
            "SELECT {CLIP_PREVIEW_COLUMNS}
             FROM clips
             WHERE is_pinned = 1
             ORDER BY pin_order IS NULL, pin_order ASC, timestamp DESC
             LIMIT ?1"
        ))?;

        let items = stmt.query_map([limit], Self::preview_from_row)?;
        items.collect()
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

    pub fn search_clip_previews(&self, query: &str) -> Result<Vec<ClipPreviewItem>> {
        log::info!("🔍 Searching previews for: {}", query);

        let query = query.trim();
        if query.is_empty() {
            return self.get_all_previews_for_search();
        }

        if query.chars().count() < 3 {
            return self.search_previews_with_like(query);
        }

        self.search_previews_with_fts(query)
    }

    pub fn backup_to_path(&self, destination_db_path: &Path) -> Result<()> {
        if let Some(parent) = destination_db_path.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent).map_err(io_to_rusqlite_error)?;
            }
        }

        let temp_db_path = temp_backup_path(destination_db_path);
        remove_sqlite_database_files(&temp_db_path)?;

        let backup_result = (|| -> Result<()> {
            let mut destination = Connection::open(&temp_db_path)?;
            let backup = rusqlite::backup::Backup::new(&self.conn, &mut destination)?;
            backup.run_to_completion(5, Duration::from_millis(25), None)?;
            Ok(())
        })();

        if let Err(error) = backup_result {
            if let Err(cleanup_error) = remove_sqlite_database_files(&temp_db_path) {
                log::warn!(
                    "Failed to clean incomplete backup {}: {}",
                    temp_db_path.display(),
                    cleanup_error
                );
            }
            return Err(error);
        }

        let mut staged_destination = stage_sqlite_files_for_replacement(destination_db_path)?;
        if let Err(error) = fs::rename(&temp_db_path, destination_db_path) {
            staged_destination.restore();
            if let Err(cleanup_error) = remove_sqlite_database_files(&temp_db_path) {
                log::warn!(
                    "Failed to clean unused backup {}: {}",
                    temp_db_path.display(),
                    cleanup_error
                );
            }
            return Err(io_to_rusqlite_error(error));
        }
        if let Err(cleanup_error) = remove_sqlite_sidecars(&temp_db_path) {
            log::warn!(
                "Failed to clean temporary backup sidecars for {}: {}",
                temp_db_path.display(),
                cleanup_error
            );
        }
        if let Err(cleanup_error) = staged_destination.cleanup() {
            log::warn!(
                "Failed to clean replaced destination backup for {}: {}",
                destination_db_path.display(),
                cleanup_error
            );
        }

        Ok(())
    }

    pub fn update_pin(&self, id: &str, is_pinned: bool) -> Result<()> {
        let tx = self.conn.unchecked_transaction()?;
        Self::update_pin_with_conn(&tx, id, is_pinned)?;
        tx.commit()
    }

    fn update_pin_with_conn(conn: &Connection, id: &str, is_pinned: bool) -> Result<()> {
        let pin_order = if is_pinned {
            let max_order: Option<i32> = conn
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

        conn.execute(
            "UPDATE clips SET is_pinned = ?1, pin_order = ?2 WHERE id = ?3",
            params![is_pinned as i32, pin_order, id],
        )?;

        Ok(())
    }

    pub fn set_clip_label(&self, id: &str, label: Option<String>) -> Result<()> {
        let tx = self.conn.unchecked_transaction()?;
        Self::set_clip_label_with_conn(&tx, id, label)?;
        tx.commit()
    }

    fn set_clip_label_with_conn(conn: &Connection, id: &str, label: Option<String>) -> Result<()> {
        let label = normalize_label(label);

        conn.execute(
            "UPDATE clips SET label = ?1 WHERE id = ?2",
            params![label, id],
        )?;
        Self::sync_fts_for_clip_id_with_conn(conn, id)?;
        Ok(())
    }

    pub fn reorder_pinned(&self, id: &str, direction: &str) -> Result<()> {
        let tx = self.conn.unchecked_transaction()?;
        Self::reorder_pinned_with_conn(&tx, id, direction)?;
        tx.commit()
    }

    fn reorder_pinned_with_conn(conn: &Connection, id: &str, direction: &str) -> Result<()> {
        let move_up = match direction {
            "up" => true,
            "down" => false,
            _ => return Err(rusqlite::Error::InvalidParameterName(direction.to_string())),
        };

        let mut pinned_ids = {
            let mut stmt = conn.prepare(
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
            if index == 0 {
                return Ok(());
            }
            index - 1
        } else {
            if index + 1 >= pinned_ids.len() {
                return Ok(());
            }
            index + 1
        };

        pinned_ids.swap(index, swap_index);

        for (index, pinned_id) in pinned_ids.iter().enumerate() {
            conn.execute(
                "UPDATE clips SET pin_order = ?1 WHERE id = ?2",
                params![(index + 1) as i32, pinned_id],
            )?;
        }

        Ok(())
    }

    pub fn delete(&self, id: &str) -> Result<()> {
        let tx = self.conn.unchecked_transaction()?;
        Self::delete_with_conn(&tx, id)?;
        tx.commit()?;
        self.reclaim_space();
        Ok(())
    }

    fn delete_with_conn(conn: &Connection, id: &str) -> Result<()> {
        conn.execute("DELETE FROM clips_fts WHERE clip_id = ?1", params![id])?;
        conn.execute("DELETE FROM clips WHERE id = ?1", params![id])?;
        Ok(())
    }

    pub fn clear_all(&self) -> Result<()> {
        let tx = self.conn.unchecked_transaction()?;
        Self::clear_all_with_conn(&tx)?;
        tx.commit()?;
        self.reclaim_space();
        Ok(())
    }

    fn clear_all_with_conn(conn: &Connection) -> Result<()> {
        log::info!("🗑️ Clearing all clipboard history");
        conn.execute("DELETE FROM clips_fts", [])?;
        conn.execute("DELETE FROM clips", [])?;
        Ok(())
    }

    pub fn clear_non_pinned(&self) -> Result<()> {
        let tx = self.conn.unchecked_transaction()?;
        Self::clear_non_pinned_with_conn(&tx)?;
        tx.commit()?;
        self.reclaim_space();
        Ok(())
    }

    fn clear_non_pinned_with_conn(conn: &Connection) -> Result<()> {
        log::info!("🗑️ Clearing non-pinned clipboard history");
        conn.execute(
            "DELETE FROM clips_fts
             WHERE clip_id IN (SELECT id FROM clips WHERE is_pinned = 0)",
            [],
        )?;
        conn.execute("DELETE FROM clips WHERE is_pinned = 0", [])?;
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

    pub fn get_preview_by_id(&self, id: &str) -> Result<Option<ClipPreviewItem>> {
        let mut stmt = self.conn.prepare(&format!(
            "SELECT {CLIP_PREVIEW_COLUMNS}
             FROM clips
             WHERE id = ?1"
        ))?;

        stmt.query_row([id], Self::preview_from_row).optional()
    }

    /// Update the timestamp of a clip item (move it to the top of recent list)
    pub fn update_timestamp(&self, id: &str, new_timestamp: i64) -> Result<()> {
        Self::update_timestamp_with_conn(&self.conn, id, new_timestamp)
    }

    fn update_timestamp_with_conn(conn: &Connection, id: &str, new_timestamp: i64) -> Result<()> {
        conn.execute(
            "UPDATE clips SET timestamp = ?1 WHERE id = ?2",
            params![new_timestamp, id],
        )?;
        log::debug!("📍 Updated timestamp for item {}", id);
        Ok(())
    }

    /// Refresh a duplicate clip on re-copy: bump its timestamp and let the most
    /// recent rich copy win, while a plain-text re-copy (html = None) keeps the
    /// previously captured html via COALESCE (D6).
    fn refresh_duplicate_with_conn(
        conn: &Connection,
        id: &str,
        new_timestamp: i64,
        html: Option<&str>,
    ) -> Result<()> {
        conn.execute(
            "UPDATE clips SET timestamp = ?1, html = COALESCE(?2, html) WHERE id = ?3",
            params![new_timestamp, html, id],
        )?;
        log::debug!("📍 Refreshed duplicate item {}", id);
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
                group_name TEXT,
                source_app TEXT,
                html TEXT
            )",
            [],
        )?;

        Self::add_column_if_missing(conn, "content_hash", "TEXT")?;
        Self::add_column_if_missing(conn, "thumbnail", "BLOB")?;
        Self::add_column_if_missing(conn, "label", "TEXT")?;
        Self::add_column_if_missing(conn, "group_name", "TEXT")?;
        Self::add_column_if_missing(conn, "source_app", "TEXT")?;
        Self::add_column_if_missing(conn, "html", "TEXT")?;

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
        // Keyset pagination (§1) orders the recent list by (timestamp DESC,
        // id DESC) so duplicate timestamps can't drop or repeat rows across
        // pages; the index must carry the id tiebreak or SQLite falls back to
        // a temp sort. The old 2-column index is dropped — the 3-column index
        // is a strict prefix superset for every query that used it.
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
        let missing_fts_rows: i64 = self.conn.query_row(
            "SELECT COUNT(*)
             FROM clips c
             LEFT JOIN clips_fts f ON f.rowid = c.rowid AND f.clip_id = c.id
             WHERE f.rowid IS NULL",
            [],
            |row| row.get(0),
        )?;
        let orphan_fts_rows: i64 = self.conn.query_row(
            "SELECT COUNT(*)
             FROM clips_fts f
             LEFT JOIN clips c ON f.rowid = c.rowid AND f.clip_id = c.id
             WHERE c.rowid IS NULL",
            [],
            |row| row.get(0),
        )?;
        Ok(missing_fts_rows > 0 || orphan_fts_rows > 0)
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
            source_app: row.get(9)?,
            html: row.get(10)?,
        })
    }

    fn preview_from_row(row: &Row<'_>) -> Result<ClipPreviewItem> {
        Ok(ClipPreviewItem {
            id: row.get(0)?,
            preview_content: row.get(1)?,
            thumbnail: row.get(2)?,
            content_type: ContentType::from_db_value(&row.get::<_, String>(3)?),
            timestamp: row.get(4)?,
            is_pinned: row.get::<_, i32>(5)? != 0,
            pin_order: row.get(6)?,
            label: row.get(7)?,
            group_name: row.get(8)?,
            source_app: row.get(9)?,
            has_html: row.get::<_, i32>(10)? != 0,
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

    fn get_all_previews_for_search(&self) -> Result<Vec<ClipPreviewItem>> {
        let mut stmt = self.conn.prepare(&format!(
            "SELECT {CLIP_PREVIEW_COLUMNS}
             FROM clips
             ORDER BY timestamp DESC
             LIMIT 1000"
        ))?;

        let items = stmt.query_map([], Self::preview_from_row)?;
        items.collect()
    }

    fn search_with_fts(&self, query: &str) -> Result<Vec<ClipItem>> {
        let fts_query = escape_fts_query(query);
        let mut stmt = self.conn.prepare(&format!(
            "SELECT {CLIP_COLUMNS_WITH_ALIAS}
             FROM clips c
             JOIN clips_fts ON clips_fts.rowid = c.rowid AND clips_fts.clip_id = c.id
             WHERE clips_fts MATCH ?1
             ORDER BY c.timestamp DESC
             LIMIT 1000"
        ))?;

        let items = stmt.query_map([fts_query], Self::clip_from_row)?;
        items.collect()
    }

    fn search_previews_with_fts(&self, query: &str) -> Result<Vec<ClipPreviewItem>> {
        let fts_query = escape_fts_query(query);
        let mut stmt = self.conn.prepare(&format!(
            "SELECT {CLIP_PREVIEW_COLUMNS_WITH_ALIAS}
             FROM clips c
             JOIN clips_fts ON clips_fts.rowid = c.rowid AND clips_fts.clip_id = c.id
             WHERE clips_fts MATCH ?1
             ORDER BY c.timestamp DESC
             LIMIT 1000"
        ))?;

        let items = stmt.query_map([fts_query], Self::preview_from_row)?;
        items.collect()
    }

    fn search_with_like(&self, query: &str) -> Result<Vec<ClipItem>> {
        let like_query = format!("%{}%", escape_like_query(query));
        let mut stmt = self.conn.prepare(&format!(
            "SELECT {CLIP_COLUMNS}
             FROM clips
             WHERE (
                content_type IN ('text','files')
                AND CAST(substr(content, 1, {TEXT_PREVIEW_BYTES}) AS TEXT) LIKE ?1 ESCAPE '\\'
             )
             OR COALESCE(label, '') LIKE ?1 ESCAPE '\\'
             ORDER BY timestamp DESC
             LIMIT 1000"
        ))?;

        let items = stmt.query_map([like_query], Self::clip_from_row)?;
        items.collect()
    }

    fn search_previews_with_like(&self, query: &str) -> Result<Vec<ClipPreviewItem>> {
        let like_query = format!("%{}%", escape_like_query(query));
        let mut stmt = self.conn.prepare(&format!(
            "SELECT {CLIP_PREVIEW_COLUMNS}
             FROM clips
             WHERE (
                content_type IN ('text','files')
                AND CAST(substr(content, 1, {TEXT_PREVIEW_BYTES}) AS TEXT) LIKE ?1 ESCAPE '\\'
             )
             OR COALESCE(label, '') LIKE ?1 ESCAPE '\\'
             ORDER BY timestamp DESC
             LIMIT 1000"
        ))?;

        let items = stmt.query_map([like_query], Self::preview_from_row)?;
        items.collect()
    }

    fn rebuild_fts_index(&self) -> Result<()> {
        let tx = self.conn.unchecked_transaction()?;
        Self::rebuild_fts_index_with_conn(&tx)?;
        tx.commit()
    }

    fn rebuild_fts_index_with_conn(conn: &Connection) -> Result<()> {
        conn.execute("DELETE FROM clips_fts", [])?;

        let mut last_rowid = 0;
        loop {
            let rows = {
                let mut stmt = conn.prepare(
                    "SELECT rowid, id
                     FROM clips
                     WHERE rowid > ?1
                     ORDER BY rowid ASC
                     LIMIT ?2",
                )?;
                let rows = stmt.query_map(params![last_rowid, FTS_REBUILD_BATCH_SIZE], |row| {
                    Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
                })?;
                rows.collect::<Result<Vec<_>>>()?
            };

            let Some((batch_last_rowid, _)) = rows.last() else {
                break;
            };
            last_rowid = *batch_last_rowid;

            for (rowid, clip_id) in rows {
                if let Some(payload) = Self::fts_payload_for_rowid_with_conn(conn, rowid, &clip_id)?
                {
                    Self::insert_fts_payload_with_conn(conn, &payload)?;
                }
            }
        }

        Ok(())
    }

    fn fts_payload_for_rowid_with_conn(
        conn: &Connection,
        rowid: i64,
        id: &str,
    ) -> Result<Option<FtsPayload>> {
        conn.query_row(
            "SELECT rowid, id,
                CASE WHEN content_type IN ('text','files') THEN content ELSE x'' END AS search_content,
                content_type, label
             FROM clips
             WHERE rowid = ?1 AND id = ?2",
            params![rowid, id],
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
        .optional()
    }

    fn sync_fts_for_clip_id_with_conn(conn: &Connection, id: &str) -> Result<()> {
        conn.execute("DELETE FROM clips_fts WHERE clip_id = ?1", params![id])?;

        let payload = conn
            .query_row(
                "SELECT rowid, id,
                    CASE WHEN content_type IN ('text','files') THEN content ELSE x'' END AS search_content,
                    content_type, label
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
            Self::insert_fts_payload_with_conn(conn, &payload)?;
        }

        Ok(())
    }

    fn insert_fts_payload_with_conn(conn: &Connection, payload: &FtsPayload) -> Result<()> {
        let search_text = search_text_for_fts(&payload.content, &payload.content_type);
        conn.execute(
            "INSERT INTO clips_fts(rowid, clip_id, search_text, label)
             VALUES (?1, ?2, ?3, ?4)",
            params![payload.rowid, payload.clip_id, search_text, payload.label,],
        )?;
        Ok(())
    }

    fn prune_history_with_conn(conn: &Connection, max_history_items: usize) -> Result<()> {
        conn.execute(
            "DELETE FROM clips_fts
             WHERE clip_id IN (
                SELECT id FROM clips
                WHERE is_pinned = 0
                ORDER BY timestamp DESC
                LIMIT -1 OFFSET ?1
             )",
            params![max_history_items],
        )?;
        conn.execute(
            "DELETE FROM clips
             WHERE id IN (
                SELECT id FROM clips
                WHERE is_pinned = 0
                ORDER BY timestamp DESC
                LIMIT -1 OFFSET ?1
             )",
            params![max_history_items],
        )?;
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

/// Canonical on-disk representation of a Files clip: absolute paths joined by
/// newlines (D3). Newlines are illegal in a single path entry, so this is a
/// lossless, FTS-friendly encoding.
pub fn join_file_paths(paths: &[String]) -> String {
    paths.join("\n")
}

/// Inverse of [`join_file_paths`]: split newline-joined paths, dropping blank
/// lines so a trailing newline or accidental gap never yields an empty path.
pub fn split_file_paths(content: &str) -> Vec<String> {
    content
        .split('\n')
        .filter(|line| !line.is_empty())
        .map(|line| line.to_string())
        .collect()
}

fn search_text_for_fts(content: &[u8], content_type: &ContentType) -> String {
    match content_type {
        // Files store their paths as newline-joined text, so they index and
        // search exactly like text (search by path).
        ContentType::Text | ContentType::Files => String::from_utf8_lossy(content).into_owned(),
        ContentType::Image => String::new(),
    }
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

fn data_dir_for_db_path(db_path: &Path) -> PathBuf {
    db_path
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."))
}

fn string_to_rusqlite_error(error: String) -> rusqlite::Error {
    rusqlite::Error::ToSqlConversionFailure(Box::new(std::io::Error::other(error)))
}

fn io_to_rusqlite_error(error: std::io::Error) -> rusqlite::Error {
    rusqlite::Error::ToSqlConversionFailure(Box::new(error))
}

fn temp_backup_path(destination_db_path: &Path) -> PathBuf {
    let file_name = destination_db_path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("clipman.db");
    destination_db_path.with_file_name(format!(".{file_name}.{}.tmp", uuid::Uuid::new_v4()))
}

fn replaced_backup_path(destination_db_path: &Path) -> PathBuf {
    let file_name = destination_db_path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("clipman.db");
    destination_db_path.with_file_name(format!(".{file_name}.{}.replaced", uuid::Uuid::new_v4()))
}

fn sqlite_sidecar_path(path: &Path, suffix: &str) -> PathBuf {
    PathBuf::from(format!("{}{}", path.display(), suffix))
}

fn remove_file_if_exists(path: &Path) -> Result<()> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(()),
        Err(error) => Err(io_to_rusqlite_error(error)),
    }
}

fn remove_sqlite_sidecars(path: &Path) -> Result<()> {
    for suffix in ["-wal", "-shm", "-journal"] {
        remove_file_if_exists(&sqlite_sidecar_path(path, suffix))?;
    }
    Ok(())
}

fn remove_sqlite_database_files(path: &Path) -> Result<()> {
    remove_sqlite_sidecars(path)?;
    remove_file_if_exists(path)
}

fn move_file_if_exists(from: &Path, to: &Path) -> Result<bool> {
    match fs::rename(from, to) {
        Ok(()) => Ok(true),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(false),
        Err(error) => Err(io_to_rusqlite_error(error)),
    }
}

struct StagedSqliteReplacement {
    moved_files: Vec<(PathBuf, PathBuf)>,
}

impl StagedSqliteReplacement {
    fn restore(&mut self) {
        for (staged_path, original_path) in self.moved_files.iter().rev() {
            if let Err(error) = fs::rename(staged_path, original_path) {
                log::warn!(
                    "Failed to restore replaced database file {} to {}: {}",
                    staged_path.display(),
                    original_path.display(),
                    error
                );
            }
        }
        self.moved_files.clear();
    }

    fn cleanup(mut self) -> Result<()> {
        for (staged_path, _) in &self.moved_files {
            remove_file_if_exists(staged_path)?;
        }
        self.moved_files.clear();
        Ok(())
    }
}

fn stage_sqlite_files_for_replacement(path: &Path) -> Result<StagedSqliteReplacement> {
    let staged_db_path = replaced_backup_path(path);
    remove_sqlite_database_files(&staged_db_path)?;

    let mut staged = StagedSqliteReplacement {
        moved_files: Vec::new(),
    };

    for suffix in ["-wal", "-shm", "-journal"] {
        let original_path = sqlite_sidecar_path(path, suffix);
        let staged_path = sqlite_sidecar_path(&staged_db_path, suffix);
        match move_file_if_exists(&original_path, &staged_path) {
            Ok(true) => staged.moved_files.push((staged_path, original_path)),
            Ok(false) => {}
            Err(error) => {
                staged.restore();
                return Err(error);
            }
        }
    }

    match move_file_if_exists(path, &staged_db_path) {
        Ok(true) => staged
            .moved_files
            .push((staged_db_path, path.to_path_buf())),
        Ok(false) => {}
        Err(error) => {
            staged.restore();
            return Err(error);
        }
    }

    Ok(staged)
}

/// Move a (possibly corrupt) sqlite database file and its `-wal`/`-shm`/
/// `-journal` sidecars out of the way to a `<name>.corrupt-<unix_seconds>`
/// backup, so a fresh database can be created in its place. Used by the
/// startup recovery path in `main.rs` when opening the default database
/// fails. Returns the path the primary db file was moved to, or `None` if
/// nothing existed at `db_path`.
pub(crate) fn quarantine_corrupt_database(db_path: &Path) -> Result<Option<PathBuf>> {
    let unix_secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0);
    let file_name = db_path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("clipman.db");
    let backup_path = db_path.with_file_name(format!("{file_name}.corrupt-{unix_secs}"));

    for suffix in ["-wal", "-shm", "-journal"] {
        let original = sqlite_sidecar_path(db_path, suffix);
        let backup = sqlite_sidecar_path(&backup_path, suffix);
        move_file_if_exists(&original, &backup)?;
    }

    let moved = move_file_if_exists(db_path, &backup_path)?;
    Ok(moved.then_some(backup_path))
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
        let _ = fs::remove_file(format!("{}-journal", path.display()));
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
            source_app: None,
            html: None,
        }
    }

    fn labeled_item(id: &str, content: &[u8], label: &str, timestamp: i64) -> ClipItem {
        ClipItem {
            label: Some(label.to_string()),
            ..test_item(id, content, timestamp, false, None)
        }
    }

    fn files_item(id: &str, paths: &[&str], timestamp: i64) -> ClipItem {
        let owned: Vec<String> = paths.iter().map(|path| path.to_string()).collect();
        ClipItem {
            content_type: ContentType::Files,
            content: join_file_paths(&owned).into_bytes(),
            ..test_item(id, b"", timestamp, false, None)
        }
    }

    fn query_plan_details(storage: &ClipStorage, sql: &str) -> Vec<String> {
        let mut stmt = storage.conn.prepare(sql).unwrap();
        stmt.query_map([], |row| row.get::<_, String>(3))
            .unwrap()
            .collect::<Result<Vec<_>>>()
            .unwrap()
    }

    #[test]
    fn new_database_uses_current_schema_and_wal() {
        let db_path = temp_db_path("schema");
        let storage = ClipStorage::new(&db_path).unwrap();

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
        assert_eq!(crate::migration::CURRENT_DB_USER_VERSION, user_version);

        drop(storage);
        cleanup_db(&db_path);
    }

    #[test]
    fn insert_stores_plaintext_content() {
        let db_path = temp_db_path("plaintext");
        let storage = ClipStorage::new(&db_path).unwrap();
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
        let storage = ClipStorage::new(&db_path).unwrap();

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
        let storage = ClipStorage::new(&db_path).unwrap();

        storage
            .insert(
                &labeled_item("labeled", b"deploy command", "work email", 1),
                100,
            )
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
        let storage = ClipStorage::new(&db_path).unwrap();

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
    fn short_search_query_only_scans_preview_window() {
        let db_path = temp_db_path("short_query_preview_window");
        let storage = ClipStorage::new(&db_path).unwrap();
        let mut content = vec![b'a'; TEXT_PREVIEW_BYTES];
        content.extend("中".as_bytes());

        storage
            .insert(&test_item("long", &content, 1, false, None), 100)
            .unwrap();

        let search_ids: Vec<String> = storage
            .search("中")
            .unwrap()
            .into_iter()
            .map(|item| item.id)
            .collect();

        assert!(search_ids.is_empty());
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
            .search("email")
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

        drop(storage);
        cleanup_db(&db_path);
    }

    #[test]
    fn duplicate_insert_preserves_existing_metadata() {
        let db_path = temp_db_path("duplicate_metadata");
        let storage = ClipStorage::new(&db_path).unwrap();

        let mut existing = test_item("pinned", b"same content", 10, true, Some(2));
        existing.label = Some("favorite".to_string());
        existing.group_name = Some("snippets".to_string());
        storage.insert(&existing, 100).unwrap();

        let incoming = test_item("new-id", b"same content", 20, false, None);
        let duplicate_id = storage.insert(&incoming, 100).unwrap();
        let stored = storage.get_by_id("pinned").unwrap().unwrap();

        assert_eq!(Some("pinned".to_string()), duplicate_id);
        assert_eq!("pinned", stored.id);
        assert_eq!(20, stored.timestamp);
        assert!(stored.is_pinned);
        assert_eq!(Some(2), stored.pin_order);
        assert_eq!(Some("favorite".to_string()), stored.label);
        assert_eq!(Some("snippets".to_string()), stored.group_name);
        assert!(storage.get_by_id("new-id").unwrap().is_none());

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
            .get_pinned_clips()
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
            .get_pinned_clips()
            .unwrap()
            .into_iter()
            .map(|item| item.id)
            .collect();
        assert_eq!(vec!["first", "second", "third"], pinned_ids);

        drop(storage);
        cleanup_db(&db_path);
    }

    #[test]
    fn delete_removes_clip_and_fts_row_together() {
        let db_path = temp_db_path("delete_atomicity");
        let storage = ClipStorage::new(&db_path).unwrap();

        let item = test_item("delete-me", b"searchable text", 1, false, None);
        storage.insert(&item, 100).unwrap();
        storage.delete("delete-me").unwrap();

        let clip_count: i64 = storage
            .conn
            .query_row(
                "SELECT COUNT(*) FROM clips WHERE id = 'delete-me'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let fts_count: i64 = storage
            .conn
            .query_row(
                "SELECT COUNT(*) FROM clips_fts WHERE clip_id = 'delete-me'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(0, clip_count);
        assert_eq!(0, fts_count);
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
    fn label_update_keeps_fts_row_in_sync() {
        let db_path = temp_db_path("label_atomicity");
        let storage = ClipStorage::new(&db_path).unwrap();

        storage
            .insert(&test_item("clip", b"body text", 1, false, None), 100)
            .unwrap();
        storage
            .set_clip_label("clip", Some("new label".to_string()))
            .unwrap();

        let label: String = storage
            .conn
            .query_row(
                "SELECT label FROM clips_fts WHERE clip_id = 'clip'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!("new label", label);
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
    fn recent_query_uses_index_without_temp_sort() {
        let db_path = temp_db_path("recent_query_plan");
        let storage = ClipStorage::new(&db_path).unwrap();

        let details = query_plan_details(
            &storage,
            "EXPLAIN QUERY PLAN
             SELECT id, content, thumbnail, content_type, timestamp, is_pinned, pin_order, label, group_name
             FROM clips
             WHERE is_pinned = 0
             ORDER BY timestamp DESC, id DESC
             LIMIT 100",
        );

        assert!(!details
            .iter()
            .any(|detail| detail.contains("USE TEMP B-TREE")));
        drop(storage);
        cleanup_db(&db_path);
    }

    #[test]
    fn recent_page_query_uses_index_without_temp_sort() {
        let db_path = temp_db_path("recent_page_query_plan");
        let storage = ClipStorage::new(&db_path).unwrap();

        // The keyset page query filters on the (timestamp, id) cursor and orders
        // by (timestamp DESC, id DESC); the extended index must satisfy both so
        // no temp b-tree sort appears.
        let details = query_plan_details(
            &storage,
            "EXPLAIN QUERY PLAN
             SELECT id, timestamp, is_pinned
             FROM clips
             WHERE is_pinned = 0
               AND (timestamp < 100 OR (timestamp = 100 AND id < 'z'))
             ORDER BY timestamp DESC, id DESC
             LIMIT 100",
        );

        assert!(!details
            .iter()
            .any(|detail| detail.contains("USE TEMP B-TREE")));
        drop(storage);
        cleanup_db(&db_path);
    }

    #[test]
    fn keyset_pagination_covers_duplicate_timestamps_without_loss_or_repeat() {
        let db_path = temp_db_path("keyset_duplicate_timestamps");
        let storage = ClipStorage::new(&db_path).unwrap();

        // Three rows share timestamp 5; a timestamp-only cursor would drop or
        // repeat rows at a page boundary that lands mid-tie. One older and one
        // newer row exercise paging across timestamps too.
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
        let mut cursor: Option<(i64, String)> = None;
        loop {
            let before = cursor.as_ref().map(|(ts, id)| (*ts, id.as_str()));
            let page = storage
                .get_recent_clip_previews_page(page_size, before)
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

        // Order is timestamp DESC then id DESC, every row exactly once.
        assert_eq!(vec!["newer", "c", "b", "a", "older"], collected);
        let mut unique = collected.clone();
        unique.sort();
        unique.dedup();
        assert_eq!(
            unique.len(),
            collected.len(),
            "no row repeated across pages"
        );

        drop(storage);
        cleanup_db(&db_path);
    }

    #[test]
    fn prune_query_uses_index_without_temp_sort() {
        let db_path = temp_db_path("prune_query_plan");
        let storage = ClipStorage::new(&db_path).unwrap();

        let details = query_plan_details(
            &storage,
            "EXPLAIN QUERY PLAN
             SELECT id FROM clips
             WHERE is_pinned = 0
             ORDER BY timestamp DESC
             LIMIT -1 OFFSET 100",
        );

        assert!(!details
            .iter()
            .any(|detail| detail.contains("USE TEMP B-TREE")));
        drop(storage);
        cleanup_db(&db_path);
    }

    #[test]
    fn pinned_query_uses_index_without_temp_sort() {
        let db_path = temp_db_path("pinned_query_plan");
        let storage = ClipStorage::new(&db_path).unwrap();

        let details = query_plan_details(
            &storage,
            "EXPLAIN QUERY PLAN
             SELECT id, content, thumbnail, content_type, timestamp, is_pinned, pin_order, label, group_name
             FROM clips
             WHERE is_pinned = 1
             ORDER BY pin_order IS NULL, pin_order ASC, timestamp DESC",
        );

        assert!(!details
            .iter()
            .any(|detail| detail.contains("USE TEMP B-TREE")));
        drop(storage);
        cleanup_db(&db_path);
    }

    #[test]
    fn current_database_does_not_rebuild_fts_on_every_open() {
        let db_path = temp_db_path("fts_no_rebuild");
        {
            let conn = Connection::open(&db_path).unwrap();
            ClipStorage::initialize_schema(&conn).unwrap();
            ClipStorage::initialize_fts(&conn).unwrap();
            conn.execute(
                "INSERT INTO clips (id, content, thumbnail, content_hash, content_type, timestamp, is_pinned, pin_order, label, group_name)
                 VALUES ('clip', x'68656c6c6f', NULL, 'hash', 'text', 1, 0, NULL, NULL, NULL)",
                [],
            )
            .unwrap();
            let rowid: i64 = conn
                .query_row("SELECT rowid FROM clips WHERE id = 'clip'", [], |row| {
                    row.get(0)
                })
                .unwrap();
            conn.execute(
                "INSERT INTO clips_fts(rowid, clip_id, search_text, label)
                 VALUES (?1, 'clip', 'sentinel-no-rebuild', NULL)",
                params![rowid],
            )
            .unwrap();
            conn.pragma_update(None, "user_version", 2).unwrap();
        }

        let storage = ClipStorage::new(&db_path).unwrap();
        let search_text: String = storage
            .conn
            .query_row(
                "SELECT search_text FROM clips_fts WHERE clip_id = 'clip'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!("sentinel-no-rebuild", search_text);
        drop(storage);
        cleanup_db(&db_path);
    }

    #[test]
    fn current_database_rebuilds_incomplete_nonempty_fts_index() {
        let db_path = temp_db_path("fts_incomplete_rebuild");
        {
            let conn = Connection::open(&db_path).unwrap();
            ClipStorage::initialize_schema(&conn).unwrap();
            ClipStorage::initialize_fts(&conn).unwrap();
            conn.execute(
                "INSERT INTO clips (id, content, thumbnail, content_hash, content_type, timestamp, is_pinned, pin_order, label, group_name)
                 VALUES ('first', x'6669727374', NULL, 'hash-1', 'text', 1, 0, NULL, NULL, NULL)",
                [],
            )
            .unwrap();
            conn.execute(
                "INSERT INTO clips (id, content, thumbnail, content_hash, content_type, timestamp, is_pinned, pin_order, label, group_name)
                 VALUES ('second', x'7365636f6e64', NULL, 'hash-2', 'text', 2, 0, NULL, NULL, NULL)",
                [],
            )
            .unwrap();
            let rowid: i64 = conn
                .query_row("SELECT rowid FROM clips WHERE id = 'first'", [], |row| {
                    row.get(0)
                })
                .unwrap();
            conn.execute(
                "INSERT INTO clips_fts(rowid, clip_id, search_text, label)
                 VALUES (?1, 'first', 'stale-first-only', NULL)",
                params![rowid],
            )
            .unwrap();
            conn.pragma_update(None, "user_version", 2).unwrap();
        }

        let storage = ClipStorage::new(&db_path).unwrap();
        let indexed_ids: Vec<String> = storage
            .conn
            .prepare("SELECT clip_id FROM clips_fts ORDER BY clip_id")
            .unwrap()
            .query_map([], |row| row.get::<_, String>(0))
            .unwrap()
            .collect::<Result<Vec<_>>>()
            .unwrap();

        assert_eq!(vec!["first".to_string(), "second".to_string()], indexed_ids);
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

        let full_search = storage.search("needle").unwrap();
        let preview_search = storage.search_clip_previews("needle").unwrap();

        assert!(full_search.is_empty());
        assert!(preview_search.is_empty());
        drop(storage);
        cleanup_db(&db_path);
    }

    #[test]
    fn fts_rebuild_handles_batch_boundaries_and_rowid_gaps() {
        let db_path = temp_db_path("fts_batch_gaps");
        {
            let conn = Connection::open(&db_path).unwrap();
            ClipStorage::initialize_schema(&conn).unwrap();
            ClipStorage::initialize_fts(&conn).unwrap();

            for index in 0..(FTS_REBUILD_BATCH_SIZE as usize + 5) {
                conn.execute(
                    "INSERT INTO clips (id, content, thumbnail, content_hash, content_type, timestamp, is_pinned, pin_order, label, group_name)
                     VALUES (?1, ?2, NULL, ?3, 'text', ?4, 0, NULL, NULL, NULL)",
                    params![
                        format!("clip-{index:03}"),
                        format!("needle{index:03}").into_bytes(),
                        format!("hash-{index:03}"),
                        index as i64,
                    ],
                )
                .unwrap();
            }
            conn.execute("DELETE FROM clips WHERE id IN ('clip-010', 'clip-077')", [])
                .unwrap();
            conn.pragma_update(None, "user_version", 2).unwrap();
        }

        let storage = ClipStorage::new(&db_path).unwrap();
        let mismatches: i64 = storage
            .conn
            .query_row(
                "SELECT COUNT(*)
                 FROM clips c
                 JOIN clips_fts f ON f.rowid = c.rowid
                 WHERE f.clip_id != c.id",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let indexed_count: i64 = storage
            .conn
            .query_row("SELECT COUNT(*) FROM clips_fts", [], |row| row.get(0))
            .unwrap();
        let search_ids: Vec<String> = storage
            .search("needle104")
            .unwrap()
            .into_iter()
            .map(|item| item.id)
            .collect();

        assert_eq!(0, mismatches);
        assert_eq!(FTS_REBUILD_BATCH_SIZE + 3, indexed_count);
        assert_eq!(vec!["clip-104".to_string()], search_ids);
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
                    timestamp: 2,
                    is_pinned: false,
                    pin_order: None,
                    label: None,
                    group_name: None,
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
    fn clip_preview_from_clip_item_truncates_text_payload() {
        let mut long_text = vec![b'a'; TEXT_PREVIEW_BYTES];
        long_text.extend(vec![b'b'; 128]);
        let item = test_item("text", &long_text, 1, false, None);

        let preview = ClipPreviewItem::from_clip_item(&item);

        assert_eq!(TEXT_PREVIEW_BYTES, preview.preview_content.len());
        assert_eq!(&long_text[..TEXT_PREVIEW_BYTES], preview.preview_content);
    }

    #[test]
    fn get_preview_by_id_does_not_return_full_text_payload() {
        let db_path = temp_db_path("preview_by_id");
        let storage = ClipStorage::new(&db_path).unwrap();
        let long_text = vec![b'a'; TEXT_PREVIEW_BYTES + 128];

        storage
            .insert(&test_item("text", &long_text, 1, false, None), 100)
            .unwrap();

        let preview = storage.get_preview_by_id("text").unwrap().unwrap();

        assert_eq!(TEXT_PREVIEW_BYTES, preview.preview_content.len());
        drop(storage);
        cleanup_db(&db_path);
    }

    #[test]
    fn full_text_mapping_returns_complete_text_payload() {
        use data_encoding::BASE64;

        let mut long_text = vec![b'a'; TEXT_PREVIEW_BYTES];
        long_text.extend(vec![b'b'; 128]);
        let item = test_item("text", &long_text, 1, false, None);

        let full = FrontendClipItem::from_full_text(item).unwrap();

        assert_eq!(BASE64.encode(&long_text), full.content);
        assert_eq!(ContentType::Text, full.content_type);
    }

    #[test]
    fn full_text_mapping_rejects_image_payloads() {
        let item = ClipItem {
            content_type: ContentType::Image,
            thumbnail: Some(b"thumbnail".to_vec()),
            ..test_item("image", b"full image payload", 1, false, None)
        };

        assert!(FrontendClipItem::from_full_text(item).is_none());
    }

    #[test]
    fn pinned_clip_previews_with_limit_bounds_query() {
        let db_path = temp_db_path("pinned_previews_limit");
        let storage = ClipStorage::new(&db_path).unwrap();

        for index in 0..5 {
            storage
                .insert(
                    &test_item(
                        &format!("pinned-{index}"),
                        format!("pinned content {index}").as_bytes(),
                        10 - index,
                        true,
                        Some(index as i32),
                    ),
                    100,
                )
                .unwrap();
        }

        let ids: Vec<String> = storage
            .get_pinned_clip_previews_with_limit(2)
            .unwrap()
            .into_iter()
            .map(|item| item.id)
            .collect();

        assert_eq!(vec!["pinned-0".to_string(), "pinned-1".to_string()], ids);
        drop(storage);
        cleanup_db(&db_path);
    }

    #[test]
    fn backup_to_path_replaces_destination_and_stale_sidecars() {
        let source_path = temp_db_path("backup_source");
        let destination_path = temp_db_path("backup_destination");
        let storage = ClipStorage::new(&source_path).unwrap();
        storage
            .insert(
                &labeled_item("backup", b"backup needle", "backup label", 1),
                100,
            )
            .unwrap();

        fs::write(&destination_path, b"stale").unwrap();
        for suffix in ["-wal", "-shm", "-journal"] {
            fs::write(sqlite_sidecar_path(&destination_path, suffix), b"stale").unwrap();
        }

        storage.backup_to_path(&destination_path).unwrap();

        assert!(destination_path.exists());
        for suffix in ["-wal", "-shm", "-journal"] {
            assert!(!sqlite_sidecar_path(&destination_path, suffix).exists());
        }
        drop(storage);

        let restored = ClipStorage::new(&destination_path).unwrap();
        let ids: Vec<String> = restored
            .search("needle")
            .unwrap()
            .into_iter()
            .map(|item| item.id)
            .collect();

        assert_eq!(vec!["backup".to_string()], ids);
        drop(restored);
        cleanup_db(&source_path);
        cleanup_db(&destination_path);
    }

    #[test]
    fn staged_sqlite_replacement_restore_recovers_original_files() {
        let destination_path = temp_db_path("stage_restore");
        fs::write(&destination_path, b"old-db").unwrap();
        for suffix in ["-wal", "-shm", "-journal"] {
            fs::write(
                sqlite_sidecar_path(&destination_path, suffix),
                format!("old{suffix}").as_bytes(),
            )
            .unwrap();
        }

        let mut staged = stage_sqlite_files_for_replacement(&destination_path).unwrap();
        assert!(!destination_path.exists());
        for suffix in ["-wal", "-shm", "-journal"] {
            assert!(!sqlite_sidecar_path(&destination_path, suffix).exists());
        }

        staged.restore();

        assert_eq!(b"old-db".to_vec(), fs::read(&destination_path).unwrap());
        for suffix in ["-wal", "-shm", "-journal"] {
            assert_eq!(
                format!("old{suffix}").into_bytes(),
                fs::read(sqlite_sidecar_path(&destination_path, suffix)).unwrap()
            );
        }
        cleanup_db(&destination_path);
    }

    #[test]
    fn search_clip_previews_return_preview_rows() {
        let db_path = temp_db_path("search_previews");
        let storage = ClipStorage::new(&db_path).unwrap();

        let mut long_text = b"needle ".to_vec();
        long_text.extend(vec![b'a'; 5000]);
        storage
            .insert(
                &ClipItem {
                    content: long_text,
                    ..test_item("text", b"", 1, false, None)
                },
                100,
            )
            .unwrap();

        let previews = storage.search_clip_previews("needle").unwrap();

        assert_eq!(
            vec!["text".to_string()],
            previews
                .iter()
                .map(|item| item.id.clone())
                .collect::<Vec<_>>()
        );
        assert_eq!(4096, previews[0].preview_content.len());

        drop(storage);
        cleanup_db(&db_path);
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
    fn file_path_join_split_roundtrip_and_filters_blank_lines() {
        let paths = vec!["/a/b.txt".to_string(), "/c/d e.png".to_string()];
        let joined = join_file_paths(&paths);

        assert_eq!("/a/b.txt\n/c/d e.png", joined);
        assert_eq!(paths, split_file_paths(&joined));

        // Trailing and doubled newlines must never produce empty path entries.
        assert_eq!(
            vec!["/a".to_string(), "/b".to_string()],
            split_file_paths("/a\n\n/b\n")
        );
        assert!(split_file_paths("").is_empty());
    }

    #[test]
    fn files_clip_roundtrips_and_is_searchable_by_path() {
        let db_path = temp_db_path("files_roundtrip");
        let storage = ClipStorage::new(&db_path).unwrap();

        let item = files_item(
            "files",
            &["/Users/alice/report.pdf", "/Users/alice/photo.png"],
            1,
        );
        storage.insert(&item, 100).unwrap();

        let stored = storage.get_by_id("files").unwrap().unwrap();
        assert_eq!(ContentType::Files, stored.content_type);
        assert_eq!(item.content, stored.content);
        assert_eq!(
            vec![
                "/Users/alice/report.pdf".to_string(),
                "/Users/alice/photo.png".to_string(),
            ],
            split_file_paths(&String::from_utf8_lossy(&stored.content))
        );

        let search_ids: Vec<String> = storage
            .search("report.pdf")
            .unwrap()
            .into_iter()
            .map(|item| item.id)
            .collect();
        assert_eq!(vec!["files"], search_ids);

        drop(storage);
        cleanup_db(&db_path);
    }

    #[test]
    fn files_preview_returns_path_text_and_html_flag() {
        let db_path = temp_db_path("files_preview");
        let storage = ClipStorage::new(&db_path).unwrap();

        storage
            .insert(
                &files_item("files", &["/Users/bob/a.txt", "/Users/bob/b.txt"], 1),
                100,
            )
            .unwrap();
        storage
            .insert(
                &ClipItem {
                    html: Some("<p>x</p>".to_string()),
                    ..test_item("rich", b"x", 2, false, None)
                },
                100,
            )
            .unwrap();

        let files_preview = storage.get_preview_by_id("files").unwrap().unwrap();
        assert_eq!(ContentType::Files, files_preview.content_type);
        assert_eq!(
            "/Users/bob/a.txt\n/Users/bob/b.txt",
            String::from_utf8_lossy(&files_preview.preview_content)
        );
        assert!(!files_preview.has_html);

        let rich_preview = storage.get_preview_by_id("rich").unwrap().unwrap();
        assert!(rich_preview.has_html);

        drop(storage);
        cleanup_db(&db_path);
    }

    #[test]
    fn html_column_roundtrips_and_duplicate_refresh_coalesces() {
        let db_path = temp_db_path("html_coalesce");
        let storage = ClipStorage::new(&db_path).unwrap();

        storage
            .insert(
                &ClipItem {
                    html: Some("<b>hello</b>".to_string()),
                    ..test_item("rich", b"hello", 1, false, None)
                },
                100,
            )
            .unwrap();
        assert_eq!(
            Some("<b>hello</b>".to_string()),
            storage.get_by_id("rich").unwrap().unwrap().html
        );

        // A plain-text re-copy of identical content (html = None) refreshes the
        // timestamp but keeps the previously captured html via COALESCE (D6).
        let dup_id = storage
            .insert(&test_item("plain-dup", b"hello", 2, false, None), 100)
            .unwrap();
        assert_eq!(Some("rich".to_string()), dup_id);
        let after_plain = storage.get_by_id("rich").unwrap().unwrap();
        assert_eq!(Some("<b>hello</b>".to_string()), after_plain.html);
        assert_eq!(2, after_plain.timestamp);

        // A newer rich re-copy wins and replaces the stored html.
        storage
            .insert(
                &ClipItem {
                    html: Some("<i>hello</i>".to_string()),
                    ..test_item("rich-again", b"hello", 3, false, None)
                },
                100,
            )
            .unwrap();
        assert_eq!(
            Some("<i>hello</i>".to_string()),
            storage.get_by_id("rich").unwrap().unwrap().html
        );

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
    fn new_database_enables_incremental_auto_vacuum() {
        let db_path = temp_db_path("auto_vacuum_new");
        let storage = ClipStorage::new(&db_path).unwrap();

        let mode: i64 = storage
            .conn
            .query_row("PRAGMA auto_vacuum", [], |row| row.get(0))
            .unwrap();
        assert_eq!(2, mode);

        drop(storage);
        cleanup_db(&db_path);
    }

    #[test]
    fn opening_legacy_none_auto_vacuum_database_upgrades_to_incremental() {
        let db_path = temp_db_path("auto_vacuum_legacy");
        {
            let conn = Connection::open(&db_path).unwrap();
            ClipStorage::initialize_schema(&conn).unwrap();
            conn.execute(
                "INSERT INTO clips (id, content, thumbnail, content_hash, content_type, timestamp, is_pinned, pin_order, label, group_name)
                 VALUES ('legacy', x'6c6567616379', NULL, 'hash', 'text', 1, 0, NULL, NULL, NULL)",
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

        // The one-time VACUUM that flips auto_vacuum must not lose data.
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

        // Large enough, and enough of them, that freeing them leaves a
        // measurable number of pages for incremental_vacuum to reclaim.
        let blob = vec![0xABu8; 200_000];
        for index in 0..20 {
            storage
                .insert(
                    &ClipItem {
                        content: blob.clone(),
                        content_type: ContentType::Image,
                        ..test_item(&format!("big-{index}"), b"", index, false, None)
                    },
                    1000,
                )
                .unwrap();
        }

        let size_before = database_byte_size(&storage);

        storage.clear_all().unwrap();

        let size_after = database_byte_size(&storage);

        assert!(
            size_after < size_before,
            "expected database file to shrink after clearing large clips: before={size_before} after={size_after}"
        );

        drop(storage);
        cleanup_db(&db_path);
    }
}
