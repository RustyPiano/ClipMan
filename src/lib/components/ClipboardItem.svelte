<script lang="ts">
  import { onDestroy } from 'svelte';
  import type { Attachment } from 'svelte/attachments';
  import { clipboardStore } from '$lib/stores/clipboard.svelte';
  import type { ClipItem } from '$lib/stores/clipboard.svelte';
  import { i18n } from '$lib/i18n';
  import { decodeClipText, decodeFilePaths } from '$lib/utils/clip-items';
  import { getNow } from '$lib/utils/now.svelte';
  import Button from './ui/Button.svelte';
  import {
    Copy,
    Check,
    Pin,
    Trash2,
    FileText,
    Image as ImageIcon,
    File,
    Files,
    Folder,
    Pencil,
    X,
  } from 'lucide-svelte';

  interface Props {
    item: ClipItem;
    selected?: boolean;
    /** ⌘/Ctrl-click multi-select membership (distinct from keyboard highlight). */
    multiSelected?: boolean;
    slotNumber?: number | null;
    onSelect?: () => void;
    /** Hover-driven selection; falls back to onSelect when not provided. */
    onHover?: () => void;
    /** Toggle multi-select membership (⌘/Ctrl-click). */
    onToggleSelect?: () => void;
    onUse?: () => void | Promise<void>;
  }

  let {
    item,
    selected = false,
    multiSelected = false,
    slotNumber = null,
    onSelect,
    onHover,
    onToggleSelect,
    onUse,
  }: Props = $props();

  const t = $derived(i18n.t);
  // Multi-select wins the visual: a stronger primary border + ring plus a check
  // badge, so it reads distinctly from the keyboard highlight (subtle bg + left
  // accent bar).
  const cardClass = $derived(
    multiSelected
      ? 'border border-primary/40 bg-primary/12 shadow-sm ring-1 ring-inset ring-primary/40'
      : selected
        ? 'border border-primary/15 bg-primary/8 shadow-sm'
        : item.isPinned
          ? 'bg-secondary/40 border border-border/40 shadow-[0_1px_3px_rgba(0,0,0,0.02)]'
          : 'border border-transparent hover:bg-secondary/20'
  );

  let isCopied = $state(false);
  let isEditingLabel = $state(false);
  let isVisible = $state(false);
  let draftLabel = $state('');
  let copyTimeout: ReturnType<typeof setTimeout>;

  const decodedText = $derived.by(() => {
    return isVisible ? decodeClipText(item, t.emptyContent, t.decodeFailed) : ' ';
  });

  const imageDataUrl = $derived(
    item.contentType === 'image' && typeof item.content === 'string' ? item.content : ''
  );
  // Files preview is a small (≤4096 byte) path list, so decode eagerly rather
  // than gating on visibility like the text branch does.
  const filePaths = $derived(item.contentType === 'files' ? decodeFilePaths(item) : []);
  const trimmedLabel = $derived((item.label ?? '').trim());

  // Pure string helpers (no disk access) for rendering Files clips.
  function fileBasename(path: string): string {
    const trimmed = path.replace(/\/+$/, '');
    const slash = trimmed.lastIndexOf('/');
    return slash >= 0 ? trimmed.slice(slash + 1) : trimmed;
  }

  function fileDirname(path: string): string {
    const trimmed = path.replace(/\/+$/, '');
    const slash = trimmed.lastIndexOf('/');
    return slash > 0 ? trimmed.slice(0, slash) : '';
  }

  // A trailing slash or an extensionless basename reads as a directory.
  function looksLikeDirectory(path: string): boolean {
    if (path.endsWith('/')) return true;
    return !fileBasename(path).includes('.');
  }
  const showPinnedLabel = $derived(item.isPinned && trimmedLabel.length > 0 && !isEditingLabel);

  const observeVisibility: Attachment = (element) => {
    const IntersectionObserverCtor = globalThis.IntersectionObserver;
    if (!IntersectionObserverCtor) {
      isVisible = true;
      return;
    }

    const observer = new IntersectionObserverCtor(
      ([entry]) => {
        if (entry.isIntersecting) {
          isVisible = true;
          observer.disconnect();
        }
      },
      { rootMargin: '160px 0px' }
    );

    observer.observe(element);

    return () => observer.disconnect();
  };

  const focusLabelInput: Attachment = (element) => {
    if (element instanceof globalThis.HTMLInputElement) {
      element.focus();
      element.select();
    }
  };

  function formatTime(timestamp: number): string {
    const date = new Date(timestamp * 1000);
    const diff = getNow() - date.getTime();

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

  function handleClick(event: MouseEvent) {
    // ⌘Click (Ctrl+Click on Windows) toggles multi-select instead of pasting;
    // a plain click uses the clip as before.
    if (event.metaKey || event.ctrlKey) {
      event.preventDefault();
      event.stopPropagation();
      onToggleSelect?.();
      return;
    }
    void handleUse();
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

  function handleStartLabelEdit(event: MouseEvent) {
    event.stopPropagation();
    draftLabel = item.label ?? '';
    isEditingLabel = true;
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
  {@attach observeVisibility}
  class="group relative"
  style="content-visibility: auto; contain-intrinsic-size: 72px;"
  role="listitem"
  onmouseenter={onHover ?? onSelect}
>
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="relative cursor-pointer overflow-hidden rounded-lg {cardClass}" onclick={handleClick}>
    <!-- Left Accent Indicator -->
    <div
      class="absolute left-0 top-[25%] bottom-[25%] w-[3px] rounded-r-full bg-primary {selected
        ? 'opacity-100'
        : 'opacity-0'}"
    ></div>

    <!-- Multi-select check badge -->
    {#if multiSelected}
      <div
        class="absolute right-1.5 top-1.5 z-10 flex h-4 w-4 items-center justify-center rounded-full bg-primary text-primary-foreground shadow-sm"
        aria-hidden="true"
      >
        <Check class="h-3 w-3" />
      </div>
    {/if}

    <div class="flex gap-2.5 p-2 pl-3.5">
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
        {:else if item.contentType === 'files'}
          {#if filePaths.length > 1}
            <Files class="h-3.5 w-3.5" />
          {:else if looksLikeDirectory(filePaths[0] ?? '')}
            <Folder class="h-3.5 w-3.5" />
          {:else}
            <File class="h-3.5 w-3.5" />
          {/if}
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
              {@attach focusLabelInput}
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
        {:else if item.contentType === 'files'}
          {#if filePaths.length <= 1}
            {@const path = filePaths[0] ?? ''}
            <p class="line-clamp-1 break-all font-mono text-[13px] leading-relaxed text-foreground">
              {fileBasename(path) || path}
            </p>
            {#if fileDirname(path)}
              <p class="mt-0.5 truncate font-mono text-[11px] text-muted-foreground/60">
                {fileDirname(path)}
              </p>
            {/if}
          {:else}
            <div class="flex flex-col gap-0.5">
              {#each filePaths.slice(0, 2) as path (path)}
                <p
                  class="line-clamp-1 break-all font-mono text-[13px] leading-relaxed text-foreground"
                >
                  {fileBasename(path) || path}
                </p>
              {/each}
              {#if filePaths.length > 2}
                <span
                  class="mt-0.5 inline-flex w-fit flex-none items-center rounded border border-border/50 bg-muted/50 px-1.5 text-[10px] font-semibold text-muted-foreground/70"
                  title={i18n.format(t.fileCount, { n: filePaths.length })}
                >
                  +{filePaths.length - 2}
                </span>
              {/if}
            </div>
          {/if}
        {:else}
          <div
            class="group/image relative max-h-20 w-fit overflow-hidden rounded-lg border border-border bg-muted/40 shadow-sm transition-all duration-200 hover:shadow-md"
          >
            {#if imageDataUrl}
              <img
                src={imageDataUrl}
                alt={trimmedLabel || t.image}
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

        <div class="mt-1 flex items-center justify-between gap-2">
          <span class="flex min-w-0 items-center gap-1.5">
            <span class="truncate text-[11px] font-medium text-muted-foreground/60">
              {formatTime(item.timestamp)}{#if item.sourceApp}
                <span class="text-muted-foreground/45"> · {item.sourceApp}</span>
              {/if}
            </span>
            {#if item.contentType === 'text' && item.hasHtml}
              <span
                class="flex-none rounded border border-border/50 bg-muted/50 px-1 text-[9px] font-semibold leading-tight text-muted-foreground/70"
                title={t.richTextBadge}
              >
                Aa
              </span>
            {/if}
          </span>

          <div
            class="flex items-center gap-1 transition-all duration-200 ease-out {selected
              ? 'opacity-100 translate-y-0 pointer-events-auto'
              : 'opacity-0 translate-y-0.5 pointer-events-none group-hover:pointer-events-auto group-hover:opacity-100 group-hover:translate-y-0 focus-within:opacity-100 focus-within:translate-y-0 focus-within:pointer-events-auto'}"
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
