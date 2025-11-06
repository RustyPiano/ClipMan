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
            log::info!("Clipboard monitoring thread started");

            let mut clipboard = match Clipboard::new() {
                Ok(cb) => cb,
                Err(e) => {
                    log::error!("Failed to create clipboard instance: {}", e);
                    return;
                }
            };

            let mut last_text = String::new();
            let mut last_image: Option<Vec<u8>> = None;

            loop {
                // Check for text changes
                if let Ok(text) = clipboard.get_text() {
                    if text != last_text && !text.is_empty() {
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

            // Update tray menu to show new item
            crate::update_tray_menu(app_handle);
            log::debug!("Tray menu updated after clipboard change");
        }
    }

    fn image_to_bytes(image: &ImageData) -> Vec<u8> {
        // Convert RGBA ImageData to PNG bytes using image crate
        use image::{ImageBuffer, RgbaImage};

        let img: RgbaImage = match ImageBuffer::from_raw(
            image.width as u32,
            image.height as u32,
            image.bytes.to_vec()
        ) {
            Some(img) => img,
            None => {
                log::error!("Failed to create image buffer");
                return image.bytes.to_vec(); // Fallback to raw bytes
            }
        };

        // Encode as PNG
        let mut png_bytes = Vec::new();
        if let Err(e) = img.write_to(&mut std::io::Cursor::new(&mut png_bytes), image::ImageFormat::Png) {
            log::error!("Failed to encode PNG: {}", e);
            return image.bytes.to_vec(); // Fallback to raw bytes
        }

        png_bytes
    }

    fn create_thumbnail(image_bytes: &[u8]) -> Vec<u8> {
        use image::{GenericImageView, imageops::FilterType, ImageFormat};

        // Try to decode the image
        let img = match image::load_from_memory(image_bytes) {
            Ok(img) => img,
            Err(e) => {
                log::warn!("Failed to decode image for thumbnail: {}. Using first 10KB as fallback", e);
                // Fallback: return truncated raw bytes
                return if image_bytes.len() > 10000 {
                    image_bytes[..10000].to_vec()
                } else {
                    image_bytes.to_vec()
                };
            }
        };

        // Calculate thumbnail dimensions (max 256x256, preserve aspect ratio)
        let (width, height) = img.dimensions();
        let max_size = 256u32;
        let (thumb_width, thumb_height) = if width > height {
            let ratio = max_size as f32 / width as f32;
            (max_size, (height as f32 * ratio) as u32)
        } else {
            let ratio = max_size as f32 / height as f32;
            ((width as f32 * ratio) as u32, max_size)
        };

        // Resize image
        let thumbnail = img.resize(thumb_width, thumb_height, FilterType::Lanczos3);

        // Encode as PNG
        let mut png_bytes = Vec::new();
        if let Err(e) = thumbnail.write_to(
            &mut std::io::Cursor::new(&mut png_bytes),
            ImageFormat::Png
        ) {
            log::error!("Failed to encode thumbnail: {}", e);
            // Fallback to truncated original
            return if image_bytes.len() > 10000 {
                image_bytes[..10000].to_vec()
            } else {
                image_bytes.to_vec()
            };
        }

        log::info!("Created thumbnail: {}x{} -> {}x{}, {} bytes",
                  width, height, thumb_width, thumb_height, png_bytes.len());

        png_bytes
    }
}
