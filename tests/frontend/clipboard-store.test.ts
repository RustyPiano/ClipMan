import { beforeEach, describe, expect, test } from 'bun:test';
import type { ClipItem } from '../../src/lib/types';

type InvokeHandler = (cmd: string, args?: Record<string, unknown>) => unknown;

const state = (<T>(value: T) => value) as (<T>(value: T) => T) & {
  raw: <T>(value: T) => T;
};
state.raw = <T>(value: T) => value;

// $state / $derived are compiled away in a real Svelte build; the .svelte.ts
// stores are imported raw here, so mock the runes as global callables.
// defineProperty (value: any) sidesteps Svelte's ambient rune global types.
Object.defineProperty(globalThis, '$state', { configurable: true, value: state });
Object.defineProperty(globalThis, '$derived', {
  configurable: true,
  value: <T>(value: T) => value,
});

Object.defineProperty(globalThis, 'window', {
  configurable: true,
  value: {
    addEventListener() {},
    removeEventListener() {},
  },
});

const { clipboardStore } = await import('../../src/lib/stores/clipboard.svelte');
const { selectionStore } = await import('../../src/lib/stores/selection.svelte');
const { toastStore } = await import('../../src/lib/stores/toast.svelte');
const { i18n } = await import('../../src/lib/i18n');

function clip(overrides: Partial<ClipItem>): ClipItem {
  return {
    id: 'clip',
    content: 'aGVsbG8=',
    contentType: 'text',
    timestamp: 1,
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

function installTauriInvoke(handler: InvokeHandler) {
  (window as Window & { __TAURI_INTERNALS__?: Record<string, unknown> }).__TAURI_INTERNALS__ = {
    invoke: handler,
    transformCallback: () => 1,
    unregisterCallback: () => {},
  };
}

function resetStore() {
  clipboardStore.recentItems = [];
  clipboardStore.pinnedItems = [];
  clipboardStore.searchResults = [];
  clipboardStore.searchQuery = '';
  clipboardStore.activeSearchQuery = '';
  clipboardStore.isLoading = false;
  clipboardStore.isSearchPending = false;
  clipboardStore.historyError = null;
  clipboardStore.searchError = null;
  clipboardStore.hasMoreRecent = false;
  clipboardStore.isLoadingMore = false;
  clipboardStore.maxHistoryItems = 100;
  clipboardStore.autoPaste = true;
  clipboardStore.pasteFormat = 'original';
  selectionStore.selectedIds.clear();
  toastStore.toasts = [];
}

describe('clipboard store races', () => {
  beforeEach(() => {
    resetStore();
  });

  test('ignores a stale debounced search after the query was externally cleared', async () => {
    const invoked: string[] = [];
    installTauriInvoke((cmd) => {
      invoked.push(cmd);
      if (cmd === 'search_clips') return [clip({ id: 'stale' })];
      if (cmd === 'get_recent_clips' || cmd === 'get_pinned_clips') return [];
      return null;
    });

    clipboardStore.setSearchQuery('abc');
    await clipboardStore.clearSearch({ reload: false });

    await clipboardStore.search('abc');

    expect(clipboardStore.searchQuery).toBe('');
    expect(clipboardStore.searchResults).toEqual([]);
    expect(invoked).not.toContain('search_clips');
  });

  test('silent clearSearch clears loading even when it reloads history', async () => {
    installTauriInvoke((cmd) => {
      if (cmd === 'get_recent_clips' || cmd === 'get_pinned_clips') return [];
      return null;
    });

    clipboardStore.setSearchQuery('abc');
    // setSearchQuery flags a pending search, not a full-screen load.
    expect(clipboardStore.isSearchPending).toBe(true);

    await clipboardStore.clearSearch();

    expect(clipboardStore.searchQuery).toBe('');
    expect(clipboardStore.isLoading).toBe(false);
  });

  test('fetchFullClip caches by id and drops the entry on delete', async () => {
    let getClipCalls = 0;
    installTauriInvoke((cmd, args) => {
      if (cmd === 'get_clip') {
        getClipCalls += 1;
        return { id: String(args?.id), contentType: 'text', text: 'full', imageUrl: null };
      }
      if (cmd === 'delete_clip') return null;
      if (cmd === 'get_recent_clips' || cmd === 'get_pinned_clips') return [];
      return null;
    });

    const first = await clipboardStore.fetchFullClip('x');
    const second = await clipboardStore.fetchFullClip('x');
    expect(getClipCalls).toBe(1); // second call served from cache
    expect(second).toEqual(first);

    await clipboardStore.deleteItem('x');
    await clipboardStore.fetchFullClip('x');
    expect(getClipCalls).toBe(2); // cache dropped on delete → refetch
  });

  test('deleteItem removes the deleted clip locally without reloading history', async () => {
    const invoked: string[] = [];
    const deleted = clip({ id: 'deleted' });
    const keptRecent = clip({ id: 'kept-recent' });
    const keptPinned = clip({ id: 'kept-pinned', isPinned: true, pinOrder: 1 });

    installTauriInvoke((cmd) => {
      invoked.push(cmd);
      if (cmd === 'delete_clip') return null;
      if (cmd === 'get_recent_clips') return [deleted, keptRecent];
      if (cmd === 'get_pinned_clips') return [keptPinned];
      return null;
    });

    clipboardStore.recentItems = [deleted, keptRecent];
    clipboardStore.pinnedItems = [deleted, keptPinned];
    clipboardStore.searchResults = [deleted, keptRecent, keptPinned];
    clipboardStore.activeSearchQuery = 'del';

    await clipboardStore.deleteItem('deleted');

    expect(invoked).toEqual(['delete_clip']);
    expect(clipboardStore.recentItems.map((item) => item.id)).toEqual(['kept-recent']);
    expect(clipboardStore.pinnedItems.map((item) => item.id)).toEqual(['kept-pinned']);
    expect(clipboardStore.searchResults.map((item) => item.id)).toEqual([
      'kept-recent',
      'kept-pinned',
    ]);
    expect(clipboardStore.isLoading).toBe(false);
  });

  test('full image previews are returned on demand without caching image data', async () => {
    let requests = 0;
    installTauriInvoke(() => {
      requests++;
      return {
        id: 'image',
        contentType: 'image',
        text: '',
        imageUrl: 'data:image/png;base64,eA==',
      };
    });
    expect((await clipboardStore.fetchFullClip('image'))?.imageUrl).toBe(
      'data:image/png;base64,eA=='
    );
    await clipboardStore.fetchFullClip('image');
    expect(requests).toBe(2);
  });

  test('replays incoming clipboard events over an older in-flight history response', async () => {
    const oldItem = clip({ id: 'old', timestamp: 1 });
    const incoming = clip({ id: 'new', timestamp: 2 });
    let resolveRecent: (items: ClipItem[]) => void = () => {};
    let resolvePinned: (items: ClipItem[]) => void = () => {};

    installTauriInvoke((cmd) => {
      if (cmd === 'get_recent_clips') {
        return new Promise<ClipItem[]>((resolve) => {
          resolveRecent = resolve;
        });
      }
      if (cmd === 'get_pinned_clips') {
        return new Promise<ClipItem[]>((resolve) => {
          resolvePinned = resolve;
        });
      }
      return null;
    });

    clipboardStore.recentItems = [oldItem];
    const load = clipboardStore.loadHistory({ showLoading: false });

    (
      clipboardStore as unknown as { applyIncomingItem: (item: ClipItem) => void }
    ).applyIncomingItem(incoming);
    resolveRecent([oldItem]);
    resolvePinned([]);

    await load;

    expect(clipboardStore.recentItems.map((item) => item.id)).toEqual(['new', 'old']);
  });

  test('reloadFromBackend refreshes history while search is active', async () => {
    const oldItem = clip({ id: 'old', timestamp: 1 });
    const copied = clip({ id: 'copied', timestamp: 20 });
    const invoked: string[] = [];

    installTauriInvoke((cmd) => {
      invoked.push(cmd);
      if (cmd === 'get_recent_clips') return [copied, oldItem];
      if (cmd === 'get_pinned_clips') return [];
      if (cmd === 'search_clips') return [copied];
      return null;
    });

    clipboardStore.recentItems = [oldItem, clip({ id: 'copied', timestamp: 2 })];
    clipboardStore.searchQuery = 'needle';
    clipboardStore.activeSearchQuery = 'needle';
    clipboardStore.isSearchPending = false;

    await (
      clipboardStore as unknown as { reloadFromBackend: () => Promise<void> }
    ).reloadFromBackend();

    expect(invoked).toEqual(['get_recent_clips', 'get_pinned_clips', 'search_clips']);
    expect(clipboardStore.recentItems.map((item) => item.id)).toEqual(['copied', 'old']);
    expect(clipboardStore.searchResults.map((item) => item.id)).toEqual(['copied']);
    // A background refresh must not flash the search spinner.
    expect(clipboardStore.isSearchPending).toBe(false);
  });

  test('useClip forwards the plain flag to paste_clip (⌥Enter → plain: true)', async () => {
    const calls: Array<{ cmd: string; args?: Record<string, unknown> }> = [];
    installTauriInvoke((cmd, args) => {
      calls.push({ cmd, args });
      return null;
    });

    const item = clip({ id: 'c1' });
    await clipboardStore.useClip(item, 'default', { plain: true });
    await clipboardStore.useClip(item, 'default');

    const pastes = calls.filter((entry) => entry.cmd === 'paste_clip');
    expect(pastes).toHaveLength(2);
    expect(pastes[0].args).toEqual({ id: 'c1', mode: 'default', plain: true });
    expect(pastes[1].args).toEqual({ id: 'c1', mode: 'default', plain: false });
  });

  test('useClip defaults to plain in takePlain/globalPlain paste-format modes', async () => {
    const calls: Array<{ cmd: string; args?: Record<string, unknown> }> = [];
    installTauriInvoke((cmd, args) => {
      calls.push({ cmd, args });
      return null;
    });

    const item = clip({ id: 'c1' });
    clipboardStore.pasteFormat = 'takePlain';
    await clipboardStore.useClip(item, 'default');
    clipboardStore.pasteFormat = 'globalPlain';
    await clipboardStore.useClip(item, 'default');

    const pastes = calls.filter((entry) => entry.cmd === 'paste_clip');
    expect(pastes.map((entry) => entry.args?.plain)).toEqual([true, true]);
  });

  test('useClip toasts paste vs copy on failure based on the resolved action', async () => {
    installTauriInvoke((cmd) => {
      if (cmd === 'paste_clip') throw new Error('boom');
      return null;
    });

    const item = clip({ id: 'c1' });

    // autoPaste on: 'default' resolves to a paste, 'opposite' (⌘Enter) to a copy.
    clipboardStore.autoPaste = true;
    await clipboardStore.useClip(item, 'default');
    expect(toastStore.toasts.at(-1)?.message).toContain(i18n.t.pasteFailed);
    expect(toastStore.toasts.at(-1)?.type).toBe('error');

    await clipboardStore.useClip(item, 'opposite');
    expect(toastStore.toasts.at(-1)?.message).toContain(i18n.t.copyFailed);

    // autoPaste off: the paste/copy mapping inverts.
    clipboardStore.autoPaste = false;
    await clipboardStore.useClip(item, 'default');
    expect(toastStore.toasts.at(-1)?.message).toContain(i18n.t.copyFailed);

    await clipboardStore.useClip(item, 'opposite');
    expect(toastStore.toasts.at(-1)?.message).toContain(i18n.t.pasteFailed);
  });

  test('useSelectedClips toasts on merge-paste failure and keeps the selection', async () => {
    installTauriInvoke((cmd) => {
      if (cmd === 'paste_clips') throw new Error('boom');
      return null;
    });

    clipboardStore.autoPaste = true;
    selectionStore.toggleSelected('a');
    selectionStore.toggleSelected('b');

    await clipboardStore.useSelectedClips('default');

    expect(toastStore.toasts.at(-1)?.message).toContain(i18n.t.pasteFailed);
    expect(toastStore.toasts.at(-1)?.type).toBe('error');
    // A failed merge-paste must not silently drop the selection.
    expect([...selectionStore.selectedIds]).toEqual(['a', 'b']);
  });

  test('loadHistory clears isLoading even when a search becomes active mid-load', async () => {
    let resolveRecent: (items: ClipItem[]) => void = () => {};
    installTauriInvoke((cmd) => {
      if (cmd === 'get_recent_clips') {
        return new Promise<ClipItem[]>((resolve) => {
          resolveRecent = resolve;
        });
      }
      if (cmd === 'get_pinned_clips') return [];
      return null;
    });

    // Fresh full load starts with the spinner up (showLoading defaults to true).
    const load = clipboardStore.loadHistory();
    expect(clipboardStore.isLoading).toBe(true);

    // User types a query before the history request resolves.
    clipboardStore.searchQuery = 'abc';

    resolveRecent([clip({ id: 'r1' })]);
    await load;

    // The spinner must clear even though a search is now active.
    expect(clipboardStore.isLoading).toBe(false);
  });

  test('loadHistory preserves visible items and exposes a retryable error', async () => {
    const existing = clip({ id: 'existing' });
    installTauriInvoke((cmd) => {
      if (cmd === 'get_recent_clips') throw new Error('database busy');
      if (cmd === 'get_pinned_clips') return [];
      return null;
    });
    clipboardStore.recentItems = [existing];

    await clipboardStore.loadHistory({ showLoading: false });

    expect(clipboardStore.recentItems).toEqual([existing]);
    expect(clipboardStore.historyError).toBe('database busy');
    expect(toastStore.toasts.at(-1)?.type).toBe('error');
  });

  test('toggleSelected preserves selection order and clears cleanly', () => {
    selectionStore.toggleSelected('a');
    selectionStore.toggleSelected('b');
    selectionStore.toggleSelected('c');
    expect([...selectionStore.selectedIds]).toEqual(['a', 'b', 'c']);

    // Toggling an existing id removes it; re-adding appends at the end.
    selectionStore.toggleSelected('b');
    expect([...selectionStore.selectedIds]).toEqual(['a', 'c']);
    selectionStore.toggleSelected('b');
    expect([...selectionStore.selectedIds]).toEqual(['a', 'c', 'b']);

    selectionStore.clearSelection();
    expect(selectionStore.selectedIds.size).toBe(0);
  });

  test('useSelectedClips merges selected ids in order then clears the selection', async () => {
    const calls: Array<{ cmd: string; args?: Record<string, unknown> }> = [];
    installTauriInvoke((cmd, args) => {
      calls.push({ cmd, args });
      return null;
    });

    selectionStore.toggleSelected('first');
    selectionStore.toggleSelected('second');
    selectionStore.toggleSelected('third');

    await clipboardStore.useSelectedClips();

    const pastes = calls.filter((entry) => entry.cmd === 'paste_clips');
    expect(pastes).toHaveLength(1);
    // Ids preserve selection order; default mode.
    expect(pastes[0].args).toEqual({ ids: ['first', 'second', 'third'], mode: 'default' });
    // Paste clears the selection.
    expect(selectionStore.selectedIds.size).toBe(0);
  });

  test('useSelectedClips forwards the resolved mode and is a no-op with no selection', async () => {
    const calls: Array<{ cmd: string; args?: Record<string, unknown> }> = [];
    installTauriInvoke((cmd, args) => {
      calls.push({ cmd, args });
      return null;
    });

    // No selection: nothing is invoked.
    await clipboardStore.useSelectedClips();
    expect(calls.filter((entry) => entry.cmd === 'paste_clips')).toHaveLength(0);

    // ⌘Enter resolves to the opposite paste mode, passed straight through.
    selectionStore.toggleSelected('x');
    await clipboardStore.useSelectedClips('opposite');
    const pastes = calls.filter((entry) => entry.cmd === 'paste_clips');
    expect(pastes).toHaveLength(1);
    expect(pastes[0].args).toEqual({ ids: ['x'], mode: 'opposite' });
  });

  test('deleting a clip drops it from the multi-selection', async () => {
    installTauriInvoke((cmd) => {
      if (cmd === 'delete_clip') return null;
      return null;
    });

    selectionStore.toggleSelected('keep');
    selectionStore.toggleSelected('gone');
    await clipboardStore.deleteItem('gone');

    expect([...selectionStore.selectedIds]).toEqual(['keep']);
  });

  test('silent search refreshes results without toggling the pending spinner', async () => {
    const match = clip({ id: 'match', timestamp: 5 });
    installTauriInvoke((cmd) => {
      if (cmd === 'search_clips') return [match];
      return null;
    });

    clipboardStore.searchQuery = 'needle';
    clipboardStore.activeSearchQuery = 'needle';
    clipboardStore.isSearchPending = false;

    await clipboardStore.search('needle', { silent: true });

    expect(clipboardStore.searchResults.map((item) => item.id)).toEqual(['match']);
    expect(clipboardStore.activeSearchQuery).toBe('needle');
    expect(clipboardStore.isSearchPending).toBe(false);
  });

  test('failed search cannot present results from an older query as current', async () => {
    installTauriInvoke((cmd) => {
      if (cmd === 'search_clips') throw new Error('search unavailable');
      return null;
    });
    clipboardStore.searchResults = [clip({ id: 'old-match' })];
    clipboardStore.activeSearchQuery = 'old';

    clipboardStore.setSearchQuery('new');
    await clipboardStore.search('new');

    expect(clipboardStore.searchResults).toEqual([]);
    expect(clipboardStore.activeSearchQuery).toBe('new');
    expect(clipboardStore.searchError).toBe('search unavailable');
    expect(clipboardStore.isSearchPending).toBe(false);
    expect(toastStore.toasts.at(-1)?.type).toBe('error');
  });

  test('failed silent refresh keeps results for the unchanged active query', async () => {
    installTauriInvoke((cmd) => {
      if (cmd === 'search_clips') throw new Error('search unavailable');
      return null;
    });
    const existing = clip({ id: 'existing-match' });
    clipboardStore.searchQuery = 'same';
    clipboardStore.activeSearchQuery = 'same';
    clipboardStore.searchResults = [existing];

    await clipboardStore.search('same', { silent: true });

    expect(clipboardStore.searchResults).toEqual([existing]);
    expect(clipboardStore.activeSearchQuery).toBe('same');
    expect(clipboardStore.searchError).toBe('search unavailable');
  });

  test('loadMoreRecent pages by the last item cursor and appends older rows', async () => {
    const calls: Array<Record<string, unknown> | undefined> = [];
    installTauriInvoke((cmd, args) => {
      if (cmd === 'get_recent_clips') {
        calls.push(args);
        return [clip({ id: 'p2a', timestamp: 3 }), clip({ id: 'p2b', timestamp: 2 })];
      }
      return null;
    });

    clipboardStore.recentItems = [
      clip({ id: 'p1a', timestamp: 5 }),
      clip({ id: 'p1b', timestamp: 4 }),
    ];
    clipboardStore.hasMoreRecent = true;

    await clipboardStore.loadMoreRecent();

    // Cursor is the (timestamp, id) of the last loaded row; limit is PAGE_SIZE + 1
    // (the sentinel row that decides hasMore).
    expect(calls).toEqual([{ limit: 101, beforeTimestamp: 4, beforeId: 'p1b' }]);
    expect(clipboardStore.recentItems.map((item) => item.id)).toEqual(['p1a', 'p1b', 'p2a', 'p2b']);
    // A short page (2 < PAGE_SIZE) marks the end.
    expect(clipboardStore.hasMoreRecent).toBe(false);
    expect(clipboardStore.isLoadingMore).toBe(false);
  });

  test('loadMoreRecent dedupes a row a live event bumped to the top mid-fetch', async () => {
    installTauriInvoke((cmd) => {
      if (cmd === 'get_recent_clips') {
        return [clip({ id: 'dup', timestamp: 9 }), clip({ id: 'p2', timestamp: 2 })];
      }
      return null;
    });

    clipboardStore.recentItems = [
      clip({ id: 'dup', timestamp: 9 }),
      clip({ id: 'p1', timestamp: 4 }),
    ];
    clipboardStore.hasMoreRecent = true;

    await clipboardStore.loadMoreRecent();

    expect(clipboardStore.recentItems.map((item) => item.id)).toEqual(['dup', 'p1', 'p2']);
  });

  test('loadMoreRecent is a no-op without more pages or while searching', async () => {
    let calls = 0;
    installTauriInvoke((cmd) => {
      if (cmd === 'get_recent_clips') calls += 1;
      return [];
    });

    clipboardStore.recentItems = [clip({ id: 'only' })];

    clipboardStore.hasMoreRecent = false;
    await clipboardStore.loadMoreRecent();
    expect(calls).toBe(0);

    clipboardStore.hasMoreRecent = true;
    clipboardStore.searchQuery = 'needle';
    clipboardStore.activeSearchQuery = 'needle';
    await clipboardStore.loadMoreRecent();
    expect(calls).toBe(0);
  });

  test('resetRecentPagination collapses accumulated pages to the first page', () => {
    const many = Array.from({ length: 150 }, (_, index) =>
      clip({ id: `item-${index}`, timestamp: 1000 - index })
    );
    clipboardStore.recentItems = many;
    clipboardStore.hasMoreRecent = false;
    clipboardStore.isLoadingMore = true;

    clipboardStore.resetRecentPagination();

    expect(clipboardStore.recentItems.length).toBe(100);
    expect(clipboardStore.recentItems[0].id).toBe('item-0');
    expect(clipboardStore.recentItems[99].id).toBe('item-99');
    // Dropping pages means there is definitely more to re-fetch on scroll.
    expect(clipboardStore.hasMoreRecent).toBe(true);
    expect(clipboardStore.isLoadingMore).toBe(false);
  });

  test('loadHistory fetches only the first page on a fresh load', async () => {
    const calls: Array<Record<string, unknown> | undefined> = [];
    installTauriInvoke((cmd, args) => {
      if (cmd === 'get_recent_clips') {
        calls.push(args);
        // Page + sentinel row (PAGE_SIZE + 1) so hasMore resolves true.
        return Array.from({ length: 101 }, (_, index) =>
          clip({ id: `r-${index}`, timestamp: 1000 - index })
        );
      }
      if (cmd === 'get_pinned_clips') return [];
      return null;
    });

    await clipboardStore.loadHistory({ showLoading: false });

    // Fresh load (recentItems empty) requests PAGE_SIZE + 1 (the sentinel), no cursor.
    expect(calls).toEqual([{ limit: 101 }]);
    // The sentinel row means older pages exist; it is trimmed from the list.
    expect(clipboardStore.hasMoreRecent).toBe(true);
    expect(clipboardStore.recentItems.length).toBe(100);
  });

  test('initialize subscribes before issuing the initial history query', async () => {
    const calls: string[] = [];
    installTauriInvoke((cmd, args) => {
      if (cmd === 'plugin:event|listen') {
        calls.push(`listen:${String(args?.event)}`);
        return calls.length;
      }
      calls.push(cmd);
      if (cmd === 'get_settings')
        return {
          autoPaste: true,
          maxHistoryItems: 100,
          capturePaused: false,
          pasteFormat: 'original',
          locale: 'en',
        };
      if (cmd === 'get_recent_clips' || cmd === 'get_pinned_clips') return [];
      return null;
    });

    await clipboardStore.initialize();

    expect(calls.filter((call) => call.startsWith('listen:'))).toEqual([
      'listen:clips-used',
      'listen:settings-changed',
      'listen:clipboard-changed',
      'listen:history-cleared',
      'listen:quickbar-hidden',
    ]);
    expect(calls.indexOf('listen:clipboard-changed')).toBeLessThan(
      calls.indexOf('get_recent_clips')
    );
  });
  test('a silent refresh shares an active search and clears the pending state', async () => {
    let finish!: (items: ClipItem[]) => void;
    let calls = 0;
    installTauriInvoke(() => {
      calls++;
      return new Promise<ClipItem[]>((resolve) => {
        finish = resolve;
      });
    });
    clipboardStore.setSearchQuery('needle');
    const foreground = clipboardStore.search('needle');
    const refresh = clipboardStore.search('needle', { silent: true });
    finish([clip({ id: 'result' })]);
    await Promise.all([foreground, refresh]);
    expect(calls).toBe(1);
    expect(clipboardStore.isSearchPending).toBe(false);
    expect(clipboardStore.searchResults[0].id).toBe('result');
  });

  test('opening during the initial history request cannot discard that request', async () => {
    let finish!: (items: ClipItem[]) => void;
    installTauriInvoke((cmd) =>
      cmd === 'get_recent_clips'
        ? new Promise<ClipItem[]>((resolve) => {
            finish = resolve;
          })
        : []
    );
    const loading = clipboardStore.loadHistory();
    clipboardStore.resetRecentPagination();
    finish([clip({ id: 'first' })]);
    await loading;
    expect(clipboardStore.isLoading).toBe(false);
    expect(clipboardStore.recentItems[0].id).toBe('first');
  });

  test('only one clipboard action can run, including copy and merge entry points', async () => {
    let finish!: (result: string) => void;
    let calls = 0;
    installTauriInvoke(() => {
      calls++;
      return new Promise<string>((resolve) => {
        finish = resolve;
      });
    });
    const first = clipboardStore.useClip(clip({ id: 'first' }));
    await clipboardStore.useClip(clip({ id: 'second' }));
    await clipboardStore.copyToClipboard(clip({ id: 'third' }));
    finish('copiedOnly');
    await first;
    expect(calls).toBe(1);
    expect(clipboardStore.isUsing).toBe(false);
    expect(clipboardStore.useNotice).toBe(i18n.t.copiedOnly);
  });

  test('search exposes the sentinel without rendering it', async () => {
    installTauriInvoke(() => Array.from({ length: 1001 }, (_, i) => clip({ id: String(i) })));
    clipboardStore.setSearchQuery('many');
    await clipboardStore.search('many');
    expect(clipboardStore.searchHasMore).toBe(true);
    expect(clipboardStore.searchResults.length).toBe(1000);
    await clipboardStore.clearSearch({ reload: false });
    expect(clipboardStore.searchHasMore).toBe(false);
  });
  test('a late detail response cannot resurrect a deleted item in the cache', async () => {
    let finish!: (value: unknown) => void;
    installTauriInvoke((cmd) =>
      cmd === 'get_clip'
        ? new Promise((resolve) => {
            finish = resolve;
          })
        : null
    );
    const detail = clipboardStore.fetchFullClip('deleted-while-loading');
    await clipboardStore.deleteItem('deleted-while-loading');
    finish({
      id: 'deleted-while-loading',
      contentType: 'text',
      text: 'private text',
      imageUrl: null,
    });
    expect(await detail).toBe(null);
    expect(clipboardStore.getCachedFullClip('deleted-while-loading')).toBe(undefined);
  });
  test('detail cache evicts old entries at sixteen and skips large text', async () => {
    installTauriInvoke((_cmd, args) => ({
      id: String(args?.id),
      contentType: 'text',
      text: 'x'.repeat(args?.id === 'large' ? 256 * 1024 + 1 : 10),
      imageUrl: null,
      truncated: false,
    }));
    for (let i = 0; i < 17; i++) await clipboardStore.fetchFullClip(`bounded-${i}`);
    expect(clipboardStore.getCachedFullClip('bounded-0')).toBeUndefined();
    expect(clipboardStore.getCachedFullClip('bounded-16')).toBeDefined();
    await clipboardStore.fetchFullClip('large');
    expect(clipboardStore.getCachedFullClip('large')).toBeUndefined();
  });

  test('a burst during history and search refreshes is coalesced without losing the last update', async () => {
    const reload = () =>
      (clipboardStore as unknown as { reloadFromBackend(): Promise<void> }).reloadFromBackend();
    let finishHistory!: (items: ClipItem[]) => void;
    let finishSearch!: (items: ClipItem[]) => void;
    let historyCalls = 0;
    let searchCalls = 0;
    installTauriInvoke((cmd) => {
      if (cmd === 'get_recent_clips') {
        historyCalls++;
        if (historyCalls === 1)
          return new Promise<ClipItem[]>((resolve) => {
            finishHistory = resolve;
          });
        return [clip({ id: 'latest' })];
      }
      if (cmd === 'search_clips') {
        searchCalls++;
        if (searchCalls === 1)
          return new Promise<ClipItem[]>((resolve) => {
            finishSearch = resolve;
          });
        return [clip({ id: 'latest' })];
      }
      return [];
    });
    clipboardStore.searchQuery = 'needle';
    const first = reload();
    const burst = Array.from({ length: 20 }, reload);
    expect(historyCalls).toBe(1);
    finishHistory([]);
    while (!finishSearch) await Promise.resolve();
    expect(historyCalls).toBe(2);
    const last = reload();
    finishSearch([]);
    await Promise.all([first, ...burst, last]);
    expect(historyCalls).toBe(3);
    expect(searchCalls).toBe(2);
    expect(clipboardStore.searchResults[0].id).toBe('latest');
    await reload();
    expect(historyCalls).toBe(4);
  });
});
