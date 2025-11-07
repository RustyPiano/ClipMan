import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

export interface ClipItem {
  id: string;
  content: number[] | string; // Array of bytes or base64 string from Rust backend
  contentType: 'text' | 'image' | 'file';
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

  // Helper: decode content (handles both array and base64 string)
  private decodeContent(content: number[] | string): Uint8Array {
    if (Array.isArray(content)) {
      // Content is already a byte array
      return new Uint8Array(content);
    } else {
      // Content is base64 string, decode it
      const binaryString = atob(content);
      const bytes = new Uint8Array(binaryString.length);
      for (let i = 0; i < binaryString.length; i++) {
        bytes[i] = binaryString.charCodeAt(i);
      }
      return bytes;
    }
  }

  // Derived state: filtered items based on search
  filteredItems = $derived.by(() => {
    if (!this.searchQuery) {
      return this.items;
    }

    return this.items.filter((item) => {
      if (item.contentType === 'text') {
        try {
          const bytes = this.decodeContent(item.content);
          const text = new TextDecoder().decode(bytes);
          return text.toLowerCase().includes(this.searchQuery.toLowerCase());
        } catch (e) {
          console.error('Failed to decode content for search:', e);
          return false;
        }
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
      console.log('🔄 Loading clipboard history...');
      const history = await invoke<ClipItem[]>('get_clipboard_history', {
        limit: 100,
      });
      console.log(`✅ Loaded ${history.length} clipboard items`);

      // Debug: log first item details
      if (history.length > 0) {
        const first = history[0];
        console.log('📋 First item details:', {
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
      console.error('❌ Failed to load clipboard history:', error);
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
      console.log('✅ Cleared all non-pinned items');
    } catch (error) {
      console.error('❌ Failed to clear non-pinned items:', error);
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
      console.log('✅ Successfully copied to clipboard');
    } catch (error) {
      console.error('❌ Failed to copy to clipboard:', error);
      throw error;
    }
  }
}

// Export a single instance
export const clipboardStore = new ClipboardStore();
