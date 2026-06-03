import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { toastStore } from './toast.svelte';
import { i18n } from '$lib/i18n';
import type { ClipItem, PasteMode, ReorderDirection } from '$lib/types';
import { applyClipboardChanged, comparePinOrder } from '$lib/utils/clip-items';
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
  maxHistoryItems = $state(100);
  autoPaste = $state(true);
  private unlisten?: () => void;
  private historyRequests = new RequestSequencer();
  private searchRequests = new RequestSequencer();
  private incomingRevision = 0;
  private incomingEvents: IncomingItemEvent[] = [];

  recentDisplayItems = $derived(
    this.activeSearchQuery.trim()
      ? this.searchResults.filter((item) => !item.isPinned)
      : this.recentItems
  );

  pinnedDisplayItems = $derived(
    this.activeSearchQuery.trim()
      ? this.searchResults.filter((item) => item.isPinned).sort(comparePinOrder)
      : this.pinnedItems
  );

  constructor() {
    this.initialize();
  }

  async initialize() {
    if (!hasTauriRuntime()) {
      this.isLoading = false;
      return;
    }

    try {
      const settings = await invoke<{ autoPaste: boolean; maxHistoryItems: number }>(
        'get_settings'
      );
      this.autoPaste = settings.autoPaste;
      this.maxHistoryItems = settings.maxHistoryItems;
    } catch (error) {
      console.error('Failed to load settings:', error);
    }

    await this.loadHistory();

    const unlistenClipboard = await listen<ClipItem>('clipboard-changed', async (event) => {
      if (this.searchQuery.trim()) {
        await this.reloadFromBackend();
        return;
      }

      this.applyIncomingItem(event.payload);
    });

    const unlistenHistoryCleared = await listen('history-cleared', async () => {
      await this.reloadFromBackend();
    });

    const unlistenQuickbarHidden = await listen(QUICKBAR_HIDDEN_EVENT, () => {
      void this.clearSearch({ reload: false });
    });

    this.unlisten = () => {
      unlistenClipboard();
      unlistenHistoryCleared();
      unlistenQuickbarHidden();
    };
  }

  destroy() {
    this.unlisten?.();
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

    try {
      const [recent, pinned] = await Promise.all([
        invoke<ClipItem[]>('get_recent_clips', { limit: this.maxHistoryItems }),
        invoke<ClipItem[]>('get_pinned_clips'),
      ]);

      if (this.historyRequests.isCurrent(requestId)) {
        const nextItems = this.replayIncomingItemsSince({
          recentItems: recent,
          pinnedItems: pinned,
          revision: startIncomingRevision,
        });

        this.recentItems = nextItems.recentItems;
        this.pinnedItems = nextItems.pinnedItems;
        if (!this.searchQuery.trim()) {
          this.searchResults = [];
        }
      }
    } catch (error) {
      if (this.historyRequests.isCurrent(requestId)) {
        console.error('[ERROR] Failed to load clipboard history:', error);
      }
    } finally {
      if (this.historyRequests.isCurrent(requestId) && !this.searchQuery.trim()) {
        this.isLoading = false;
      }
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

  async search(query: string) {
    if (query.trim() && this.searchQuery !== query) {
      return;
    }

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

    this.isSearchPending = true;
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
      if (this.searchRequests.isCurrent(requestId) && this.searchQuery === query) {
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
      await invoke('clear_non_pinned_history');
      await this.reloadFromBackend();
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
      await this.reloadFromBackend();
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

  async useClip(item: ClipItem, mode: PasteMode = 'default') {
    try {
      await invoke('paste_clip', { id: item.id, mode });
    } catch (error) {
      console.error(`[ERROR] Failed to ${mode} clip:`, error);
      if (mode === 'copy') {
        toastStore.add(i18n.t.copyFailed, 'error');
      }
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

  private async reloadFromBackend() {
    if (this.searchQuery.trim()) {
      await this.search(this.searchQuery);
    } else {
      await this.loadHistory();
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
