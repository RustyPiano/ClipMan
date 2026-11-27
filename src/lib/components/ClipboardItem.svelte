<script lang="ts">
  import { onDestroy } from 'svelte';
  import type { ClipItem } from '$lib/types';
  import { i18n } from '$lib/i18n';
  import {
    decodeClipText,
    decodeFilePaths,
    fileBasename,
    fileDirname,
    formatClipTime,
    highlightSegments,
    isSingleUrl,
    looksLikeCode,
    looksLikeDirectory,
    parseColorValue,
  } from '$lib/utils/clip-items';
  import { ROW_HEIGHT_REM, SEARCH_INPUT_ID } from '$lib/constants';
  import { getNow } from '$lib/utils/now.svelte';
  import {
    ArrowDown,
    ArrowUp,
    CodeXml,
    Link2,
    Files,
    File as FileIcon,
    Folder,
    Image as ImageIcon,
    Star,
    Check,
    Copy,
    Pencil,
    Trash2,
    X,
  } from 'lucide-svelte';
  import { clipboardStore } from '$lib/stores/clipboard.svelte';
  import { toastStore } from '$lib/stores/toast.svelte';
  import { appIconStore } from '$lib/stores/app-icons.svelte';
  import Button from './ui/Button.svelte';

  const GLYPHS = {
    image: ImageIcon,
    files: Files,
    folder: Folder,
    file: FileIcon,
    link: Link2,
    code: CodeXml,
  };

  let {
    item,
    selected = false,
    multiSelected = false,
    multiSelecting = false,
    showReorder = false,
    canMoveUp = false,
    canMoveDown = false,
    searchQuery = '',
    slotLabel = null,
    position,
    onSelect,
    onHover,
    onToggleSelect,
    onExtendSelect,
    onUse,
    onReorder,
  }: {
    item: ClipItem;
    selected?: boolean;
    multiSelected?: boolean;
    multiSelecting?: boolean;
    showReorder?: boolean;
    canMoveUp?: boolean;
    canMoveDown?: boolean;
    searchQuery?: string;
    slotLabel?: string | null;
    position: number;
    onSelect: () => void;
    onHover: () => void;
    onToggleSelect: () => void;
    onExtendSelect: () => void;
    onUse: () => void | Promise<void>;
    onReorder?: (direction: 'up' | 'down') => void;
  } = $props();

  const t = $derived(i18n.t);
  const paths = $derived(decodeFilePaths(item));
  const text = $derived(decodeClipText(item, t.emptyContent));
  const visiblePaths = $derived(item.label ? paths.slice(0, 1) : paths.slice(0, 2));
  const isUrl = $derived(item.contentType === 'text' && isSingleUrl(text));
  const color = $derived(item.contentType === 'text' ? parseColorValue(text) : null);
  const code = $derived(item.contentType === 'text' && !color && looksLikeCode(text));
  const kind = $derived(
    item.contentType === 'image'
      ? 'image'
      : item.contentType === 'files'
        ? item.fileCount > 1
          ? 'files'
          : looksLikeDirectory(paths[0] ?? '')
            ? 'folder'
            : 'file'
        : isUrl
          ? 'link'
          : code
            ? 'code'
            : 'text'
  );
  const time = $derived(
    formatClipTime(item.timestamp * 1000, getNow(), i18n.locale, {
      justNow: t.justNow,
      minutesAgo: (n) => i18n.format(t.minutesAgo, { n }),
      yesterday: t.yesterday,
    })
  );

  const meta = $derived(
    [
      item.sourceApp,
      time,
      item.contentType === 'files' ? i18n.format(t.fileCount, { n: item.fileCount }) : null,
      item.hasHtml ? 'HTML' : null,
    ]
      .filter(Boolean)
      .join('\u00a0·\u00a0')
  );

  const appIcon = $derived(item.sourceApp ? appIconStore.get(item.sourceApp) : null);
  const textSegments = $derived(highlightSegments(text, searchQuery));
  const labelSegments = $derived(item.label ? highlightSegments(item.label, searchQuery) : []);

  let editing = $state(false);
  let draft = $state('');
  let saving = $state(false);
  let isCopied = $state(false);
  let pinAnimKey = $state(0);
  let copyTimeout: ReturnType<typeof setTimeout>;
  const busy = $derived(clipboardStore.isUsing || clipboardStore.isSearchPending || saving);

  onDestroy(() => {
    clearTimeout(copyTimeout);
    if (clipboardStore.editingLabelId === item.id) {
      clipboardStore.editingLabelId = null;
    }
  });

  async function handleCopy() {
    if (await clipboardStore.copyToClipboard(item)) {
      isCopied = true;
      clearTimeout(copyTimeout);
      copyTimeout = setTimeout(() => (isCopied = false), 1500);
    }
  }

  function handlePin() {
    pinAnimKey += 1;
    void clipboardStore.togglePin(item.id);
  }

  function closeLabelEditor() {
    editing = false;
    if (clipboardStore.editingLabelId === item.id) {
      clipboardStore.editingLabelId = null;
    }
    document.getElementById(SEARCH_INPUT_ID)?.focus({ preventScroll: true });
  }

  async function saveLabel(event: SubmitEvent) {
    event.preventDefault();
    if (busy) return;
    saving = true;
    try {
      await clipboardStore.setClipLabel(item.id, draft);
      closeLabelEditor();
    } catch (error) {
      toastStore.add(String(error), 'error');
    } finally {
      saving = false;
    }
  }

  function use(event: MouseEvent) {
    if (event.shiftKey) onExtendSelect();
    else if (event.metaKey || event.ctrlKey) onToggleSelect();
    else {
      onSelect();
      void onUse();
    }
  }
</script>

<div
  role="row"
  tabindex="-1"
  id={`clip-item-${item.id}`}
  aria-selected={multiSelecting ? multiSelected : selected}
  aria-rowindex={position + 1}
  data-active={selected}
  data-multi={multiSelected}
  data-pinned={item.isPinned}
  style:height={`${ROW_HEIGHT_REM}rem`}
  class="clip-row group relative flex w-full items-center gap-2.5 rounded-lg px-2.5"
  onmouseenter={() => {
    if (!editing) onHover();
  }}
>
  <div role="gridcell" class="flex flex-none items-center">
    <button
      type="button"
      class="clip-glyph flex h-6 w-6 items-center justify-center rounded-md"
      title={t.toggleSelection}
      aria-label={t.toggleSelection}
      aria-pressed={multiSelected}
      tabindex={selected ? 0 : -1}
      disabled={busy || editing}
      onmousedown={(event) => event.preventDefault()}
      onclick={(event) => {
        if (event.shiftKey) return onExtendSelect();
        onSelect();
        onToggleSelect();
      }}
    >
      <span
        class="clip-check items-center justify-center rounded-full {multiSelected || multiSelecting
          ? 'flex'
          : 'hidden group-hover:flex'}"
        aria-hidden="true"
      >
        {#if multiSelected}<Check class="h-2.5 w-2.5" strokeWidth={3} />{/if}
      </span>
      <span class={multiSelected || multiSelecting ? 'hidden' : 'group-hover:hidden'}>
        {#if color}
          <span
            class="block h-3.5 w-3.5 rounded-[4px] border border-black/15 dark:border-white/20"
            style:background-color={color.css}
            aria-hidden="true"
          ></span>
        {:else if kind === 'text'}
          <svg class="h-[15px] w-[15px]" viewBox="0 0 16 16" fill="none" aria-hidden="true">
            <path
              d="M3 4.5h10M3 8h10M3 11.5h6"
              stroke="currentColor"
              stroke-width="1.35"
              stroke-linecap="round"
            />
          </svg>
        {:else}
          {@const Glyph = GLYPHS[kind]}
          <Glyph class="h-[15px] w-[15px]" strokeWidth={1.5} />
        {/if}
      </span>
    </button>
  </div>

  {#if editing}
    <div role="gridcell" class="min-w-0 flex-1 self-stretch">
      <form data-row-editor class="flex h-full items-center gap-1" onsubmit={saveLabel}>
        <input
          aria-label={t.editLabel}
          class="h-8 min-w-0 flex-1 rounded-md border border-input bg-background px-2.5 text-xs text-foreground"
          bind:value={draft}
          disabled={saving}
          {@attach (element) => {
            element.focus();
            element.select();
          }}
          onkeydown={(event) => {
            if (event.isComposing || event.keyCode === 229) return;
            if (event.key === 'Escape') {
              event.preventDefault();
              closeLabelEditor();
            }
          }}
        />
        <Button
          type="submit"
          size="icon"
          variant="ghost"
          class="h-7 w-7"
          title={t.save}
          aria-label={t.save}
          disabled={busy}><Check class="h-3.5 w-3.5" /></Button
        >
        <Button
          size="icon"
          variant="ghost"
          class="h-7 w-7"
          title={t.cancel}
          aria-label={t.cancel}
          disabled={saving}
          onclick={closeLabelEditor}><X class="h-3.5 w-3.5" /></Button
        >
      </form>
    </div>
  {:else}
    <div role="gridcell" class="min-w-0 flex-1 self-stretch">
      <button
        type="button"
        tabindex="-1"
        class="clip-hit flex h-full w-full min-w-0 flex-col justify-center pr-10 text-left"
        disabled={busy}
        onmousedown={(event) => event.preventDefault()}
        onclick={use}
      >
        <span class="flex min-w-0 flex-col justify-center">
          {#if item.label}
            <span class="truncate text-[13px] font-medium leading-[1.3] text-foreground">
              {#each labelSegments as seg, i (i)}
                {#if seg.match}<mark class="qb-mark">{seg.text}</mark>{:else}{seg.text}{/if}
              {/each}
            </span>
          {/if}
          {#if item.contentType === 'text'}
            <span
              class="{item.label
                ? 'truncate text-[12px] text-muted-foreground'
                : 'truncate text-[13px] text-foreground'} leading-[1.35] {code
                ? 'font-mono text-[12px]'
                : isUrl
                  ? 'qb-link-text'
                  : ''}"
            >
              {#each textSegments as seg, i (i)}
                {#if seg.match}<mark class="qb-mark">{seg.text}</mark>{:else}{seg.text}{/if}
              {/each}
            </span>
          {:else if item.contentType === 'files'}
            <span class="flex min-w-0 items-center gap-1">
              {#each visiblePaths as path, index (`${path}-${index}`)}
                {#if index > 0}
                  <span class="flex-none text-xs text-muted-foreground/50" aria-hidden="true"
                    >·</span
                  >
                {/if}
                <span class="truncate text-[13px] leading-[1.35] text-foreground"
                  >{fileBasename(path) || path}</span
                >
              {/each}
              {#if item.fileCount > visiblePaths.length}
                <span
                  class="flex-none text-[11px] font-medium text-muted-foreground"
                  title={i18n.format(t.fileCount, { n: item.fileCount })}
                  >+{item.fileCount - visiblePaths.length}</span
                >
              {/if}
            </span>
            {#if item.fileCount === 1 && !item.label && fileDirname(paths[0])}
              <span class="truncate text-[11px] leading-[1.25] text-muted-foreground"
                >{fileDirname(paths[0])}</span
              >
            {/if}
          {:else if item.content}
            <img
              src={item.content}
              alt={item.label || t.image}
              class="{item.label
                ? 'h-4'
                : 'h-7'} w-auto max-w-[120px] self-start rounded border border-border object-cover"
              loading="lazy"
            />
          {/if}
        </span>

        <span class="row-meta mt-0.5 flex h-4 min-w-0 flex-none items-center gap-1.5">
          {#if appIcon}<img src={appIcon} alt="" class="h-3.5 w-3.5 flex-none rounded-[3px]" />{/if}
          <span class="truncate text-[11px] leading-4 text-muted-foreground/85">{meta}</span>
        </span>
      </button>
    </div>

    <div
      class="pointer-events-none absolute right-2.5 top-1/2 flex -translate-y-1/2 items-center gap-1.5 transition-opacity group-hover:opacity-0 group-focus-within:opacity-0"
    >
      {#if item.isPinned}
        {#key pinAnimKey}
          <span
            class="qb-pin-text animate-pin-pop inline-flex items-center"
            data-pin-badge
            title={t.pinnedBadge}
            aria-label={t.pinnedBadge}
          >
            <Star class="h-3 w-3 fill-current" />
          </span>
        {/key}
      {/if}
      {#if slotLabel}
        <span
          class="slot-hint text-[10px] font-medium tabular-nums text-muted-foreground/45"
          aria-hidden="true">{slotLabel}</span
        >
      {/if}
    </div>

    <div
      role="gridcell"
      class="row-actions absolute right-2 top-1/2 flex -translate-y-1/2 items-center gap-0.5 rounded-md pl-2"
    >
      {#if showReorder}
        <button
          type="button"
          class="row-action-btn"
          title={t.moveUp}
          aria-label={t.moveUp}
          tabindex={selected ? 0 : -1}
          disabled={busy || !canMoveUp}
          onmousedown={(event) => event.preventDefault()}
          onclick={() => onReorder?.('up')}
        >
          <ArrowUp class="h-3.5 w-3.5" />
        </button>
        <button
          type="button"
          class="row-action-btn"
          title={t.moveDown}
          aria-label={t.moveDown}
          tabindex={selected ? 0 : -1}
          disabled={busy || !canMoveDown}
          onmousedown={(event) => event.preventDefault()}
          onclick={() => onReorder?.('down')}
        >
          <ArrowDown class="h-3.5 w-3.5" />
        </button>
      {/if}
      <button
        type="button"
        class="row-action-btn {isCopied ? 'is-copied' : ''}"
        title={t.copy}
        aria-label={t.copy}
        tabindex={selected ? 0 : -1}
        disabled={busy}
        onmousedown={(event) => event.preventDefault()}
        onclick={() => {
          onSelect();
          void handleCopy();
        }}
      >
        {#if isCopied}
          <Check class="h-3.5 w-3.5" />
        {:else}
          <Copy class="h-3.5 w-3.5" />
        {/if}
      </button>
      <button
        type="button"
        class="row-action-btn {item.isPinned ? 'is-pinned' : ''}"
        title={item.isPinned ? t.unpin : t.pin}
        aria-label={item.isPinned ? t.unpin : t.pin}
        aria-pressed={item.isPinned}
        tabindex={selected ? 0 : -1}
        disabled={busy}
        onmousedown={(event) => event.preventDefault()}
        onclick={() => {
          onSelect();
          handlePin();
        }}
      >
        {#key pinAnimKey}
          <Star class="h-3.5 w-3.5 {item.isPinned ? 'animate-pin-pop fill-current' : ''}" />
        {/key}
      </button>
      <button
        type="button"
        class="row-action-btn"
        title={t.editLabel}
        aria-label={t.editLabel}
        tabindex={selected ? 0 : -1}
        disabled={busy}
        onmousedown={(event) => event.preventDefault()}
        onclick={() => {
          onSelect();
          draft = item.label ?? '';
          clipboardStore.editingLabelId = item.id;
          editing = true;
        }}
      >
        <Pencil class="h-3.5 w-3.5" />
      </button>
      <button
        type="button"
        class="row-action-btn is-danger"
        title={t.delete}
        aria-label={t.delete}
        tabindex={selected ? 0 : -1}
        disabled={busy}
        onmousedown={(event) => event.preventDefault()}
        onclick={() => {
          onSelect();
          void clipboardStore.deleteItem(item.id);
        }}
      >
        <Trash2 class="h-3.5 w-3.5" />
      </button>
    </div>
  {/if}
</div>

<style>
  .clip-row {
    --row-surface: var(--qb-surface);
    background: var(--row-surface);
    border-block: 1px solid transparent;
    background-clip: padding-box;
  }
  .clip-row:hover {
    --row-surface: var(--qb-hover);
  }
  .clip-row[data-multi='true'] {
    --row-surface: var(--qb-multi);
  }
  .clip-row[data-active='true'] {
    --row-surface: var(--qb-selection);
  }
  .clip-glyph {
    color: color-mix(in srgb, var(--muted-foreground) 80%, transparent);
  }
  .clip-row[data-active='true'] .clip-glyph {
    color: var(--foreground);
  }
  .clip-check {
    width: 15px;
    height: 15px;
    box-shadow: inset 0 0 0 1.3px color-mix(in srgb, var(--muted-foreground) 70%, transparent);
  }
  .clip-row[data-multi='true'] .clip-check {
    color: var(--primary-foreground);
    background: var(--primary);
    box-shadow: none;
  }
  .row-actions {
    background: linear-gradient(90deg, transparent, var(--row-surface) 18%, var(--row-surface));
    opacity: 0;
    pointer-events: none;
    transition: opacity 90ms ease-out;
  }
  .group:hover .row-actions,
  .group:focus-within .row-actions {
    opacity: 1;
    pointer-events: auto;
  }
  .row-action-btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 1.625rem;
    height: 1.625rem;
    border-radius: 5px;
    color: var(--muted-foreground);
  }
  .row-action-btn:hover:not(:disabled) {
    background: color-mix(in srgb, var(--foreground) 7%, transparent);
    color: var(--foreground);
  }
  .row-action-btn.is-pinned {
    color: var(--pin-accent);
  }
  .row-action-btn.is-pinned:hover:not(:disabled) {
    color: var(--pin-accent);
  }
  .row-action-btn.is-copied {
    color: var(--success-accent);
  }
  .row-action-btn.is-danger:hover:not(:disabled) {
    color: var(--destructive);
  }
  .row-action-btn:disabled {
    opacity: 0.3;
    pointer-events: none;
  }
  @media (hover: none) {
    .row-actions {
      opacity: 1;
      pointer-events: auto;
    }
  }
</style>
