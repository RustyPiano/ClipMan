import { describe, expect, test } from 'bun:test';
import {
  applyClipboardChanged,
  comparePinOrder,
  decodeClipText,
} from '../../src/lib/utils/clip-items';
import type { ClipItem } from '../../src/lib/types';

function encodeText(text: string) {
  return Buffer.from(text, 'utf8').toString('base64');
}

function clip(overrides: Partial<ClipItem>): ClipItem {
  return {
    id: 'clip-1',
    content: encodeText('hello'),
    contentType: 'text',
    timestamp: 100,
    isPinned: false,
    pinOrder: null,
    label: null,
    groupName: null,
    ...overrides,
  };
}

describe('clip item helpers', () => {
  test('decodes base64 text content', () => {
    expect(decodeClipText(clip({ content: encodeText('ClipMan 文本') }), '[empty]', '[bad]')).toBe(
      'ClipMan 文本'
    );
  });

  test('does not cache localized fallback strings', () => {
    const empty = clip({ content: '' });
    expect(decodeClipText(empty, '[empty zh]', '[bad zh]')).toBe('[empty zh]');
    expect(decodeClipText(empty, '[empty en]', '[bad en]')).toBe('[empty en]');

    const originalError = console.error;
    console.error = () => {};
    try {
      const invalid = clip({ content: '%%%' });
      expect(decodeClipText(invalid, '[empty zh]', '[bad zh]')).toBe('[bad zh]');
      expect(decodeClipText(invalid, '[empty en]', '[bad en]')).toBe('[bad en]');
    } finally {
      console.error = originalError;
    }
  });

  test('incrementally inserts a new recent item at the top', () => {
    const existing = clip({ id: 'old', timestamp: 10 });
    const incoming = clip({ id: 'new', timestamp: 20 });

    const next = applyClipboardChanged({
      recentItems: [existing],
      pinnedItems: [],
      incoming,
      maxHistoryItems: 10,
    });

    expect(next.recentItems.map((item) => item.id)).toEqual(['new', 'old']);
    expect(next.pinnedItems).toEqual([]);
  });

  test('incrementally updates an existing pinned item without duplicating it', () => {
    const existing = clip({ id: 'pinned', isPinned: true, pinOrder: 2, timestamp: 10 });
    const incoming = clip({ id: 'pinned', isPinned: true, pinOrder: 1, timestamp: 20 });

    const next = applyClipboardChanged({
      recentItems: [],
      pinnedItems: [existing],
      incoming,
      maxHistoryItems: 10,
    });

    expect(next.pinnedItems).toHaveLength(1);
    expect(next.pinnedItems[0].timestamp).toBe(20);
    expect(next.recentItems).toEqual([]);
  });

  test('uses authoritative incoming metadata for existing duplicate items', () => {
    const existing = clip({
      id: 'pinned',
      isPinned: true,
      pinOrder: 2,
      label: 'favorite',
      groupName: 'snippets',
      timestamp: 10,
    });
    const incoming = clip({
      id: 'pinned',
      isPinned: false,
      pinOrder: null,
      label: null,
      groupName: null,
      timestamp: 20,
    });

    const next = applyClipboardChanged({
      recentItems: [],
      pinnedItems: [existing],
      incoming,
      maxHistoryItems: 10,
    });

    expect(next.pinnedItems).toEqual([]);
    expect(next.recentItems).toHaveLength(1);
    expect(next.recentItems[0]).toMatchObject({
      id: 'pinned',
      isPinned: false,
      pinOrder: null,
      label: null,
      groupName: null,
      timestamp: 20,
    });
  });

  test('sorts pinned items by explicit pin order before timestamp', () => {
    const items = [
      clip({ id: 'later-no-order', isPinned: true, pinOrder: null, timestamp: 30 }),
      clip({ id: 'second', isPinned: true, pinOrder: 2, timestamp: 20 }),
      clip({ id: 'first', isPinned: true, pinOrder: 1, timestamp: 10 }),
    ];

    expect(items.toSorted(comparePinOrder).map((item) => item.id)).toEqual([
      'first',
      'second',
      'later-no-order',
    ]);
  });
});
