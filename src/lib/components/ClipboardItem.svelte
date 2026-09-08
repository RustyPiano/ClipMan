<script lang="ts">
  import type { ClipItem } from '$lib/types';
  import { i18n } from '$lib/i18n';
  import { decodeClipText, decodeFilePaths } from '$lib/utils/clip-items';
  import { getNow } from '$lib/utils/now.svelte';
  import { FileText, Files, Pin, Check, Copy, Pencil, Trash2, X, Square } from 'lucide-svelte';
  import { clipboardStore } from '$lib/stores/clipboard.svelte';
  import { toastStore } from '$lib/stores/toast.svelte';
  import Button from './ui/Button.svelte';

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
  const title = $derived(
    item.label?.trim() ||
      (item.contentType === 'text'
        ? text
        : item.contentType === 'files'
          ? paths[0]?.split(/[/\\]/).pop() || t.files
          : t.image)
  );
  const age = $derived(Math.max(0, getNow() - item.timestamp * 1000));
  const time = $derived(
    age < 60000
      ? t.justNow
      : age < 3600000
        ? i18n.format(t.minutesAgo, { n: Math.floor(age / 60000) })
        : new Date(item.timestamp * 1000).toLocaleDateString(i18n.locale)
  );

  let editing = $state(false);
  let draft = $state('');
  let saving = $state(false);
  const busy = $derived(clipboardStore.isUsing || clipboardStore.isSearchPending || saving);

  const rowActions = $derived([
    {
      icon: multiSelected ? Check : Square,
      label: t.toggleSelection,
      run: onToggleSelect,
      pressed: multiSelected,
    },
    { icon: Copy, label: t.copy, run: () => clipboardStore.copyToClipboard(item) },
    {
      icon: Pin,
      label: item.isPinned ? t.unpin : t.pin,
      run: () => clipboardStore.togglePin(item.id),
      pressed: item.isPinned,
    },
    {
      icon: Pencil,
      label: t.editLabel,
      run: () => {
        draft = item.label ?? '';
        editing = true;
      },
    },
    { icon: Trash2, label: t.delete, run: () => clipboardStore.deleteItem(item.id) },
  ]);

  async function saveLabel(event: globalThis.SubmitEvent) {
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
  class="clip-row group relative flex h-16 w-full items-center rounded-lg pr-2"
  onmouseenter={() => {
    if (!editing) onHover();
  }}
>
  <div role="gridcell" class="min-w-0 flex-1">
    {#if editing}
      <form data-row-editor class="flex items-center gap-1 pl-3" onsubmit={saveLabel}>
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
    {:else}
      <button
        type="button"
        tabindex="-1"
        class="flex h-16 w-full min-w-0 items-center gap-3 px-3 text-left"
        disabled={busy}
        onmousedown={(event) => event.preventDefault()}
        onclick={use}
      >
        <span
          class="flex h-8 w-8 flex-none items-center justify-center overflow-hidden row-icon rounded-lg text-muted-foreground"
        >
          {#if item.contentType === 'image'}<img
              src={item.content}
              alt={t.image}
              width="32"
              height="32"
              class="h-8 w-8 object-contain"
              loading="lazy"
            />
          {:else if item.contentType === 'files'}<Files class="h-4 w-4" />
          {:else}<FileText class="h-4 w-4" />{/if}
        </span>
        <span class="min-w-0 flex-1">
          <span class="block truncate text-sm font-medium text-foreground">{title}</span>
          <span class="mt-1 block truncate text-[11px] text-muted-foreground">
            {item.sourceApp ? `${item.sourceApp} · ` : ''}{time}
            {#if item.contentType === 'files'}
              · {i18n.format(t.fileCount, { n: item.fileCount ?? paths.length })}{/if}
            {#if item.hasHtml}
              · Aa{/if}
          </span>
        </span>
        {#if item.isPinned}<Pin
            class="mt-3 h-3.5 w-3.5 flex-none self-start text-muted-foreground"
          />{/if}
        {#if slotNumber}<span
            class="mt-3 w-3 flex-none self-start text-right text-[10px] tabular-nums text-muted-foreground"
            aria-hidden="true">{slotNumber}</span
          >{/if}
      </button>
    {/if}
  </div>
  <div
    role="gridcell"
    class="row-actions absolute bottom-0.5 right-2 {editing
      ? 'hidden'
      : 'flex'} items-center gap-0.5 rounded-md px-0.5 {selected || editing || multiSelected
      ? 'active'
      : ''}"
  >
    {#each rowActions as action (action.icon)}
      <Button
        variant="ghost"
        size="icon"
        class={action.pressed ? 'text-primary' : 'text-muted-foreground'}
        style="width: 1.75rem; height: 1.75rem"
        title={action.label}
        aria-label={action.label}
        aria-pressed={action.pressed}
        tabindex={selected ? 0 : -1}
        disabled={busy || editing}
        onclick={() => {
          onSelect();
          void action.run();
        }}><action.icon class="h-3.5 w-3.5" /></Button
      >
    {/each}
  </div>
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
  .row-icon {
    background: color-mix(in srgb, var(--foreground) 4%, var(--row-surface));
  }
  .row-actions {
    background: var(--row-surface);
    opacity: 0;
    pointer-events: none;
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
