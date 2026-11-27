<script lang="ts">
  import { onDestroy } from 'svelte';
  import type { ClipItem, ClipDetail } from '$lib/types';
  import { clipboardStore } from '$lib/stores/clipboard.svelte';
  import { appIconStore } from '$lib/stores/app-icons.svelte';
  import { toastStore } from '$lib/stores/toast.svelte';
  import { i18n } from '$lib/i18n';
  import { isMac } from '$lib/utils/platform';
  import { SEARCH_INPUT_ID } from '$lib/constants';
  import {
    decodeClipText,
    decodeFilePaths,
    fileBasename,
    fileDirname,
    formatBytes,
    isSingleUrl,
    looksLikeCode,
    looksLikeDirectory,
    parseColorValue,
  } from '$lib/utils/clip-items';
  import Button from './ui/Button.svelte';
  import {
    Check,
    Copy,
    ExternalLink,
    File as FileIcon,
    Folder,
    FolderOpen,
    Pencil,
    Star,
    Trash2,
    X,
  } from 'lucide-svelte';

  const AUTO_LOAD_IMAGE_BYTES = 4 * 1024 * 1024;
  const MAX_EDITABLE_TEXT_BYTES = 1024 * 1024;

  let { item }: { item: ClipItem | undefined } = $props();
  const t = $derived(i18n.t);
  let detail = $state.raw<ClipDetail | null>(null);
  let loading = $state(false);
  let body: HTMLDivElement | undefined = $state();
  let imageDimensions = $state<{ width: number; height: number } | null>(null);
  let editingContent = $state(false);
  let contentDraft = $state('');
  let savingContent = $state(false);
  let copiedFormat = $state<string | null>(null);
  let copiedItem = $state(false);
  let copyTimer: ReturnType<typeof setTimeout>;
  let formatTimer: ReturnType<typeof setTimeout>;
  let generation = 0;

  onDestroy(() => {
    clearTimeout(copyTimer);
    clearTimeout(formatTimer);
  });

  async function load(currentId: string, revision: number) {
    loading = true;
    const full = await clipboardStore.fetchFullClip(currentId);
    if (revision === generation) {
      detail = full;
      loading = false;
    }
  }

  const itemId = $derived(item?.id ?? null);
  const itemType = $derived(item?.contentType ?? null);
  const itemBytes = $derived(item?.contentBytes ?? 0);

  $effect(() => {
    const id = itemId;
    const type = itemType;
    const bytes = itemBytes;
    const revision = ++generation;
    const savedDraft = id ? clipboardStore.contentDrafts[id] : undefined;
    if (id && clipboardStore.editingClipId === id && savedDraft !== undefined) {
      editingContent = true;
      contentDraft = savedDraft;
    } else {
      editingContent = false;
    }
    imageDimensions = null;
    copiedItem = false;
    const cached = id ? (clipboardStore.getCachedFullClip(id) ?? null) : null;
    detail = cached;
    loading = false;
    if (body) body.scrollTop = 0;
    if (!id || !type || cached) return;

    const shouldAutoLoad =
      (type !== 'image' && bytes <= 256 * 1024) ||
      (type === 'image' && bytes <= AUTO_LOAD_IMAGE_BYTES);
    if (!shouldAutoLoad) return;

    const delay = type === 'image' ? 120 : 40;
    const timer = setTimeout(() => {
      void load(id, revision);
    }, delay);
    return () => {
      clearTimeout(timer);
      generation += 1;
    };
  });

  const full = $derived(detail?.id === item?.id ? detail : null);
  const text = $derived(
    full?.text ??
      (item?.contentType === 'text'
        ? decodeClipText(item, t.emptyContent)
        : item
          ? decodeFilePaths(item).join('\n')
          : '')
  );
  const filePaths = $derived(
    item?.contentType === 'files' ? (full?.filePaths ?? decodeFilePaths(item)) : []
  );
  const imageUrl = $derived(full?.imageUrl || item?.content || '');
  const color = $derived(item?.contentType === 'text' ? parseColorValue(text) : null);
  const isUrl = $derived(item?.contentType === 'text' && isSingleUrl(text));
  const isCode = $derived(item?.contentType === 'text' && !color && looksLikeCode(text));
  const appIcon = $derived(item?.sourceApp ? appIconStore.get(item.sourceApp) : null);

  const typeTitle = $derived(
    item?.label ||
      (item?.contentType === 'image'
        ? t.image
        : item?.contentType === 'files'
          ? t.files
          : color
            ? t.colorPreview
            : isUrl
              ? t.filterLink
              : t.text)
  );

  const formattedDate = $derived(
    item
      ? new Date(item.timestamp * 1000).toLocaleString(i18n.locale, {
          dateStyle: 'medium',
          timeStyle: 'short',
        })
      : ''
  );

  const lineCount = $derived(text ? text.split('\n').length : 0);
  const charCount = $derived(full && !full.truncated ? [...full.text].length : [...text].length);
  const canEdit = $derived(
    item?.contentType === 'text' && !full?.truncated && item.contentBytes <= MAX_EDITABLE_TEXT_BYTES
  );

  async function startEditContent() {
    if (!item || item.contentType !== 'text') return;
    if (item.contentBytes > MAX_EDITABLE_TEXT_BYTES || full?.truncated) {
      toastStore.add(t.cannotEditTruncated, 'error');
      return;
    }
    const targetId = item.id;
    let loaded = full;
    if (!loaded) {
      loading = true;
      loaded = await clipboardStore.fetchFullClip(targetId);
      if (item?.id === targetId) loading = false;
    }
    if (!item || item.id !== targetId) return;
    if (!loaded) {
      toastStore.add(t.loadContentFailed, 'error');
      return;
    }
    if (loaded.truncated) {
      detail = loaded;
      toastStore.add(t.cannotEditTruncated, 'error');
      return;
    }
    detail = loaded;
    contentDraft = clipboardStore.contentDrafts[targetId] ?? loaded.text;
    clipboardStore.setContentDraft(targetId, contentDraft);
    clipboardStore.editingClipId = targetId;
    editingContent = true;
  }

  function cancelEditContent() {
    if (item) clipboardStore.clearContentDraft(item.id);
    clipboardStore.editingClipId = null;
    editingContent = false;
    document.getElementById(SEARCH_INPUT_ID)?.focus({ preventScroll: true });
  }

  async function saveContent() {
    if (!item || savingContent || !contentDraft.trim()) return;
    savingContent = true;
    const targetId = item.id;
    const nextContent = contentDraft;
    try {
      await clipboardStore.updateClipContent(targetId, nextContent);
      clipboardStore.clearContentDraft(targetId);
      if (clipboardStore.editingClipId === targetId) {
        clipboardStore.editingClipId = null;
      }
      if (item?.id === targetId) {
        detail = {
          id: targetId,
          contentType: 'text',
          text: nextContent,
          filePaths: [],
          imageUrl: null,
          truncated: false,
        };
        editingContent = false;
      }
      document.getElementById(SEARCH_INPUT_ID)?.focus({ preventScroll: true });
    } catch (error) {
      toastStore.add(String(error), 'error');
    } finally {
      savingContent = false;
    }
  }

  async function handleCopy() {
    if (!item) return;
    if (await clipboardStore.copyToClipboard(item)) {
      copiedItem = true;
      clearTimeout(copyTimer);
      copyTimer = setTimeout(() => (copiedItem = false), 1500);
    }
  }

  async function copyColorFormat(key: string, value: string) {
    if (await clipboardStore.copyPlainText(value)) {
      copiedFormat = key;
      toastStore.add(`${t.copied}: ${value}`, 'success');
      clearTimeout(formatTimer);
      formatTimer = setTimeout(() => {
        if (copiedFormat === key) copiedFormat = null;
      }, 1500);
    }
  }
</script>

{#if item}
  <div class="flex h-full flex-col text-foreground">
    <header
      class="flex flex-none items-center justify-between gap-2 border-b border-border px-4 py-2.5"
    >
      <div class="flex min-w-0 items-center gap-1.5">
        {#if item.isPinned}
          <Star class="qb-pin-text h-3 w-3 flex-none fill-current" aria-hidden="true" />
        {/if}
        <span class="truncate text-[12px] font-medium text-muted-foreground">{typeTitle}</span>
      </div>
      <div class="flex flex-none items-center gap-0.5">
        {#if isUrl}
          <button
            type="button"
            class="preview-icon-btn"
            title={t.openLink}
            aria-label={t.openLink}
            onmousedown={(event) => event.preventDefault()}
            onclick={() => void clipboardStore.openUrl(text)}
          >
            <ExternalLink class="h-3.5 w-3.5" />
          </button>
        {/if}
        <button
          type="button"
          class="preview-icon-btn {item.isPinned ? 'is-pinned' : ''}"
          title={item.isPinned ? t.unpin : t.pin}
          aria-label={item.isPinned ? t.unpin : t.pin}
          aria-pressed={item.isPinned}
          onmousedown={(event) => event.preventDefault()}
          onclick={() => void clipboardStore.togglePin(item.id)}
        >
          <Star class="h-3.5 w-3.5 {item.isPinned ? 'fill-current' : ''}" />
        </button>
        {#if item.contentType === 'text' && !editingContent}
          <button
            type="button"
            class="preview-icon-btn"
            title={canEdit ? t.editContent : t.cannotEditTruncated}
            aria-label={t.editContent}
            disabled={!canEdit}
            onmousedown={(event) => event.preventDefault()}
            onclick={() => void startEditContent()}
          >
            <Pencil class="h-3.5 w-3.5" />
          </button>
        {/if}
        <button
          type="button"
          class="preview-icon-btn is-danger"
          title={t.delete}
          aria-label={t.delete}
          onmousedown={(event) => event.preventDefault()}
          onclick={() => void clipboardStore.deleteItem(item.id)}
        >
          <Trash2 class="h-3.5 w-3.5" />
        </button>
      </div>
    </header>

    <div bind:this={body} class="min-h-0 flex-1 overflow-auto p-4" aria-busy={loading}>
      {#if editingContent}
        <div data-row-editor class="flex h-full flex-col gap-2">
          <textarea
            aria-label={t.editContent}
            class="min-h-[180px] flex-1 resize-none rounded-md border border-input bg-background p-3 font-mono text-xs leading-relaxed text-foreground focus-visible:outline-none"
            bind:value={contentDraft}
            disabled={savingContent}
            oninput={() => {
              if (item) clipboardStore.setContentDraft(item.id, contentDraft);
            }}
            {@attach (el) => el.focus()}
            onkeydown={(event) => {
              if (event.isComposing || event.keyCode === 229) return;
              if (event.key === 'Escape') {
                event.preventDefault();
                cancelEditContent();
              } else if ((event.metaKey || event.ctrlKey) && event.key === 'Enter') {
                event.preventDefault();
                void saveContent();
              }
            }}></textarea>
          <div class="flex justify-end gap-1.5">
            <Button variant="ghost" size="sm" disabled={savingContent} onclick={cancelEditContent}>
              <X class="mr-1 h-3.5 w-3.5" />{t.cancel}
            </Button>
            <Button
              size="sm"
              disabled={savingContent || !contentDraft.trim()}
              onclick={() => void saveContent()}
            >
              <Check class="mr-1 h-3.5 w-3.5" />{t.save}
            </Button>
          </div>
        </div>
      {:else if item.contentType === 'image'}
        <div
          class="img-checkerboard flex min-h-[160px] items-center justify-center overflow-hidden rounded-lg border border-border p-3"
        >
          <img
            src={imageUrl}
            alt={item.label || t.image}
            class="max-h-[280px] max-w-full rounded object-contain"
            onload={(event) => {
              const img = event.currentTarget as HTMLImageElement;
              if (img.naturalWidth && img.naturalHeight) {
                imageDimensions = { width: img.naturalWidth, height: img.naturalHeight };
              }
            }}
          />
        </div>
      {:else if color}
        <div class="flex flex-col gap-4">
          <div class="img-checkerboard h-24 w-full overflow-hidden rounded-lg border border-border">
            <div class="h-full w-full" style:background-color={color.css}></div>
          </div>
          <div class="divide-y divide-border border-y border-border">
            {#each [['HEX', color.hex], ['RGB', color.rgb], ['HSL', color.hsl]] as [label, value] (label)}
              <button
                type="button"
                class="group flex w-full items-center justify-between py-2 text-left hover:text-foreground"
                onmousedown={(event) => event.preventDefault()}
                onclick={() => void copyColorFormat(label, value)}
              >
                <span class="text-[11px] text-muted-foreground">{label}</span>
                <span class="flex items-center gap-2 font-mono text-xs text-foreground">
                  {value}
                  {#if copiedFormat === label}
                    <Check class="qb-success-text h-3 w-3" />
                  {:else}
                    <Copy
                      class="h-3 w-3 text-muted-foreground/40 transition-colors group-hover:text-foreground"
                    />
                  {/if}
                </span>
              </button>
            {/each}
          </div>
        </div>
      {:else if item.contentType === 'files'}
        <ul class="divide-y divide-border">
          {#each filePaths as path, index (`${path}-${index}`)}
            {@const isDir = looksLikeDirectory(path)}
            <li class="group flex items-center gap-2.5 py-2 first:pt-0">
              {#if isDir}
                <Folder class="h-4 w-4 flex-none text-muted-foreground" />
              {:else}
                <FileIcon class="h-4 w-4 flex-none text-muted-foreground" />
              {/if}
              <div class="min-w-0 flex-1">
                <div class="truncate text-[13px] font-medium text-foreground">
                  {fileBasename(path) || path}
                </div>
                {#if fileDirname(path)}
                  <div class="truncate font-mono text-[11px] text-muted-foreground/80">
                    {fileDirname(path)}
                  </div>
                {/if}
              </div>
              <button
                type="button"
                class="preview-icon-btn flex-none opacity-60 group-hover:opacity-100"
                title={isMac ? t.revealInFinder : t.revealInFolder}
                aria-label={isMac ? t.revealInFinder : t.revealInFolder}
                onmousedown={(event) => event.preventDefault()}
                onclick={() => void clipboardStore.revealInFolder(path)}
              >
                <FolderOpen class="h-3.5 w-3.5" />
              </button>
            </li>
          {/each}
        </ul>
      {:else}
        <pre
          class="m-0 whitespace-pre-wrap [overflow-wrap:anywhere] selection:bg-foreground/15 {isCode
            ? 'font-mono text-[12px] leading-[1.6]'
            : isUrl
              ? 'qb-link-text font-sans text-[13px] leading-[1.6]'
              : 'font-sans text-[13px] leading-[1.65]'}">{text}</pre>
      {/if}

      {#if !editingContent && (!full || full.truncated)}
        <div class="mt-3 flex flex-col gap-2 text-[11px] text-muted-foreground">
          {#if !full && !loading}
            <Button
              variant="outline"
              size="sm"
              disabled={loading}
              onclick={() => item && load(item.id, generation)}>{t.fullPreview}</Button
            >
          {/if}
          {#if full?.truncated}
            <span>{t.previewLimited}</span>
          {:else if !full && !loading}
            <span>{t.previewTruncated}</span>
          {/if}
        </div>
      {/if}
    </div>

    <div class="flex-none border-t border-border px-4 py-2.5 text-[11px]">
      <dl class="grid grid-cols-[auto_1fr] gap-x-3 gap-y-1">
        {#if item.sourceApp}
          <dt class="text-muted-foreground">{t.infoSource}</dt>
          <dd class="flex min-w-0 items-center justify-end gap-1.5 text-foreground/90">
            {#if appIcon}<img
                src={appIcon}
                alt=""
                class="h-3.5 w-3.5 flex-none rounded-[3px]"
              />{/if}
            <span class="truncate">{item.sourceApp}</span>
          </dd>
        {/if}
        <dt class="text-muted-foreground">{t.infoTime}</dt>
        <dd class="truncate text-right tabular-nums text-foreground/90">{formattedDate}</dd>
        {#if item.contentType === 'text'}
          <dt class="text-muted-foreground">{t.infoChars}</dt>
          <dd class="text-right tabular-nums text-foreground/90">
            {i18n.format(t.characterCount, { n: charCount.toLocaleString() })} · {i18n.format(
              t.infoLines,
              { n: lineCount }
            )}
          </dd>
          <dt class="text-muted-foreground">{t.infoFormat}</dt>
          <dd class="text-right text-foreground/90">
            {item.hasHtml ? t.richTextFormat : t.plainTextFormat}
          </dd>
        {:else if item.contentType === 'image'}
          <dt class="text-muted-foreground">{t.infoSize}</dt>
          <dd class="text-right tabular-nums text-foreground/90">
            {#if imageDimensions}
              {imageDimensions.width} × {imageDimensions.height} ·
            {/if}
            {formatBytes(item.contentBytes)}
          </dd>
        {:else}
          <dt class="text-muted-foreground">{t.files}</dt>
          <dd class="text-right tabular-nums text-foreground/90">
            {i18n.format(t.fileCount, { n: item.fileCount })}
          </dd>
        {/if}
      </dl>
    </div>

    <footer
      class="flex flex-none items-center justify-between gap-1.5 border-t border-border px-3 py-2"
    >
      <div class="flex items-center gap-1">
        <button
          type="button"
          class="preview-ghost-btn gap-1 {copiedItem ? 'qb-success-text' : ''}"
          disabled={clipboardStore.isUsing}
          onmousedown={(event) => event.preventDefault()}
          onclick={() => void handleCopy()}
        >
          {#if copiedItem}
            <Check class="h-3.5 w-3.5" />
            {t.copied}
          {:else}
            {t.copy}
          {/if}
        </button>
        {#if item.hasHtml}
          <button
            type="button"
            class="preview-ghost-btn"
            disabled={clipboardStore.isUsing}
            onmousedown={(event) => event.preventDefault()}
            onclick={() => void clipboardStore.useClip(item, 'default', { plain: true })}
          >
            {t.pastePlain}
          </button>
        {/if}
      </div>
      <button
        type="button"
        class="inline-flex h-7 items-center gap-1.5 rounded-md bg-primary px-2.5 text-xs font-medium text-primary-foreground hover:opacity-90 disabled:opacity-45"
        disabled={clipboardStore.isUsing}
        onmousedown={(event) => event.preventDefault()}
        onclick={() => void clipboardStore.useClip(item)}
      >
        {clipboardStore.autoPaste ? t.paste : t.copy}
        <span class="text-[10px] opacity-65">↵</span>
      </button>
    </footer>
  </div>
{/if}

<style>
  .preview-icon-btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 1.625rem;
    height: 1.625rem;
    border-radius: 5px;
    color: var(--muted-foreground);
  }
  .preview-icon-btn:hover:not(:disabled) {
    background: var(--qb-hover);
    color: var(--foreground);
  }
  .preview-icon-btn:disabled {
    opacity: 0.35;
    pointer-events: none;
  }
  .preview-icon-btn.is-pinned {
    color: var(--pin-accent);
  }
  .preview-icon-btn.is-pinned:hover {
    color: var(--pin-accent);
  }
  .preview-icon-btn.is-danger:hover {
    color: var(--destructive);
  }
  .preview-ghost-btn {
    display: inline-flex;
    align-items: center;
    height: 1.75rem;
    padding: 0 0.5rem;
    border-radius: 5px;
    font-size: 12px;
    color: var(--muted-foreground);
  }
  .preview-ghost-btn:hover:not(:disabled) {
    background: var(--qb-hover);
    color: var(--foreground);
  }
  .preview-ghost-btn:disabled {
    opacity: 0.4;
  }
</style>
