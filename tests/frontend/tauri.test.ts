import { describe, expect, test } from 'bun:test';
import { hasTauriRuntime } from '../../src/lib/utils/tauri';

describe('hasTauriRuntime', () => {
  test('detects the Tauri runtime without depending on other suites', () => {
    const original = Object.getOwnPropertyDescriptor(globalThis, 'window');
    try {
      Object.defineProperty(globalThis, 'window', { configurable: true, value: {} });
      expect(hasTauriRuntime()).toBe(false);
      (window as Window & { __TAURI_INTERNALS__?: object }).__TAURI_INTERNALS__ = {};
      expect(hasTauriRuntime()).toBe(true);
    } finally {
      if (original) Object.defineProperty(globalThis, 'window', original);
      else delete (globalThis as { window?: Window }).window;
    }
  });
});
