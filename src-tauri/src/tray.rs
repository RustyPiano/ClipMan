use lru::LruCache;
use std::num::NonZeroUsize;
use std::sync::Mutex;
use tauri::menu::{
    CheckMenuItemBuilder, IconMenuItemBuilder, MenuBuilder, MenuEvent, MenuItemBuilder,
};
use tauri::{AppHandle, Emitter, Manager};

use crate::storage::{join_file_paths, split_file_paths, ClipPreviewItem, ContentType};
use crate::AppState;
#[cfg(not(target_os = "linux"))]
use tauri_plugin_notification::NotificationExt;

pub const TRAY_ICON_SIZE: u32 = 32;
const ICON_CACHE_SIZE: usize = 50;
pub const TRAY_ID: &str = "main";

pub struct TrayI18n {
    pub open_quickbar: &'static str,
    pub install_update: &'static str,
    pub restart_to_update: &'static str,
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
                open_quickbar: "打开 ClipLeaf",
                install_update: "安装新版本",
                restart_to_update: "重启以完成更新",
                pinned_header: "收藏",
                recent_header: "最近复制",
                image: "图片",
                clear: "清除",
                pause_capture: "暂停采集",
                settings: "设置",
                quit: "退出",
            }
        } else {
            Self {
                open_quickbar: "Open ClipLeaf",
                install_update: "Install version",
                restart_to_update: "Restart to Finish Update",
                pinned_header: "Favorites",
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

        let img = image::load_from_memory(content)
            .map_err(|e| log::warn!("Failed to decode image for clip {}: {}", id, e))
            .ok()?;
        let (orig_width, orig_height) = (img.width(), img.height());
        let min_side = orig_width.min(orig_height);
        let scale = TRAY_ICON_SIZE as f32 / min_side as f32;

        let new_width = (orig_width as f32 * scale) as u32;
        let new_height = (orig_height as f32 * scale) as u32;

        let rgba = img
            .resize_exact(new_width, new_height, image::imageops::FilterType::Lanczos3)
            .to_rgba8()
            .into_raw();
        let icon = tauri::image::Image::new_owned(rgba, new_width, new_height);

        self.cache.lock().unwrap().put(id.to_string(), icon.clone());

        Some(icon)
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
        (
            storage.get_pinned_clip_previews(max_pinned_in_tray as i64)?,
            storage.get_recent_clip_previews(max_recent_in_tray)?,
        )
    };

    let mut menu_builder = MenuBuilder::new(app)
        .item(&MenuItemBuilder::with_id("open_quickbar", i18n.open_quickbar).build(app)?);
    if crate::updates::is_ready_to_restart(app) {
        menu_builder = menu_builder.item(
            &MenuItemBuilder::with_id("restart_to_update", i18n.restart_to_update).build(app)?,
        );
    } else if let Some(version) = crate::updates::pending_version(app) {
        menu_builder = menu_builder.item(
            &MenuItemBuilder::with_id(
                "install_update",
                format!("{} {version}", i18n.install_update),
            )
            .build(app)?,
        );
    }
    menu_builder = menu_builder.separator();

    for (header_id, header_text, items) in [
        ("pinned_header", i18n.pinned_header, &pinned_items),
        ("recent_header", i18n.recent_header, &recent_items),
    ] {
        if items.is_empty() {
            continue;
        }
        let header = MenuItemBuilder::with_id(header_id, header_text)
            .enabled(false)
            .build(app)?;
        menu_builder = menu_builder.item(&header);

        for item in items {
            let menu_item = add_clip_menu_item(app, item, &state.icon_cache, max_len, &i18n)?;
            menu_builder = menu_builder.item(&*menu_item);
        }

        menu_builder = menu_builder.separator();
    }

    let pause_capture_item = CheckMenuItemBuilder::with_id("pause_capture", i18n.pause_capture)
        .checked(settings.capture_paused)
        .build(app)?;

    menu_builder = menu_builder
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
        "open_quickbar" => {
            log::info!("Open QuickBar menu clicked");
            let state = app.state::<AppState>();
            if let Err(e) = crate::window::show_quickbar(
                app,
                &state.quickbar_foreground_window,
                &state.quickbar_panel,
            ) {
                log::error!("Failed to show QuickBar from tray: {}", e);
            }
        }
        "quit" => {
            log::info!("Quit menu clicked");
            app.exit(0);
        }
        "clear_non_pinned" => {
            log::info!("Clear non-pinned menu clicked");
            let app_clone = app.clone();
            tauri::async_runtime::spawn(async move {
                match crate::commands::clear_non_pinned_history(
                    app_clone.clone(),
                    app_clone.state(),
                )
                .await
                {
                    Ok(None) => {}
                    Ok(Some(warning)) => log::warn!("{warning}"),
                    Err(e) => log::error!("Failed to clear non-pinned history: {}", e),
                }
            });
        }
        "restart_to_update" => app.restart(),
        "settings" | "install_update" => {
            let tab = (event_id == "install_update").then_some("about");
            if let Err(e) = crate::window::open_settings_window(app, tab) {
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
                match crate::paste::copy_clip_to_clipboard_internal(&app_clone, &clip_id).await {
                    Ok(_) => notify_copied(&app_clone),
                    Err(e) => log::error!("Failed to copy clip: {}", e),
                }
            });
        }
        _ => {
            log::debug!("Unhandled menu event: {}", event_id);
        }
    }
}

#[cfg(not(target_os = "linux"))]
fn notify_copied(app: &AppHandle) {
    let chinese = app.state::<AppState>().settings.get().locale == "zh-CN";
    let body = if chinese {
        "内容已复制到剪贴板"
    } else {
        "Content copied to clipboard"
    };
    let _ = app
        .notification()
        .builder()
        .title(if chinese { "已复制" } else { "Copied" })
        .body(body)
        .show();
}

#[cfg(target_os = "linux")]
fn notify_copied(_app: &AppHandle) {}

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
        let content = b"Line 1\nLine 2\r\nLine 3";
        let result = truncate_content(content, &ContentType::Text, 50, &i18n);
        assert_eq!(result, "Line 1 Line 2 Line 3");
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
    fn test_truncate_content_files_renders_path_text() {
        let i18n = TrayI18n::new("en");
        let content = crate::storage::encode_file_paths(&[
            "/Users/alice/a\nfile.txt".into(),
            "/Users/alice/b.png".into(),
        ]);
        let result = truncate_content(content.as_bytes(), &ContentType::Files, 100, &i18n);

        assert_eq!("/Users/alice/a file.txt /Users/alice/b.png", result);
    }
}
