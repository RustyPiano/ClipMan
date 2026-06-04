import { describe, expect, test } from 'bun:test';
import { normalizeSettingsLocale } from '../../src/lib/utils/settings';
import type { Settings } from '../../src/lib/types';

function settings(overrides: Partial<Settings> = {}): Settings {
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
    ...overrides,
  };
}

describe('settings helpers', () => {
  test('normalizes invalid backend locale and marks settings for persistence', () => {
    const result = normalizeSettingsLocale(settings({ locale: 'fr-FR' as Settings['locale'] }));

    expect(result.settings.locale).toBe('zh-CN');
    expect(result.needsSave).toBe(true);
  });

  test('keeps valid backend locale without marking settings dirty', () => {
    const result = normalizeSettingsLocale(settings({ locale: 'en' }));

    expect(result.settings.locale).toBe('en');
    expect(result.needsSave).toBe(false);
  });
});
