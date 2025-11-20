use rusqlite::{Connection, params, Result};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use crate::crypto::Crypto;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum ContentType {
    Text,
    Image,
    File,
    Html,
    Rtf,
}

impl ContentType {
    fn to_string(&self) -> &str {
        match self {
            ContentType::Text => "text",
            ContentType::Image => "image",
            ContentType::File => "file",
            ContentType::Html => "html",
            ContentType::Rtf => "rtf",
        }
    }

    fn from_string(s: &str) -> Self {
        match s {
            "image" => ContentType::Image,
            "file" => ContentType::File,
            "html" => ContentType::Html,
            "rtf" => ContentType::Rtf,
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

        // Migration: Add content_hash column if it doesn't exist
        // Check if column exists by querying pragma
        let has_content_hash: bool = conn
            .prepare("PRAGMA table_info(clips)")?
            .query_map([], |row| {
                let name: String = row.get(1)?;
                Ok(name)
            })?
            .filter_map(Result::ok)
            .any(|name| name == "content_hash");

        if !has_content_hash {
            log::info!("📦 Migrating database: adding content_hash column");
            conn.execute("ALTER TABLE clips ADD COLUMN content_hash TEXT", [])?;
        }

        // Create index for fast queries
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_timestamp ON clips(timestamp DESC)",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_pinned ON clips(is_pinned, pin_order)",
            [],
        )?;

        // Create index for content hash to speed up duplicate checks
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_content_hash ON clips(content_hash, content_type)",
            [],
        )?;

        Ok(Self { conn, crypto })
    }

    pub fn insert(&self, item: &ClipItem) -> Result<()> {
        use sha2::{Sha256, Digest};

        // Calculate content hash for deduplication
        let mut hasher = Sha256::new();
        hasher.update(&item.content);
        let content_hash = format!("{:x}", hasher.finalize());

        // Check if content already exists in recent items (last 100)
        let exists: bool = self.conn.query_row(
            "SELECT EXISTS(
                SELECT 1 FROM clips 
                WHERE content_hash = ?1 AND content_type = ?2
                ORDER BY timestamp DESC
                LIMIT 100
            )",
            params![content_hash, item.content_type.to_string()],
            |row| row.get(0)
        )?;

        if exists {
            log::debug!("⏭️ Duplicate content detected (hash: {}), skipping", &content_hash[..8]);
            return Ok(());
        }

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
            "INSERT INTO clips (id, content, content_hash, content_type, timestamp, is_pinned, pin_order)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                item.id,
                content_to_store,
                content_hash,
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
            let content = match self.decrypt_content(encrypted_content.clone()) {
                Ok(c) => c,
                Err(e) => {
                    // 解密失败，记录错误并跳过此项
                    let id: String = row.get(0).unwrap_or_else(|_| "unknown".to_string());
                    log::warn!("⚠️ Failed to decrypt item {}: {:?}. Skipping.", id, e);
                    // 返回空内容以避免整个查询失败
                    Vec::new()
                }
            };

            // 如果内容为空（解密失败），我们仍然返回item但标记为无效
            Ok(ClipItem {
                id: row.get(0)?,
                content,
                content_type: ContentType::from_string(&row.get::<_, String>(2)?),
                timestamp: row.get(3)?,
                is_pinned: row.get::<_, i32>(4)? != 0,
                pin_order: row.get(5)?,
            })
        })?;

        // 过滤掉解密失败的项目（内容为空）
        items.filter_map(|item| {
            match item {
                Ok(clip_item) if !clip_item.content.is_empty() => Some(Ok(clip_item)),
                Ok(_) => None, // 跳过空内容的项目
                Err(e) => Some(Err(e)),
            }
        }).collect()
    }

    pub fn search(&self, query: &str) -> Result<Vec<ClipItem>> {
        // FTS5 需要对特殊字符进行转义，或者使用简单的 LIKE 查询
        // 对于用户输入，我们使用更简单的方法：直接搜索文本内容
        log::info!("🔍 Searching for: {}", query);

        let mut stmt = self.conn.prepare(
            "SELECT id, content, content_type, timestamp, is_pinned, pin_order
             FROM clips
             WHERE content_type = 'text'
             ORDER BY timestamp DESC
             LIMIT 500"
        )?;

        let items = stmt.query_map([], |row| {
            let encrypted_content: Vec<u8> = row.get(1)?;
            let content = match self.decrypt_content(encrypted_content.clone()) {
                Ok(c) => c,
                Err(e) => {
                    let id: String = row.get(0).unwrap_or_else(|_| "unknown".to_string());
                    log::warn!("⚠️ Failed to decrypt search result {}: {:?}. Skipping.", id, e);
                    Vec::new()
                }
            };

            Ok(ClipItem {
                id: row.get(0)?,
                content,
                content_type: ContentType::from_string(&row.get::<_, String>(2)?),
                timestamp: row.get(3)?,
                is_pinned: row.get::<_, i32>(4)? != 0,
                pin_order: row.get(5)?,
            })
        })?;

        // 在内存中过滤搜索结果
        let query_lower = query.to_lowercase();
        items.filter_map(|item| {
            match item {
                Ok(clip_item) if !clip_item.content.is_empty() => {
                    // 解码内容并检查是否包含查询字符串
                    if let Ok(text) = String::from_utf8(clip_item.content.clone()) {
                        if text.to_lowercase().contains(&query_lower) {
                            Some(Ok(clip_item))
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                }
                Ok(_) => None,
                Err(e) => Some(Err(e)),
            }
        }).collect()
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

    pub fn clear_all(&self) -> Result<()> {
        log::info!("🗑️ Clearing all clipboard history");
        self.conn.execute("DELETE FROM clips", [])?;
        Ok(())
    }

    pub fn clear_non_pinned(&self) -> Result<()> {
        log::info!("🗑️ Clearing non-pinned clipboard history");
        self.conn.execute("DELETE FROM clips WHERE is_pinned = 0", [])?;
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
            let content = match self.decrypt_content(encrypted_content.clone()) {
                Ok(c) => c,
                Err(e) => {
                    let id: String = row.get(0).unwrap_or_else(|_| "unknown".to_string());
                    log::warn!("⚠️ Failed to decrypt pinned item {}: {:?}. Skipping.", id, e);
                    Vec::new()
                }
            };

            Ok(ClipItem {
                id: row.get(0)?,
                content,
                content_type: ContentType::from_string(&row.get::<_, String>(2)?),
                timestamp: row.get(3)?,
                is_pinned: row.get::<_, i32>(4)? != 0,
                pin_order: row.get(5)?,
            })
        })?;

        items.filter_map(|item| {
            match item {
                Ok(clip_item) if !clip_item.content.is_empty() => Some(Ok(clip_item)),
                Ok(_) => None,
                Err(e) => Some(Err(e)),
            }
        }).collect()
    }
}
