import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { selectionStore } from './selection.svelte';
import { toastStore } from './toast.svelte';
import { confirmStore } from './confirm.svelte';
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
  private pendingReload: Promise<void> | null = null;
  private reloadNeeded = false;
  private incomingEvents: ClipItem[] | null = null;
  private cacheRevision = 0;
  private fullClipCache = new Map<string, ClipDetail>();
  private static readonly FULL_CLIP_CACHE_LIMIT = 16;
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

  async initialize() {
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

    await listen('quickbar-hidden', () => {
      selectionStore.reset();
      void this.clearSearch({ reload: false });
      this.resetRecentPagination();
    });

    await this.refreshSettings();
    await this.loadHistory();
  }

  async loadHistory(options: { showLoading?: boolean } = {}) {
    const showLoading = options.showLoading ?? true;
    const requestId = this.historyRequests.next();
    this.pageRequests.next();
    this.isLoadingMore = false;
    this.incomingEvents = [];

    if (showLoading) {
      this.isLoading = true;
    }
    this.historyError = null;

    const pageLimit = Math.max(ClipboardStore.PAGE_SIZE, this.recentItems.length);

    try {
      const [recentRaw, pinned] = await Promise.all([
        invoke<ClipItem[]>('get_recent_clips', { limit: pageLimit + 1 }),
        invoke<ClipItem[]>('get_pinned_clips'),
      ]);

      if (this.historyRequests.isCurrent(requestId)) {
        const hasMore = recentRaw.length > pageLimit;
        const recent = hasMore ? recentRaw.slice(0, pageLimit) : recentRaw;

        let nextItems = { recentItems: recent, pinnedItems: pinned };
        for (const incoming of this.incomingEvents ?? []) {
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
      if (this.historyRequests.isCurrent(requestId)) {
        this.isLoading = false;
        this.incomingEvents = null;
      }
    }
  }

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
    const requestId = this.pageRequests.next();

    try {
      const page = await invoke<ClipItem[]>('get_recent_clips', {
        limit: ClipboardStore.PAGE_SIZE + 1,
        beforeTimestamp: cursor.timestamp,
        beforeId: cursor.id,
      });

      if (!this.pageRequests.isCurrent(requestId)) return;

      const hasMore = page.length > ClipboardStore.PAGE_SIZE;
      const pageItems = hasMore ? page.slice(0, ClipboardStore.PAGE_SIZE) : page;

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

  setSearchQuery(query: string, pending = true) {
    this.searchRequests.next();
    this.pendingSearch = null;
    selectionStore.cancelUse();
    this.searchQuery = query;
    this.isSearchPending = pending;
    this.searchError = null;
  }

  search(query: string, options: { silent?: boolean } = {}) {
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

    const silent = options.silent ?? false;
    const requestId = this.searchRequests.next();
    this.searchError = null;

    if (!silent) {
      this.isSearchPending = true;
    }
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

  async clearSearch(options: { reload?: boolean } = {}) {
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
    const t = i18n.t;
    const confirmed = await confirmStore.ask({
      title: t.clearNonPinned,
      message: t.confirmClearHistory,
      confirmLabel: t.clear,
      destructive: true,
    });
    if (!confirmed) return;
    try {
      await invoke('clear_non_pinned_history');
    } catch (error) {
      toastStore.add(`${t.clearFailed}: ${String(error)}`, 'error');
    }
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
    await invoke('set_clip_label', { id, label });
    await this.reloadFromBackend();
  }

  async reorderPinned(id: string, direction: ReorderDirection) {
    await invoke('reorder_pinned', { id, direction });
    await this.reloadFromBackend();
  }

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
      const isPaste = (mode === 'default') === this.autoPaste;
      const failure = isPaste ? i18n.t.pasteFailed : i18n.t.copyFailed;
      toastStore.add(`${failure}: ${String(error)}`, 'error');
      return false;
    } finally {
      this.isUsing = false;
    }
  }

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

  getCachedFullClip(id: string): ClipDetail | undefined {
    return this.fullClipCache.get(id);
  }

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
        if (!this.reloadNeeded && this.searchQuery.trim()) {
          await this.search(this.searchQuery, { silent: true });
        }
      } while (this.reloadNeeded);
    } finally {
      this.pendingReload = null;
    }
  }

  private applyIncomingItem(incoming: ClipItem) {
    this.incomingEvents?.push(incoming);

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
    this.incomingEvents = null;
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
