use arboard::{Clipboard, ImageData};
use std::thread;
use std::time::Duration;
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Manager, Emitter};
use uuid::Uuid;
use chrono::Utc;
use image::GenericImageView;

use crate::storage::{ClipItem, ContentType};

pub struct ClipboardMonitor {
    app_handle: AppHandle,
    last_copied_by_us: Arc<Mutex<Option<String>>>,
}

impl ClipboardMonitor {
    pub fn new(app_handle: AppHandle, last_copied_by_us: Arc<Mutex<Option<String>>>) -> Self {
        Self {
            app_handle,
            last_copied_by_us,
        }
    }

    pub fn start(&self) {
        let app_handle = self.app_handle.clone();
        let last_copied_by_us = self.last_copied_by_us.clone();

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
                        // Check if this was copied by us (to avoid re-capturing)
                        let should_skip = {
                            if let Ok(last_copied) = last_copied_by_us.lock() {
                                if let Some(ref copied_text) = *last_copied {
                                    if copied_text == &text {
                                        log::info!("⏭️ Skipping self-copied text ({} chars)", text.len());
                                        true
                                    } else {
                                        false
                                    }
                                } else {
                                    false
                                }
                            } else {
                                false
                            }
                        };

                        if !should_skip {
                            log::info!("📋 Text clipboard changed: {} chars", text.len());
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
                        } else {
                            // Still update last_text to avoid detecting it again
                            last_text = text.clone();
                        }
                    }
                }

                // Check for image changes
                if let Ok(image) = clipboard.get_image() {
                    let image_bytes = Self::image_to_bytes(&image);

                    if last_image.as_ref() != Some(&image_bytes) {
                        log::info!("Image clipboard changed: {} bytes", image_bytes.len());
                        last_image = Some(image_bytes.clone());

                        // Check settings for image quality preference
                        let store_original = if let Some(state) = app_handle.try_state::<crate::AppState>() {
                            state.settings.get().store_original_image
                        } else {
                            false // Default to thumbnail
                        };

                        let content = if store_original {
                            // Store original or size-limited version
                            Self::process_full_image(&image_bytes)
                        } else {
                            // Store thumbnail
                            Self::create_thumbnail(&image_bytes)
                        };

                        let item = ClipItem {
                            id: Uuid::new_v4().to_string(),
                            content,
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
        use crate::update_tray_menu;

        // Clone item for emission
        let item_for_emit = item.clone();

        let state = app_handle.state::<AppState>();
        let result = {
            // 使用 unwrap_or_else 处理 poisoned lock
            let storage = state.storage.lock().unwrap_or_else(|poisoned| {
                log::warn!("⚠️ Recovered from poisoned lock in clipboard monitor");
                poisoned.into_inner()
            });
            storage.insert(&item)
        };

        if let Err(e) = result {
            log::error!("Failed to save clipboard item: {}", e);
        } else {
            // Emit event to frontend
            app_handle.emit("clipboard-changed", &item_for_emit).ok();

            // Update tray menu
            // We run this in the monitor thread, which is fine as it won't block the UI thread
            // but it might delay the next clipboard check slightly.
            log::debug!("Updating tray menu...");
            update_tray_menu(app_handle);
            
            log::debug!("Clipboard item saved and tray updated");
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
        const THUMBNAIL_SIZE: u32 = 256;
        
        match image::load_from_memory(image_bytes) {
            Ok(img) => {
                let thumbnail = img.resize(
                    THUMBNAIL_SIZE,
                    THUMBNAIL_SIZE,
                    image::imageops::FilterType::Lanczos3,
                );
                
                let mut buffer = Vec::new();
                if thumbnail.write_to(&mut std::io::Cursor::new(&mut buffer), image::ImageFormat::Png).is_ok() {
                    log::info!("Created thumbnail: {}x{} -> {}x{}, {} bytes",
                               img.width(), img.height(), thumbnail.width(), thumbnail.height(), buffer.len());
                    buffer
                } else {
                    log::error!("Failed to encode thumbnail. Returning original bytes.");
                    image_bytes.to_vec()
                }
            }
            Err(e) => {
                log::warn!("Failed to decode image for thumbnail: {}. Returning original bytes.", e);
                image_bytes.to_vec()
            },
        }
    }

    // Process full-resolution image with size limit
    fn process_full_image(image_bytes: &[u8]) -> Vec<u8> {
        const MAX_SIZE: u32 = 2048;
        
        match image::load_from_memory(image_bytes) {
            Ok(img) => {
                let (w, h) = img.dimensions();
                
                // Limit to 2048px on longest side to avoid huge database
                let final_img = if w > MAX_SIZE || h > MAX_SIZE {
                    img.resize(MAX_SIZE, MAX_SIZE, image::imageops::FilterType::Lanczos3)
                } else {
                    img
                };
                
                let mut buffer = Vec::new();
                if final_img.write_to(&mut std::io::Cursor::new(&mut buffer), image::ImageFormat::Png).is_ok() {
                    log::info!("Stored high-quality image: {}x{} -> {} bytes", 
                               final_img.width(), final_img.height(), buffer.len());
                    buffer
                } else {
                    log::error!("Failed to encode high-quality image. Falling back to thumbnail.");
                    // Fallback to thumbnail on encode failure
                    Self::create_thumbnail(image_bytes)
                }
            }
            Err(e) => {
                log::warn!("Failed to decode image for high-quality storage: {}. Falling back to thumbnail.", e);
                Self::create_thumbnail(image_bytes)
            },
        }
    }
}
