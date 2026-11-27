import { describe, expect, test } from 'bun:test';
import {
  applyClipboardChanged,
  comparePinOrder,
  decodeClipText,
  decodeFilePaths,
  encodeClipTextPreview,
  fileBasename,
  fileDirname,
  formatBytes,
  formatClipTime,
  getRecentDisplayItems,
  highlightSegments,
  isSingleUrl,
  looksLikeDirectory,
  looksLikeCode,
  matchesClipFilter,
  parseColorValue,
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
    expect(decodeClipText(clip({ content: encodeText('ClipMan 文本') }), '[empty]')).toBe(
      'ClipMan 文本'
    );
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

  test('replaces an existing item with the authoritative incoming copy', () => {
    const existing = clip({
      id: 'pinned',
      isPinned: true,
      pinOrder: 2,
      label: 'favorite',
      timestamp: 10,
    });

    const repinned = applyClipboardChanged({
      recentItems: [],
      pinnedItems: [existing],
      incoming: clip({ id: 'pinned', isPinned: true, pinOrder: 1, timestamp: 20 }),
      maxHistoryItems: 10,
    });
    expect(repinned.pinnedItems).toHaveLength(1);
    expect(repinned.pinnedItems[0].timestamp).toBe(20);
    expect(repinned.recentItems).toEqual([]);

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
  test('only code-like text uses the monospace font and URLs are classified as links', () => {
    for (const code of [
      '{ "a": 1 }',
      'bun run check && bun test',
      '  indented()',
      'const x = 1;',
    ])
      expect(looksLikeCode(code)).toBe(true);
    for (const nonCode of [
      'https://example.com/a',
      'Keep common actions close.',
      '周会纪要：确认搜索体验。',
      'a | b is fine',
    ])
      expect(looksLikeCode(nonCode)).toBe(false);
    expect(isSingleUrl('https://example.com/a')).toBe(true);
    expect(isSingleUrl('visit https://example.com/a now')).toBe(false);
  });

  test('parses hex, rgb, and hsl colors into normalized formats', () => {
    expect(parseColorValue('#3b82f6')).toEqual({
      css: 'rgb(59, 130, 246)',
      hex: '#3B82F6',
      rgb: 'rgb(59, 130, 246)',
      hsl: 'hsl(217, 91%, 60%)',
    });
    expect(parseColorValue('#fff')?.hex).toBe('#FFFFFF');
    expect(parseColorValue('rgb(255, 0, 0)')?.hex).toBe('#FF0000');
    expect(parseColorValue('hsl(120, 100%, 50%)')?.hex).toBe('#00FF00');
    expect(parseColorValue('not-a-color')).toBeNull();
    expect(parseColorValue('rgb(300, 0, 0)')).toBeNull();
  });

  test('highlights matching search terms case-insensitively and filters by type', () => {
    expect(highlightSegments('ClipMan QuickBar', 'clip')).toEqual([
      { text: 'Clip', match: true },
      { text: 'Man QuickBar', match: false },
    ]);
    expect(highlightSegments('İfoo bar', 'foo')).toEqual([
      { text: 'İ', match: false },
      { text: 'foo', match: true },
      { text: ' bar', match: false },
    ]);
    const urlItem = clip({ content: encodeText('https://clipman.top') });
    const textItem = clip({ content: encodeText('plain note') });
    const imgItem = clip({ contentType: 'image', content: 'data:image/png;base64,AA==' });
    expect(matchesClipFilter(urlItem, 'link')).toBe(true);
    expect(matchesClipFilter(textItem, 'link')).toBe(false);
    expect(matchesClipFilter(imgItem, 'image')).toBe(true);
    expect(formatBytes(512)).toBe('512 B');
    expect(formatBytes(2048)).toBe('2.0 KB');
  });

  test('encodeClipTextPreview truncates on UTF-8 character boundaries without replacement chars', () => {
    const text = 'a' + '中'.repeat(2000);
    const encoded = encodeClipTextPreview(text);
    const decoded = decodeClipText(clip({ content: encoded }), '');
    expect(decoded.includes('\uFFFD')).toBe(false);
    const byteLength = new TextEncoder().encode(decoded).length;
    expect(byteLength).toBeLessThanOrEqual(4096);
    expect(byteLength).toBeGreaterThan(4090);
  });

  test('clip times get coarser with age', () => {
    const labels = { justNow: 'now', minutesAgo: (n: number) => `${n}m`, yesterday: 'Yesterday' };
    const now = new Date(2026, 8, 30, 15, 0).getTime();
    const at = (...args: [number, number, number, number, number]) =>
      formatClipTime(new Date(...args).getTime(), now, 'en-US', labels);
    expect(at(2026, 8, 30, 14, 59)).toBe('1m');
    expect(at(2026, 8, 30, 9, 5)).toMatch(/^9:05\sAM$/);
    expect(at(2026, 8, 29, 23, 30)).toMatch(/^Yesterday 11:30\sPM$/);
    expect(formatClipTime(new Date(2026, 8, 30, 9, 5).getTime(), now, 'zh-CN', labels)).toBe(
      '09:05'
    );
    expect(at(2026, 8, 1, 12, 0)).toBe('Sep 1');
    expect(at(2025, 11, 31, 12, 0)).toBe('Dec 31, 2025');
  });
});
