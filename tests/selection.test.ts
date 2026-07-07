import { beforeEach, expect, test } from 'bun:test';

// Mock the $state rune as a global callable (the store is imported raw, not compiled).
Object.defineProperty(globalThis, '$state', {
  configurable: true,
  value: <T>(value: T) => value,
});

const { selectionStore } = await import('../src/lib/stores/selection.svelte');

beforeEach(() => {
  selectionStore.reset('recent');
});

test('up from the first item wraps to the last item', () => {
  selectionStore.move(-1, 3);

  expect(selectionStore.selectedIndex).toBe(2);
});

test('down from the last item wraps to the first item', () => {
  selectionStore.setSelectedIndex(2, 3);

  selectionStore.move(1, 3);

  expect(selectionStore.selectedIndex).toBe(0);
});

test('clamp writes a stale selected index back into range', () => {
  selectionStore.setSelectedIndex(8, 10);

  selectionStore.clamp(3);

  expect(selectionStore.selectedIndex).toBe(2);
});
