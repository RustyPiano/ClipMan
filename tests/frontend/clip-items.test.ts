import { describe, expect, test } from 'bun:test';
import {
  applyClipboardChanged,
  comparePinOrder,
  decodeClipText,
  decodeFilePaths,
  fileBasename,
  fileDirname,
  formatClipTime,
  getRecentDisplayItems,
  looksLikeDirectory,
  looksLikeCode,
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
    sourceApp: null,
    hasHtml: false,
    contentBytes: 5,
    fileCount: 0,
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

  test('returns no file paths for non-files clips', () => {
    expect(decodeFilePaths(clip({ contentType: 'text', content: encodeText('hi') }))).toEqual([]);
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
      timestamp: 10,
    });
    const incoming = clip({
      id: 'pinned',
      isPinned: false,
      pinOrder: null,
      label: null,
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

  test('history display includes pinned and recent items ordered by timestamp', () => {
    const items = getRecentDisplayItems({
      activeSearchQuery: '',
      searchResults: [],
      recentItems: [clip({ id: 'recent', timestamp: 20 })],
      pinnedItems: [
        clip({ id: 'pinned-new', isPinned: true, pinOrder: 2, timestamp: 30 }),
        clip({ id: 'pinned-old', isPinned: true, pinOrder: 1, timestamp: 10 }),
      ],
    });

    expect(items.map((item) => item.id)).toEqual(['pinned-new', 'recent', 'pinned-old']);
  });

  test('history search display includes pinned matches', () => {
    const items = getRecentDisplayItems({
      activeSearchQuery: 'needle',
      searchResults: [
        clip({ id: 'recent-match', timestamp: 20 }),
        clip({ id: 'pinned-match', isPinned: true, pinOrder: 1, timestamp: 10 }),
      ],
      recentItems: [],
      pinnedItems: [],
    });

    expect(items.map((item) => item.id)).toEqual(['recent-match', 'pinned-match']);
  });
});

test('file previews decode JSON without splitting newlines inside one filename', () => {
  const paths = ['/tmp/one\ntwo.txt', '/tmp/quoted"name.txt'];
  const item = {
    contentType: 'files',
    content: Buffer.from(JSON.stringify(paths), 'utf8').toString('base64'),
  } as ClipItem;
  expect(decodeFilePaths(item)).toEqual(paths);
});

describe('file path display helpers', () => {
  test('fileBasename splits on both POSIX and Windows separators', () => {
    expect(fileBasename('/tmp/report.pdf')).toBe('report.pdf');
    expect(fileBasename('C:\\Users\\me\\report.docx')).toBe('report.docx');
    expect(fileBasename('report.pdf')).toBe('report.pdf');
    expect(fileBasename('/tmp/dir/')).toBe('dir');
    expect(fileBasename('C:\\dir\\')).toBe('dir');
  });

  test('fileDirname returns the directory part or empty for bare names', () => {
    expect(fileDirname('/tmp/report.pdf')).toBe('/tmp');
    expect(fileDirname('C:\\Users\\me\\report.docx')).toBe('C:\\Users\\me');
    expect(fileDirname('report.pdf')).toBe('');
    expect(fileDirname('/report.pdf')).toBe('');
  });

  test('looksLikeDirectory flags trailing separators and extensionless basenames', () => {
    expect(looksLikeDirectory('/tmp/dir/')).toBe(true);
    expect(looksLikeDirectory('C:\\Users\\me\\')).toBe(true);
    expect(looksLikeDirectory('/tmp/dir')).toBe(true);
    expect(looksLikeDirectory('C:\\Users\\me\\Projects')).toBe(true);
    expect(looksLikeDirectory('/tmp/report.pdf')).toBe(false);
    expect(looksLikeDirectory('C:\\Users\\me\\report.docx')).toBe(false);
  });
});

describe('row display helpers', () => {
  test('only code-like text uses the monospace font', () => {
    for (const code of [
      'https://example.com/a',
      '{ "a": 1 }',
      'bun run check && bun test',
      '  indented()',
      'const x = 1;',
    ])
      expect(looksLikeCode(code)).toBe(true);
    for (const prose of ['Keep common actions close.', '周会纪要：确认搜索体验。', 'a | b is fine'])
      expect(looksLikeCode(prose)).toBe(false);
  });

  test('clip times get coarser with age', () => {
    const labels = { justNow: 'now', minutesAgo: (n: number) => `${n}m`, yesterday: 'Yesterday' };
    const now = new Date(2026, 8, 30, 15, 0).getTime();
    const at = (...args: [number, number, number, number, number]) =>
      formatClipTime(new Date(...args).getTime(), now, 'en-US', labels);
    expect(at(2026, 8, 30, 14, 59)).toBe('1m');
    expect(at(2026, 8, 30, 9, 5)).toBe('09:05 AM');
    expect(at(2026, 8, 29, 23, 30)).toBe('Yesterday 11:30 PM');
    expect(at(2026, 8, 1, 12, 0)).toBe('Sep 1');
    expect(at(2025, 11, 31, 12, 0)).toBe('Dec 31, 2025');
  });
});
