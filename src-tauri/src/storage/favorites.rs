use std::io::Write;
use std::path::Path;

use rusqlite::{params, OptionalExtension, Result};

use super::{
    current_timestamp, hash_bytes, normalize_label, ClipStorage, ContentType, ExportedClip,
    FavoritesFile, FAVORITES_FILE_VERSION,
};

impl ClipStorage {
    pub fn export_pinned_to(&self, path: &Path) -> std::result::Result<usize, String> {
        let file = self.export_pinned().map_err(|e| e.to_string())?;
        let json = serde_json::to_vec_pretty(&file).map_err(|e| e.to_string())?;
        let file_name = path
            .file_name()
            .ok_or_else(|| format!("{} is not a file path", path.display()))?;
        let mut temp_name = std::ffi::OsString::from(".");
        temp_name.push(file_name);
        temp_name.push(format!(".{}.tmp", uuid::Uuid::new_v4()));
        let temp_path = path.with_file_name(temp_name);
        let written = (|| -> std::io::Result<()> {
            let mut temp = std::fs::File::create_new(&temp_path)?;
            temp.write_all(&json)?;
            temp.sync_all()?;
            std::fs::rename(&temp_path, path)
        })();
        if let Err(error) = written {
            if let Err(cleanup_error) = std::fs::remove_file(&temp_path) {
                if cleanup_error.kind() != std::io::ErrorKind::NotFound {
                    log::warn!("Failed to remove {}: {cleanup_error}", temp_path.display());
                }
            }
            return Err(format!("Failed to write {}: {error}", path.display()));
        }
        Ok(file.items.len())
    }

    pub fn export_pinned(&self) -> Result<FavoritesFile> {
        use data_encoding::BASE64;

        let mut stmt = self.conn.prepare(
            "SELECT content_type, content, thumbnail, label, html
             FROM clips WHERE is_pinned = 1
             ORDER BY pin_order IS NULL, pin_order, timestamp DESC",
        )?;
        let items = stmt
            .query_map([], |row| {
                let content_type: ContentType = row.get(0)?;
                let content: Vec<u8> = row.get(1)?;
                let thumbnail: Option<Vec<u8>> = row.get(2)?;
                let content = match content_type {
                    ContentType::Image => BASE64.encode(&content),
                    ContentType::Text | ContentType::Files => {
                        String::from_utf8(content).map_err(|e| {
                            rusqlite::Error::FromSqlConversionFailure(
                                1,
                                rusqlite::types::Type::Blob,
                                Box::new(e),
                            )
                        })?
                    }
                };
                Ok(ExportedClip {
                    content_type: content_type.as_db_value().to_string(),
                    content,
                    thumbnail: thumbnail.map(|bytes| BASE64.encode(&bytes)),
                    label: row.get(3)?,
                    html: row.get(4)?,
                })
            })?
            .collect::<Result<Vec<_>>>()?;
        Ok(FavoritesFile {
            version: FAVORITES_FILE_VERSION,
            items,
        })
    }

    pub fn import_pinned(
        &self,
        file: FavoritesFile,
    ) -> std::result::Result<(usize, usize), String> {
        use data_encoding::BASE64;

        if file.version != FAVORITES_FILE_VERSION {
            return Err(format!(
                "Unsupported favorites file version {}",
                file.version
            ));
        }
        let decoded = file
            .items
            .into_iter()
            .enumerate()
            .map(|(index, item)| {
                let invalid = |reason: String| format!("Item {}: {reason}", index + 1);
                let (content_type, content) = match item.content_type.as_str() {
                    "text" => (ContentType::Text, item.content.into_bytes()),
                    "files" => {
                        serde_json::from_str::<Vec<String>>(&item.content).map_err(|e| {
                            invalid(format!("file list is not a JSON string array: {e}"))
                        })?;
                        (ContentType::Files, item.content.into_bytes())
                    }
                    "image" => {
                        let image = BASE64
                            .decode(item.content.as_bytes())
                            .map_err(|e| invalid(format!("image is not valid base64: {e}")))?;
                        image::load_from_memory(&image)
                            .map_err(|e| invalid(format!("image cannot be decoded: {e}")))?;
                        (ContentType::Image, image)
                    }
                    other => return Err(invalid(format!("unknown content type {other}"))),
                };
                let thumbnail = item
                    .thumbnail
                    .map(|value| BASE64.decode(value.as_bytes()))
                    .transpose()
                    .map_err(|e| invalid(format!("thumbnail is not valid base64: {e}")))?;
                if let Some(thumbnail) = &thumbnail {
                    image::load_from_memory(thumbnail)
                        .map_err(|e| invalid(format!("thumbnail cannot be decoded: {e}")))?;
                }
                Ok((
                    content_type,
                    content,
                    thumbnail,
                    normalize_label(item.label),
                    item.html,
                ))
            })
            .collect::<std::result::Result<Vec<_>, String>>()?;

        let tx = self
            .conn
            .unchecked_transaction()
            .map_err(|e| e.to_string())?;
        let mut imported = 0;
        let mut skipped = 0;
        for (content_type, content, thumbnail, label, html) in decoded {
            let content_hash = hash_bytes(&content);
            let existing: Option<(String, bool)> = tx
                .query_row(
                    "SELECT id, is_pinned FROM clips
                     WHERE content_hash = ?1 AND content_type = ?2
                     ORDER BY timestamp DESC LIMIT 1",
                    params![content_hash, content_type.as_db_value()],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )
                .optional()
                .map_err(|e| e.to_string())?;
            if matches!(existing, Some((_, true))) {
                skipped += 1;
                continue;
            }
            let pin_order: i32 = tx
                .query_row(
                    "SELECT COALESCE(MAX(pin_order), 0) + 1 FROM clips WHERE is_pinned = 1",
                    [],
                    |row| row.get(0),
                )
                .map_err(|e| e.to_string())?;
            let id = match existing {
                Some((id, _)) => {
                    tx.execute(
                        "UPDATE clips SET is_pinned = 1, pin_order = ?1, label = COALESCE(label, ?2),
                                html = COALESCE(html, ?3), thumbnail = COALESCE(thumbnail, ?4)
                         WHERE id = ?5",
                        params![pin_order, label, html, thumbnail, id],
                    )
                    .map_err(|e| e.to_string())?;
                    id
                }
                None => {
                    let id = uuid::Uuid::new_v4().to_string();
                    tx.execute(
                        "INSERT INTO clips (
                            id, content, thumbnail, content_hash, content_type, timestamp,
                            is_pinned, pin_order, label, source_app, html
                         )
                         VALUES (?1, ?2, ?3, ?4, ?5, ?6, 1, ?7, ?8, 'ClipLeaf', ?9)",
                        params![
                            id,
                            content,
                            thumbnail,
                            content_hash,
                            content_type.as_db_value(),
                            current_timestamp(),
                            pin_order,
                            label,
                            html
                        ],
                    )
                    .map_err(|e| e.to_string())?;
                    id
                }
            };
            Self::sync_fts_for_clip_id_with_conn(&tx, &id).map_err(|e| e.to_string())?;
            imported += 1;
        }
        tx.commit().map_err(|e| e.to_string())?;
        Ok((imported, skipped))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::test_support::*;
    use std::fs;
    use uuid::Uuid;

    fn tiny_png() -> Vec<u8> {
        let mut bytes = std::io::Cursor::new(Vec::new());
        image::RgbaImage::new(2, 2)
            .write_to(&mut bytes, image::ImageFormat::Png)
            .unwrap();
        bytes.into_inner()
    }

    #[test]
    fn favorites_export_and_import_round_trip_and_skip_duplicates() {
        let source_root = std::env::temp_dir().join(format!("clipleaf-export-{}", Uuid::new_v4()));
        let target_root = std::env::temp_dir().join(format!("clipleaf-import-{}", Uuid::new_v4()));
        fs::create_dir(&source_root).unwrap();
        fs::create_dir(&target_root).unwrap();
        let source = ClipStorage::new(&source_root.join("clipleaf.db")).unwrap();
        source
            .create_pinned_text("first favorite", Some("label".into()))
            .unwrap();
        let mut files = test_item("files", br#"["/a.txt","/b c.txt"]"#, 2, true, Some(5));
        files.content_type = ContentType::Files;
        source.insert(&files, 10).unwrap();
        let mut image = test_item("image", &tiny_png(), 3, true, Some(6));
        image.content_type = ContentType::Image;
        image.thumbnail = Some(tiny_png());
        source.insert(&image, 10).unwrap();
        let mut rich = test_item("rich", b"rich favorite", 4, true, Some(7));
        rich.html = Some("<b>rich favorite</b>".into());
        source.insert(&rich, 10).unwrap();
        source
            .insert(&test_item("history", b"not pinned", 5, false, None), 10)
            .unwrap();

        let export_path = source_root.join("favorites.json");
        fs::write(&export_path, b"previous backup").unwrap();
        assert_eq!(4, source.export_pinned_to(&export_path).unwrap());
        let json = fs::read(&export_path).unwrap();
        let exported: FavoritesFile = serde_json::from_slice(&json).unwrap();
        assert_eq!("first favorite", exported.items[0].content);
        assert_eq!(Some("label".to_string()), exported.items[0].label);
        assert!(!fs::read_dir(&source_root).unwrap().any(|entry| entry
            .unwrap()
            .file_name()
            .to_string_lossy()
            .ends_with(".tmp")));

        let target = ClipStorage::new(&target_root.join("clipleaf.db")).unwrap();
        target
            .insert(
                &test_item("existing", b"first favorite", 1, false, None),
                10,
            )
            .unwrap();
        target
            .insert(&test_item("plain", b"rich favorite", 2, false, None), 10)
            .unwrap();
        let (imported, skipped) = target
            .import_pinned(serde_json::from_slice(&json).unwrap())
            .unwrap();
        assert_eq!((4, 0), (imported, skipped));
        let pinned = target.get_pinned_clip_previews(10).unwrap();
        assert_eq!(4, pinned.len());
        assert!(pinned.iter().any(|item| item.id == "existing"));
        let restored_image = pinned
            .iter()
            .find(|item| item.content_type == ContentType::Image)
            .unwrap();
        assert_eq!(Some(tiny_png()), restored_image.thumbnail);
        let merged_html: Option<String> = target
            .conn
            .query_row("SELECT html FROM clips WHERE id = 'plain'", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(Some("<b>rich favorite</b>".to_string()), merged_html);
        assert_eq!(1, target.search_clip_previews("b c.txt").unwrap().len());

        let (imported, skipped) = target
            .import_pinned(serde_json::from_slice(&json).unwrap())
            .unwrap();
        assert_eq!((0, 4), (imported, skipped));

        drop(source);
        drop(target);
        fs::remove_dir_all(&source_root).unwrap();
        fs::remove_dir_all(&target_root).unwrap();
    }

    #[test]
    fn favorites_import_rejects_invalid_files_without_writing() {
        let root = std::env::temp_dir().join(format!("clipleaf-import-bad-{}", Uuid::new_v4()));
        fs::create_dir(&root).unwrap();
        let storage = ClipStorage::new(&root.join("clipleaf.db")).unwrap();
        let file = |content_type: &str, content: &str| FavoritesFile {
            version: FAVORITES_FILE_VERSION,
            items: vec![
                ExportedClip {
                    content_type: "text".into(),
                    content: "valid".into(),
                    thumbnail: None,
                    label: None,
                    html: None,
                },
                ExportedClip {
                    content_type: content_type.into(),
                    content: content.into(),
                    thumbnail: None,
                    label: None,
                    html: None,
                },
            ],
        };
        for (content_type, content) in [
            ("files", "/a.txt\n/b.txt"),
            ("image", "not base64!"),
            ("image", "iVBORw=="),
            ("video", "x"),
        ] {
            assert!(storage.import_pinned(file(content_type, content)).is_err());
        }
        assert!(storage
            .import_pinned(FavoritesFile {
                version: FAVORITES_FILE_VERSION + 1,
                items: Vec::new(),
            })
            .is_err());
        assert!(storage.get_pinned_clip_previews(10).unwrap().is_empty());
        drop(storage);
        fs::remove_dir_all(&root).unwrap();
    }
}
