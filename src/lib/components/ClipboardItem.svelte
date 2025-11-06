<script lang="ts">
import type { ClipItem } from '$lib/stores/clipboard.svelte';
import { clipboardStore } from '$lib/stores/clipboard.svelte';

let { item }: { item: ClipItem } = $props();

// Format timestamp
const formattedTime = $derived(() => {
  const date = new Date(item.timestamp * 1000);
  const now = new Date();
  const diffMs = now.getTime() - date.getTime();
  const diffMins = Math.floor(diffMs / 60000);

  if (diffMins < 1) return '刚刚';
  if (diffMins < 60) return `${diffMins}分钟前`;
  if (diffMins < 1440) return `${Math.floor(diffMins / 60)}小时前`;
  return date.toLocaleDateString('zh-CN');
});

// Get preview text
const previewText = $derived(() => {
  if (item.contentType !== 'Text') return '';
  const text = new TextDecoder().decode(item.content);
  return text.slice(0, 200);
});

async function handleCopy() {
  await clipboardStore.copyToClipboard(item);
}

async function handleTogglePin() {
  await clipboardStore.togglePin(item.id);
}

async function handleDelete() {
  await clipboardStore.deleteItem(item.id);
}
</script>

<div
  class="clip-item"
  onclick={handleCopy}
  onkeydown={(e) => e.key === 'Enter' && handleCopy()}
  role="button"
  tabindex="0"
  aria-label="复制到剪切板"
>
  <div class="clip-content">
    {#if item.contentType === 'Text'}
      <p class="preview-text">{previewText()}</p>
    {:else if item.contentType === 'Image'}
      <div class="image-preview">
        <img
          src={`data:image/png;base64,${btoa(
            String.fromCharCode(...item.content)
          )}`}
          alt="预览"
        />
      </div>
    {:else}
      <p class="file-preview">📎 文件</p>
    {/if}
  </div>

  <div class="clip-meta">
    <span class="timestamp">{formattedTime()}</span>
    <div class="actions">
      <button
        class="action-btn"
        onclick={(e) => {
          e.stopPropagation();
          handleTogglePin();
        }}
        aria-label={item.isPinned ? '取消置顶' : '置顶'}
      >
        {item.isPinned ? '📌' : '📍'}
      </button>
      <button
        class="action-btn delete-btn"
        onclick={(e) => {
          e.stopPropagation();
          handleDelete();
        }}
        aria-label="删除"
      >
        🗑️
      </button>
    </div>
  </div>
</div>

<style>
  .clip-item {
    padding: 0.75rem 1rem;
    border-bottom: 1px solid #e5e7eb;
    cursor: pointer;
    transition: background-color 0.15s ease;
  }

  .clip-item:hover {
    background-color: #f9fafb;
  }

  .clip-content {
    margin-bottom: 0.5rem;
  }

  .preview-text {
    font-size: 0.875rem;
    line-height: 1.5;
    color: #374151;
    white-space: pre-wrap;
    word-break: break-word;
  }

  .image-preview {
    max-width: 100%;
    max-height: 128px;
    overflow: hidden;
    border-radius: 0.375rem;
  }

  .image-preview img {
    width: 100%;
    height: auto;
    object-fit: contain;
  }

  .file-preview {
    font-size: 0.875rem;
    color: #6b7280;
  }

  .clip-meta {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .timestamp {
    font-size: 0.75rem;
    color: #9ca3af;
  }

  .actions {
    display: flex;
    gap: 0.5rem;
  }

  .action-btn {
    padding: 0.25rem;
    background: none;
    border: none;
    cursor: pointer;
    font-size: 1rem;
    opacity: 0.6;
    transition: opacity 0.15s ease;
  }

  .action-btn:hover {
    opacity: 1;
  }

  .delete-btn:hover {
    filter: brightness(0.8);
  }
</style>
