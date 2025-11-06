use rusqlite::{Connection, params, Result};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use crate::crypto::Crypto;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ContentType {
    Text,
    Image,
    File,
}

impl ContentType {
    fn to_string(&self) -> &str {
        match self {
            ContentType::Text => "text",
            ContentType::Image => "image",
            ContentType::File => "file",
        }
    }

    fn from_string(s: &str) -> Self {
        match s {
            "image" => ContentType::Image,
            "file" => ContentType::File,
            _ => ContentType::Text,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClipItem {
    pub id: String,
    #[serde(with = "serde_bytes")]
    pub content: Vec<u8>,
    pub content_type: ContentType,
    pub timestamp: i64,
    pub is_pinned: bool,
    pub pin_order: Option<i32>,
}

pub struct ClipStorage {
    conn: Connection,
    crypto: Option<Arc<Crypto>>,
}

impl ClipStorage {
    pub fn new(db_path: &str, crypto: Option<Arc<Crypto>>) -> Result<Self> {
        let conn = Connection::open(db_path)?;

        // Create main table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS clips (
                id TEXT PRIMARY KEY,
                content BLOB NOT NULL,
                content_type TEXT NOT NULL,
                timestamp INTEGER NOT NULL,
                is_pinned INTEGER DEFAULT 0,
                pin_order INTEGER
            )",
            [],
        )?;

        // Create FTS5 virtual table for full-text search
        conn.execute(
            "CREATE VIRTUAL TABLE IF NOT EXISTS clips_fts
             USING fts5(id, content_text, content=clips, content_rowid=rowid)",
            [],
        )?;

        // Create triggers to keep FTS in sync
        conn.execute(
            "CREATE TRIGGER IF NOT EXISTS clips_ai AFTER INSERT ON clips BEGIN
                INSERT INTO clips_fts(rowid, id, content_text)
                VALUES (new.rowid, new.id, CASE
                    WHEN new.content_type = 'text' THEN new.content
                    ELSE ''
                END);
             END",
            [],
        )?;

        conn.execute(
            "CREATE TRIGGER IF NOT EXISTS clips_ad AFTER DELETE ON clips BEGIN
                INSERT INTO clips_fts(clips_fts, rowid, id, content_text)
                VALUES('delete', old.rowid, old.id, '');
             END",
            [],
        )?;

        // Create index for fast queries
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_timestamp ON clips(timestamp DESC)",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_pinned ON clips(is_pinned, pin_order)",
            [],
        )?;

        Ok(Self { conn, crypto })
    }

    pub fn insert(&self, item: &ClipItem) -> Result<()> {
        // Encrypt content if crypto is available
        let content_to_store = if let Some(crypto) = &self.crypto {
            crypto.encrypt(&item.content)
                .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    e
                ))))?
        } else {
            item.content.clone()
        };

        self.conn.execute(
            "INSERT INTO clips (id, content, content_type, timestamp, is_pinned, pin_order)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                item.id,
                content_to_store,
                item.content_type.to_string(),
                item.timestamp,
                item.is_pinned as i32,
                item.pin_order,
            ],
        )?;

        // Auto-cleanup old items (keep last 100)
        self.conn.execute(
            "DELETE FROM clips
             WHERE id IN (
                SELECT id FROM clips
                WHERE is_pinned = 0
                ORDER BY timestamp DESC
                LIMIT -1 OFFSET 100
             )",
            [],
        )?;

        Ok(())
    }

    // Helper method to decrypt content
    fn decrypt_content(&self, encrypted: Vec<u8>) -> Result<Vec<u8>> {
        if let Some(crypto) = &self.crypto {
            crypto.decrypt(&encrypted)
                .map_err(|e| rusqlite::Error::FromSqlConversionFailure(
                    0,
                    rusqlite::types::Type::Blob,
                    Box::new(std::io::Error::new(std::io::ErrorKind::Other, e))
                ))
        } else {
            Ok(encrypted)
        }
    }

    pub fn get_recent(&self, limit: usize) -> Result<Vec<ClipItem>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, content, content_type, timestamp, is_pinned, pin_order
             FROM clips
             ORDER BY timestamp DESC
             LIMIT ?1"
        )?;

        let items = stmt.query_map([limit], |row| {
            let encrypted_content: Vec<u8> = row.get(1)?;
            let content = self.decrypt_content(encrypted_content)?;

            Ok(ClipItem {
                id: row.get(0)?,
                content,
                content_type: ContentType::from_string(&row.get::<_, String>(2)?),
                timestamp: row.get(3)?,
                is_pinned: row.get::<_, i32>(4)? != 0,
                pin_order: row.get(5)?,
            })
        })?;

        items.collect()
    }

    pub fn search(&self, query: &str) -> Result<Vec<ClipItem>> {
        let mut stmt = self.conn.prepare(
            "SELECT c.id, c.content, c.content_type, c.timestamp, c.is_pinned, c.pin_order
             FROM clips c
             JOIN clips_fts ON clips_fts.id = c.id
             WHERE clips_fts MATCH ?1
             ORDER BY c.timestamp DESC
             LIMIT 50"
        )?;

        let items = stmt.query_map([query], |row| {
            let encrypted_content: Vec<u8> = row.get(1)?;
            let content = self.decrypt_content(encrypted_content)?;

            Ok(ClipItem {
                id: row.get(0)?,
                content,
                content_type: ContentType::from_string(&row.get::<_, String>(2)?),
                timestamp: row.get(3)?,
                is_pinned: row.get::<_, i32>(4)? != 0,
                pin_order: row.get(5)?,
            })
        })?;

        items.collect()
    }

    pub fn update_pin(&self, id: &str, is_pinned: bool) -> Result<()> {
        let pin_order = if is_pinned {
            // Get next pin order
            let max_order: Option<i32> = self.conn.query_row(
                "SELECT MAX(pin_order) FROM clips WHERE is_pinned = 1",
                [],
                |row| row.get(0),
            ).unwrap_or(None);

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

    pub fn delete(&self, id: &str) -> Result<()> {
        self.conn.execute("DELETE FROM clips WHERE id = ?1", params![id])?;
        Ok(())
    }

    pub fn get_pinned(&self) -> Result<Vec<ClipItem>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, content, content_type, timestamp, is_pinned, pin_order
             FROM clips
             WHERE is_pinned = 1
             ORDER BY pin_order ASC"
        )?;

        let items = stmt.query_map([], |row| {
            let encrypted_content: Vec<u8> = row.get(1)?;
            let content = self.decrypt_content(encrypted_content)?;

            Ok(ClipItem {
                id: row.get(0)?,
                content,
                content_type: ContentType::from_string(&row.get::<_, String>(2)?),
                timestamp: row.get(3)?,
                is_pinned: row.get::<_, i32>(4)? != 0,
                pin_order: row.get(5)?,
            })
        })?;

        items.collect()
    }
}
