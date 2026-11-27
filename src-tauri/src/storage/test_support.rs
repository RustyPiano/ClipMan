use std::path::{Path, PathBuf};
use uuid::Uuid;

use super::maintenance::remove_sqlite_database_files;
use super::{ClipItem, ContentType};

pub(super) fn temp_db_path(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("clipleaf_{}_{}.db", name, Uuid::new_v4()))
}

pub(super) fn cleanup_db(path: &Path) {
    let _ = remove_sqlite_database_files(path);
}

pub(super) fn test_item(
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
