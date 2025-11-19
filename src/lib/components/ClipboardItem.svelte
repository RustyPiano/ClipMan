<script lang="ts">
import type { ClipItem } from '$lib/stores/clipboard.svelte';
import { clipboardStore } from '$lib/stores/clipboard.svelte';

let { item }: { item: ClipItem } = $props();

// Helper function to decode content (handles both array and base64 string)
function decodeContent(content: number[] | string): Uint8Array {
  if (Array.isArray(content)) {
    return new Uint8Array(content);
  } else {
    const binaryString = atob(content);
    const bytes = new Uint8Array(binaryString.length);
    for (let i = 0; i < binaryString.length; i++) {
      bytes[i] = binaryString.charCodeAt(i);
    }
    return bytes;
  }
}

// Compute these values once when component is created
let formattedTime = $state('');
let previewText = $state('');
let imageDataUrl = $state('');

// Update values when item changes
$effect(() => {
  // Format timestamp
  const date = new Date(item.timestamp * 1000);
  const now = new Date();
  const diffMs = now.getTime() - date.getTime();
  const diffMins = Math.floor(diffMs / 60000);

  if (diffMins < 1) formattedTime = '刚刚';
  else if (diffMins < 60) formattedTime = `${diffMins}分钟前`;
  else if (diffMins < 1440) formattedTime = `${Math.floor(diffMins / 60)}小时前`;
  else formattedTime = date.toLocaleDateString('zh-CN');

  // Decode text content
  if (item.contentType === 'text') {
    if (!item.content || item.content.length === 0) {
      previewText = '[内容为空]';
    } else {
      try {
        const bytes = decodeContent(item.content);
        const text = new TextDecoder().decode(bytes);
        previewText = text.slice(0, 200);
      } catch (e) {
        console.error('Failed to decode content:', e);
        previewText = '[解码失败]';
      }
    }
  } else {
    previewText = ''; // Clear previewText if not text content
  }

  // Convert image to data URL
  if (item.contentType === 'image') {
    try {
      let base64: string;
      if (Array.isArray(item.content)) {
        const bytes = new Uint8Array(item.content);
        let binary = '';
        for (let i = 0; i < bytes.length; i++) {
          binary += String.fromCharCode(bytes[i]);
        }
        base64 = btoa(binary);
      } else {
        base64 = item.content;
      }
      imageDataUrl = `data:image/png;base64,${base64}`;
    } catch (e) {
      console.error('Failed to convert image:', e);
      imageDataUrl = '';
    }
  } else {
    imageDataUrl = ''; // Clear imageDataUrl if not image content
  }
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
    {#if item.contentType === 'text'}
      <p class="preview-text">{previewText}</p>
    {:else if item.contentType === 'image'}
      <div class="image-preview">
        <img
          src={imageDataUrl}
          alt="预览"
        />
      </div>
    {:else}
      <p class="file-preview">📎 文件</p>
    {/if}
  </div>

  <div class="clip-meta">
    <span class="timestamp">{formattedTime}</span>
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
