import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { toastStore } from './toast.svelte';

export interface ClipItem {
  id: string;
  content: string; // Base64 string or data URL from Rust backend
  contentType: 'text' | 'image' | 'file';
  timestamp: number;
  isPinned: boolean;
  pinOrder: number | null;
}

class ClipboardStore {
  items = $state<ClipItem[]>([]);
  searchQuery = $state('');
  isLoading = $state(false);
  maxHistoryItems = $state(100); // Default limit
  private unlisten?: () => void;

  // Derived state: pinned items sorted by pin order
  pinnedItems = $derived(
    this.items
      .filter((item) => item.isPinned)
      .sort((a, b) => (a.pinOrder || 0) - (b.pinOrder || 0))
  );



  // Derived state: filtered items based on search
  filteredItems = $derived(this.items);

  constructor() {
    this.initialize();
  }

  async initialize() {
    // Load settings first to get maxHistoryItems
    try {
      const settings = await invoke<{ maxHistoryItems: number }>('get_settings');
      this.maxHistoryItems = settings.maxHistoryItems;
    } catch (e) {
      console.error('Failed to load settings:', e);
    }

    // Load initial history
    await this.loadHistory();

    // Listen for clipboard changes from Rust backend
    const unlistenClipboard = await listen<ClipItem>('clipboard-changed', (event) => {
      // Backend now sends FrontendClipItem which matches our interface directly (Base64 content)
      const newItem = event.payload;

      // Deduplication: Remove existing item with same ID if present
      const existingIndex = this.items.findIndex(i => i.id === newItem.id);
      let currentItems = [...this.items];

      if (existingIndex !== -1) {
        // Remove existing item so we can add updated one to top
        currentItems.splice(existingIndex, 1);
      }

      // Add new item to the beginning
      currentItems.unshift(newItem);

      // Enforce limit on non-pinned items
      // We need to be careful not to remove pinned items
      if (currentItems.length > this.maxHistoryItems) {
        // Find the last non-pinned item to remove
        // We iterate from the end
        for (let i = currentItems.length - 1; i >= 0; i--) {
          if (!currentItems[i].isPinned) {
            currentItems.splice(i, 1);
            // Check if we are within limit now
            // Note: The backend limit applies to non-pinned items mostly, 
            // but here we just enforce total list size for simplicity or match backend logic
            // The backend logic is: keep all pinned + N recent non-pinned.
            // So we should count non-pinned items.
            const nonPinnedCount = currentItems.filter(item => !item.isPinned).length;
            if (nonPinnedCount <= this.maxHistoryItems) {
              break;
            }
          }
        }
      }

      this.items = currentItems;
    });

    // Listen for history cleared event from menu bar
    const unlistenHistoryCleared = await listen('history-cleared', async () => {
      console.log('[INFO] History cleared from menu bar, reloading...');
      await this.loadHistory();
    });

    // Store both unlisten functions
    this.unlisten = () => {
      unlistenClipboard();
      unlistenHistoryCleared();
    };
  }

  destroy() {
    // Clean up event listener
    this.unlisten?.();
  }

  async loadHistory() {
    this.isLoading = true;
    try {
      console.log('[INFO] Loading clipboard history...');
      const history = await invoke<ClipItem[]>('get_clipboard_history', {
        limit: 100,
      });
      console.log(`[SUCCESS] Loaded ${history.length} clipboard items`);

      // Debug: log first item details
      if (history.length > 0) {
        const first = history[0];
        console.log('[DEBUG] First item details:', {
          id: first.id,
          contentType: first.contentType,
          contentIsString: typeof first.content === 'string',
          contentLength: first.content?.length,
          contentPreview: typeof first.content === 'string' ? first.content.substring(0, 50) : 'NOT STRING',
          timestamp: first.timestamp
        });
      }

      this.items = history;
    } catch (error) {
      console.error('[ERROR] Failed to load clipboard history:', error);
    } finally {
      this.isLoading = false;
    }
  }

  async search(query: string) {
    this.searchQuery = query;

    // When query is empty, reload full history
    if (!query) {
      await this.loadHistory();
      return;
    }

    // Use full-text search for complex queries
    this.isLoading = true;
    try {
      const results = await invoke<ClipItem[]>('search_clips', { query });
      this.items = results;
    } catch (error) {
      console.error('Search failed:', error);
    } finally {
      this.isLoading = false;
    }
  }

  async clearNonPinned() {
    try {
      await invoke('clear_non_pinned_history');
      await this.loadHistory();
      console.log('[SUCCESS] Cleared all non-pinned items');
    } catch (error) {
      console.error('[ERROR] Failed to clear non-pinned items:', error);
      throw error;
    }
  }

  async togglePin(id: string) {
    const item = this.items.find((i) => i.id === id);
    if (!item) return;

    try {
      await invoke('toggle_pin', { id, isPinned: !item.isPinned });
      item.isPinned = !item.isPinned;

      // Reload to get updated pin order
      await this.loadHistory();
    } catch (error) {
      console.error('Failed to toggle pin:', error);
    }
  }

  async deleteItem(id: string) {
    try {
      await invoke('delete_clip', { id });
      this.items = this.items.filter((item) => item.id !== id);
    } catch (error) {
      console.error('Failed to delete item:', error);
    }
  }

  async copyToClipboard(item: ClipItem) {
    try {
      // 使用后端命令来复制，这样可以防止重复捕获
      await invoke('copy_to_system_clipboard', { clipId: item.id });
      console.log('[SUCCESS] Successfully copied to clipboard');

      // Show success toast
      const contentPreview = item.contentType === 'text'
        ? '文本'
        : item.contentType === 'image'
          ? '图片'
          : '文件';
      toastStore.add(`已复制${contentPreview}到剪贴板`, 'success');
    } catch (error) {
      console.error('[ERROR] Failed to copy to clipboard:', error);
      toastStore.add('复制失败', 'error');
      throw error;
    }
  }
}

// Export a single instance
export const clipboardStore = new ClipboardStore();

