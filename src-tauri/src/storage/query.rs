use rusqlite::{params, OptionalExtension, Result, Row};

use super::fts::FILES_TEXT_SQL;
use super::{
    join_file_paths, png_data_url, split_file_paths, string_to_rusqlite_error, ClipFilter,
    ClipItem, ClipPreviewItem, ClipStorage, ContentType, FrontendClipDetail, MAX_TEXT_DETAIL_BYTES,
};

const CLIP_COLUMNS: &str =
    "id, content, thumbnail, content_type, timestamp, is_pinned, pin_order, label, source_app, html";
pub(super) const TEXT_HEAD_PREVIEW: &str = "substr(content, 1, 4096)";
const TEXT_MATCH_EXCERPT: &str = "CAST(substr(CAST(content AS TEXT), max(1, instr(lower(CAST(content AS TEXT)), lower(?2)) - 64), 512) AS BLOB)";

fn preview_columns(text_preview: &str) -> String {
    format!(
        "id,
     CASE WHEN content_type='text' THEN {text_preview}
       WHEN content_type='files' THEN CAST((SELECT json_group_array(substr(value, 1, 4096))
         FROM json_each(CAST(content AS TEXT)) WHERE key < 2) AS BLOB)
       ELSE x'' END AS preview_content,
     thumbnail, content_type, timestamp, is_pinned, pin_order, label, source_app,
     (html IS NOT NULL) AS has_html, length(content) AS content_bytes,
     CASE WHEN content_type='files' THEN json_array_length(CAST(content AS TEXT)) ELSE 0 END AS file_count,
     ({}) AS is_link",
        ClipFilter::Link.sql_clause()
    )
}

impl ClipFilter {
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

fn escape_fts_query(query: &str) -> String {
    format!("\"{}\"", query.replace('"', "\"\""))
}

fn escape_like_query(query: &str) -> String {
    query
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}

impl ClipStorage {
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
                "SELECT content_type, length(content), CASE
                   WHEN content_type='image' THEN CASE WHEN length(content)<=?3 THEN content ELSE x'' END
                   WHEN content_type='files' THEN content
                   ELSE substr(content,1,?2) END
                 FROM clips WHERE id=?1",
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
        let detail = match content_type {
            ContentType::Image => {
                if size > IMAGE_LIMIT {
                    return Err("Image exceeds the 16 MiB preview limit".into());
                }
                FrontendClipDetail {
                    id: id.into(),
                    content_type,
                    text: String::new(),
                    file_paths: Vec::new(),
                    image_url: Some(png_data_url(&bytes)),
                    truncated: false,
                }
            }
            ContentType::Files => {
                let paths = split_file_paths(&String::from_utf8_lossy(&bytes));
                let total = paths.len();
                let mut file_paths = Vec::new();
                let mut text_bytes = 0;
                for path in paths {
                    let added = usize::from(!file_paths.is_empty()) + path.len();
                    if text_bytes + added > MAX_TEXT_DETAIL_BYTES {
                        break;
                    }
                    text_bytes += added;
                    file_paths.push(path);
                }
                FrontendClipDetail {
                    id: id.into(),
                    content_type,
                    text: join_file_paths(&file_paths),
                    truncated: file_paths.len() < total,
                    file_paths,
                    image_url: None,
                }
            }
            ContentType::Text => {
                let truncated = size > bytes.len();
                let end = if truncated {
                    std::str::from_utf8(&bytes).map_or_else(|e| e.valid_up_to(), |_| bytes.len())
                } else {
                    bytes.len()
                };
                FrontendClipDetail {
                    id: id.into(),
                    content_type,
                    text: String::from_utf8_lossy(&bytes[..end]).into_owned(),
                    file_paths: Vec::new(),
                    image_url: None,
                    truncated,
                }
            }
        };
        Ok(Some(detail))
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
            is_link: row.get(12)?,
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::encode_file_paths;
    use crate::storage::test_support::*;
    use std::fs;
    use uuid::Uuid;

    #[test]
    fn json_file_paths_and_bounded_merge_reads_preserve_content() {
        let root = std::env::temp_dir().join(format!("clipleaf-paths-{}", Uuid::new_v4()));
        fs::create_dir(&root).unwrap();
        let storage = ClipStorage::new(&root.join("clipleaf.db")).unwrap();
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
        assert_eq!(
            storage.get_detail("files").unwrap().unwrap().file_paths,
            paths
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
                &test_item("url2", b"HTTPS://CLIPLEAF.APP/alpha", 5, false, None),
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

        let long_url = format!("https://example.com/{}/tailneedle", "segment".repeat(40));
        storage
            .insert(
                &test_item("url-long", long_url.as_bytes(), 8, false, None),
                100,
            )
            .unwrap();
        let tail_hits = storage
            .search_clip_previews_filtered("tailneedle", ClipFilter::All)
            .unwrap();
        assert_eq!(tail_hits.len(), 1);
        assert!(!tail_hits[0].preview_content.starts_with(b"https://"));
        assert!(tail_hits[0].is_link);
        assert!(!storage
            .search_clip_previews_filtered("alpha", ClipFilter::All)
            .unwrap()
            .iter()
            .find(|item| item.id == "url-multi" || item.id == "t1")
            .is_some_and(|item| item.is_link));

        drop(storage);
        cleanup_db(&db_path);
    }
}
