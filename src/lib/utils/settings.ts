import type { Locale } from '$lib/i18n';
import type { Settings } from '$lib/types';

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
