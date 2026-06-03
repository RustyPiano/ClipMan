import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { toastStore } from './toast.svelte';
import { i18n } from '$lib/i18n';
import type { ClipItem, PasteMode } from '$lib/types';

// Re-export type for convenience
export type { ClipItem } from '$lib/types';

class ClipboardStore {
  recentItems = $state<ClipItem[]>([]);
  pinnedItems = $state<ClipItem[]>([]);
  searchResults = $state<ClipItem[]>([]);
  searchQuery = $state('');
  isLoading = $state(false);
  maxHistoryItems = $state(100);
  autoPaste = $state(true);
  private unlisten?: () => void;

  items = $derived([...this.pinnedItems, ...this.recentItems]);

  recentDisplayItems = $derived(
    this.searchQuery.trim()
      ? this.searchResults.filter((item) => !item.isPinned)
      : this.recentItems,
  );

  pinnedDisplayItems = $derived(
    this.searchQuery.trim()
      ? this.searchResults.filter((item) => item.isPinned)
      : this.pinnedItems,
  );

  // Compatibility alias for older UI code that still expects filteredItems.
  filteredItems = $derived(this.recentDisplayItems);

  constructor() {
    this.initialize();
  }

  async initialize() {
    try {
      const settings = await invoke<{ autoPaste: boolean; maxHistoryItems: number }>('get_settings');
      this.autoPaste = settings.autoPaste;
      this.maxHistoryItems = settings.maxHistoryItems;
    } catch (error) {
      console.error('Failed to load settings:', error);
    }

    await this.loadHistory();

    const unlistenClipboard = await listen('clipboard-changed', async () => {
      await this.reloadFromBackend();
    });

    const unlistenHistoryCleared = await listen('history-cleared', async () => {
      await this.reloadFromBackend();
    });

    this.unlisten = () => {
      unlistenClipboard();
      unlistenHistoryCleared();
    };
  }

  destroy() {
    this.unlisten?.();
  }

  async loadHistory() {
    this.isLoading = true;
    try {
      const [recent, pinned] = await Promise.all([
        invoke<ClipItem[]>('get_recent_clips', { limit: this.maxHistoryItems }),
        invoke<ClipItem[]>('get_pinned_clips'),
      ]);

      this.recentItems = recent;
      this.pinnedItems = pinned;
      this.searchResults = [];
    } catch (error) {
      console.error('[ERROR] Failed to load clipboard history:', error);
    } finally {
      this.isLoading = false;
    }
  }

  async refreshSettings() {
    try {
      const settings = await invoke<{ autoPaste: boolean; maxHistoryItems: number }>(
        'get_settings',
      );
      this.autoPaste = settings.autoPaste;
      this.maxHistoryItems = settings.maxHistoryItems;
    } catch (error) {
      console.error('Failed to refresh settings:', error);
    }
  }

  async search(query: string) {
    this.searchQuery = query;

    if (!query.trim()) {
      await this.loadHistory();
      return;
    }

    this.isLoading = true;
    try {
      this.searchResults = await invoke<ClipItem[]>('search_clips', { query });
    } catch (error) {
      console.error('Search failed:', error);
    } finally {
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

  async useClip(item: ClipItem, mode: PasteMode = 'paste') {
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
      const contentPreview =
        item.contentType === 'text' ? t.text : item.contentType === 'image' ? t.image : t.file;
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

  private findItem(id: string) {
    return (
      this.recentItems.find((item) => item.id === id) ??
      this.pinnedItems.find((item) => item.id === id) ??
      this.searchResults.find((item) => item.id === id)
    );
  }
}

export const clipboardStore = new ClipboardStore();
