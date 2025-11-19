<script lang="ts">
  import { onDestroy } from 'svelte';
  import { clipboardStore } from '$lib/stores/clipboard.svelte';
  import type { ClipItem } from '$lib/stores/clipboard.svelte';
  import Card from './ui/Card.svelte';
  import Button from './ui/Button.svelte';
  import { Copy, Pin, Trash2, FileText, Image as ImageIcon, File } from 'lucide-svelte';

  interface Props {
    item: ClipItem;
  }

  let { item }: Props = $props();

  // Track blob URLs for cleanup
  let blobUrls: string[] = [];

  function formatTime(timestamp: number): string {
    const date = new Date(timestamp * 1000);
    const now = new Date();
    const diff = now.getTime() - date.getTime();
    
    // Less than 1 minute
    if (diff < 60000) {
      return '刚刚';
    }
    
    // Less than 1 hour
    if (diff < 3600000) {
      return `${Math.floor(diff / 60000)}分钟前`;
    }
    
    // Less than 24 hours
    if (diff < 86400000) {
      return `${Math.floor(diff / 3600000)}小时前`;
    }
    
    // Otherwise show date
    return date.toLocaleDateString('zh-CN');
  }

  // Helper to decode UTF-8 text from byte array or base64 string
  function decodeText(content: number[] | string): string {
    if (!content || (Array.isArray(content) && content.length === 0) || (typeof content === 'string' && content.length === 0)) {
      return '[内容为空]';
    }
    try {
      let bytes: Uint8Array;
      if (Array.isArray(content)) {
        bytes = new Uint8Array(content);
      } else {
        const binaryString = atob(content);
        bytes = new Uint8Array(binaryString.length);
        for (let i = 0; i < binaryString.length; i++) {
          bytes[i] = binaryString.charCodeAt(i);
        }
      }
      return new TextDecoder().decode(bytes);
    } catch (e) {
      console.error('Failed to decode text content:', e);
      return '[解码失败]';
    }
  }

  // Helper to create blob URL for images from byte array or base64 string
  function createImageSrc(content: number[] | string): string {
    let blob: Blob;
    if (Array.isArray(content)) {
      blob = new Blob([new Uint8Array(content)], { type: 'image/png' });
    } else {
      const binary = atob(content);
      const bytes = new Uint8Array(binary.length);
      for (let i = 0; i < binary.length; i++) {
        bytes[i] = binary.charCodeAt(i);
      }
      blob = new Blob([bytes], { type: 'image/png' });
    }
    const url = URL.createObjectURL(blob);
    blobUrls.push(url); // Track for cleanup
    return url;
  }

  // Cleanup blob URLs on component destroy
  onDestroy(() => {
    blobUrls.forEach(url => URL.revokeObjectURL(url));
  });
</script>

<div 
  class="group relative transition-all duration-200 ease-in-out hover:scale-[1.01]"
  role="listitem"
>
  <Card class="overflow-hidden border-l-4 {item.isPinned ? 'border-l-primary bg-primary/5' : 'border-l-transparent hover:border-l-primary/50'}">
    <div class="p-3 flex gap-3">
      <!-- Content Type Icon -->
      <div class="flex-none pt-1 text-muted-foreground">
        {#if item.contentType === 'text'}
          <FileText class="h-4 w-4" />
        {:else if item.contentType === 'image'}
          <ImageIcon class="h-4 w-4" />
        {:else}
          <File class="h-4 w-4" />
        {/if}
      </div>

      <!-- Main Content -->
      <div class="flex-1 min-w-0">
        {#if item.contentType === 'text'}
          <p class="text-sm text-foreground line-clamp-3 break-all font-mono leading-relaxed">
            {decodeText(item.content)}
          </p>
        {:else if item.contentType === 'image'}
          <div class="relative rounded-md overflow-hidden border border-border bg-muted/50 max-h-32 w-fit">
            <img 
              src={createImageSrc(item.content)} 
              alt="Clipboard content" 
              class="max-w-full h-auto object-contain max-h-32"
              loading="lazy"
            />
          </div>
        {:else}
          <p class="text-sm text-muted-foreground italic">
            [Binary File Data]
          </p>
        {/if}
        
        <div class="mt-2 flex items-center justify-between">
          <span class="text-xs text-muted-foreground font-medium">
            {formatTime(item.timestamp)}
          </span>
          
          <!-- Actions (visible on hover) -->
          <div class="flex items-center gap-1 opacity-0 group-hover:opacity-100 transition-opacity">
            <Button 
              variant="ghost" 
              size="icon" 
              class="h-7 w-7"
              title="复制"
              onclick={async () => {
                await clipboardStore.copyToClipboard(item);
              }}
            >
              <Copy class="h-3.5 w-3.5" />
            </Button>
            
            <Button 
              variant="ghost" 
              size="icon" 
              class="h-7 w-7 {item.isPinned ? 'text-primary' : 'text-muted-foreground'}"
              title={item.isPinned ? "取消置顶" : "置顶"}
              onclick={async () => {
                await clipboardStore.togglePin(item.id);
              }}
            >
              <Pin class="h-3.5 w-3.5 {item.isPinned ? 'fill-current' : ''}" />
            </Button>
            
            <Button 
              variant="ghost" 
              size="icon" 
              class="h-7 w-7 text-muted-foreground hover:text-destructive"
              title="删除"
              onclick={async () => {
                await clipboardStore.deleteItem(item.id);
              }}
            >
              <Trash2 class="h-3.5 w-3.5" />
            </Button>
          </div>
        </div>
      </div>
    </div>
  </Card>
</div>
