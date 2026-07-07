import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { SvelteSet } from 'svelte/reactivity';
import { toastStore } from './toast.svelte';
import { i18n } from '$lib/i18n';
import type { ClipItem, PasteMode, ReorderDirection } from '$lib/types';
import {
  applyClipboardChanged,
  getPinnedDisplayItems,
  getRecentDisplayItems,
} from '$lib/utils/clip-items';
import { RequestSequencer } from '$lib/utils/request-sequencer';
import { hasTauriRuntime } from '$lib/utils/tauri';

// Re-export type for convenience
export type { ClipItem } from '$lib/types';

interface LoadHistoryOptions {
  showLoading?: boolean;
}

interface ClearSearchOptions extends LoadHistoryOptions {
  reload?: boolean;
}

interface IncomingItemEvent {
  revision: number;
  item: ClipItem;
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
  // Keyset pagination of the recent list: `recentItems` accumulates pages of
  // unpinned clips ordered (timestamp DESC, id DESC). `hasMoreRecent` gates the
  // scroll/keyboard "load more" trigger; `isLoadingMore` debounces it.
  hasMoreRecent = $state(false);
  isLoadingMore = $state(false);
  maxHistoryItems = $state(100);
  autoPaste = $state(true);
  // Multi-select set for merge paste (task #13). Insertion order == selection
  // order, which the backend merge preserves. SvelteSet makes membership/size
  // reads reactive so the list and footer update as items toggle. Cleared at the
  // deliberate reset points (panel switch / quickbar refresh / search / hide).
  selectedIds = new SvelteSet<string>();
  private static readonly PAGE_SIZE = 100;
  private historyRequests = new RequestSequencer();
  private searchRequests = new RequestSequencer();
  private incomingRevision = 0;
  private incomingEvents: IncomingItemEvent[] = [];
  // Full-text cache for the preview pane, keyed by id. A clip's content is
  // immutable for a given id (only label/pin/timestamp change), so entries are
  // only dropped when the clip is deleted/cleared. Images are not cached — the
  // preview reuses the 256px thumbnail the list already holds, so large image
  // data URLs never accumulate in renderer memory.
  private fullClipCache = new Map<string, ClipItem>();
  private static readonly FULL_CLIP_CACHE_LIMIT = 100;
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
    if (!hasTauriRuntime()) {
      this.isLoading = false;
      return;
    }

    await this.refreshSettings();
    await this.loadHistory();

    await listen<ClipItem>('clipboard-changed', async (event) => {
      if (this.searchQuery.trim()) {
        await this.reloadFromBackend();
        return;
      }

      this.applyIncomingItem(event.payload);
    });

    await listen('history-cleared', async () => {
      this.fullClipCache.clear();
      this.clearSelection();
      await this.reloadFromBackend();
    });

    await listen(QUICKBAR_HIDDEN_EVENT, () => {
      this.clearSelection();
      void this.clearSearch({ reload: false });
    });
  }

  async loadHistory(options: LoadHistoryOptions = {}) {
    if (!hasTauriRuntime()) {
      this.isLoading = false;
      return;
    }

    const showLoading = options.showLoading ?? true;
    const requestId = this.historyRequests.next();
    const startIncomingRevision = this.incomingRevision;

    if (showLoading) {
      this.isLoading = true;
    }

    // Fetch only the first page on a fresh load; on a reload (pin/delete/label/
    // clear) preserve however many pages the user already scrolled through, so
    // those actions don't snap the list back to the top.
    const pageLimit = Math.max(ClipboardStore.PAGE_SIZE, this.recentItems.length);

    try {
      // Ask for one sentinel row past the page: its presence — not a merely full
      // page — is the authoritative "there are older rows" signal, so a history
      // whose size is an exact multiple of the page no longer offers an empty
      // page turn. The sentinel is trimmed before it reaches the list.
      const [recentRaw, pinned] = await Promise.all([
        invoke<ClipItem[]>('get_recent_clips', { limit: pageLimit + 1 }),
        invoke<ClipItem[]>('get_pinned_clips'),
      ]);

      if (this.historyRequests.isCurrent(requestId)) {
        const hasMore = recentRaw.length > pageLimit;
        const recent = hasMore ? recentRaw.slice(0, pageLimit) : recentRaw;

        const nextItems = this.replayIncomingItemsSince({
          recentItems: recent,
          pinnedItems: pinned,
          revision: startIncomingRevision,
        });

        this.recentItems = nextItems.recentItems;
        this.pinnedItems = nextItems.pinnedItems;
        this.hasMoreRecent = hasMore;
        this.isLoadingMore = false;
        if (!this.searchQuery.trim()) {
          this.searchResults = [];
        }
      }
    } catch (error) {
      if (this.historyRequests.isCurrent(requestId)) {
        console.error('[ERROR] Failed to load clipboard history:', error);
      }
    } finally {
      // Clear the full-screen spinner for the current request regardless of the
      // search state. The previous `!searchQuery.trim()` guard let isLoading stick
      // forever when a search became active mid-load (e.g. typing immediately on
      // launch): the finally then skipped the clear and the search flow only
      // manages isSearchPending, so the spinner never went away.
      if (this.historyRequests.isCurrent(requestId)) {
        this.isLoading = false;
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
    if (!hasTauriRuntime()) return;
    if (this.isLoadingMore || !this.hasMoreRecent) return;
    if (this.searchQuery.trim() || this.activeSearchQuery.trim()) return;

    const cursor = this.recentItems[this.recentItems.length - 1];
    if (!cursor) {
      this.hasMoreRecent = false;
      return;
    }

    this.isLoadingMore = true;
    // Share the history sequencer so a reset (loadHistory / resetRecentPagination)
    // that lands mid-fetch supersedes this page and it drops its stale write.
    const requestId = this.historyRequests.next();

    try {
      // limit + 1: the sentinel row past the page is the authoritative "there is
      // another page" signal, so an exact page boundary no longer triggers an
      // empty page turn. The sentinel is trimmed before it reaches the list.
      const page = await invoke<ClipItem[]>('get_recent_clips', {
        limit: ClipboardStore.PAGE_SIZE + 1,
        beforeTimestamp: cursor.timestamp,
        beforeId: cursor.id,
      });

      if (!this.historyRequests.isCurrent(requestId)) return;

      const hasMore = page.length > ClipboardStore.PAGE_SIZE;
      const pageItems = hasMore ? page.slice(0, ClipboardStore.PAGE_SIZE) : page;

      // Append older rows, deduping by id: a live clipboard-changed event may
      // have bumped one of these rows to the top mid-fetch, and it must not
      // reappear lower in the list.
      const existingIds = new Set(this.recentItems.map((item) => item.id));
      const olderItems = pageItems.filter((item) => !existingIds.has(item.id));
      this.recentItems = [...this.recentItems, ...olderItems];
      this.hasMoreRecent = hasMore;
    } catch (error) {
      if (this.historyRequests.isCurrent(requestId)) {
        console.error('[ERROR] Failed to load more clipboard history:', error);
      }
    } finally {
      if (this.historyRequests.isCurrent(requestId)) {
        this.isLoadingMore = false;
      }
    }
  }

  /**
   * Collapse accumulated pages back to the first page (§1 reset points: panel
   * switch to recent, quickbar-opened). No IPC — the live clipboard-changed
   * stream keeps page 1 fresh while hidden; this only drops the extra pages a
   * scroll accumulated, and supersedes any in-flight page load.
   */
  resetRecentPagination() {
    this.historyRequests.next();
    this.isLoadingMore = false;
    if (this.recentItems.length > ClipboardStore.PAGE_SIZE) {
      this.recentItems = this.recentItems.slice(0, ClipboardStore.PAGE_SIZE);
      this.hasMoreRecent = true;
    }
  }

  async refreshSettings() {
    if (!hasTauriRuntime()) return;

    try {
      const settings = await invoke<{ autoPaste: boolean; maxHistoryItems: number }>(
        'get_settings'
      );
      this.autoPaste = settings.autoPaste;
      this.maxHistoryItems = settings.maxHistoryItems;
    } catch (error) {
      console.error('Failed to refresh settings:', error);
    }
  }

  setSearchQuery(query: string) {
    this.searchRequests.next();
    this.searchQuery = query;
    if (!query.trim()) {
      this.activeSearchQuery = '';
      this.searchResults = [];
      this.isSearchPending = false;
      this.isLoading = false;
    } else {
      this.isSearchPending = hasTauriRuntime();
    }
  }

  setSearchDraft(query: string) {
    this.searchRequests.next();
    this.searchQuery = query;
    this.isSearchPending = false;
  }

  async search(query: string, options: { silent?: boolean } = {}) {
    if (query.trim() && this.searchQuery !== query) {
      return;
    }

    // A silent search refreshes the results of the *same* query in place — e.g.
    // after a copy bumps a row's timestamp while a query is active. It must not
    // toggle the pending spinner, otherwise the search icon flickers on every
    // background refresh even though the user never typed anything.
    const silent = options.silent ?? false;
    const requestId = this.searchRequests.next();
    this.searchQuery = query;

    if (!query.trim()) {
      this.activeSearchQuery = '';
      this.searchResults = [];
      this.isSearchPending = false;
      await this.loadHistory({ showLoading: false });
      return;
    }

    if (!hasTauriRuntime()) {
      this.activeSearchQuery = '';
      this.searchResults = [];
      this.isSearchPending = false;
      this.isLoading = false;
      return;
    }

    if (!silent) {
      this.isSearchPending = true;
    }
    try {
      const results = await invoke<ClipItem[]>('search_clips', { query });
      if (this.searchRequests.isCurrent(requestId) && this.searchQuery === query) {
        this.searchResults = results;
        this.activeSearchQuery = query;
      }
    } catch (error) {
      if (this.searchRequests.isCurrent(requestId) && this.searchQuery === query) {
        console.error('Search failed:', error);
      }
    } finally {
      if (!silent && this.searchRequests.isCurrent(requestId) && this.searchQuery === query) {
        this.isSearchPending = false;
      }
    }
  }

  async clearSearch(options: ClearSearchOptions = {}) {
    const reload = options.reload ?? true;

    this.searchRequests.next();
    this.searchQuery = '';
    this.activeSearchQuery = '';
    this.searchResults = [];
    this.isSearchPending = false;
    this.isLoading = false;

    if (reload) {
      await this.loadHistory({ showLoading: options.showLoading ?? false });
    } else {
      this.isLoading = false;
    }
  }

  async clearNonPinned() {
    try {
      // The backend emits `history-cleared` after clearing (the tray's "clear"
      // item fires the same command), and our listener there clears the cache
      // and reloads. So this path must NOT reload again — that was a redundant
      // round trip on every clear.
      await invoke('clear_non_pinned_history');
      console.log('[SUCCESS] Cleared all non-pinned items');
    } catch (error) {
      console.error('[ERROR] Failed to clear non-pinned items:', error);
      throw error;
    }
  }

  async togglePin(id: string) {
    const item = this.findItem(id);
    if (!item) return;

    try {
      await invoke('toggle_pin', { id, isPinned: !item.isPinned });
      await this.reloadFromBackend();
    } catch (error) {
      console.error('Failed to toggle pin:', error);
    }
  }

  async deleteItem(id: string) {
    try {
      await invoke('delete_clip', { id });
      this.removeClipLocally(id);
    } catch (error) {
      console.error('Failed to delete item:', error);
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

  async useClip(item: ClipItem, mode: PasteMode = 'default', options: { plain?: boolean } = {}) {
    try {
      // `plain` (⌥Enter) forces a plain-text paste. The backend ignores it for
      // non-text clips, so it is passed through without a frontend type branch.
      await invoke('paste_clip', { id: item.id, mode, plain: options.plain ?? false });
    } catch (error) {
      console.error('[ERROR] Failed to use clip:', error);
      toastStore.add(this.pasteFailureMessage(mode), 'error');
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

  isSelected(id: string): boolean {
    return this.selectedIds.has(id);
  }

  toggleSelected(id: string) {
    if (this.selectedIds.has(id)) {
      this.selectedIds.delete(id);
    } else {
      this.selectedIds.add(id);
    }
  }

  clearSelection() {
    if (this.selectedIds.size > 0) {
      this.selectedIds.clear();
    }
  }

  /**
   * Merge the multi-selected clips (in selection order) into a single clipboard
   * write and paste them, newline-separated (task #13). No-op when nothing is
   * selected; clears the selection once the paste is dispatched.
   */
  async useSelectedClips(mode: PasteMode = 'default') {
    const ids = [...this.selectedIds];
    if (ids.length === 0) return;

    try {
      await invoke('paste_clips', { ids, mode, separator: '\n' });
      this.clearSelection();
    } catch (error) {
      console.error('[ERROR] Failed to merge-paste clips:', error);
      toastStore.add(this.pasteFailureMessage(mode), 'error');
    }
  }

  async hideQuickbar() {
    try {
      await invoke('hide_quickbar');
    } catch (error) {
      console.error('[ERROR] Failed to hide QuickBar:', error);
    }
  }

  async copyToClipboard(item: ClipItem) {
    try {
      // 使用后端命令来复制，这样可以防止重复捕获
      await invoke('copy_to_system_clipboard', { clipId: item.id });
      console.log('[SUCCESS] Successfully copied to clipboard');

      const t = i18n.t;
      const contentPreview = item.contentType === 'image' ? t.image : t.text;
      toastStore.add(`${t.copied} ${contentPreview}`, 'success');
    } catch (error) {
      console.error('[ERROR] Failed to copy to clipboard:', error);
      toastStore.add(i18n.t.copyFailed, 'error');
      throw error;
    }
  }

  /** Synchronously read a cached full clip (no fetch). */
  getCachedFullClip(id: string): ClipItem | undefined {
    return this.fullClipCache.get(id);
  }

  /**
   * Fetch the full, untruncated text or files clip by id for the preview pane.
   * Both carry small newline-joined text payloads. Images use list thumbnails and
   * are ignored here so full image payloads never cross IPC. Results are cached;
   * callers should guard against stale selections since this resolves async.
   */
  async fetchFullClip(id: string): Promise<ClipItem | null> {
    const cached = this.getCachedFullClip(id);
    if (cached) return cached;
    if (!hasTauriRuntime()) return null;

    try {
      const full = await invoke<ClipItem | null>('get_clip', { id });
      // A null result (or any non text/files type) has no cacheable preview
      // payload; this guard also narrows `full` to non-null for the cache call.
      if (full?.contentType !== 'text' && full?.contentType !== 'files') return null;
      this.cacheFullClip(full);
      return full;
    } catch (error) {
      console.error('[ERROR] Failed to fetch full clip:', error);
      return null;
    }
  }

  private cacheFullClip(item: ClipItem) {
    if (item.contentType !== 'text' && item.contentType !== 'files') return;
    if (item.content.length > ClipboardStore.MAX_CACHEABLE_CONTENT_LENGTH) return;

    this.fullClipCache.set(item.id, item);
    while (this.fullClipCache.size > ClipboardStore.FULL_CLIP_CACHE_LIMIT) {
      const oldestKey = this.fullClipCache.keys().next().value;
      if (!oldestKey) break;
      this.fullClipCache.delete(oldestKey);
    }
  }

  private async reloadFromBackend() {
    await this.loadHistory({ showLoading: false });

    if (this.searchQuery.trim()) {
      await this.search(this.searchQuery, { silent: true });
    }
  }

  private applyIncomingItem(incoming: ClipItem) {
    this.recordIncomingItem(incoming);

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
    this.historyRequests.next();
    this.searchRequests.next();
    this.fullClipCache.delete(id);
    this.selectedIds.delete(id);
    this.recentItems = this.recentItems.filter((item) => item.id !== id);
    this.pinnedItems = this.pinnedItems.filter((item) => item.id !== id);
    this.searchResults = this.searchResults.filter((item) => item.id !== id);
    this.isLoading = false;
    this.isSearchPending = false;
  }

  private recordIncomingItem(item: ClipItem) {
    this.incomingRevision += 1;
    this.incomingEvents.push({ revision: this.incomingRevision, item });

    if (this.incomingEvents.length > this.maxHistoryItems) {
      this.incomingEvents = this.incomingEvents.slice(-this.maxHistoryItems);
    }
  }

  private replayIncomingItemsSince({
    recentItems,
    pinnedItems,
    revision,
  }: {
    recentItems: readonly ClipItem[];
    pinnedItems: readonly ClipItem[];
    revision: number;
  }) {
    let nextItems = {
      recentItems: [...recentItems],
      pinnedItems: [...pinnedItems],
    };

    for (const event of this.incomingEvents) {
      if (event.revision <= revision) continue;

      nextItems = applyClipboardChanged({
        recentItems: nextItems.recentItems,
        pinnedItems: nextItems.pinnedItems,
        incoming: event.item,
        maxHistoryItems: this.maxHistoryItems,
      });
    }

    return nextItems;
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
