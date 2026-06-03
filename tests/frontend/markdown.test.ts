import { describe, expect, test } from 'bun:test';
import {
  markdownBlockTokens,
  markdownListItemBlockTokens,
  markdownListItemInlineTokens,
  safeMarkdownUrl,
} from '../../src/lib/utils/markdown';

describe('markdown helpers', () => {
  test('drops raw HTML tokens before the Svelte renderer sees them', () => {
    const tokens = markdownBlockTokens(
      [
        '## Release',
        '',
        '[safe](https://example.com)',
        '<script>alert("x")</script>',
        '<img src=x onerror="alert(1)">',
        '<a href="javascript:alert(1)">bad</a>',
      ].join('\n')
    );

    expect(JSON.stringify(tokens)).toContain('Release');
    expect(JSON.stringify(tokens)).not.toContain('<script');
    expect(JSON.stringify(tokens)).not.toContain('onerror');
  });

  test('allows safe markdown links and blocks javascript URLs', () => {
    expect(safeMarkdownUrl('https://example.com', 'href')).toBe('https://example.com');
    expect(safeMarkdownUrl('javascript:alert(1)', 'href')).toBeNull();
    expect(safeMarkdownUrl('data:image/png;base64,abc', 'src')).toBeNull();
  });

  test('preserves GFM task list metadata for the Svelte renderer', () => {
    const tokens = markdownBlockTokens(['- [x] done', '- [ ] todo'].join('\n'));
    const list = tokens[0];

    expect(list.type).toBe('list');
    if (list.type !== 'list') return;

    expect(list.items[0].task).toBe(true);
    expect(list.items[0].checked).toBe(true);
    expect(list.items[1].task).toBe(true);
    expect(list.items[1].checked).toBe(false);
  });

  test('removes checkbox marker tokens from rendered task item content', () => {
    const [list] = markdownBlockTokens('- [x] done');
    expect(list.type).toBe('list');
    if (list.type !== 'list') return;

    expect(markdownListItemBlockTokens(list.items[0]).map((token) => token.type)).toEqual(['text']);
    expect(markdownListItemInlineTokens(list.items[0]).map((token) => token.raw)).toEqual(['done']);
  });
});
