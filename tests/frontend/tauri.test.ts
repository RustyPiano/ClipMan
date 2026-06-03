import { describe, expect, test } from 'bun:test';
import { hasTauriRuntime } from '../../src/lib/utils/tauri';

describe('hasTauriRuntime', () => {
  test('returns false outside a Tauri webview', () => {
    expect(hasTauriRuntime()).toBe(false);
  });
});
