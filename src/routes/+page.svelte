<script lang="ts">
import { clipboardStore } from '$lib/stores/clipboard.svelte';
import { router } from '$lib/stores/router.svelte';
import SearchBar from '$lib/components/SearchBar.svelte';
import ClipboardItem from '$lib/components/ClipboardItem.svelte';
import SettingsPage from './settings/+page.svelte';
import PermissionCheck from '$lib/components/PermissionCheck.svelte';

// Reactive state showing pinned vs all
let showPinned = $state(false);

// Derived: current list to display
const displayItems = $derived(
  showPinned
    ? clipboardStore.pinnedItems
    : clipboardStore.filteredItems
);
</script>

{#if router.currentRoute === 'settings'}
  <SettingsPage />
{:else}
  <div class="app">
    <PermissionCheck />
    <header class="app-header">
      <h1 class="app-title">ClipMan</h1>
      <div class="header-actions">
        <button
          class="tab-btn"
          class:active={!showPinned}
          onclick={() => (showPinned = false)}
        >
          历史记录
        </button>
        <button
          class="tab-btn"
          class:active={showPinned}
          onclick={() => (showPinned = true)}
        >
          置顶 ({clipboardStore.pinnedItems.length})
        </button>
        <button
          class="action-icon-btn"
          title="清除非置顶"
          onclick={async () => {
            if (confirm('确定要清除所有非置顶的历史记录吗？')) {
              await clipboardStore.clearNonPinned();
            }
          }}
        >
          🗑️
        </button>
        <button class="action-icon-btn" title="设置" onclick={() => router.goToSettings()}>⚙️</button>
      </div>
    </header>

    <SearchBar />

    <main class="clip-list">
      {#if clipboardStore.isLoading}
        <div class="loading">加载中...</div>
      {:else if displayItems.length === 0}
        <div class="empty">
          {#if showPinned}
            <p>暂无置顶项目</p>
            <p class="empty-hint">点击 📍 置顶常用内容</p>
          {:else}
            <p>暂无剪切板历史</p>
            <p class="empty-hint">复制内容后会自动出现在这里</p>
            <p class="debug-info" style="margin-top: 1rem; font-size: 0.75rem; color: #999;">
              总共 {clipboardStore.items.length} 项 |
              文本: {clipboardStore.items.filter(i => i.contentType === 'text').length} |
              图片: {clipboardStore.items.filter(i => i.contentType === 'image').length} |
              其他: {clipboardStore.items.filter(i => i.contentType !== 'text' && i.contentType !== 'image').length}
            </p>
          {/if}
        </div>
      {:else}
        <div class="debug-info" style="padding: 0.5rem 1rem; font-size: 0.75rem; color: #666; border-bottom: 1px solid #e5e7eb;">
          显示 {displayItems.length} 项 |
          文本: {displayItems.filter(i => i.contentType === 'text').length} |
          图片: {displayItems.filter(i => i.contentType === 'image').length}
        </div>
        {#each displayItems as item (item.id)}
          <ClipboardItem {item} />
        {/each}
      {/if}
    </main>
  </div>
{/if}

<style>
  .app {
    display: flex;
    flex-direction: column;
    height: 100vh;
    background-color: #ffffff;
  }

  .app-header {
    padding: 1rem;
    border-bottom: 1px solid #e5e7eb;
    background-color: #f9fafb;
  }

  .app-title {
    font-size: 1.5rem;
    font-weight: 700;
    color: #111827;
    margin: 0 0 0.75rem 0;
  }

  .header-actions {
    display: flex;
    gap: 0.5rem;
  }

  .tab-btn {
    padding: 0.5rem 1rem;
    border: 1px solid #e5e7eb;
    border-radius: 0.375rem;
    background-color: #ffffff;
    color: #6b7280;
    font-size: 0.875rem;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .tab-btn:hover {
    background-color: #f3f4f6;
  }

  .tab-btn.active {
    background-color: #3b82f6;
    color: #ffffff;
    border-color: #3b82f6;
  }

  .action-icon-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 0.5rem;
    border: 1px solid #e5e7eb;
    border-radius: 0.375rem;
    background-color: #ffffff;
    font-size: 1.2rem;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .action-icon-btn:hover {
    background-color: #f3f4f6;
    border-color: #d1d5db;
  }

  .action-icon-btn:last-child {
    margin-left: auto;
  }

  .clip-list {
    flex: 1;
    overflow-y: auto;
  }

  .loading,
  .empty {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 3rem 1rem;
    color: #9ca3af;
    text-align: center;
  }

  .empty p {
    margin: 0;
    font-size: 0.875rem;
  }

  .empty-hint {
    margin-top: 0.5rem;
    font-size: 0.75rem;
    color: #d1d5db;
  }
</style>
