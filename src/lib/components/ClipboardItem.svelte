<script lang="ts">
  import { onDestroy } from 'svelte';
  import type { ClipItem } from '$lib/types';
  import { i18n } from '$lib/i18n';
  import {
    decodeClipText,
    decodeFilePaths,
    fileBasename,
    fileDirname,
    looksLikeDirectory,
  } from '$lib/utils/clip-items';
  import { ROW_HEIGHT_REM } from '$lib/constants';
  import { getNow } from '$lib/utils/now.svelte';
  import {
    FileText,
    Files,
    File as FileIcon,
    Folder,
    Image as ImageIcon,
    Pin,
    Check,
    Copy,
    Pencil,
    Trash2,
    X,
    Square,
  } from 'lucide-svelte';
  import { clipboardStore } from '$lib/stores/clipboard.svelte';
  import { toastStore } from '$lib/stores/toast.svelte';
  import Button from './ui/Button.svelte';

  type RowAction = {
    id: string;
    icon: typeof Copy;
    label: string;
    run: () => void | Promise<void>;
    pressed?: boolean;
    success?: boolean;
  };

  let {
    item,
    selected = false,
    multiSelected = false,
    multiSelecting = false,
    slotNumber = null,
    position,
    onSelect,
    onHover,
    onToggleSelect,
    onUse,
  }: {
    item: ClipItem;
    selected?: boolean;
    multiSelected?: boolean;
    multiSelecting?: boolean;
    slotNumber?: number | null;
    position: number;
    onSelect: () => void;
    onHover: () => void;
    onToggleSelect: () => void;
    onUse: () => void | Promise<void>;
  } = $props();
  const t = $derived(i18n.t);
  const paths = $derived(item.contentType === 'files' ? decodeFilePaths(item) : []);
  const text = $derived(
    item.contentType === 'text' ? decodeClipText(item, t.emptyContent, t.decodeFailed) : ''
  );
  // Three sections inside the fixed-height row: an optional label title line,
  // a content preview (two text lines / file paths / an inline image thumbnail;
  // compressed to one line when a label shares the row) and a metadata line.
  const visiblePaths = $derived(item.label ? paths.slice(0, 1) : paths.slice(0, 2));
  const age = $derived(Math.max(0, getNow() - item.timestamp * 1000));
  const time = $derived(
    age < 60000
      ? t.justNow
      : age < 3600000
        ? i18n.format(t.minutesAgo, { n: Math.floor(age / 60000) })
        : new Date(item.timestamp * 1000).toLocaleDateString(i18n.locale)
  );

  // 用字符串拼接元信息：模板里的换行在 WebKit 下会多渲染一个空格。
  const meta = $derived(
    [
      time,
      item.sourceApp,
      item.contentType === 'files' ? i18n.format(t.fileCount, { n: item.fileCount }) : null,
    ]
      .filter(Boolean)
      .join('\u00a0·\u00a0')
  );

  let editing = $state(false);
  let draft = $state('');
  let saving = $state(false);
  // Per-instance copy feedback: if the row scrolls out of the virtualized
  // window before the 1.5s timeout fires, the component unmounts and the
  // checkmark simply disappears early — acceptable for a transient hint.
  let isCopied = $state(false);
  let copyTimeout: ReturnType<typeof setTimeout>;
  const busy = $derived(clipboardStore.isUsing || clipboardStore.isSearchPending || saving);

  onDestroy(() => clearTimeout(copyTimeout));

  async function handleCopy() {
    if (await clipboardStore.copyToClipboard(item)) {
      isCopied = true;
      clearTimeout(copyTimeout);
      copyTimeout = setTimeout(() => (isCopied = false), 1500);
    }
  }

  // Stable ids keep the keyed each (and keyboard focus) intact when the pin
  // or multi-select labels/icons flip.
  const rowActions = $derived<RowAction[]>([
    {
      id: 'select',
      icon: multiSelected ? Check : Square,
      label: t.toggleSelection,
      run: onToggleSelect,
      pressed: multiSelected,
    },
    {
      id: 'copy',
      icon: isCopied ? Check : Copy,
      label: t.copy,
      run: () => handleCopy(),
      success: isCopied,
    },
    {
      id: 'pin',
      icon: Pin,
      label: item.isPinned ? t.unpin : t.pin,
      run: () => clipboardStore.togglePin(item.id),
      pressed: item.isPinned,
    },
    {
      id: 'label',
      icon: Pencil,
      label: t.editLabel,
      run: () => {
        draft = item.label ?? '';
        editing = true;
      },
    },
    { id: 'delete', icon: Trash2, label: t.delete, run: () => clipboardStore.deleteItem(item.id) },
  ]);

  async function saveLabel(event: SubmitEvent) {
    event.preventDefault();
    if (busy) return;
    saving = true;
    try {
      await clipboardStore.setClipLabel(item.id, draft);
      editing = false;
    } catch (error) {
      toastStore.add(String(error), 'error');
    } finally {
      saving = false;
    }
  }

  function use(event: MouseEvent) {
    if (event.metaKey || event.ctrlKey) onToggleSelect();
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
  data-active={selected || multiSelected}
  style:height={`${ROW_HEIGHT_REM}rem`}
  class="clip-row group relative flex w-full items-stretch rounded-lg pr-2"
  onmouseenter={() => {
    if (!editing) onHover();
  }}
>
  {#if editing}
    <div role="gridcell" class="min-w-0 flex-1 self-stretch">
      <form data-row-editor class="flex h-full items-center gap-1 pl-3" onsubmit={saveLabel}>
        <input
          aria-label={t.editLabel}
          class="h-8 min-w-0 flex-1 rounded border border-input bg-background px-2 text-sm"
          bind:value={draft}
          disabled={saving}
          {@attach (element) => {
            element.focus();
            element.select();
          }}
          onkeydown={(event) => {
            if (event.key === 'Escape') {
              event.preventDefault();
              editing = false;
            }
          }}
        />
        <Button
          type="submit"
          size="icon"
          variant="ghost"
          class="h-8 w-8"
          title={t.save}
          aria-label={t.save}
          disabled={busy}><Check class="h-4 w-4" /></Button
        >
        <Button
          size="icon"
          variant="ghost"
          class="h-8 w-8"
          title={t.cancel}
          aria-label={t.cancel}
          disabled={saving}
          onclick={() => (editing = false)}><X class="h-4 w-4" /></Button
        >
      </form>
    </div>
  {:else}
    {#if selected || multiSelected}
      <!-- Left accent indicator; decoration only, never eats row clicks -->
      <div
        class="pointer-events-none absolute bottom-[25%] left-0 top-[25%] w-[3px] rounded-r-full bg-primary"
        aria-hidden="true"
      ></div>
    {/if}
    {#if multiSelected}
      <!-- Multi-select check badge -->
      <div
        class="pointer-events-none absolute right-2 top-2 z-10 flex h-4 w-4 items-center justify-center rounded-full bg-primary text-primary-foreground shadow-sm"
        aria-hidden="true"
      >
        <Check class="h-3 w-3" />
      </div>
    {/if}
    <div role="gridcell" class="min-w-0 flex-1">
      <button
        type="button"
        tabindex="-1"
        class="flex h-full w-full min-w-0 items-stretch gap-2.5 pl-3.5 text-left"
        disabled={busy}
        onmousedown={(event) => event.preventDefault()}
        onclick={use}
      >
        <!-- Left column: number slot + content type icon -->
        <span
          class="flex w-6 flex-none flex-col items-center gap-1 self-center text-muted-foreground"
        >
          {#if slotNumber}
            <kbd class="kbd-keycap tabular-nums h-4 min-w-4 justify-center scale-95 text-[9px]"
              >{slotNumber}</kbd
            >
          {/if}
          <span class="flex h-4 w-4 items-center justify-center">
            {#if item.contentType === 'image'}
              <ImageIcon class="h-3.5 w-3.5" />
            {:else if item.contentType === 'files'}
              {#if paths.length > 1}
                <Files class="h-3.5 w-3.5" />
              {:else if looksLikeDirectory(paths[0] ?? '')}
                <Folder class="h-3.5 w-3.5" />
              {:else}
                <FileIcon class="h-3.5 w-3.5" />
              {/if}
            {:else}
              <FileText class="h-3.5 w-3.5" />
            {/if}
          </span>
        </span>
        <!-- Content column: title, preview, metadata -->
        <span class="flex min-w-0 flex-1 flex-col gap-0.5 py-1 {multiSelected ? 'pr-5' : ''}">
          {#if item.label}
            <span class="truncate text-sm font-semibold text-foreground">{item.label}</span>
          {/if}
          {#if item.contentType === 'text'}
            <span
              class={item.label
                ? 'block truncate font-mono text-[13px] leading-5 text-foreground'
                : 'block break-all font-mono text-[13px] leading-5 text-foreground line-clamp-2'}
              >{text}</span
            >
          {:else if item.contentType === 'files'}
            {#each visiblePaths as path, index (path)}
              {#if index === visiblePaths.length - 1 && paths.length > visiblePaths.length}
                <span class="flex min-w-0 items-center gap-1">
                  <span class="truncate font-mono text-[13px] leading-5 text-foreground"
                    >{fileBasename(path) || path}</span
                  >
                  <span
                    class="w-fit flex-none rounded border border-border/50 bg-muted/50 px-1.5 text-[10px] font-semibold leading-4 text-muted-foreground/70"
                    title={i18n.format(t.fileCount, { n: item.fileCount })}
                  >
                    +{paths.length - visiblePaths.length}
                  </span>
                </span>
              {:else}
                <span class="block truncate font-mono text-[13px] leading-5 text-foreground"
                  >{fileBasename(path) || path}</span
                >
              {/if}
            {/each}
            {#if paths.length === 1 && fileDirname(paths[0])}
              <span class="block truncate font-mono text-[11px] leading-4 text-muted-foreground/70"
                >{fileDirname(paths[0])}</span
              >
            {/if}
          {:else if item.content}
            <!-- Inline image thumbnail; shrinks when a label title shares the row. -->
            <img
              src={item.content}
              alt={item.label || t.image}
              class={item.label
                ? 'max-h-8 w-fit self-start rounded-md border border-border object-contain'
                : 'max-h-[52px] w-fit self-start rounded-md border border-border object-contain'}
              loading="lazy"
            />
          {/if}
          <span class="row-meta mt-auto flex min-w-0 items-center gap-1.5 pt-0.5 pr-[8.75rem]">
            <span class="truncate text-[11px] font-medium text-muted-foreground/70">{meta}</span>
            {#if item.hasHtml}
              <span
                class="flex-none rounded border border-border/50 bg-muted/50 px-1 text-[9px] font-semibold leading-tight text-muted-foreground/70"
                title={t.richTextBadge}
              >
                Aa
              </span>
            {/if}
          </span>
        </span>
      </button>
    </div>
    <div
      role="gridcell"
      class="row-actions absolute bottom-0.5 right-2 flex items-center gap-0.5 rounded-md px-0.5 {selected ||
      multiSelected
        ? 'active'
        : ''}"
    >
      {#each rowActions as action (action.id)}
        <Button
          variant="ghost"
          size="icon"
          class="rounded-md {action.success
            ? 'text-emerald-500'
            : action.pressed
              ? 'text-primary'
              : 'text-muted-foreground'}"
          style="width: 1.5rem; height: 1.5rem"
          title={action.label}
          aria-label={action.label}
          aria-pressed={action.pressed}
          tabindex={selected ? 0 : -1}
          disabled={busy}
          onclick={() => {
            onSelect();
            void action.run();
          }}><action.icon class="h-3.5 w-3.5" /></Button
        >
      {/each}
    </div>
  {/if}
</div>

<style>
  .clip-row {
    --row-surface: var(--qb-surface);
    background: var(--row-surface);
  }
  .clip-row:hover {
    --row-surface: var(--qb-hover);
  }
  .clip-row[data-active='true'] {
    --row-surface: var(--qb-selection);
  }
  /* Actions overlay the metadata line's reserved right padding (pr-[8.75rem]
     mirrors the 5 × 1.5rem buttons + gaps + offsets, so the text itself is
     never covered); they fade in on hover/selection without layout shift. */
  .row-actions {
    background: var(--row-surface);
    opacity: 0;
    pointer-events: none;
    transition: opacity 120ms ease-out;
  }
  .group:hover .row-actions,
  .group:focus-within .row-actions,
  .row-actions.active {
    opacity: 1;
    pointer-events: auto;
  }
  @media (hover: none) {
    .row-actions {
      opacity: 1;
      pointer-events: auto;
    }
  }
</style>
