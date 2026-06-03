<script lang="ts">
  import { onDestroy } from 'svelte';
  import { clipboardStore } from '$lib/stores/clipboard.svelte';
  import type { ClipItem } from '$lib/stores/clipboard.svelte';
  import { i18n } from '$lib/i18n';
  import Card from './ui/Card.svelte';
  import Button from './ui/Button.svelte';
  import { Copy, Check, Pin, Trash2, FileText, Image as ImageIcon, File } from 'lucide-svelte';

  interface Props {
    item: ClipItem;
    selected?: boolean;
    slotNumber?: number | null;
    onSelect?: () => void;
    onUse?: () => void | Promise<void>;
  }

  let { item, selected = false, slotNumber = null, onSelect, onUse }: Props = $props();

  const t = $derived(i18n.t);
  const cardClass = $derived(
    selected
      ? 'border-l-primary bg-primary/10 ring-1 ring-primary/30 shadow-sm'
      : item.isPinned
        ? 'border-l-primary bg-primary/5'
        : 'border-l-transparent hover:border-l-primary/50 hover:bg-muted/30',
  );

  let isCopied = $state(false);
  let copyTimeout: ReturnType<typeof setTimeout>;

  const decodedText = $derived.by(() => {
    if (item.contentType !== 'text') return '';

    const content = item.content;
    if (!content || (typeof content === 'string' && content.length === 0)) {
      return t.emptyContent;
    }

    try {
      if (typeof content === 'string') {
        const binaryString = atob(content);
        const bytes = new Uint8Array(binaryString.length);
        for (let i = 0; i < binaryString.length; i++) {
          bytes[i] = binaryString.charCodeAt(i);
        }
        return new TextDecoder().decode(bytes);
      }
      return t.decodeFailed;
    } catch (error) {
      console.error('Failed to decode text content:', error);
      return t.decodeFailed;
    }
  });

  const imageDataUrl = $derived(
    item.contentType === 'image' && typeof item.content === 'string' ? item.content : '',
  );

  function formatTime(timestamp: number): string {
    const date = new Date(timestamp * 1000);
    const now = new Date();
    const diff = now.getTime() - date.getTime();

    if (diff < 60000) {
      return t.justNow;
    }

    if (diff < 3600000) {
      return i18n.format(t.minutesAgo, { n: Math.floor(diff / 60000) });
    }

    if (diff < 86400000) {
      return i18n.format(t.hoursAgo, { n: Math.floor(diff / 3600000) });
    }

    return date.toLocaleDateString(i18n.locale === 'zh-CN' ? 'zh-CN' : 'en-US');
  }

  async function handleUse() {
    onSelect?.();
    await onUse?.();
  }

  async function handleCopy(event: MouseEvent) {
    event.stopPropagation();

    try {
      await clipboardStore.copyToClipboard(item);
      isCopied = true;
      clearTimeout(copyTimeout);
      copyTimeout = setTimeout(() => {
        isCopied = false;
      }, 2000);
    } catch (_error) {
      // Error handled in store
    }
  }

  async function handleTogglePin(event: MouseEvent) {
    event.stopPropagation();
    await clipboardStore.togglePin(item.id);
  }

  async function handleDelete(event: MouseEvent) {
    event.stopPropagation();
    await clipboardStore.deleteItem(item.id);
  }

  onDestroy(() => {
    clearTimeout(copyTimeout);
  });
</script>

<div
  id={`clip-item-${item.id}`}
  class="group relative transition-all duration-200 ease-in-out hover:scale-[1.01]"
  role="listitem"
  onmouseenter={onSelect}
>
  <Card
    class="cursor-pointer overflow-hidden border-l-4 transition-colors duration-200 {cardClass}"
    onclick={handleUse}
  >
    <div class="flex gap-3 p-3">
      <div class="flex w-8 flex-none flex-col items-center gap-1 pt-0.5 text-muted-foreground">
        {#if slotNumber}
          <span
            class="flex h-5 min-w-5 items-center justify-center rounded border px-1 text-xs font-semibold {selected
              ? 'border-primary bg-primary text-primary-foreground'
              : 'border-border bg-muted text-muted-foreground'}"
          >
            {slotNumber}
          </span>
        {/if}

        {#if item.contentType === 'text'}
          <FileText class="h-4 w-4" />
        {:else if item.contentType === 'image'}
          <ImageIcon class="h-4 w-4" />
        {:else}
          <File class="h-4 w-4" />
        {/if}
      </div>

      <div class="min-w-0 flex-1">
        {#if item.contentType === 'text'}
          <p
            class="line-clamp-3 break-all font-mono text-sm leading-relaxed text-foreground selection:bg-primary/20"
          >
            {decodedText}
          </p>
        {:else if item.contentType === 'image'}
          <div
            class="group/image relative max-h-32 w-fit overflow-hidden rounded-md border border-border bg-muted/50"
          >
            {#if imageDataUrl}
              <img
                src={imageDataUrl}
                alt="Clipboard content"
                class="max-h-32 max-w-full object-contain transition-transform duration-300 group-hover/image:scale-105"
                loading="lazy"
              />
            {:else}
              <div class="flex h-20 w-20 items-center justify-center text-xs text-muted-foreground">
                {t.loading}
              </div>
            {/if}
          </div>
        {:else}
          <div class="flex items-center gap-2 rounded bg-muted/50 p-2 text-sm text-muted-foreground">
            <File class="h-4 w-4" />
            <span class="italic">{t.binaryFileData}</span>
          </div>
        {/if}

        <div class="mt-2 flex items-center justify-between">
          <span class="text-xs font-medium text-muted-foreground opacity-70">
            {formatTime(item.timestamp)}
          </span>

          <div
            class="flex translate-y-1 items-center gap-1 opacity-0 transition-all duration-200 group-hover:translate-y-0 group-hover:opacity-100"
          >
            <Button
              variant="ghost"
              size="icon"
              class="h-7 w-7 hover:bg-primary/10 hover:text-primary"
              title={t.copy}
              onclick={handleCopy}
            >
              {#if isCopied}
                <Check class="h-3.5 w-3.5 text-green-500" />
              {:else}
                <Copy class="h-3.5 w-3.5" />
              {/if}
            </Button>

            <Button
              variant="ghost"
              size="icon"
              class="h-7 w-7 {item.isPinned
                ? 'text-primary'
                : 'text-muted-foreground hover:bg-primary/10 hover:text-primary'}"
              title={item.isPinned ? t.unpin : t.pin}
              onclick={handleTogglePin}
            >
              <Pin class="h-3.5 w-3.5 {item.isPinned ? 'fill-current' : ''}" />
            </Button>

            <Button
              variant="ghost"
              size="icon"
              class="h-7 w-7 text-muted-foreground hover:bg-destructive/10 hover:text-destructive"
              title={t.delete}
              onclick={handleDelete}
            >
              <Trash2 class="h-3.5 w-3.5" />
            </Button>
          </div>
        </div>
      </div>
    </div>
  </Card>
</div>
