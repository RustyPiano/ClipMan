<script lang="ts">
  import { onDestroy, tick } from 'svelte';
  import { clipboardStore } from '$lib/stores/clipboard.svelte';
  import type { ClipItem } from '$lib/stores/clipboard.svelte';
  import { i18n } from '$lib/i18n';
  import Button from './ui/Button.svelte';
  import { Copy, Check, Pin, Trash2, FileText, Image as ImageIcon, Pencil, X } from 'lucide-svelte';

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
      ? 'bg-primary/8 ring-1 ring-primary/15 shadow-sm'
      : item.isPinned
        ? 'bg-secondary/40 border border-border/40 shadow-[0_1px_3px_rgba(0,0,0,0.02)]'
        : 'border border-transparent hover:bg-secondary/20'
  );

  let isCopied = $state(false);
  let isEditingLabel = $state(false);
  let draftLabel = $state('');
  let labelInput = $state<HTMLInputElement | null>(null);
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
    item.contentType === 'image' && typeof item.content === 'string' ? item.content : ''
  );
  const trimmedLabel = $derived((item.label ?? '').trim());
  const showPinnedLabel = $derived(item.isPinned && trimmedLabel.length > 0 && !isEditingLabel);

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

  async function handleStartLabelEdit(event: MouseEvent) {
    event.stopPropagation();
    draftLabel = item.label ?? '';
    isEditingLabel = true;
    await tick();
    labelInput?.focus();
    labelInput?.select();
  }

  async function handleLabelSubmit(event: Event) {
    event.preventDefault();
    event.stopPropagation();
    await clipboardStore.setClipLabel(item.id, draftLabel);
    isEditingLabel = false;
  }

  function handleCancelLabelEdit(event: MouseEvent | KeyboardEvent) {
    event.preventDefault();
    event.stopPropagation();
    isEditingLabel = false;
    draftLabel = item.label ?? '';
  }

  function handleLabelKeydown(event: KeyboardEvent) {
    event.stopPropagation();
    if (event.key === 'Escape') {
      handleCancelLabelEdit(event);
    }
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
  class="group relative transition-all duration-150"
  role="listitem"
  onmouseenter={onSelect}
>
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    class="relative cursor-pointer overflow-hidden rounded-lg transition-all duration-200 ease-out {cardClass}"
    onclick={handleUse}
  >
    <!-- Left Accent Indicator -->
    {#if selected}
      <div
        class="absolute left-0 top-[25%] bottom-[25%] w-[3px] rounded-r-full bg-primary transition-all duration-300 animate-in fade-in slide-in-from-left-1"
      ></div>
    {/if}

    <div class="flex gap-2.5 p-2 transition-all duration-200 {selected ? 'pl-3.5' : ''}">
      <div class="flex w-6 flex-none flex-col items-center gap-0.5 pt-0.5 text-muted-foreground">
        {#if slotNumber}
          <kbd
            class="kbd-keycap min-w-4 text-[9px] h-4 scale-95 {selected
              ? '!border-primary !border-b-primary !bg-primary !text-primary-foreground shadow-none'
              : ''}"
          >
            {slotNumber}
          </kbd>
        {/if}

        {#if item.contentType === 'image'}
          <ImageIcon class="h-3.5 w-3.5" />
        {:else}
          <FileText class="h-3.5 w-3.5" />
        {/if}
      </div>

      <div class="min-w-0 flex-1">
        {#if isEditingLabel}
          <!-- svelte-ignore a11y_click_events_have_key_events -->
          <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
          <form
            class="mb-2 flex items-center gap-2"
            onsubmit={handleLabelSubmit}
            onclick={(event) => event.stopPropagation()}
          >
            <input
              bind:this={labelInput}
              bind:value={draftLabel}
              aria-label={t.editLabel}
              class="h-8 min-w-0 flex-1 rounded-md border border-input bg-background px-2 text-sm outline-none ring-primary/30 transition focus:ring-2"
              placeholder={t.labelPlaceholder}
              onkeydown={handleLabelKeydown}
            />
            <Button
              type="submit"
              variant="ghost"
              size="icon"
              class="h-8 w-8 text-primary hover:bg-primary/10"
              title={t.save}
            >
              <Check class="h-3.5 w-3.5" />
            </Button>
            <Button
              type="button"
              variant="ghost"
              size="icon"
              class="h-8 w-8 text-muted-foreground hover:bg-muted"
              title={t.cancel}
              onclick={handleCancelLabelEdit}
            >
              <X class="h-3.5 w-3.5" />
            </Button>
          </form>
        {:else if showPinnedLabel}
          <p class="line-clamp-1 break-words text-sm font-semibold text-foreground">
            {trimmedLabel}
          </p>
        {/if}

        {#if item.contentType === 'text'}
          <p
            class={showPinnedLabel
              ? 'mt-0.5 line-clamp-1 break-all text-xs leading-relaxed text-muted-foreground selection:bg-primary/20'
              : 'line-clamp-2 break-all font-mono text-[13px] leading-relaxed text-foreground selection:bg-primary/20'}
          >
            {decodedText}
          </p>
        {:else}
          <div
            class="group/image relative max-h-20 w-fit overflow-hidden rounded-lg border border-border bg-muted/40 shadow-sm transition-all duration-200 hover:shadow-md"
          >
            {#if imageDataUrl}
              <img
                src={imageDataUrl}
                alt="Clipboard content"
                class="max-h-20 max-w-full object-contain transition-transform duration-300 group-hover/image:scale-105"
                loading="lazy"
              />
            {:else}
              <div class="flex h-20 w-20 items-center justify-center text-xs text-muted-foreground">
                {t.loading}
              </div>
            {/if}
          </div>
        {/if}

        <div class="mt-1 flex items-center justify-between">
          <span class="text-[11px] font-medium text-muted-foreground/60">
            {formatTime(item.timestamp)}
          </span>

          <div
            class="flex items-center gap-1 opacity-0 translate-y-0.5 pointer-events-none group-hover:pointer-events-auto group-hover:opacity-100 group-hover:translate-y-0 transition-all duration-200 ease-out"
          >
            <Button
              variant="ghost"
              size="icon"
              class="h-6 w-6 rounded-md hover:bg-primary/10 hover:text-primary active:scale-90 transition-all duration-150"
              title={t.copy}
              onclick={handleCopy}
            >
              {#if isCopied}
                <Check class="h-3.5 w-3.5 text-emerald-500 animate-in zoom-in-50 duration-200" />
              {:else}
                <Copy class="h-3.5 w-3.5" />
              {/if}
            </Button>

            {#if item.isPinned}
              <Button
                variant="ghost"
                size="icon"
                class="h-6 w-6 rounded-md text-muted-foreground hover:bg-primary/10 hover:text-primary active:scale-90 transition-all duration-150"
                title={t.editLabel}
                onclick={handleStartLabelEdit}
              >
                <Pencil class="h-3.5 w-3.5" />
              </Button>
            {/if}

            <Button
              variant="ghost"
              size="icon"
              class="h-6 w-6 rounded-md active:scale-90 transition-all duration-150 {item.isPinned
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
              class="h-6 w-6 rounded-md text-muted-foreground hover:bg-destructive/10 hover:text-destructive active:scale-90 transition-all duration-150"
              title={t.delete}
              onclick={handleDelete}
            >
              <Trash2 class="h-3.5 w-3.5" />
            </Button>
          </div>
        </div>
      </div>
    </div>
  </div>
</div>
