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

const items = [{ id: 'a' }, { id: 'b' }, { id: 'c' }];

test('keyboard navigation wraps in both directions and handles empty results', () => {
  selectionStore.move(-1, items);
  expect(selectionStore.index(items)).toBe(2);
  selectionStore.move(1, items);
  expect(selectionStore.index(items)).toBe(0);
  selectionStore.move(1, []);
  expect(selectionStore.selectedId).toBe(null);
});

test('selection follows its ID across reordering, and reset always selects the first row', () => {
  selectionStore.setSelectedIndex(2, items);
  const reordered = [items[2], items[0], items[1]];
  expect(selectionStore.index(reordered)).toBe(0);
  selectionStore.reset('pinned');
  expect(selectionStore.selectedId).toBe(null);
  expect(selectionStore.index(items)).toBe(0);
});

test('removing the selected item falls back to the first available result', () => {
  selectionStore.setSelectedIndex(2, items);
  expect(selectionStore.index(items.slice(0, 2))).toBe(0);
});

test('new input, navigation and window reset invalidate a pending use intent', () => {
  let revision = selectionStore.beginUse();
  selectionStore.cancelUse();
  expect(selectionStore.isCurrentUse(revision)).toBe(false);
  revision = selectionStore.beginUse();
  selectionStore.move(1, items);
  expect(selectionStore.isCurrentUse(revision)).toBe(false);
  revision = selectionStore.beginUse();
  selectionStore.reset();
  expect(selectionStore.isCurrentUse(revision)).toBe(false);
});

test('shift-arrows grow and shrink a range; collapsing to one row or a plain arrow clears it', () => {
  const rows = [...items, { id: 'd' }];
  selectionStore.extendSelection(1, rows);
  selectionStore.extendSelection(1, rows);
  expect([...selectionStore.selectedIds]).toEqual(['a', 'b', 'c']);
  selectionStore.extendSelection(-1, rows);
  expect([...selectionStore.selectedIds]).toEqual(['a', 'b']);
  selectionStore.extendSelection(-1, rows);
  expect(selectionStore.selectedIds.size).toBe(0);
  selectionStore.extendSelection(-1, rows);
  expect(selectionStore.selectedIds.size).toBe(0);

  selectionStore.setSelectedIndex(1, rows);
  selectionStore.extendSelection(1, rows);
  expect([...selectionStore.selectedIds]).toEqual(['b', 'c']);
  selectionStore.move(1, rows);
  expect(selectionStore.selectedIds.size).toBe(0);
  expect(selectionStore.selectedId).toBe('d');
});

test('shift-click starts a selection at the clicked row, then selects the range from it', () => {
  const rows = [...items, { id: 'd' }];
  selectionStore.extendTo('b', rows);
  expect([...selectionStore.selectedIds]).toEqual(['b']);
  selectionStore.extendTo('d', rows);
  expect([...selectionStore.selectedIds]).toEqual(['b', 'c', 'd']);
  expect(selectionStore.selectedId).toBe('d');
  selectionStore.extendTo('a', rows);
  expect([...selectionStore.selectedIds]).toEqual(['a', 'b']);
  selectionStore.extendSelection(1, rows);
  expect(selectionStore.selectedIds.size).toBe(0);
});
