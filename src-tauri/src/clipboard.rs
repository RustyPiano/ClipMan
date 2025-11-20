use arboard::{Clipboard, ImageData};
use clipboard_master::{CallbackResult, ClipboardHandler, Master};
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

struct Handler {
    app_handle: AppHandle,
    last_copied_by_us: Arc<Mutex<Option<String>>>,
    last_text: Arc<Mutex<String>>,
    last_image: Arc<Mutex<Option<Vec<u8>>>>,
}

impl ClipboardHandler for Handler {
    fn on_clipboard_change(&mut self) -> CallbackResult {
        let mut clipboard = match Clipboard::new() {
            Ok(cb) => cb,
            Err(e) => {
                log::error!("Failed to create clipboard instance in handler: {}", e);
                return CallbackResult::Next;
            }
        };

        // Check for text changes
        if let Ok(text) = clipboard.get_text() {
            let mut last_text = self.last_text.lock().unwrap();
            if text != *last_text && !text.is_empty() {
                // Check if this was copied by us
                let should_skip = {
                    if let Ok(last_copied) = self.last_copied_by_us.lock() {
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
                    
                    let item = ClipItem {
                        id: Uuid::new_v4().to_string(),
                        content: text.clone().into_bytes(),
                        content_type: ContentType::Text,
                        timestamp: Utc::now().timestamp(),
                        is_pinned: false,
                        pin_order: None,
                    };

                    ClipboardMonitor::save_to_storage(&self.app_handle, item);
                }
                
                *last_text = text;
            }
        }

        // Check for image changes
        if let Ok(image) = clipboard.get_image() {
            let image_bytes = ClipboardMonitor::image_to_bytes(&image);
            let mut last_image = self.last_image.lock().unwrap();

            if last_image.as_ref() != Some(&image_bytes) {
                log::info!("Image clipboard changed: {} bytes", image_bytes.len());

                // Check settings for image quality preference
                let store_original = if let Some(state) = self.app_handle.try_state::<crate::AppState>() {
                    state.settings.get().store_original_image
                } else {
                    false
                };

                let content = if store_original {
                    ClipboardMonitor::process_full_image(&image_bytes)
                } else {
                    ClipboardMonitor::create_thumbnail(&image_bytes)
                };

                let item = ClipItem {
                    id: Uuid::new_v4().to_string(),
                    content,
                    content_type: ContentType::Image,
                    timestamp: Utc::now().timestamp(),
                    is_pinned: false,
                    pin_order: None,
                };

                ClipboardMonitor::save_to_storage(&self.app_handle, item);
                *last_image = Some(image_bytes);
            }
        }

        CallbackResult::Next
    }

    fn on_clipboard_error(&mut self, error: std::io::Error) -> CallbackResult {
        log::error!("Clipboard error: {}", error);
        CallbackResult::Next
    }
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

        // Start event-driven monitoring thread
        std::thread::spawn(move || {
            log::info!("Clipboard monitoring thread started (event-driven)");

            let handler = Handler {
                app_handle: app_handle.clone(),
                last_copied_by_us: last_copied_by_us.clone(),
                last_text: Arc::new(Mutex::new(String::new())),
                last_image: Arc::new(Mutex::new(None)),
            };

            // Start the clipboard master - this blocks until the application exits
            match Master::new(handler) {
                Ok(mut master) => {
                    if let Err(e) = master.run() {
                        log::error!("Clipboard master failed: {}", e);
                        log::warn!("Falling back to polling mode...");
                        Self::start_polling(app_handle, last_copied_by_us);
                    }
                }
                Err(e) => {
                    log::error!("Failed to create clipboard master: {}", e);
                    log::warn!("Falling back to polling mode...");
                    Self::start_polling(app_handle, last_copied_by_us);
                }
            }
        });
    }

    // Fallback polling implementation
    fn start_polling(app_handle: AppHandle, last_copied_by_us: Arc<Mutex<Option<String>>>) {
        use std::thread;
        use std::time::Duration;

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

                    let store_original = if let Some(state) = app_handle.try_state::<crate::AppState>() {
                        state.settings.get().store_original_image
                    } else {
                        false
                    };

                    let content = if store_original {
                        Self::process_full_image(&image_bytes)
                    } else {
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
    }

    fn save_to_storage(app_handle: &AppHandle, item: ClipItem) {
        use crate::AppState;
        use crate::update_tray_menu;

        let item_for_emit = item.clone();

        let state = app_handle.state::<AppState>();
        let result = {
            let storage = state.storage.lock().unwrap_or_else(|poisoned| {
                log::warn!("⚠️ Recovered from poisoned lock in clipboard monitor");
                poisoned.into_inner()
            });
            storage.insert(&item)
        };

        if let Err(e) = result {
            log::error!("Failed to save clipboard item: {}", e);
        } else {
            app_handle.emit("clipboard-changed", &item_for_emit).ok();
            log::debug!("Updating tray menu...");
            update_tray_menu(app_handle);
            log::debug!("Clipboard item saved and tray updated");
        }
    }

    fn image_to_bytes(image: &ImageData) -> Vec<u8> {
        use image::{ImageBuffer, RgbaImage};

        let img: RgbaImage = match ImageBuffer::from_raw(
            image.width as u32,
            image.height as u32,
            image.bytes.to_vec()
        ) {
            Some(img) => img,
            None => {
                log::error!("Failed to create image buffer");
                return image.bytes.to_vec();
            }
        };

        let mut png_bytes = Vec::new();
        if let Err(e) = img.write_to(&mut std::io::Cursor::new(&mut png_bytes), image::ImageFormat::Png) {
            log::error!("Failed to encode PNG: {}", e);
            return image.bytes.to_vec();
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

    fn process_full_image(image_bytes: &[u8]) -> Vec<u8> {
        const MAX_SIZE: u32 = 2048;
        
        match image::load_from_memory(image_bytes) {
            Ok(img) => {
                let (w, h) = img.dimensions();
                
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
