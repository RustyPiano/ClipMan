use lru::LruCache;
use std::num::NonZeroUsize;
use std::sync::Mutex;
use tauri::menu::{
    CheckMenuItemBuilder, IconMenuItemBuilder, MenuBuilder, MenuEvent, MenuItemBuilder,
};
use tauri::{AppHandle, Emitter, Manager};

use crate::storage::{join_file_paths, split_file_paths, ClipPreviewItem, ContentType};
use crate::AppState;

pub const TRAY_ICON_SIZE: u32 = 32;
const ICON_CACHE_SIZE: usize = 50;
pub const TRAY_ID: &str = "main";

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
        if let Some(icon) = self.cache.lock().unwrap().get(id) {
            return Some(icon.clone());
        }

        match image::load_from_memory(content) {
            Ok(img) => {
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

                let icon = tauri::image::Image::new_owned(rgba, width, height);

                self.cache.lock().unwrap().put(id.to_string(), icon.clone());

                Some(icon)
            }
            Err(e) => {
                log::warn!("Failed to decode image for clip {}: {}", id, e);
                None
            }
        }
    }

    pub fn clear(&self) {
        self.cache.lock().unwrap().clear();
        log::info!("Icon cache cleared");
    }
}

pub fn truncate_content(
    content: &[u8],
    content_type: &ContentType,
    max_len: usize,
    i18n: &TrayI18n,
) -> String {
    if *content_type == ContentType::Image {
        return i18n.image.to_string();
    }
    let text = String::from_utf8_lossy(content);
    let text = if *content_type == ContentType::Files {
        join_file_paths(&split_file_paths(&text))
    } else {
        text.into_owned()
    };
    let text = text.split_whitespace().collect::<Vec<_>>().join(" ");

    let char_count = text.chars().count();
    if char_count > max_len {
        if max_len <= 3 {
            return text.chars().take(max_len).collect();
        }
        let start_len = ((max_len - 3) * 2 / 3).max(1);
        let end_len = max_len - start_len - 3;
        format!(
            "{}...{}",
            text.chars().take(start_len).collect::<String>(),
            text.chars().skip(char_count - end_len).collect::<String>()
        )
    } else {
        text
    }
}

fn add_clip_menu_item(
    app: &AppHandle,
    item: &ClipPreviewItem,
    icon_cache: &TrayIconCache,
    max_len: usize,
    i18n: &TrayI18n,
) -> Result<Box<dyn tauri::menu::IsMenuItem<tauri::Wry>>, tauri::Error> {
    let preview = truncate_content(&item.preview_content, &item.content_type, max_len, i18n);

    let icon = matches!(item.content_type, ContentType::Image)
        .then(|| item.thumbnail.as_deref())
        .flatten()
        .and_then(|content| icon_cache.get_or_create(&item.id, content));

    Ok(match icon {
        Some(icon) => Box::new(
            IconMenuItemBuilder::with_id(format!("clip:{}", item.id), preview)
                .icon(icon)
                .build(app)?,
        ),
        None => {
            Box::new(MenuItemBuilder::with_id(format!("clip:{}", item.id), preview).build(app)?)
        }
    })
}

pub fn build_tray_menu(
    app: &AppHandle,
) -> Result<tauri::menu::Menu<tauri::Wry>, Box<dyn std::error::Error>> {
    let state = app.state::<AppState>();

    let settings = state.settings.get();
    let max_pinned_in_tray = settings.max_pinned_in_tray;
    let max_recent_in_tray = settings.max_recent_in_tray;
    let max_len = settings.tray_text_length;
    let i18n = TrayI18n::new(&settings.locale);

    let (pinned_items, recent_items) = {
        let storage = state.storage.lock().unwrap();
        let pinned_items = if max_pinned_in_tray == 0 {
            Vec::new()
        } else {
            storage.get_pinned_clip_previews(max_pinned_in_tray as i64)?
        };
        let recent_items = if max_recent_in_tray == 0 {
            Vec::new()
        } else {
            storage.get_recent_clip_previews(max_recent_in_tray)?
        };
        (pinned_items, recent_items)
    };

    let mut menu_builder = MenuBuilder::new(app);

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

    let pause_capture_item = CheckMenuItemBuilder::with_id("pause_capture", i18n.pause_capture)
        .checked(settings.capture_paused)
        .build(app)?;

    menu_builder = menu_builder
        .separator()
        .item(&MenuItemBuilder::with_id("clear_non_pinned", i18n.clear).build(app)?)
        .item(&pause_capture_item)
        .item(&MenuItemBuilder::with_id("settings", i18n.settings).build(app)?)
        .item(&MenuItemBuilder::with_id("quit", i18n.quit).build(app)?);

    Ok(menu_builder.build()?)
}

pub fn handle_tray_menu_event(app: &AppHandle, event: MenuEvent) {
    let event_id = event.id().as_ref();
    log::debug!("Menu event: {}", event_id);

    match event_id {
        "quit" => {
            log::info!("Quit menu clicked");
            app.exit(0);
        }
        "clear_non_pinned" => {
            log::info!("Clear non-pinned menu clicked");
            let app_clone = app.clone();
            tauri::async_runtime::spawn(async move {
                if let Err(e) =
                    crate::commands::clear_non_pinned_history(app_clone.clone(), app_clone.state())
                        .await
                {
                    log::error!("Failed to clear non-pinned history: {}", e);
                }
            });
        }
        "settings" => {
            log::info!("Settings menu clicked");
            if let Err(e) = crate::window::open_settings_window(app) {
                log::error!("Failed to open settings window: {}", e);
            }
        }
        "pause_capture" => {
            let app = app.clone();
            tauri::async_runtime::spawn_blocking(move || {
                let state: tauri::State<AppState> = app.state();
                let _settings_write_guard = state.settings_write_lock.lock().unwrap();

                let mut settings = state.settings.get();
                settings.capture_paused = !settings.capture_paused;
                let now_paused = settings.capture_paused;
                if let Err(e) = state.settings.save(&app, &settings) {
                    log::error!("Failed to persist capture_paused toggle: {}", e);
                }
                state.settings.set(settings);
                log::info!(
                    "Clipboard capture {} via tray menu",
                    if now_paused { "paused" } else { "resumed" }
                );

                let _ = app.emit("settings-changed", ());
                update_tray_menu(&app);
            });
        }

        id if id.starts_with("clip:") => {
            let clip_id = id.strip_prefix("clip:").unwrap().to_string();
            log::info!("Clip item clicked: {}", clip_id);

            let app_clone = app.clone();
            tauri::async_runtime::spawn(async move {
                if let Err(e) =
                    crate::commands::copy_clip_to_clipboard_internal(&app_clone, &clip_id, true)
                        .await
                {
                    log::error!("Failed to copy clip: {}", e);
                }
            });
        }
        _ => {
            log::debug!("Unhandled menu event: {}", event_id);
        }
    }
}

pub fn update_tray_menu(app: &AppHandle) {
    let result = build_tray_menu(app).and_then(|menu| {
        if let Some(tray) = app.tray_by_id(TRAY_ID) {
            tray.set_menu(Some(menu))?;
        }
        Ok(())
    });
    if let Err(error) = result {
        log::error!("Failed to refresh tray menu; keeping previous menu: {error}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::ContentType;

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
        let content = crate::storage::encode_file_paths(&[
            "/Users/alice/a\nfile.txt".into(),
            "/Users/alice/b.png".into(),
        ]);
        let result = truncate_content(content.as_bytes(), &ContentType::Files, 100, &i18n);

        assert_eq!("/Users/alice/a file.txt /Users/alice/b.png", result);
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
