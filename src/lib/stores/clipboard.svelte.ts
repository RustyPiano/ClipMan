import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { selectionStore, type QuickBarPanel } from './selection.svelte';
import { toastStore } from './toast.svelte';
import { confirmStore } from './confirm.svelte';
import { i18n } from '$lib/i18n';
import type {
  ClipFilter,
  ClipItem,
  ClipDetail,
  PasteFormat,
  PasteMode,
  ReorderDirection,
  Settings,
} from '$lib/types';
import {
  applyClipboardChanged,
  comparePinOrder,
  encodeClipTextPreview,
  getPinnedDisplayItems,
  getRecentDisplayItems,
  matchesClipFilter,
} from '$lib/utils/clip-items';
import { RequestSequencer } from '$lib/utils/request-sequencer';

class ClipboardStore {
  recentItems = $state.raw<ClipItem[]>([]);
  pinnedItems = $state.raw<ClipItem[]>([]);
  searchResults = $state.raw<ClipItem[]>([]);
  searchQuery = $state('');
  activeSearchQuery = $state('');
  filter = $state<ClipFilter>('all');
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
  newFavoriteOpen = $state(false);
  newFavoriteLabel = $state('');
  newFavoriteContent = $state('');
  contentDrafts = $state<Record<string, string>>({});
  editingClipId = $state<string | null>(null);
  editingLabelId = $state<string | null>(null);
  private pageRequests = new RequestSequencer();
  private pendingSearch: Promise<void> | null = null;
  private pendingSearchQuery = '';
  private pendingSearchFilter: ClipFilter = 'all';
  private static readonly PAGE_SIZE = 100;
  private historyRequests = new RequestSequencer();
  private searchRequests = new RequestSequencer();
  private pendingReload: Promise<void> | null = null;
  private reloadNeeded = false;
  private incomingEvents: ClipItem[] | null = null;
  private cacheRevision = 0;
  private fullClipCache = new Map<string, ClipDetail>();
  private imageDetailCache = new Map<string, ClipDetail>();
  private pendingPinIds = new Set<string>();
  private static readonly FULL_CLIP_CACHE_LIMIT = 16;
  private static readonly IMAGE_DETAIL_CACHE_LIMIT = 4;
  private static readonly MAX_CACHEABLE_CONTENT_LENGTH = 256 * 1024;

  get isEditing(): boolean {
    return this.editingClipId !== null || this.editingLabelId !== null || this.newFavoriteOpen;
  }

  recentDisplayItems = $derived(
    getRecentDisplayItems({
      activeSearchQuery: this.activeSearchQuery,
      searchResults: this.searchResults,
      recentItems: this.recentItems,
      pinnedItems: this.pinnedItems,
    }).filter((item) => matchesClipFilter(item, this.filter))
  );

  pinnedDisplayItems = $derived(
    getPinnedDisplayItems({
      activeSearchQuery: this.activeSearchQuery,
      searchResults: this.searchResults,
      recentItems: this.recentItems,
      pinnedItems: this.pinnedItems,
    }).filter((item) => matchesClipFilter(item, this.filter))
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
      this.imageDetailCache.clear();
      this.clearUnpinnedDrafts();
      selectionStore.clearSelection();
      await this.reloadFromBackend();
    });

    await listen('quickbar-hidden', () => {
      selectionStore.reset();
      const hadFilter = this.filter !== 'all';
      this.filter = 'all';
      void this.clearSearch({ reload: hadFilter });
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
    const activeFilter = this.filter;

    try {
      const [recentRaw, pinned] = await Promise.all([
        invoke<ClipItem[]>('get_recent_clips', {
          limit: pageLimit + 1,
          ...(activeFilter !== 'all' ? { filter: activeFilter } : {}),
        }),
        invoke<ClipItem[]>('get_pinned_clips'),
      ]);

      if (this.historyRequests.isCurrent(requestId)) {
        const hasMore = recentRaw.length > pageLimit;
        const recent = hasMore ? recentRaw.slice(0, pageLimit) : recentRaw;

        let nextItems = { recentItems: recent, pinnedItems: pinned };
        for (const incoming of this.incomingEvents ?? []) {
          if (
            activeFilter !== 'all' &&
            !incoming.isPinned &&
            !matchesClipFilter(incoming, activeFilter)
          ) {
            continue;
          }
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

    const activeFilter = this.filter;

    try {
      const page = await invoke<ClipItem[]>('get_recent_clips', {
        limit: ClipboardStore.PAGE_SIZE + 1,
        beforeTimestamp: cursor.timestamp,
        beforeId: cursor.id,
        ...(activeFilter !== 'all' ? { filter: activeFilter } : {}),
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
      toastStore.add(`${i18n.t.loadSettingsFailed}: ${String(error)}`, 'error');
    }
  }

  setFilter(next: ClipFilter) {
    if (this.filter === next) return;
    this.filter = next;
    selectionStore.resetSelection();
    if (this.searchQuery.trim()) {
      void this.search(this.searchQuery);
    } else {
      this.recentItems = [];
      this.hasMoreRecent = false;
      void this.loadHistory({ showLoading: false });
    }
  }

  setContentDraft(id: string, value: string) {
    this.contentDrafts = { ...this.contentDrafts, [id]: value };
  }

  clearContentDraft(id: string) {
    if (!(id in this.contentDrafts)) return;
    const next = { ...this.contentDrafts };
    delete next[id];
    this.contentDrafts = next;
  }

  resetNewFavoriteDraft() {
    this.newFavoriteOpen = false;
    this.newFavoriteLabel = '';
    this.newFavoriteContent = '';
  }

  syncQuickbarPanel(panel: QuickBarPanel) {
    void invoke('set_quickbar_panel', { panel }).catch(() => {});
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
      this.pendingSearchFilter === this.filter &&
      this.searchQuery === query &&
      this.isSearchPending
    ) {
      return this.pendingSearch;
    }
    this.pendingSearchQuery = query;
    this.pendingSearchFilter = this.filter;
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
    const activeFilter = this.filter;
    try {
      const results = await invoke<ClipItem[]>('search_clips', {
        query,
        ...(activeFilter !== 'all' ? { filter: activeFilter } : {}),
      });
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
    if (this.pendingPinIds.has(id)) return;
    const item = this.findItem(id);
    if (!item) return;

    const nextPinned = !item.isPinned;
    this.pendingPinIds.add(id);
    this.historyRequests.next();
    this.incomingEvents = null;
    this.applyPinLocally([id], nextPinned);

    try {
      await invoke('toggle_pin', { id, isPinned: nextPinned });
      toastStore.add(
        nextPinned ? i18n.t.pinnedToast : i18n.t.unpinnedToast,
        nextPinned ? 'pin' : 'info'
      );
      await this.reloadFromBackend();
    } catch (error) {
      toastStore.add(String(error), 'error');
      await this.reloadFromBackend();
    } finally {
      this.pendingPinIds.delete(id);
    }
  }

  async batchTogglePin(ids: string[]) {
    if (!ids.length) return;
    const items = ids.map((id) => this.findItem(id)).filter((item): item is ClipItem => !!item);
    if (!items.length) return;

    const nextPinned = items.some((item) => !item.isPinned);
    const targetIds = items
      .filter((item) => item.isPinned !== nextPinned && !this.pendingPinIds.has(item.id))
      .map((item) => item.id);
    if (!targetIds.length) return;

    for (const id of targetIds) this.pendingPinIds.add(id);
    this.historyRequests.next();
    this.incomingEvents = null;
    this.applyPinLocally(targetIds, nextPinned);

    try {
      const changed = await invoke<number>('batch_toggle_pin', {
        ids: targetIds,
        isPinned: nextPinned,
      });
      selectionStore.clearSelection();
      const count = typeof changed === 'number' ? changed : targetIds.length;
      if (count > 0) {
        toastStore.add(
          i18n.format(nextPinned ? i18n.t.batchPinnedToast : i18n.t.batchUnpinnedToast, {
            n: count,
          }),
          nextPinned ? 'pin' : 'info'
        );
      }
      await this.reloadFromBackend();
    } catch (error) {
      toastStore.add(String(error), 'error');
      await this.reloadFromBackend();
    } finally {
      for (const id of targetIds) this.pendingPinIds.delete(id);
    }
  }

  async deleteItem(id: string) {
    try {
      await invoke('delete_clip', { id });
      this.removeClipsLocally([id]);
      toastStore.add(i18n.t.deletedToast, 'info');
    } catch (error) {
      toastStore.add(String(error), 'error');
    }
  }

  async batchDeleteItems(ids: string[]) {
    if (!ids.length) return;
    const t = i18n.t;
    const confirmed = await confirmStore.ask({
      title: t.deleteSelected,
      message: i18n.format(t.confirmDeleteSelected, { n: ids.length }),
      confirmLabel: t.delete,
      destructive: true,
    });
    if (!confirmed) return;

    try {
      await invoke('batch_delete_clips', { ids });
      this.removeClipsLocally(ids);
      toastStore.add(i18n.format(t.batchDeletedToast, { n: ids.length }), 'info');
    } catch (error) {
      toastStore.add(String(error), 'error');
    }
  }

  async setClipLabel(id: string, label: string) {
    const normalized = label.trim() || null;
    await invoke('set_clip_label', { id, label });
    const patch = (item: ClipItem) => (item.id === id ? { ...item, label: normalized } : item);
    this.recentItems = this.recentItems.map(patch);
    this.pinnedItems = this.pinnedItems.map(patch);
    this.searchResults = this.searchResults.map(patch);
    toastStore.add(i18n.t.labelSavedToast, 'success');
    await this.reloadFromBackend();
  }

  async updateClipContent(id: string, content: string) {
    await invoke('update_clip_content', { id, content });
    const encoded = encodeClipTextPreview(content);
    const contentBytes = new TextEncoder().encode(content).length;
    const patch = (item: ClipItem) =>
      item.id === id ? { ...item, content: encoded, hasHtml: false, contentBytes } : item;
    this.recentItems = this.recentItems.map(patch);
    this.pinnedItems = this.pinnedItems.map(patch);
    this.searchResults = this.searchResults.map(patch);
    this.cacheRevision += 1;
    this.fullClipCache.delete(id);
    toastStore.add(i18n.t.contentSavedToast, 'success');
    await this.reloadFromBackend();
  }

  async createPinnedClip(content: string, label: string): Promise<ClipItem | null> {
    try {
      const created = await invoke<ClipItem>('create_pinned_clip', {
        content,
        label: label.trim() || null,
      });
      this.pinnedItems = [...this.pinnedItems, created].sort(comparePinOrder);
      toastStore.add(i18n.t.createdPinnedToast, 'pin');
      return created;
    } catch (error) {
      toastStore.add(String(error), 'error');
      return null;
    }
  }

  async reorderPinned(id: string, direction: ReorderDirection) {
    await invoke('reorder_pinned', { id, direction });
    await this.reloadFromBackend();
  }

  async openUrl(url: string) {
    try {
      await invoke('open_url', { url });
    } catch (error) {
      toastStore.add(String(error), 'error');
    }
  }

  async revealInFolder(path: string) {
    try {
      await invoke('reveal_in_folder', { path });
    } catch (error) {
      toastStore.add(String(error), 'error');
    }
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
      toastStore.add(String(error), 'error');
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

  async copyPlainText(text: string): Promise<boolean> {
    try {
      await invoke('copy_plain_text', { text });
      return true;
    } catch (error) {
      toastStore.add(String(error), 'error');
      return false;
    }
  }

  getCachedFullClip(id: string): ClipDetail | undefined {
    return this.fullClipCache.get(id) ?? this.imageDetailCache.get(id);
  }

  async fetchFullClip(id: string): Promise<ClipDetail | null> {
    const cached = this.getCachedFullClip(id);
    if (cached) return cached;

    const revision = this.cacheRevision;
    try {
      const full = await invoke<ClipDetail | null>('get_clip', { id });
      if (revision !== this.cacheRevision) return null;
      if (full) {
        if (full.contentType === 'image' && full.imageUrl) {
          this.imageDetailCache.delete(id);
          this.imageDetailCache.set(id, full);
          while (this.imageDetailCache.size > ClipboardStore.IMAGE_DETAIL_CACHE_LIMIT) {
            this.imageDetailCache.delete(this.imageDetailCache.keys().next().value!);
          }
        } else if (
          full.contentType !== 'image' &&
          full.text.length <= ClipboardStore.MAX_CACHEABLE_CONTENT_LENGTH
        ) {
          this.fullClipCache.set(id, full);
          while (this.fullClipCache.size > ClipboardStore.FULL_CLIP_CACHE_LIMIT) {
            this.fullClipCache.delete(this.fullClipCache.keys().next().value!);
          }
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
    if (this.filter !== 'all' && !incoming.isPinned && !matchesClipFilter(incoming, this.filter)) {
      return;
    }

    const nextItems = applyClipboardChanged({
      recentItems: this.recentItems,
      pinnedItems: this.pinnedItems,
      incoming,
      maxHistoryItems: this.maxHistoryItems,
    });

    this.recentItems = nextItems.recentItems;
    this.pinnedItems = nextItems.pinnedItems;
  }

  private applyPinLocally(ids: string[], nextPinned: boolean) {
    const idSet = new Set(ids);
    const maxOrder = this.pinnedItems.reduce((max, item) => Math.max(max, item.pinOrder ?? 0), 0);
    let order = maxOrder + 1;

    if (nextPinned) {
      const newlyPinned: ClipItem[] = [];
      for (const id of ids) {
        if (this.pinnedItems.some((item) => item.id === id)) continue;
        const found = this.findItem(id);
        if (found) {
          newlyPinned.push({ ...found, isPinned: true, pinOrder: order++ });
        }
      }
      this.recentItems = this.recentItems.filter((item) => !idSet.has(item.id));
      this.pinnedItems = [...this.pinnedItems, ...newlyPinned].sort(comparePinOrder);
    } else {
      const unpinned = this.pinnedItems
        .filter((item) => idSet.has(item.id))
        .map((item) => ({ ...item, isPinned: false, pinOrder: null }));
      this.pinnedItems = this.pinnedItems.filter((item) => !idSet.has(item.id));
      const existingRecentIds = new Set(this.recentItems.map((item) => item.id));
      const merged = [
        ...this.recentItems,
        ...unpinned.filter((item) => !existingRecentIds.has(item.id)),
      ];
      merged.sort((a, b) => b.timestamp - a.timestamp);
      this.recentItems = merged;
    }

    this.searchResults = this.searchResults.map((item) =>
      idSet.has(item.id)
        ? {
            ...item,
            isPinned: nextPinned,
            pinOrder: nextPinned ? (item.pinOrder ?? order++) : null,
          }
        : item
    );
  }

  private removeClipsLocally(ids: string[]) {
    const idSet = new Set(ids);
    selectionStore.cancelUse();
    this.historyRequests.next();
    this.incomingEvents = null;
    this.pageRequests.next();
    this.searchRequests.next();
    this.cacheRevision += 1;
    for (const id of ids) {
      this.fullClipCache.delete(id);
      this.imageDetailCache.delete(id);
      this.clearContentDraft(id);
      if (this.editingClipId === id) this.editingClipId = null;
      if (this.editingLabelId === id) this.editingLabelId = null;
      selectionStore.selectedIds.delete(id);
    }
    this.recentItems = this.recentItems.filter((item) => !idSet.has(item.id));
    this.pinnedItems = this.pinnedItems.filter((item) => !idSet.has(item.id));
    this.searchResults = this.searchResults.filter((item) => !idSet.has(item.id));
    this.isLoading = false;
    this.isSearchPending = false;
  }

  private clearUnpinnedDrafts() {
    const pinnedIds = new Set(this.pinnedItems.map((item) => item.id));
    const nextDrafts: Record<string, string> = {};
    for (const [id, value] of Object.entries(this.contentDrafts)) {
      if (pinnedIds.has(id)) nextDrafts[id] = value;
    }
    this.contentDrafts = nextDrafts;
    if (this.editingClipId && !pinnedIds.has(this.editingClipId)) {
      this.editingClipId = null;
    }
    if (this.editingLabelId && !pinnedIds.has(this.editingLabelId)) {
      this.editingLabelId = null;
    }
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
