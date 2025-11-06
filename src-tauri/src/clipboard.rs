use arboard::{Clipboard, ImageData};
use std::thread;
use std::time::Duration;
use tauri::{AppHandle, Manager, Emitter};
use uuid::Uuid;
use chrono::Utc;

use crate::storage::{ClipItem, ContentType};

pub struct ClipboardMonitor {
    app_handle: AppHandle,
}

impl ClipboardMonitor {
    pub fn new(app_handle: AppHandle) -> Self {
        Self {
            app_handle,
        }
    }

    pub fn start(&self) {
        let app_handle = self.app_handle.clone();

        // Start monitoring thread
        thread::spawn(move || {
            let mut clipboard = Clipboard::new().unwrap();
            let mut last_text = String::new();
            let mut last_image: Option<Vec<u8>> = None;

            loop {
                // Check for text changes
                if let Ok(text) = clipboard.get_text() {
                    if text != last_text {
                        log::info!("Text clipboard changed: {} chars", text.len());
                        last_text = text.clone();

                        // Save to storage
                        let item = ClipItem {
                            id: Uuid::new_v4().to_string(),
                            content: text.into_bytes(),
                            content_type: ContentType::Text,
                            timestamp: Utc::now().timestamp(),
                            is_pinned: false,
                            pin_order: None,
                        };

                        Self::save_to_storage(&app_handle, item);
                    }
                }

                // Check for image changes
                if let Ok(image) = clipboard.get_image() {
                    let image_bytes = Self::image_to_bytes(&image);

                    if last_image.as_ref() != Some(&image_bytes) {
                        log::info!("Image clipboard changed: {} bytes", image_bytes.len());
                        last_image = Some(image_bytes.clone());

                        // Create thumbnail
                        let thumbnail = Self::create_thumbnail(&image_bytes);

                        let item = ClipItem {
                            id: Uuid::new_v4().to_string(),
                            content: thumbnail,
                            content_type: ContentType::Image,
                            timestamp: Utc::now().timestamp(),
                            is_pinned: false,
                            pin_order: None,
                        };

                        Self::save_to_storage(&app_handle, item);
                    }
                }

                thread::sleep(Duration::from_millis(500));
            }
        });
    }

    fn save_to_storage(app_handle: &AppHandle, item: ClipItem) {
        use crate::AppState;

        // Clone item for emission
        let item_for_emit = item.clone();

        let state = app_handle.state::<AppState>();
        let result = {
            let storage = state.storage.lock();
            if let Ok(storage) = storage {
                storage.insert(&item)
            } else {
                return;
            }
        };

        if let Err(e) = result {
            log::error!("Failed to save clipboard item: {}", e);
        } else {
            // Emit event to frontend
            app_handle.emit("clipboard-changed", &item_for_emit).ok();
        }
    }

    fn image_to_bytes(image: &ImageData) -> Vec<u8> {
        // Convert RGBA image to PNG bytes
        // This is a simplified version - in production use image crate
        image.bytes.to_vec()
    }

    fn create_thumbnail(image_bytes: &[u8]) -> Vec<u8> {
        // Create 128x128 thumbnail
        // This is a placeholder - in production use image crate for proper resizing
        if image_bytes.len() > 10000 {
            image_bytes[..10000].to_vec()
        } else {
            image_bytes.to_vec()
        }
    }
}
