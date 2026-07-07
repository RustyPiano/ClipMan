// Tray menu management module
use lru::LruCache;
use std::num::NonZeroUsize;
use std::sync::Mutex;
use tauri::menu::{CheckMenuItemBuilder, IconMenuItemBuilder, MenuBuilder, MenuItemBuilder};
use tauri::{AppHandle, Manager};

use crate::storage::{ClipPreviewItem, ContentType};
use crate::AppState;

// Tray configuration constants
pub const TRAY_ICON_SIZE: u32 = 32;
const ICON_CACHE_SIZE: usize = 50;

/// Tray menu translations
pub struct TrayI18n {
    pub pinned_header: &'static str,
    pub recent_header: &'static str,
    pub image: &'static str,
    pub clear: &'static str,
    pub pause_capture: &'static str,
    pub settings: &'static str,
    pub quit: &'static str,
}

impl TrayI18n {
    pub fn new(locale: &str) -> Self {
        if locale.starts_with("zh") {
            Self {
                pinned_header: "置顶项",
                recent_header: "最近复制",
                image: "图片",
                clear: "清除",
                pause_capture: "暂停采集",
                settings: "设置",
                quit: "退出",
            }
        } else {
            Self {
                pinned_header: "Pinned",
                recent_header: "Recent",
                image: "Image",
                clear: "Clear",
                pause_capture: "Pause Capture",
                settings: "Settings",
                quit: "Quit",
            }
        }
    }
}

/// Icon cache for tray menu images
pub struct TrayIconCache {
    cache: Mutex<LruCache<String, tauri::image::Image<'static>>>,
}

impl Default for TrayIconCache {
    fn default() -> Self {
        Self::new()
    }
}

impl TrayIconCache {
    pub fn new() -> Self {
        Self {
            cache: Mutex::new(LruCache::new(NonZeroUsize::new(ICON_CACHE_SIZE).unwrap())),
        }
    }

    pub fn get_or_create(&self, id: &str, content: &[u8]) -> Option<tauri::image::Image<'static>> {
        // Check cache first
        {
            let mut cache = self.cache.lock().unwrap();
            if let Some(icon) = cache.get(id) {
                log::debug!("🎯 Icon cache hit for {}", id);
                return Some(icon.clone());
            }
        }

        // Cache miss - decode and resize image
        log::debug!("📸 Icon cache miss for {}, decoding...", id);
        match image::load_from_memory(content) {
            Ok(img) => {
                // Resize so shortest side is TRAY_ICON_SIZE, preserving aspect ratio
                let (orig_width, orig_height) = (img.width(), img.height());
                let min_side = orig_width.min(orig_height);
                let scale = TRAY_ICON_SIZE as f32 / min_side as f32;

                let new_width = (orig_width as f32 * scale) as u32;
                let new_height = (orig_height as f32 * scale) as u32;

                let resized =
                    img.resize_exact(new_width, new_height, image::imageops::FilterType::Lanczos3);
                let width = resized.width();
                let height = resized.height();
                let rgba = resized.to_rgba8().into_raw();

                // Create owned image for caching
                let icon = tauri::image::Image::new_owned(rgba, width, height);

                // Cache it
                {
                    let mut cache = self.cache.lock().unwrap();
                    cache.put(id.to_string(), icon.clone());
                }

                Some(icon)
            }
            Err(e) => {
                log::warn!("Failed to decode image for clip {}: {}", id, e);
                None
            }
        }
    }

    pub fn clear(&self) {
        let mut cache = self.cache.lock().unwrap();
        cache.clear();
        log::info!("Icon cache cleared");
    }
}

/// Truncate content for menu display (handles Unicode safely)
pub fn truncate_content(
    content: &[u8],
    content_type: &ContentType,
    max_len: usize,
    i18n: &TrayI18n,
) -> String {
    match content_type {
        // Files store their paths as newline-joined text; the newline→space
        // collapse below makes the path list render on one readable line.
        ContentType::Text | ContentType::Files => {
            let text = String::from_utf8_lossy(content);
            // Replace newlines and carriage returns, then collapse whitespace
            let text: String = text
                .chars()
                .map(|c| if c == '\n' || c == '\r' { ' ' } else { c })
                .collect::<String>()
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ");

            // Smart truncation: show start...end for long text
            let char_count = text.chars().count();
            if char_count > max_len {
                if max_len <= 3 {
                    return text.chars().take(max_len).collect();
                }

                let available = max_len - 3;
                let start_len = (available * 2 / 3).max(1);
                let end_len = max_len - start_len - 3;

                let start: String = text.chars().take(start_len).collect();
                let end: String = text.chars().skip(char_count - end_len).collect();
                format!("{}...{}", start, end)
            } else {
                text
            }
        }
        ContentType::Image => i18n.image.to_string(),
    }
}

/// Helper function to add a clip menu item
fn add_clip_menu_item(
    app: &AppHandle,
    item: &ClipPreviewItem,
    icon_cache: &TrayIconCache,
    max_len: usize,
    i18n: &TrayI18n,
) -> Result<Box<dyn tauri::menu::IsMenuItem<tauri::Wry>>, tauri::Error> {
    let preview = truncate_content(&item.preview_content, &item.content_type, max_len, i18n);

    if matches!(item.content_type, ContentType::Image) {
        if let Some(icon) = item
            .thumbnail
            .as_deref()
            .and_then(|content| icon_cache.get_or_create(&item.id, content))
        {
            let menu_item = IconMenuItemBuilder::with_id(format!("clip:{}", item.id), preview)
                .icon(icon)
                .build(app)?;
            Ok(Box::new(menu_item))
        } else {
            let menu_item =
                MenuItemBuilder::with_id(format!("clip:{}", item.id), preview).build(app)?;
            Ok(Box::new(menu_item))
        }
    } else {
        let menu_item =
            MenuItemBuilder::with_id(format!("clip:{}", item.id), preview).build(app)?;
        Ok(Box::new(menu_item))
    }
}

/// Build dynamic tray menu
pub fn build_tray_menu(app: &AppHandle) -> Result<tauri::menu::Menu<tauri::Wry>, tauri::Error> {
    let state = app.state::<AppState>();

    // Get settings for tray menu limits
    let settings = state.settings.get();
    let max_pinned_in_tray = settings.max_pinned_in_tray;
    let max_recent_in_tray = settings.max_recent_in_tray;
    let max_len = settings.tray_text_length;
    let i18n = TrayI18n::new(&settings.locale);

    // Quick lock acquisition - get data and release immediately
    let (pinned_items, recent_items) = {
        let storage = crate::safe_lock(&state.storage);
        let pinned_items = if max_pinned_in_tray == 0 {
            Vec::new()
        } else {
            storage
                .get_pinned_clip_previews_with_limit(max_pinned_in_tray)
                .unwrap_or_default()
        };
        let recent_items = if max_recent_in_tray == 0 {
            Vec::new()
        } else {
            storage
                .get_recent_clip_previews(max_recent_in_tray)
                .unwrap_or_default()
        };
        (pinned_items, recent_items)
    };

    let mut menu_builder = MenuBuilder::new(app);

    // Add pinned items
    let pinned_count = pinned_items.len();
    if pinned_count > 0 {
        let pinned_header = MenuItemBuilder::with_id("pinned_header", i18n.pinned_header)
            .enabled(false)
            .build(app)?;
        menu_builder = menu_builder.item(&pinned_header);

        for item in &pinned_items {
            let menu_item = add_clip_menu_item(app, item, &state.icon_cache, max_len, &i18n)?;
            menu_builder = menu_builder.item(&*menu_item);
        }

        menu_builder = menu_builder.separator();
    }

    // Add recent items (excluding pinned)
    if !recent_items.is_empty() {
        let recent_header = MenuItemBuilder::with_id("recent_header", i18n.recent_header)
            .enabled(false)
            .build(app)?;
        menu_builder = menu_builder.item(&recent_header);

        for item in &recent_items {
            let menu_item = add_clip_menu_item(app, item, &state.icon_cache, max_len, &i18n)?;
            menu_builder = menu_builder.item(&*menu_item);
        }
    }

    // Bottom actions. The pause-capture item is a checkbox reflecting
    // `capture_paused`; main.rs's menu event handler flips the setting,
    // persists it, and rebuilds this menu so the check mark stays in sync.
    let pause_capture_item = CheckMenuItemBuilder::with_id("pause_capture", i18n.pause_capture)
        .checked(settings.capture_paused)
        .build(app)?;

    menu_builder = menu_builder
        .separator()
        .item(&MenuItemBuilder::with_id("clear_non_pinned", i18n.clear).build(app)?)
        .item(&pause_capture_item)
        .item(&MenuItemBuilder::with_id("settings", i18n.settings).build(app)?)
        .item(&MenuItemBuilder::with_id("quit", i18n.quit).build(app)?);

    menu_builder.build()
}

/// Update tray menu
pub fn update_tray_menu(app: &AppHandle) {
    if let Ok(new_menu) = build_tray_menu(app) {
        if let Some(tray) = app.tray_by_id("main") {
            if let Err(e) = tray.set_menu(Some(new_menu)) {
                log::error!("Failed to update tray menu: {}", e);
            } else {
                log::debug!("Tray menu updated successfully");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::ContentType;

    #[test]
    fn test_tray_i18n_chinese() {
        let i18n = TrayI18n::new("zh-CN");
        assert_eq!(i18n.pinned_header, "置顶项");
        assert_eq!(i18n.recent_header, "最近复制");
        assert_eq!(i18n.pause_capture, "暂停采集");
        assert_eq!(i18n.quit, "退出");
    }

    #[test]
    fn test_tray_i18n_english() {
        let i18n = TrayI18n::new("en");
        assert_eq!(i18n.pinned_header, "Pinned");
        assert_eq!(i18n.recent_header, "Recent");
        assert_eq!(i18n.quit, "Quit");
    }

    #[test]
    fn test_truncate_content_text() {
        let i18n = TrayI18n::new("en");
        let content = b"Hello, World!";
        let result = truncate_content(content, &ContentType::Text, 50, &i18n);
        assert_eq!(result, "Hello, World!");
    }

    #[test]
    fn test_truncate_content_long_text() {
        let i18n = TrayI18n::new("en");
        let content = b"This is a very long text that should be truncated";
        let result = truncate_content(content, &ContentType::Text, 20, &i18n);
        assert!(result.contains("..."));
        assert!(result.len() <= 20);
    }

    #[test]
    fn test_truncate_content_tiny_limits_do_not_underflow() {
        let i18n = TrayI18n::new("en");
        let content = b"abcdefg";

        let tiny = truncate_content(content, &ContentType::Text, 3, &i18n);
        let compact = truncate_content(content, &ContentType::Text, 6, &i18n);

        assert_eq!("abc", tiny);
        assert_eq!("ab...g", compact);
    }

    #[test]
    fn test_truncate_content_image() {
        let i18n_zh = TrayI18n::new("zh-CN");
        let i18n_en = TrayI18n::new("en");

        let result_zh = truncate_content(b"", &ContentType::Image, 50, &i18n_zh);
        let result_en = truncate_content(b"", &ContentType::Image, 50, &i18n_en);

        assert_eq!(result_zh, "图片");
        assert_eq!(result_en, "Image");
    }

    #[test]
    fn test_truncate_content_files_renders_path_text() {
        let i18n = TrayI18n::new("en");
        let content = b"/Users/alice/a.txt\n/Users/alice/b.png";
        let result = truncate_content(content, &ContentType::Files, 100, &i18n);

        // Path list is shown as readable text (newline collapsed to a space),
        // not the generic image placeholder.
        assert_eq!("/Users/alice/a.txt /Users/alice/b.png", result);
    }

    #[test]
    fn test_truncate_content_newlines() {
        let i18n = TrayI18n::new("en");
        let content = b"Line 1\nLine 2\r\nLine 3";
        let result = truncate_content(content, &ContentType::Text, 50, &i18n);
        assert!(!result.contains('\n'));
        assert!(!result.contains('\r'));
    }
}
