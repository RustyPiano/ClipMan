import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

export interface ClipItem {
  id: string;
  content: Uint8Array;
  contentType: 'Text' | 'Image' | 'File';
  timestamp: number;
  isPinned: boolean;
  pinOrder: number | null;
}

class ClipboardStore {
  items = $state<ClipItem[]>([]);
  searchQuery = $state('');
  isLoading = $state(false);
  private unlisten?: () => void;

  // Derived state: pinned items sorted by pin order
  pinnedItems = $derived(
    this.items
      .filter((item) => item.isPinned)
      .sort((a, b) => (a.pinOrder || 0) - (b.pinOrder || 0))
  );

  // Derived state: filtered items based on search
  filteredItems = $derived.by(() => {
    if (!this.searchQuery) {
      return this.items;
    }

    return this.items.filter((item) => {
      if (item.contentType === 'Text') {
        const text = new TextDecoder().decode(item.content);
        return text.toLowerCase().includes(this.searchQuery.toLowerCase());
      }
      return false;
    });
  });

  constructor() {
    this.initialize();
  }

  async initialize() {
    // Load initial history
    await this.loadHistory();

    // Listen for clipboard changes from Rust backend
    this.unlisten = await listen<ClipItem>('clipboard-changed', (event) => {
      // Add new item to the beginning
      this.items = [event.payload, ...this.items];
    });
  }

  destroy() {
    // Clean up event listener
    this.unlisten?.();
  }

  async loadHistory() {
    this.isLoading = true;
    try {
      const history = await invoke<ClipItem[]>('get_clipboard_history', {
        limit: 100,
      });
      this.items = history;
    } catch (error) {
      console.error('Failed to load clipboard history:', error);
    } finally {
      this.isLoading = false;
    }
  }

  async search(query: string) {
    this.searchQuery = query;

    // Local filtering via derived state when query is empty
    if (!query) {
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
      if (item.contentType === 'Text') {
        const text = new TextDecoder().decode(item.content);
        const { writeText } = await import('@tauri-apps/plugin-clipboard-manager');
        await writeText(text);
      }
    } catch (error) {
      console.error('Failed to copy to clipboard:', error);
    }
  }
}

// Export a single instance
export const clipboardStore = new ClipboardStore();
