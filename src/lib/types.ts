// Shared type definitions for ClipMan frontend

import type { Locale } from './i18n';
export type { Locale };

/**
 * Clipboard item content types
 */
export type ContentType = 'text' | 'image' | 'files';

/**
 * Clipboard item from backend
 */
export interface ClipItem {
  id: string;
  /** Base64 encoded content or data URL for images */
  content: string;
  contentType: ContentType;
  /** Unix seconds, including the fractional part for chronological ordering. */
  timestamp: number;
  isPinned: boolean;
  pinOrder: number | null;
  label: string | null;
  groupName: string | null;
  /** App that was frontmost when the clip was captured (copy source). */
  sourceApp: string | null;
  /** Whether a text clip carries an HTML (rich-text) companion. */
  hasHtml: boolean;
  contentBytes?: number;
  fileCount?: number;
}

/** Bounded, decoded content fetched only for a selected preview. */
export interface ClipDetail {
  id: string;
  contentType: ContentType;
  text: string;
  imageUrl: string | null;
  truncated: boolean;
}

/**
 * Application settings
 */
/**
 * How text clips are pasted. Backend values (settings.rs PASTE_FORMATS):
 * original (as copied), takePlain (ClipMan-mediated takes are plain),
 * globalPlain (captured rich text is flattened system-wide).
 */
export type PasteFormat = 'original' | 'takePlain' | 'globalPlain';

export interface Settings {
  globalShortcut: string;
  autoPaste: boolean;
  pasteFormat: PasteFormat;
  ignoreConcealed: boolean;
  pinnedShortcut: string | null;
  maxHistoryItems: number;
  trayTextLength: number;
  maxPinnedInTray: number;
  maxRecentInTray: number;
  customDataPath: string | null;
  enableAutostart: boolean;
  locale: Locale;
  /** App names whose copies are never captured (matched case-insensitively). */
  ignoredApps: string[];
  /** Skip capturing Text clips that look like a high-confidence secret. */
  skipSecrets: boolean;
  /** Text/Files clips larger than this many bytes are skipped at capture time. */
  maxTextBytes: number;
  /** Images whose longest side exceeds this many pixels are downsampled; 0 disables downscaling. */
  maxImageDimension: number;
  /** Capture is fully paused (toggled from the tray menu; not shown in Settings UI). */
  capturePaused: boolean;
}

// The frontend only ever issues 'default' (honor the auto-paste setting) or
// 'opposite' (⌘Enter swap). The Rust `PasteMode` enum additionally accepts
// 'paste'/'copy', but nothing in the UI emits them — narrowing here makes the
// paste/copy failure toast branch on a real, exhaustive set of modes.
export type PasteMode = 'default' | 'opposite';

export type ReorderDirection = 'up' | 'down';

/**
 * Update check result
 */
export interface UpdateInfo {
  available: boolean;
  current_version: string;
  latest_version?: string;
  body?: string;
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
 * Settings page tabs
 */
export type SettingsTab = 'general' | 'clipboard' | 'tray' | 'storage' | 'about' | 'appearance';
