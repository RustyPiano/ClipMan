<script lang="ts">
  import type { ClipItem, ClipDetail } from '$lib/types';
  import { clipboardStore } from '$lib/stores/clipboard.svelte';
  import { i18n } from '$lib/i18n';
  import { decodeClipText, decodeFilePaths } from '$lib/utils/clip-items';
  import Button from './ui/Button.svelte';

  let { item }: { item: ClipItem | undefined } = $props();
  const t = $derived(i18n.t);
  let detail = $state.raw<ClipDetail | null>(null);
  let settled = $state.raw<ClipItem | undefined>();
  let loading = $state(false);
  let body: HTMLDivElement | undefined = $state();
  let generation = 0;

  async function load(current: ClipItem, revision: number) {
    loading = true;
    const full = await clipboardStore.fetchFullClip(current.id);
    if (revision === generation) {
      detail = full;
      loading = false;
    }
  }

  $effect(() => {
    const current = item;
    const revision = ++generation;
    // Highlight changes immediately; the expensive body follows a settled selection.
    const timer = setTimeout(() => {
      settled = current;
      detail = current ? (clipboardStore.getCachedFullClip(current.id) ?? null) : null;
      loading = false;
      if (body) body.scrollTop = 0;
      if (current && current.contentType !== 'image' && (current.contentBytes ?? 0) <= 256 * 1024) {
        void load(current, revision);
      }
    }, 70);
    return () => {
      clearTimeout(timer);
      generation += 1;
    };
  });

  const full = $derived(detail?.id === settled?.id ? detail : null);
  const text = $derived(
    full?.text ??
      (settled?.contentType === 'text'
        ? decodeClipText(settled, t.emptyContent, t.decodeFailed)
        : settled
          ? decodeFilePaths(settled).join('\n')
          : '')
  );
  const imageUrl = $derived(full?.imageUrl || settled?.content || '');
</script>

<div class="flex h-full flex-col text-foreground">
  <header class="flex flex-none flex-col gap-1 px-5 pb-4 pt-5">
    <span class="truncate text-sm font-medium">
      {settled?.label ||
        (settled?.contentType === 'image'
          ? t.image
          : settled?.contentType === 'files'
            ? t.files
            : t.text)}</span
    >
    {#if settled}<span class="truncate text-[11px] text-muted-foreground">
        {settled.sourceApp ? `${settled.sourceApp} · ` : ''}{new Date(
          settled.timestamp * 1000
        ).toLocaleString(i18n.locale)}
      </span>{/if}
  </header>
  <div
    bind:this={body}
    class="min-h-0 flex-1 overflow-auto px-5 pb-5"
    aria-busy={loading || item?.id !== settled?.id}
  >
    {#if settled?.contentType === 'image'}
      <img
        src={imageUrl}
        alt={settled.label || t.image}
        class="mx-auto max-w-full rounded object-contain"
      />
    {:else}
      <pre
        class="m-0 whitespace-pre-wrap break-words font-mono text-[13px] leading-6 selection:bg-primary/20">{text}</pre>
    {/if}
  </div>
  {#if settled && (!full || full?.truncated)}<footer
      class="flex flex-none flex-col gap-2 px-5 pb-4 pt-2 text-[11px] text-muted-foreground"
    >
      {#if settled && !full}
        <Button
          variant="outline"
          disabled={loading || item?.id !== settled.id}
          onclick={() => settled && load(settled, generation)}
          >{loading ? t.loading : t.fullPreview}</Button
        >
      {/if}
      {#if full?.truncated}<span>{t.previewLimited}</span>
      {:else if settled && !full}<span>{t.previewTruncated}</span>{/if}
    </footer>{/if}
</div>
