// Shared type definitions for ClipMan frontend

/**
 * Clipboard item content types
 */
export type ContentType = 'text' | 'image' | 'file' | 'html' | 'rtf';

/**
 * Clipboard item from backend
 */
export interface ClipItem {
  id: string;
  /** Base64 encoded content or data URL for images */
  content: string;
  contentType: ContentType;
  /** Unix timestamp in seconds */
  timestamp: number;
  isPinned: boolean;
  pinOrder: number | null;
}

/**
 * Application settings
 */
export interface Settings {
  globalShortcut: string;
  maxHistoryItems: number;
  autoCleanup: boolean;
  trayTextLength: number;
  storeOriginalImage: boolean;
  maxPinnedInTray: number;
  maxRecentInTray: number;
  customDataPath: string | null;
  enableAutostart: boolean;
  locale: string;
}

/**
 * Update check result
 */
export interface UpdateInfo {
  available: boolean;
  current_version: string;
  latest_version?: string;
  body?: string;
  date?: string;
}

/**
 * Theme options
 */
export type Theme = 'light' | 'dark' | 'light-pink' | 'system';

/**
 * Toast notification types
 */
export type ToastType = 'success' | 'error' | 'info';

/**
 * Toast notification item
 */
export interface Toast {
  id: number;
  message: string;
  type: ToastType;
}

/**
 * Router routes
 */
export type Route = 'home' | 'settings';

/**
 * Settings page tabs
 */
export type SettingsTab = 'general' | 'clipboard' | 'tray' | 'storage' | 'about' | 'appearance';
