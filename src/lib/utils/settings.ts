import type { Locale } from '$lib/i18n';
import type { Settings } from '$lib/types';

/**
 * Fresh copy of the default settings. A factory (not a shared const) so each
 * caller — the initial `$state` and the "Reset" action — gets its own object
 * and nested `ignoredApps` array to mutate independently.
 */
export function createDefaultSettings(): Settings {
  return {
    globalShortcut: 'CommandOrControl+Shift+V',
    autoPaste: true,
    ignoreConcealed: true,
    pinnedShortcut: null,
    maxHistoryItems: 100,
    trayTextLength: 70,
    maxPinnedInTray: 5,
    maxRecentInTray: 20,
    customDataPath: null,
    enableAutostart: false,
    locale: 'zh-CN',
    ignoredApps: [],
    skipSecrets: true,
    maxTextBytes: 2000000,
    maxImageDimension: 4096,
    capturePaused: false,
  };
}

export function isValidLocale(value: unknown): value is Locale {
  return value === 'zh-CN' || value === 'en';
}

export function normalizeSettingsLocale(settings: Settings): {
  settings: Settings;
  needsSave: boolean;
} {
  if (isValidLocale(settings.locale)) {
    return { settings, needsSave: false };
  }

  return {
    settings: {
      ...settings,
      locale: 'zh-CN',
    },
    needsSave: true,
  };
}
