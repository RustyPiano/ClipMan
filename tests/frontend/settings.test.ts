import { describe, expect, test } from 'bun:test';
import { createDefaultSettings, normalizeSettingsLocale } from '../../src/lib/utils/settings';
import type { Settings } from '../../src/lib/types';

// Base on the real defaults so new Settings fields can't silently drift out of
// this fixture.
function settings(overrides: Partial<Settings> = {}): Settings {
  return { ...createDefaultSettings(), ...overrides };
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
