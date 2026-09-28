import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { selectionStore } from './selection.svelte';
import { toastStore } from './toast.svelte';
import { i18n } from '$lib/i18n';
import type {
  ClipItem,
  ClipDetail,
  PasteFormat,
  PasteMode,
  ReorderDirection,
  Settings,
} from '$lib/types';
import {
  applyClipboardChanged,
  getPinnedDisplayItems,
  getRecentDisplayItems,
} from '$lib/utils/clip-items';
import { RequestSequencer } from '$lib/utils/request-sequencer';

interface LoadHistoryOptions {
  showLoading?: boolean;
}

interface ClearSearchOptions {
  reload?: boolean;
}

const QUICKBAR_HIDDEN_EVENT = 'quickbar-hidden';

class ClipboardStore {
  recentItems = $state.raw<ClipItem[]>([]);
  pinnedItems = $state.raw<ClipItem[]>([]);
  searchResults = $state.raw<ClipItem[]>([]);
  searchQuery = $state('');
  activeSearchQuery = $state('');
  isLoading = $state(false);
  isSearchPending = $state(false);
  historyError = $state<string | null>(null);
  searchError = $state<string | null>(null);
  // Recent pages accumulate until a deliberate session/panel reset.
  hasMoreRecent = $state(false);
  isLoadingMore = $state(false);
  maxHistoryItems = $state(100);
  autoPaste = $state(true);
  pasteFormat = $state<PasteFormat>('original');
  isUsing = $state(false);
  useNotice = $state('');
  capturePaused = $state(false);
  searchHasMore = $state(false);
  private pageRequests = new RequestSequencer();
  private pendingSearch: Promise<void> | null = null;
  private pendingSearchQuery = '';
  private static readonly PAGE_SIZE = 100;
  private historyRequests = new RequestSequencer();
  private searchRequests = new RequestSequencer();
  private historyPending = false;
  private pendingReload: Promise<void> | null = null;
  private reloadNeeded = false;
  private incomingEvents: ClipItem[] = [];
  // One bounded cache of decoded details; original images are never cached.
  private cacheRevision = 0;
  private fullClipCache = new Map<string, ClipDetail>();
  private static readonly FULL_CLIP_CACHE_LIMIT = 16;
  // Skip caching very large payloads to bound memory; they re-fetch on demand
  // (rare, and only one item is previewed at a time).
  private static readonly MAX_CACHEABLE_CONTENT_LENGTH = 256 * 1024;

  recentDisplayItems = $derived(
    getRecentDisplayItems({
      activeSearchQuery: this.activeSearchQuery,
      searchResults: this.searchResults,
      recentItems: this.recentItems,
      pinnedItems: this.pinnedItems,
    })
  );

  pinnedDisplayItems = $derived(
    getPinnedDisplayItems({
      activeSearchQuery: this.activeSearchQuery,
      searchResults: this.searchResults,
      recentItems: this.recentItems,
      pinnedItems: this.pinnedItems,
    })
  );

  // Lazily initialized by the main QuickBar window in onMount. The settings
  // window imports this same singleton but must NOT initialize it — otherwise it
  // would needlessly pull a page of history and subscribe to every event.
  async initialize() {
    // Subscribe before the first query. Any copy that lands while history is
    // loading is recorded and replayed over the response by loadHistory.
    await listen('clips-used', () => {
      void this.reloadFromBackend();
    });
    await listen('settings-changed', () => {
      void this.refreshSettings();
    });
    await listen<ClipItem>('clipboard-changed', async (event) => {
      if (this.searchQuery.trim()) {
        await this.reloadFromBackend();
        return;
      }

      this.applyIncomingItem(event.payload);
    });

    await listen('history-cleared', async () => {
      this.cacheRevision += 1;
      this.fullClipCache.clear();
      selectionStore.clearSelection();
      await this.reloadFromBackend();
    });

    await listen(QUICKBAR_HIDDEN_EVENT, () => {
      selectionStore.reset();
      void this.clearSearch({ reload: false });
      this.resetRecentPagination();
    });

    await this.refreshSettings();
    await this.loadHistory();
  }

  async loadHistory(options: LoadHistoryOptions = {}) {
    const showLoading = options.showLoading ?? true;
    const requestId = this.historyRequests.next();
    this.pageRequests.next();
    this.isLoadingMore = false;
    this.historyPending = true;
    this.incomingEvents = [];

    if (showLoading) {
      this.isLoading = true;
    }
    this.historyError = null;

    // Fetch only the first page on a fresh load; on a reload (pin/delete/label/
    // clear) preserve however many pages the user already scrolled through, so
    // those actions don't snap the list back to the top.
    const pageLimit = Math.max(ClipboardStore.PAGE_SIZE, this.recentItems.length);

    try {
      // 多取一行作为哨兵：它存在才说明还有更早的记录，历史条数恰好是页大小的整数倍时
      // 不会出现一次空翻页。哨兵行在进入列表前去掉。
      const [recentRaw, pinned] = await Promise.all([
        invoke<ClipItem[]>('get_recent_clips', { limit: pageLimit + 1 }),
        invoke<ClipItem[]>('get_pinned_clips'),
      ]);

      if (this.historyRequests.isCurrent(requestId)) {
        const hasMore = recentRaw.length > pageLimit;
        const recent = hasMore ? recentRaw.slice(0, pageLimit) : recentRaw;

        let nextItems = { recentItems: recent, pinnedItems: pinned };
        for (const incoming of this.incomingEvents) {
          nextItems = applyClipboardChanged({
            ...nextItems,
            incoming,
            maxHistoryItems: this.maxHistoryItems,
          });
        }

        this.recentItems = nextItems.recentItems;
        this.pinnedItems = nextItems.pinnedItems;
        this.hasMoreRecent = hasMore;
        this.isLoadingMore = false;
        this.historyError = null;
        if (!this.searchQuery.trim()) {
          this.searchResults = [];
        }
      }
    } catch (error) {
      if (this.historyRequests.isCurrent(requestId)) {
        console.error('[ERROR] Failed to load clipboard history:', error);
        this.historyError = error instanceof Error ? error.message : String(error);
        toastStore.add(`${i18n.t.history}: ${this.historyError}`, 'error');
      }
    } finally {
      // 无论是否有搜索都结束当前请求的加载状态：搜索流程只管理 isSearchPending，
      // 加载中途开始的搜索不会替它清除 isLoading。
      if (this.historyRequests.isCurrent(requestId)) {
        this.isLoading = false;
        this.historyPending = false;
        this.incomingEvents = [];
      }
    }
  }

  /**
   * Load the next keyset page of recent clips, using the last loaded item's
   * (timestamp, id) as the cursor. Debounced by `isLoadingMore` and gated by
   * `hasMoreRecent`; never runs while a search is active (search keeps its own
   * capped result set). Triggered by scrolling near the bottom or arrowing past
   * the loaded tail.
   */
  async loadMoreRecent() {
    if (this.isLoadingMore || !this.hasMoreRecent) return;
    if (this.searchQuery.trim() || this.activeSearchQuery.trim()) return;

    const cursor = this.recentItems[this.recentItems.length - 1];
    if (!cursor) {
      this.hasMoreRecent = false;
      return;
    }

    this.isLoadingMore = true;
    this.historyError = null;
    // Page resets invalidate append requests without cancelling the initial load.
    const requestId = this.pageRequests.next();

    try {
      // 与 loadHistory 相同，多取的一行是哨兵。
      const page = await invoke<ClipItem[]>('get_recent_clips', {
        limit: ClipboardStore.PAGE_SIZE + 1,
        beforeTimestamp: cursor.timestamp,
        beforeId: cursor.id,
      });

      if (!this.pageRequests.isCurrent(requestId)) return;

      const hasMore = page.length > ClipboardStore.PAGE_SIZE;
      const pageItems = hasMore ? page.slice(0, ClipboardStore.PAGE_SIZE) : page;

      // Append older rows, deduping by id: a live clipboard-changed event may
      // have bumped one of these rows to the top mid-fetch, and it must not
      // reappear lower in the list.
      const existingIds = new Set(this.recentItems.map((item) => item.id));
      const olderItems = pageItems.filter((item) => !existingIds.has(item.id));
      this.recentItems = [...this.recentItems, ...olderItems];
      this.hasMoreRecent = hasMore;
      this.historyError = null;
    } catch (error) {
      if (this.pageRequests.isCurrent(requestId)) {
        console.error('[ERROR] Failed to load more clipboard history:', error);
        this.historyError = error instanceof Error ? error.message : String(error);
        toastStore.add(`${i18n.t.history}: ${this.historyError}`, 'error');
      }
    } finally {
      if (this.pageRequests.isCurrent(requestId)) {
        this.isLoadingMore = false;
      }
    }
  }

  /**
   * Collapse accumulated pages back to the first page (panel switch to recent,
   * quickbar-opened). No IPC — the live clipboard-changed
   * stream keeps page 1 fresh while hidden; this only drops the extra pages a
   * scroll accumulated, and supersedes any in-flight page load.
   */
  resetRecentPagination() {
    this.pageRequests.next();
    this.isLoadingMore = false;
    if (this.recentItems.length > ClipboardStore.PAGE_SIZE) {
      this.recentItems = this.recentItems.slice(0, ClipboardStore.PAGE_SIZE);
      this.hasMoreRecent = true;
    }
  }

  async refreshSettings() {
    try {
      const settings = await invoke<Settings>('get_settings');
      this.autoPaste = settings.autoPaste;
      this.capturePaused = settings.capturePaused;
      this.maxHistoryItems = settings.maxHistoryItems;
      this.pasteFormat = settings.pasteFormat;
      i18n.setLocale(settings.locale);
    } catch (error) {
      console.error('Failed to refresh settings:', error);
    }
  }

  /** 暂存非空的搜索草稿；pending 表示随后会发起搜索（输入法组合期间为 false）。 */
  setSearchQuery(query: string, pending = true) {
    this.searchRequests.next();
    this.pendingSearch = null;
    selectionStore.cancelUse();
    this.searchQuery = query;
    this.isSearchPending = pending;
    this.searchError = null;
  }

  search(query: string, options: { silent?: boolean } = {}) {
    // Share an in-flight query. A background refresh must not steal its pending state.
    if (
      this.pendingSearch &&
      this.pendingSearchQuery === query &&
      this.searchQuery === query &&
      this.isSearchPending
    ) {
      return this.pendingSearch;
    }
    this.pendingSearchQuery = query;
    const task = this.runSearch(query, options);
    this.pendingSearch = task;
    void task.finally(() => {
      if (this.pendingSearch === task) this.pendingSearch = null;
    });
    return task;
  }

  private async runSearch(query: string, options: { silent?: boolean } = {}) {
    if (query.trim() && this.searchQuery !== query) {
      return;
    }
    if (!query.trim()) {
      return this.clearSearch();
    }

    // A silent search refreshes the results of the *same* query in place — e.g.
    // after a copy bumps a row's timestamp while a query is active. It must not
    // toggle the pending spinner, otherwise the search icon flickers on every
    // background refresh even though the user never typed anything.
    const silent = options.silent ?? false;
    const requestId = this.searchRequests.next();
    this.searchQuery = query;
    this.searchError = null;

    if (!silent) {
      this.isSearchPending = true;
    }
    // `isCurrent` alone guarantees `searchQuery` is still `query`: every
    // assignment to `searchQuery` bumps the sequencer first.
    try {
      const results = await invoke<ClipItem[]>('search_clips', { query });
      if (this.searchRequests.isCurrent(requestId)) {
        this.searchHasMore = results.length > 1000;
        this.searchResults = results.slice(0, 1000);
        if (this.activeSearchQuery !== query) selectionStore.resetSelection();
        this.activeSearchQuery = query;
        this.searchError = null;
      }
    } catch (error) {
      if (this.searchRequests.isCurrent(requestId)) {
        console.error('Search failed:', error);
        // A foreground failure must not leave results from an older query under
        // the new input. A silent refresh is always for the same active query,
        // so keep its still-valid visible results.
        if (!silent) {
          this.searchResults = [];
          this.activeSearchQuery = query;
        }
        this.searchError = error instanceof Error ? error.message : String(error);
        toastStore.add(`${i18n.t.errorLabel}: ${this.searchError}`, 'error');
      }
    } finally {
      if (this.searchRequests.isCurrent(requestId)) {
        this.isSearchPending = false;
      }
    }
  }

  async clearSearch(options: ClearSearchOptions = {}) {
    this.searchRequests.next();
    this.pendingSearch = null;
    selectionStore.cancelUse();
    this.searchHasMore = false;
    this.searchQuery = '';
    this.activeSearchQuery = '';
    this.searchResults = [];
    this.isSearchPending = false;
    this.searchError = null;
    this.isLoading = false;

    if (options.reload ?? true) {
      await this.loadHistory({ showLoading: false });
    }
  }

  async clearNonPinned() {
    // 后端清除后发出 history-cleared，由该事件的监听负责清缓存和重新加载。
    await invoke('clear_non_pinned_history');
  }

  async togglePin(id: string) {
    const item = this.findItem(id);
    if (!item) return;

    try {
      await invoke('toggle_pin', { id, isPinned: !item.isPinned });
      await this.reloadFromBackend();
    } catch (error) {
      toastStore.add(String(error), 'error');
    }
  }

  async deleteItem(id: string) {
    try {
      await invoke('delete_clip', { id });
      this.removeClipLocally(id);
    } catch (error) {
      toastStore.add(String(error), 'error');
    }
  }

  async setClipLabel(id: string, label: string) {
    const normalizedLabel = label.trim();

    try {
      await invoke('set_clip_label', {
        id,
        label: normalizedLabel.length > 0 ? normalizedLabel : null,
      });
      await this.reloadFromBackend();
    } catch (error) {
      console.error('Failed to set clip label:', error);
      throw error;
    }
  }

  async reorderPinned(id: string, direction: ReorderDirection) {
    try {
      await invoke('reorder_pinned', { id, direction });
      await this.reloadFromBackend();
    } catch (error) {
      console.error('Failed to reorder pinned item:', error);
      throw error;
    }
  }

  /** takePlain/globalPlain: ClipMan-mediated takes write plain text only. */
  get takesPlainText(): boolean {
    return this.pasteFormat !== 'original';
  }

  async useClip(item: ClipItem, mode: PasteMode = 'default', options: { plain?: boolean } = {}) {
    await this.performUse(
      'paste_clip',
      { id: item.id, mode, plain: options.plain ?? this.takesPlainText },
      mode
    );
  }

  private async performUse(command: string, args: Record<string, unknown>, mode: PasteMode) {
    if (this.isUsing) return false;
    this.isUsing = true;
    this.useNotice = '';
    try {
      const result = await invoke<'copied' | 'pasteRequested' | 'copiedOnly'>(command, args);
      if (result === 'copiedOnly') this.useNotice = i18n.t.copiedOnly;
      return true;
    } catch (error) {
      toastStore.add(`${this.pasteFailureMessage(mode)}: ${String(error)}`, 'error');
      return false;
    } finally {
      this.isUsing = false;
    }
  }

  /**
   * Toast text for a failed paste/copy. The resolved action mirrors the backend:
   * mode 'default' honors the auto-paste setting, 'opposite' (⌘Enter) inverts it.
   * So the action is a paste iff `(mode === 'default') === autoPaste` — the same
   * mapping the footer hints use — which decides whether to say "paste" or "copy".
   */
  private pasteFailureMessage(mode: PasteMode): string {
    const isPaste = (mode === 'default') === this.autoPaste;
    return isPaste ? i18n.t.pasteFailed : i18n.t.copyFailed;
  }

  /**
   * Merge the multi-selected clips (in selection order) into a single clipboard
   * write and paste them, newline-separated. No-op when nothing is
   * selected; clears the selection once the paste is dispatched.
   */
  async useSelectedClips(mode: PasteMode = 'default') {
    const ids = [...selectionStore.selectedIds];
    if (!ids.length || this.isSearchPending) return;
    if (await this.performUse('paste_clips', { ids, mode }, mode)) {
      selectionStore.clearSelection();
    }
  }

  async hideQuickbar() {
    selectionStore.cancelUse();
    try {
      await invoke('hide_quickbar');
    } catch (error) {
      console.error('[ERROR] Failed to hide QuickBar:', error);
    }
  }

  async copyToClipboard(item: ClipItem): Promise<boolean> {
    const succeeded = await this.performUse(
      'copy_to_system_clipboard',
      { clipId: item.id },
      this.autoPaste ? 'opposite' : 'default'
    );
    if (succeeded) {
      toastStore.add(i18n.t.copied, 'success');
    }
    return succeeded;
  }

  /** Synchronously read a cached full clip (no fetch). */
  getCachedFullClip(id: string): ClipDetail | undefined {
    return this.fullClipCache.get(id);
  }

  /** Fetch decoded details on demand, caching only small text/file payloads. */
  async fetchFullClip(id: string): Promise<ClipDetail | null> {
    const cached = this.getCachedFullClip(id);
    if (cached) return cached;

    const revision = this.cacheRevision;
    try {
      const full = await invoke<ClipDetail | null>('get_clip', { id });
      if (revision !== this.cacheRevision) return null;
      if (
        full &&
        full.contentType !== 'image' &&
        full.text.length <= ClipboardStore.MAX_CACHEABLE_CONTENT_LENGTH
      ) {
        this.fullClipCache.set(id, full);
        while (this.fullClipCache.size > ClipboardStore.FULL_CLIP_CACHE_LIMIT) {
          this.fullClipCache.delete(this.fullClipCache.keys().next().value!);
        }
      }
      return full;
    } catch (error) {
      toastStore.add(String(error), 'error');
      return null;
    }
  }

  private reloadFromBackend(): Promise<void> {
    this.reloadNeeded = true;
    if (this.pendingReload) return this.pendingReload;
    this.pendingReload = this.drainReloads();
    return this.pendingReload;
  }

  private async drainReloads() {
    try {
      do {
        this.reloadNeeded = false;
        await this.loadHistory({ showLoading: false });
        await this.pendingSearch;
        // A newer invalidation needs fresh history before another search.
        if (!this.reloadNeeded && this.searchQuery.trim()) {
          await this.search(this.searchQuery, { silent: true });
        }
      } while (this.reloadNeeded);
    } finally {
      this.pendingReload = null;
    }
  }

  private applyIncomingItem(incoming: ClipItem) {
    if (this.historyPending) this.incomingEvents.push(incoming);

    const nextItems = applyClipboardChanged({
      recentItems: this.recentItems,
      pinnedItems: this.pinnedItems,
      incoming,
      maxHistoryItems: this.maxHistoryItems,
    });

    this.recentItems = nextItems.recentItems;
    this.pinnedItems = nextItems.pinnedItems;
  }

  private removeClipLocally(id: string) {
    selectionStore.cancelUse();
    this.historyRequests.next();
    this.historyPending = false;
    this.incomingEvents = [];
    this.pageRequests.next();
    this.searchRequests.next();
    this.cacheRevision += 1;
    this.fullClipCache.delete(id);
    selectionStore.selectedIds.delete(id);
    this.recentItems = this.recentItems.filter((item) => item.id !== id);
    this.pinnedItems = this.pinnedItems.filter((item) => item.id !== id);
    this.searchResults = this.searchResults.filter((item) => item.id !== id);
    this.isLoading = false;
    this.isSearchPending = false;
  }

  private findItem(id: string) {
    return (
      this.recentItems.find((item) => item.id === id) ??
      this.pinnedItems.find((item) => item.id === id) ??
      this.searchResults.find((item) => item.id === id)
    );
  }
}

export const clipboardStore = new ClipboardStore();
