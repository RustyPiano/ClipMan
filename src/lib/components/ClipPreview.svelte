<script lang="ts">
  import type { ClipItem } from '$lib/types';
  import { clipboardStore } from '$lib/stores/clipboard.svelte';
  import { i18n } from '$lib/i18n';
  import { decodeClipText } from '$lib/utils/clip-items';
  import { FileText, Image as ImageIcon, Pin, Clock } from 'lucide-svelte';

  interface Props {
    item: ClipItem | undefined;
  }

  let { item }: Props = $props();

  const t = $derived(i18n.t);
  // Wait for the selection to settle before fetching, so holding an arrow key to
  // scroll the list doesn't fire an IPC request per row.
  const SETTLE_MS = 90;

  // Full-fidelity text payload for the selected item.
  let fullItem = $state<ClipItem | null>(null);

  $effect(() => {
    const current = item;

    // Only text needs a full fetch (the list preview truncates it to 4096 bytes).
    // Images reuse the 256px thumbnail already on the list item — no full-res
    // fetch, so large image payloads never cross IPC or sit in the cache.
    if (!current || current.contentType !== 'text') {
      fullItem = null;
      return;
    }

    // Cached → show full content immediately (content is immutable per id).
    const cached = clipboardStore.getCachedFullClip(current.id);
    if (cached) {
      fullItem = cached;
      return;
    }

    // Not cached: keep showing the truncated list preview, then upgrade once the
    // selection settles. Guard against a stale fetch resolving after the
    // selection moved on.
    fullItem = null;
    const wantedId = current.id;
    const timer = setTimeout(() => {
      void clipboardStore.fetchFullClip(wantedId).then((full) => {
        if (item?.id === wantedId) fullItem = full;
      });
    }, SETTLE_MS);

    return () => clearTimeout(timer);
  });

  // Content comes from the full payload once ready, otherwise the truncated list
  // item so the pane is never blank. Mutable fields (label/pin/time) are always
  // read from the live list item.
  const contentItem = $derived(fullItem && item && fullItem.id === item.id ? fullItem : item);
  const isImage = $derived(item?.contentType === 'image');
  const imageUrl = $derived(
    isImage && typeof contentItem?.content === 'string' ? contentItem.content : ''
  );
  const fullText = $derived(
    contentItem && contentItem.contentType === 'text'
      ? decodeClipText(contentItem, t.emptyContent, t.decodeFailed)
      : ''
  );
  const charCount = $derived(Array.from(fullText).length);
  const trimmedLabel = $derived((item?.label ?? '').trim());

  function formatFullTime(timestamp: number): string {
    return new Date(timestamp * 1000).toLocaleString(i18n.locale === 'zh-CN' ? 'zh-CN' : 'en-US');
  }
</script>

<div class="flex h-full min-h-0 flex-col bg-muted/10">
  {#if !item}
    <div
      class="flex h-full flex-col items-center justify-center gap-2 p-6 text-center text-muted-foreground"
    >
      <FileText class="h-7 w-7 opacity-20" />
      <p class="text-xs opacity-70">{t.selectToPreview}</p>
    </div>
  {:else}
    <!-- Header: type / label / pinned -->
    <div class="flex flex-none items-center gap-2 border-b border-border/60 px-3 py-2">
      {#if isImage}
        <ImageIcon class="h-3.5 w-3.5 flex-none text-muted-foreground" />
      {:else}
        <FileText class="h-3.5 w-3.5 flex-none text-muted-foreground" />
      {/if}
      {#if trimmedLabel}
        <span class="min-w-0 flex-1 truncate text-xs font-semibold text-foreground"
          >{trimmedLabel}</span
        >
      {:else}
        <span class="min-w-0 flex-1 truncate text-xs font-medium text-muted-foreground"
          >{isImage ? t.image : t.text}</span
        >
      {/if}
      {#if item.isPinned}
        <Pin class="h-3.5 w-3.5 flex-none fill-current text-primary" />
      {/if}
    </div>

    <!-- Body: full content -->
    <div class="min-h-0 flex-1 overflow-auto p-3">
      {#if isImage}
        {#if imageUrl}
          <img
            src={imageUrl}
            alt={trimmedLabel || t.image}
            class="mx-auto max-w-full rounded-md border border-border bg-muted/40 object-contain"
          />
        {/if}
      {:else}
        <pre
          class="m-0 whitespace-pre-wrap break-words font-mono text-[12px] leading-relaxed text-foreground selection:bg-primary/20">{fullText}</pre>
      {/if}
    </div>

    <!-- Footer: metadata -->
    <div
      class="flex flex-none items-center justify-between gap-2 border-t border-border/60 px-3 py-1.5 text-[10px] text-muted-foreground"
    >
      <span class="flex items-center gap-1">
        <Clock class="h-3 w-3" />
        {formatFullTime(item.timestamp)}
      </span>
      {#if !isImage}
        <span class="tabular-nums">{i18n.format(t.charCount, { n: charCount })}</span>
      {/if}
    </div>
  {/if}
</div>
