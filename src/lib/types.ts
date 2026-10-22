import type { Locale } from './i18n';

type ContentType = 'text' | 'image' | 'files';

export interface ClipItem {
  id: string;
  /** 文本和文件摘要使用 Base64，图片缩略图使用 data URL。 */
  content: string;
  contentType: ContentType;
  /** Unix 秒数，保留小数以区分同一秒内的记录。 */
  timestamp: number;
  isPinned: boolean;
  pinOrder: number | null;
  label: string | null;
  sourceApp: string | null;
  hasHtml: boolean;
  contentBytes: number;
  fileCount: number;
}

/** 预览按需读取的解码内容，大小受后端限制。 */
export interface ClipDetail {
  id: string;
  contentType: ContentType;
  text: string;
  imageUrl: string | null;
  truncated: boolean;
}

/** 与 settings.rs 的 PASTE_FORMATS 保持一致。 */
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
  /** 按应用名或 bundle identifier 排除采集，忽略大小写。 */
  ignoredApps: string[];
  skipSecrets: boolean;
  /** 超过此字节数的文本或文件路径列表不参与采集。 */
  maxTextBytes: number;
  /** 图片最长边的像素上限；0 表示不缩小。 */
  maxImageDimension: number;
  /** 由托盘菜单控制，设置页面不修改此项。 */
  capturePaused: boolean;
}

// default 遵循自动粘贴设置，opposite 对本次操作取反。
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
