import { beforeEach, describe, expect, test } from 'bun:test';
import type { ClipItem } from '../../src/lib/types';

type InvokeHandler = (cmd: string, args?: Record<string, unknown>) => unknown;

const state = (<T>(value: T) => value) as (<T>(value: T) => T) & {
  raw: <T>(value: T) => T;
};
state.raw = <T>(value: T) => value;

(globalThis as typeof globalThis & { $state: typeof state }).$state = state;
(globalThis as typeof globalThis & { $derived: <T>(value: T) => T }).$derived = (value) => value;

Object.defineProperty(globalThis, 'navigator', {
  configurable: true,
  value: {
    language: 'en-US',
    platform: 'MacIntel',
  },
});

const storage = new Map<string, string>();

(globalThis as typeof globalThis & { localStorage: Storage }).localStorage = {
  get length() {
    return storage.size;
  },
  clear() {
    storage.clear();
  },
  getItem(key: string) {
    return storage.get(key) ?? null;
  },
  key(index: number) {
    return [...storage.keys()][index] ?? null;
  },
  removeItem(key: string) {
    storage.delete(key);
  },
  setItem(key: string, value: string) {
    storage.set(key, value);
  },
};

(globalThis as typeof globalThis & { window: Window }).window = {
  addEventListener() {},
  removeEventListener() {},
} as Window;

const { clipboardStore } = await import('../../src/lib/stores/clipboard.svelte');

function clip(overrides: Partial<ClipItem>): ClipItem {
  return {
    id: 'clip',
    content: 'aGVsbG8=',
    contentType: 'text',
    timestamp: 1,
    isPinned: false,
    pinOrder: null,
    label: null,
    groupName: null,
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
  clipboardStore.isLoading = false;
  clipboardStore.maxHistoryItems = 100;
  clipboardStore.autoPaste = true;
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
    // setSearchQuery flags a pending search (not a full-screen load) when a Tauri
    // runtime is present, which installTauriInvoke provides.
    expect(clipboardStore.isSearchPending).toBe(true);

    await clipboardStore.clearSearch({ showLoading: false });

    expect(clipboardStore.searchQuery).toBe('');
    expect(clipboardStore.isLoading).toBe(false);
  });

  test('fetchFullClip caches by id and drops the entry on delete', async () => {
    let getClipCalls = 0;
    installTauriInvoke((cmd, args) => {
      if (cmd === 'get_clip') {
        getClipCalls += 1;
        return clip({ id: String(args?.id), content: 'ZnVsbA==' });
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

  test('fetchFullClip ignores non-text responses and does not cache them', async () => {
    let getClipCalls = 0;
    installTauriInvoke((cmd, args) => {
      if (cmd === 'get_clip') {
        getClipCalls += 1;
        return clip({
          id: String(args?.id),
          content: 'data:image/png;base64,aW1hZ2U=',
          contentType: 'image',
        });
      }
      return null;
    });

    await expect(clipboardStore.fetchFullClip('image')).resolves.toBeNull();
    await expect(clipboardStore.fetchFullClip('image')).resolves.toBeNull();
    expect(getClipCalls).toBe(2);
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
});
