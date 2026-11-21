import type { Locale } from './i18n';

type ContentType = 'text' | 'image' | 'files';

export interface ClipItem {
  id: string;
  content: string;
  contentType: ContentType;
  timestamp: number;
  isPinned: boolean;
  pinOrder: number | null;
  label: string | null;
  sourceApp: string | null;
  hasHtml: boolean;
  contentBytes: number;
  fileCount: number;
}

export interface ClipDetail {
  id: string;
  contentType: ContentType;
  text: string;
  imageUrl: string | null;
  truncated: boolean;
}

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
  ignoredApps: string[];
  skipSecrets: boolean;
  maxTextBytes: number;
  maxImageDimension: number;
  capturePaused: boolean;
}

export type PasteMode = 'default' | 'opposite';

export type ReorderDirection = 'up' | 'down';

export interface UpdateInfo {
  available: boolean;
  current_version: string;
  latest_version?: string;
  body?: string;
}

export type Theme = 'light' | 'dark' | 'light-pink' | 'system';

export type ToastType = 'success' | 'error' | 'info';

export interface Toast {
  id: number;
  message: string;
  type: ToastType;
}

export type SettingsTab = 'general' | 'clipboard' | 'tray' | 'storage' | 'about' | 'appearance';
