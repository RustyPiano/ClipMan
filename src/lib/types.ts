import type { Settings } from './bindings/Settings';

export type { BackupFile } from './bindings/BackupFile';
export type { ClipDetail } from './bindings/ClipDetail';
export type { ClipItem } from './bindings/ClipItem';
export type { ImportResult } from './bindings/ImportResult';
export type { Settings } from './bindings/Settings';
export type { SettingsUpdateResult } from './bindings/SettingsUpdateResult';
export type { StorageUsage } from './bindings/StorageUsage';
export type { UpdateInfo } from './bindings/UpdateInfo';
export type { UpdateProgress } from './bindings/UpdateProgress';
export type { UseOutcome } from './bindings/UseOutcome';

export type PasteFormat = Settings['pasteFormat'];

export type PasteMode = 'default' | 'opposite';

export type ReorderDirection = 'up' | 'down';

export type ClipFilter = 'all' | 'text' | 'image' | 'files' | 'link';

export type Theme = 'light' | 'dark' | 'light-pink' | 'system';

export type ToastType = 'success' | 'error' | 'info' | 'pin';

export interface Toast {
  id: number;
  message: string;
  type: ToastType;
}

export type SettingsTab = 'general' | 'clipboard' | 'storage' | 'about' | 'appearance';
