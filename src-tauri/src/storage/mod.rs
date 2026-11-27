mod favorites;
mod fts;
mod maintenance;
mod query;
mod schema;
#[cfg(test)]
mod test_support;
mod writes;

use rusqlite::types::{FromSql, FromSqlError, FromSqlResult, ValueRef};
use rusqlite::Connection;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

pub(crate) use maintenance::{is_corrupt_database_error, quarantine_corrupt_database};

pub fn current_timestamp() -> f64 {
    chrono::Utc::now().timestamp_micros() as f64 / 1_000_000.0
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
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
    pub is_link: bool,
}

#[derive(Debug, Clone, Serialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename = "ClipItem")]
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
    pub is_link: bool,
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
            is_link: item.is_link,
        }
    }
}

#[derive(Debug, Serialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename = "ClipDetail")]
pub struct FrontendClipDetail {
    pub id: String,
    pub content_type: ContentType,
    pub text: String,
    pub file_paths: Vec<String>,
    pub image_url: Option<String>,
    pub truncated: bool,
}

#[derive(Debug, Serialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct StorageUsage {
    pub total: usize,
    pub pinned: usize,
    pub images: usize,
    pub database_bytes: u64,
}

#[derive(Debug, Serialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct BackupFile {
    pub name: String,
    pub bytes: u64,
}

pub const FAVORITES_FILE_VERSION: u32 = 1;

#[derive(Debug, Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FavoritesFile {
    pub version: u32,
    pub items: Vec<ExportedClip>,
}

#[derive(Debug, Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportedClip {
    pub content_type: String,
    pub content: String,
    pub thumbnail: Option<String>,
    pub label: Option<String>,
    pub html: Option<String>,
}

pub struct ClipStorage {
    conn: Connection,
    data_dir: PathBuf,
}

impl ClipStorage {
    pub fn data_directory(&self) -> &Path {
        &self.data_dir
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

fn string_to_rusqlite_error(error: String) -> rusqlite::Error {
    rusqlite::Error::ToSqlConversionFailure(Box::new(std::io::Error::other(error)))
}

fn io_to_rusqlite_error(error: std::io::Error) -> rusqlite::Error {
    rusqlite::Error::ToSqlConversionFailure(Box::new(error))
}

#[cfg(test)]
mod tests {
    use super::test_support::*;
    use super::*;

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
}
