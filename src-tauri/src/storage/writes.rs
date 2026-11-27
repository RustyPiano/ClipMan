use rusqlite::{params, Connection, OptionalExtension, Result};

use super::{
    current_timestamp, hash_bytes, normalize_label, ClipItem, ClipPreviewItem, ClipStorage,
    ContentType, MAX_TEXT_DETAIL_BYTES,
};

impl ClipStorage {
    pub fn insert(&self, item: &ClipItem, max_history_items: usize) -> Result<Option<String>> {
        let tx = self.conn.unchecked_transaction()?;
        let result = Self::insert_with_conn(&tx, item, max_history_items)?;
        tx.commit()?;
        self.reclaim_space();
        Ok(result)
    }

    pub(super) fn insert_with_conn(
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

    pub fn clear_non_pinned(&self) -> Result<bool> {
        log::info!("Clearing non-pinned clipboard history");
        let tx = self.conn.unchecked_transaction()?;
        tx.execute(
            "DELETE FROM clips_fts
             WHERE clip_id IN (SELECT id FROM clips WHERE is_pinned = 0)",
            [],
        )?;
        tx.execute("DELETE FROM clips WHERE is_pinned = 0", [])?;
        tx.execute("INSERT INTO clips_fts(clips_fts) VALUES('optimize')", [])?;
        tx.commit()?;
        self.reclaim_space();
        let busy: i64 = self
            .conn
            .query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |row| row.get(0))?;
        Ok(busy == 0)
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

    pub fn enforce_history_limit(&self, limit: usize) -> Result<usize> {
        let tx = self.conn.unchecked_transaction()?;
        let removed = Self::prune_history_with_conn(&tx, limit)?;
        tx.commit()?;
        if removed > 0 {
            self.reclaim_space();
        }
        Ok(removed)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::test_support::*;
    use std::fs;
    use uuid::Uuid;

    #[test]
    fn history_limit_applies_to_recopies_unpins_and_delayed_images() {
        let root = std::env::temp_dir().join(format!("clipleaf-limit-{}", Uuid::new_v4()));
        fs::create_dir(&root).unwrap();
        let storage = ClipStorage::new(&root.join("clipleaf.db")).unwrap();
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
        assert!(storage.clear_non_pinned().unwrap());

        let blocks_with_cleared_text: i64 = storage
            .conn
            .query_row(
                "SELECT COUNT(*) FROM clips_fts_data WHERE instr(block, 'recent') > 0",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(0, blocks_with_cleared_text);

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
    fn clear_non_pinned_reports_a_wal_held_by_another_reader() {
        let db_path = temp_db_path("clear_non_pinned_busy_wal");
        let storage = ClipStorage::new(&db_path).unwrap();
        storage
            .insert(&test_item("recent", b"recent text", 1, false, None), 100)
            .unwrap();

        let reader = Connection::open(&db_path).unwrap();
        reader.execute_batch("BEGIN").unwrap();
        let _: i64 = reader
            .query_row("SELECT COUNT(*) FROM clips", [], |row| row.get(0))
            .unwrap();
        assert!(!storage.clear_non_pinned().unwrap());

        reader.execute_batch("COMMIT").unwrap();
        assert!(storage.clear_non_pinned().unwrap());
        drop(reader);
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
}
