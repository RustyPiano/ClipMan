<script lang="ts">
  import {
    Check,
    Copy,
    FileText,
    Folder,
    Image as ImageIcon,
    Link2,
    Pencil,
    Search,
    Star,
    X,
  } from 'lucide-svelte';
  import {
    COPY,
    createInitialClips,
    type ClipCategory,
    type Locale,
    type SandboxClip,
  } from '../lib/i18n';

  interface Props {
    locale: Locale;
  }

  let { locale }: Props = $props();

  let clips = $derived(createInitialClips(locale));
  let query = $state('');
  let viewTab = $state<'history' | 'favorites'>('history');
  let category = $state<ClipCategory>('all');
  let selectedId = $state('c1');
  let editingId = $derived.by<string | null>(() => {
    void locale;
    return null;
  });
  let draftText = $state('');
  let toastText = $state<string | null>(null);
  let poppedId = $state<string | null>(null);
  let focusedPulse = $state(false);
  let sandboxEl = $state<HTMLDivElement | null>(null);
  let searchInputEl = $state<HTMLInputElement | null>(null);
  let toastTimer: ReturnType<typeof setTimeout> | null = null;

  let copy = $derived(COPY[locale]);

  let filteredClips = $derived.by(() => {
    const q = query.trim().toLowerCase();
    return clips.filter((clip) => {
      if (viewTab === 'favorites' && !clip.favorite) return false;
      if (category !== 'all') {
        if (category === 'text' && clip.kind !== 'text' && clip.kind !== 'color') return false;
        if (category !== 'text' && clip.kind !== category) return false;
      }
      if (!q) return true;
      return (
        clip.title.toLowerCase().includes(q) ||
        clip.preview.toLowerCase().includes(q) ||
        clip.content.toLowerCase().includes(q) ||
        clip.sourceApp.toLowerCase().includes(q)
      );
    });
  });

  let activeClip = $derived.by(() => {
    const found = filteredClips.find((item) => item.id === selectedId);
    return found ?? filteredClips[0] ?? null;
  });

  function showToast(message: string): void {
    toastText = message;
    if (toastTimer) clearTimeout(toastTimer);
    toastTimer = setTimeout(() => {
      toastText = null;
    }, 2400);
  }

  function focusSearch(): void {
    searchInputEl?.focus();
    searchInputEl?.select();
    focusedPulse = true;
    setTimeout(() => {
      focusedPulse = false;
    }, 450);
  }

  function highlightParts(text: string, term: string): Array<{ text: string; hit: boolean }> {
    const trimmed = term.trim();
    if (!trimmed) return [{ text, hit: false }];
    const lowerText = text.toLowerCase();
    const lowerTerm = trimmed.toLowerCase();
    const parts: Array<{ text: string; hit: boolean }> = [];
    let cursor = 0;
    let index = lowerText.indexOf(lowerTerm, cursor);
    while (index !== -1) {
      if (index > cursor) {
        parts.push({ text: text.slice(cursor, index), hit: false });
      }
      parts.push({ text: text.slice(index, index + trimmed.length), hit: true });
      cursor = index + trimmed.length;
      index = lowerText.indexOf(lowerTerm, cursor);
    }
    if (cursor < text.length) {
      parts.push({ text: text.slice(cursor), hit: false });
    }
    return parts;
  }

  async function copyText(value: string, label: string): Promise<void> {
    if (!navigator.clipboard?.writeText) {
      showToast(copy.sandboxCopyFailedToast(value));
      return;
    }
    try {
      await navigator.clipboard.writeText(value);
      showToast(copy.sandboxCopiedToast(label));
    } catch {
      showToast(copy.sandboxCopyFailedToast(value));
    }
  }

  function triggerPaste(clip: SandboxClip, plain: boolean): void {
    showToast(copy.sandboxPastedToast(clip.title, plain));
  }

  function revealFile(path: string): void {
    const name = path.split('/').pop() ?? path;
    showToast(copy.sandboxRevealToast(name));
  }

  function toggleFavorite(clip: SandboxClip): void {
    clips = clips.map((item) =>
      item.id === clip.id ? { ...item, favorite: !item.favorite } : item
    );
    poppedId = clip.id;
    setTimeout(() => {
      if (poppedId === clip.id) poppedId = null;
    }, 280);
  }

  function startEditing(clip: SandboxClip): void {
    editingId = clip.id;
    draftText = clip.content;
  }

  function saveEdit(clip: SandboxClip): void {
    const next = draftText.trim();
    if (next) {
      const firstLine = next.split('\n')[0] ?? next;
      clips = clips.map((item) =>
        item.id === clip.id ? { ...item, content: next, preview: firstLine } : item
      );
      showToast(copy.sandboxSavedToast);
    }
    editingId = null;
  }

  function moveSelection(delta: number): void {
    if (filteredClips.length === 0) return;
    const currentIndex = filteredClips.findIndex((item) => item.id === activeClip?.id);
    const base = currentIndex === -1 ? 0 : currentIndex;
    const nextIndex = (base + delta + filteredClips.length) % filteredClips.length;
    selectedId = filteredClips[nextIndex].id;
    editingId = null;
  }

  function handleWindowKeydown(event: KeyboardEvent): void {
    if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === 'k') {
      event.preventDefault();
      searchInputEl?.scrollIntoView({ behavior: 'smooth', block: 'center' });
      focusSearch();
      return;
    }

    const active = document.activeElement as HTMLElement | null;
    if (!sandboxEl || !active || !sandboxEl.contains(active) || active.tagName === 'TEXTAREA') {
      return;
    }

    if (event.isComposing || event.keyCode === 229) {
      return;
    }

    if (event.key === 'ArrowDown') {
      event.preventDefault();
      moveSelection(1);
      return;
    }
    if (event.key === 'ArrowUp') {
      event.preventDefault();
      moveSelection(-1);
      return;
    }
    if (event.key === 'Enter' && active.tagName === 'INPUT') {
      event.preventDefault();
      if (activeClip) {
        triggerPaste(activeClip, event.altKey);
      }
    }
  }

  let categories = $derived<Array<{ id: ClipCategory; label: string }>>([
    { id: 'all', label: copy.sandboxCatAll },
    { id: 'text', label: copy.sandboxCatText },
    { id: 'image', label: copy.sandboxCatImage },
    { id: 'file', label: copy.sandboxCatFile },
    { id: 'link', label: copy.sandboxCatLink },
  ]);
</script>

<svelte:window onkeydown={handleWindowKeydown} />

<div class="mx-auto w-full max-w-[960px] rounded-[28px] bg-[var(--stage-bg)] p-3 sm:rounded-[36px] sm:p-8">
  <div
    bind:this={sandboxEl}
    role="region"
    aria-label="ClipLeaf QuickBar interactive preview"
    class={[
      'qb-panel relative overflow-hidden rounded-[20px] text-left transition-shadow duration-300',
      focusedPulse ? 'ring-2 ring-[var(--accent)]/40' : '',
    ]}
  >
    <div
      class="flex flex-wrap items-center gap-3 border-b border-[var(--hairline)] px-5 py-3.5 sm:flex-nowrap"
    >
      <Search class="h-4 w-4 shrink-0 text-[var(--ink-faint)]" strokeWidth={1.6} aria-hidden="true" />
      <label for="cl-sandbox-search" class="sr-only">{copy.sandboxSearchPlaceholder}</label>
      <input
        id="cl-sandbox-search"
        bind:this={searchInputEl}
        bind:value={query}
        type="text"
        autocomplete="off"
        spellcheck="false"
        placeholder={copy.sandboxSearchPlaceholder}
        class="min-w-[160px] flex-1 bg-transparent text-sm text-[var(--ink)] placeholder:text-[var(--ink-faint)] focus:outline-none"
      />
      {#if query}
        <button
          type="button"
          onclick={() => {
            query = '';
            searchInputEl?.focus();
          }}
          aria-label="Clear search"
          class="flex h-6 w-6 shrink-0 items-center justify-center rounded-md text-[var(--ink-faint)] transition hover:bg-[var(--surface-subtle)] hover:text-[var(--ink)]"
        >
          <X class="h-3.5 w-3.5" strokeWidth={1.6} aria-hidden="true" />
        </button>
      {/if}
      <div
        class="flex items-center gap-0.5 rounded-lg bg-[var(--surface-subtle)] p-0.5 text-xs"
        role="group"
        aria-label="ClipLeaf view"
      >
        <button
          type="button"
          onclick={() => {
            viewTab = 'history';
            editingId = null;
          }}
          aria-pressed={viewTab === 'history'}
          class={[
            'rounded-md px-3 py-1 font-medium transition',
            viewTab === 'history'
              ? 'bg-[var(--surface)] text-[var(--ink)]'
              : 'text-[var(--ink-muted)] hover:text-[var(--ink)]',
          ]}
        >
          {copy.sandboxTabHistory}
        </button>
        <button
          type="button"
          onclick={() => {
            viewTab = 'favorites';
            editingId = null;
          }}
          aria-pressed={viewTab === 'favorites'}
          class={[
            'rounded-md px-3 py-1 font-medium transition',
            viewTab === 'favorites'
              ? 'bg-[var(--surface)] text-[var(--ink)]'
              : 'text-[var(--ink-muted)] hover:text-[var(--ink)]',
          ]}
        >
          {copy.sandboxTabFavorites}
        </button>
      </div>
    </div>

    <div class="flex items-center justify-between gap-2 border-b border-[var(--hairline)] px-5 py-2">
      <div class="flex flex-wrap items-center gap-1" role="group" aria-label="Category filter">
        {#each categories as cat (cat.id)}
          <button
            type="button"
            onclick={() => {
              category = cat.id;
              editingId = null;
            }}
            aria-pressed={category === cat.id}
            class={[
              'relative rounded-md px-2.5 py-1 text-xs font-medium transition',
              category === cat.id
                ? 'text-[var(--ink)]'
                : 'text-[var(--ink-muted)] hover:text-[var(--ink)]',
            ]}
          >
            {cat.label}
            {#if category === cat.id}
              <span class="absolute right-2 -bottom-2 left-2 h-[1.5px] bg-[var(--ink)]"></span>
            {/if}
          </button>
        {/each}
      </div>
      <button
        type="button"
        onclick={focusSearch}
        title={copy.sandboxShortcutTip}
        aria-label={copy.sandboxShortcutTip}
        class="hidden items-center gap-1 text-[11px] text-[var(--ink-faint)] transition hover:opacity-75 sm:flex"
      >
        <span class="kbd-cap">⌘</span>
        <span class="kbd-cap">⇧</span>
        <span class="kbd-cap">V</span>
      </button>
    </div>

    <div class="grid min-h-[384px] grid-cols-1 md:grid-cols-[minmax(0,1fr)_minmax(0,1.08fr)]">
      <div class="max-h-[216px] overflow-y-auto py-2 md:max-h-[404px]">
        {#if filteredClips.length === 0}
          <div class="flex h-full min-h-[180px] items-center justify-center px-6 text-sm text-[var(--ink-muted)]">
            {copy.sandboxEmpty}
          </div>
        {:else}
          {#each filteredClips as clip (clip.id)}
            {@const isSelected = activeClip?.id === clip.id}
            <div
              class={[
                'group mx-2 flex items-center gap-2 rounded-xl px-3 py-2.5 transition',
                isSelected
                  ? 'bg-[var(--surface-subtle)]'
                  : 'hover:bg-[var(--surface-subtle)]/55',
              ]}
            >
              <button
                type="button"
                onclick={() => {
                  selectedId = clip.id;
                  editingId = null;
                }}
                ondblclick={() => triggerPaste(clip, false)}
                class="flex min-w-0 flex-1 items-center gap-3 text-left focus:outline-none"
              >
                <span class="flex h-6 w-6 shrink-0 items-center justify-center text-[var(--ink-muted)]">
                  {#if clip.kind === 'color' && clip.colorHex}
                    <span
                      class="h-4 w-4 rounded-full ring-1 ring-black/10"
                      style:background={clip.colorHex}
                    ></span>
                  {:else if clip.kind === 'image'}
                    <ImageIcon class="h-4 w-4" strokeWidth={1.6} aria-hidden="true" />
                  {:else if clip.kind === 'file'}
                    <Folder class="h-4 w-4" strokeWidth={1.6} aria-hidden="true" />
                  {:else if clip.kind === 'link'}
                    <Link2 class="h-4 w-4" strokeWidth={1.6} aria-hidden="true" />
                  {:else}
                    <FileText class="h-4 w-4" strokeWidth={1.6} aria-hidden="true" />
                  {/if}
                </span>

                <span class="min-w-0 flex-1">
                  <span class="flex items-center justify-between gap-2">
                    <span class="truncate text-[13px] font-medium text-[var(--ink)]">
                      {#each highlightParts(clip.title, query) as part, idx (`t-${idx}`)}
                        {#if part.hit}
                          <mark class="qb-mark">{part.text}</mark>
                        {:else}
                          {part.text}
                        {/if}
                      {/each}
                    </span>
                    <span class="shrink-0 font-mono text-[11px] text-[var(--ink-faint)]">
                      {clip.timestamp}
                    </span>
                  </span>
                  <span class="mt-0.5 block truncate text-xs text-[var(--ink-muted)]">
                    {#each highlightParts(clip.preview, query) as part, idx (`p-${idx}`)}
                      {#if part.hit}
                        <mark class="qb-mark">{part.text}</mark>
                      {:else}
                        {part.text}
                      {/if}
                    {/each}
                  </span>
                </span>
              </button>

              <button
                type="button"
                onclick={() => toggleFavorite(clip)}
                aria-label={clip.favorite ? 'Unstar' : 'Star'}
                aria-pressed={clip.favorite}
                class={[
                  'flex h-7 w-7 shrink-0 items-center justify-center rounded-lg transition',
                  clip.favorite
                    ? 'text-[var(--accent)] opacity-100'
                    : 'text-[var(--ink-faint)] opacity-0 hover:text-[var(--ink)] group-focus-within:opacity-100 group-hover:opacity-100',
                  poppedId === clip.id ? 'animate-leaf-pop' : '',
                ]}
              >
                <Star
                  class="h-3.5 w-3.5"
                  strokeWidth={1.6}
                  fill={clip.favorite ? 'currentColor' : 'none'}
                  aria-hidden="true"
                />
              </button>
            </div>
          {/each}
        {/if}
      </div>

      <div
        class="group/preview flex flex-col justify-between border-t border-[var(--hairline)] bg-[var(--stage-bg)]/45 p-5 md:border-t-0 md:border-l"
      >
        {#if activeClip}
          <div class="space-y-4">
            <div class="flex items-center justify-between gap-2 text-xs text-[var(--ink-faint)]">
              <span class="font-medium text-[var(--ink-muted)]">{activeClip.sourceApp}</span>
              <span class="font-mono text-[11px]">{activeClip.timestamp}</span>
            </div>

            {#if activeClip.kind === 'color' && activeClip.colorHex}
              <div class="space-y-3">
                <div
                  class="flex h-32 items-end rounded-2xl p-4 text-white"
                  style:background={activeClip.colorHex}
                >
                  <div>
                    <div class="font-mono text-xl font-semibold tracking-tight">
                      {activeClip.colorHex}
                    </div>
                    <div class="mt-0.5 text-xs text-white/80">{activeClip.preview}</div>
                  </div>
                </div>

                <div class="divide-y divide-[var(--hairline)]">
                  {#each [
                    { k: 'HEX', v: activeClip.colorHex },
                    { k: 'RGB', v: activeClip.colorRgb ?? '' },
                    { k: 'HSL', v: activeClip.colorHsl ?? '' },
                  ] as swatch (swatch.k)}
                    <button
                      type="button"
                      onclick={() => copyText(swatch.v, `${swatch.k} (${swatch.v})`)}
                      class="group/row flex w-full items-center justify-between gap-3 py-2.5 text-left font-mono text-xs text-[var(--ink)] transition hover:text-[var(--accent)]"
                    >
                      <span class="flex items-center gap-3 truncate">
                        <span class="w-8 shrink-0 text-[11px] text-[var(--ink-faint)]">
                          {swatch.k}
                        </span>
                        <span class="truncate">{swatch.v}</span>
                      </span>
                      <Copy
                        class="h-3.5 w-3.5 shrink-0 text-[var(--ink-faint)] opacity-0 transition group-focus-visible/row:opacity-100 group-hover/row:opacity-100"
                        strokeWidth={1.6}
                        aria-hidden="true"
                      />
                    </button>
                  {/each}
                </div>
              </div>
            {:else if activeClip.kind === 'image'}
              <div class="space-y-3">
                <div class="img-checkerboard flex h-48 items-center justify-center rounded-2xl p-4">
                  <img
                    src="/icon.png"
                    alt="ClipLeaf icon preview"
                    width="124"
                    height="124"
                    class="h-[124px] w-[124px] object-contain drop-shadow-md select-none"
                  />
                </div>
                {#if activeClip.imageMeta}
                  <div class="flex items-center justify-between font-mono text-xs text-[var(--ink-muted)]">
                    <span>{activeClip.title}</span>
                    <span>
                      {activeClip.imageMeta.width} × {activeClip.imageMeta.height} · {activeClip.imageMeta.size}
                    </span>
                  </div>
                {/if}
              </div>
            {:else if activeClip.kind === 'file' && activeClip.files}
              <div class="divide-y divide-[var(--hairline)]">
                {#each activeClip.files as path (path)}
                  <button
                    type="button"
                    onclick={() => revealFile(path)}
                    class="flex w-full items-center gap-2.5 py-2.5 text-left font-mono text-xs text-[var(--ink)] transition hover:text-[var(--accent)]"
                  >
                    <Folder class="h-4 w-4 shrink-0 text-[var(--ink-faint)]" strokeWidth={1.6} aria-hidden="true" />
                    <span class="truncate">{path}</span>
                  </button>
                {/each}
              </div>
            {:else if activeClip.kind === 'link'}
              <div class="space-y-3 py-2">
                <div class="flex items-center justify-between gap-2">
                  <span class="truncate font-mono text-sm font-medium text-[var(--ink)]">
                    {activeClip.title}
                  </span>
                  <button
                    type="button"
                    onclick={() => copyText(activeClip.content, activeClip.title)}
                    class="inline-flex shrink-0 items-center gap-1 rounded-lg bg-[var(--surface)] px-2.5 py-1 text-xs text-[var(--ink-muted)] transition hover:text-[var(--ink)]"
                  >
                    <Copy class="h-3 w-3" strokeWidth={1.6} aria-hidden="true" />
                    <span>{copy.sandboxCopyLink}</span>
                  </button>
                </div>
                <p class="text-sm leading-relaxed text-[var(--ink-muted)]">{activeClip.preview}</p>
              </div>
            {:else if editingId === activeClip.id}
              <div class="space-y-3">
                <label for="cl-inline-edit" class="sr-only">{copy.sandboxEditInline}</label>
                <textarea
                  id="cl-inline-edit"
                  bind:value={draftText}
                  rows="6"
                  class="w-full rounded-xl bg-[var(--surface)] p-3.5 font-mono text-xs leading-relaxed text-[var(--ink)] ring-1 ring-[var(--hairline-strong)] focus:outline-none"
                ></textarea>
                <div class="flex justify-end">
                  <button
                    type="button"
                    onclick={() => saveEdit(activeClip)}
                    class="inline-flex items-center gap-1.5 rounded-lg bg-[var(--primary)] px-3.5 py-1.5 text-xs font-medium text-[var(--primary-fg)] transition hover:opacity-90"
                  >
                    <Check class="h-3.5 w-3.5" strokeWidth={1.8} aria-hidden="true" />
                    {copy.sandboxSaveEdit}
                  </button>
                </div>
              </div>
            {:else}
              <pre
                class="max-h-52 overflow-auto font-mono text-xs leading-relaxed whitespace-pre-wrap text-[var(--ink)]"
              >{activeClip.content}</pre>
            {/if}
          </div>

          <div class="mt-6 flex items-center justify-between gap-2 pt-3">
            <div class="flex items-center gap-2">
              {#if activeClip.kind === 'text' && editingId !== activeClip.id}
                <button
                  type="button"
                  onclick={() => startEditing(activeClip)}
                  class="inline-flex items-center gap-1 rounded-lg px-2.5 py-1.5 text-xs font-medium text-[var(--ink-muted)] transition hover:bg-[var(--surface)] hover:text-[var(--ink)]"
                >
                  <Pencil class="h-3.5 w-3.5" strokeWidth={1.6} aria-hidden="true" />
                  <span>{copy.sandboxEditInline}</span>
                </button>
              {/if}
              <button
                type="button"
                onclick={() => triggerPaste(activeClip, true)}
                class="inline-flex items-center gap-1.5 rounded-lg px-2.5 py-1.5 text-xs font-medium text-[var(--ink-muted)] transition hover:bg-[var(--surface)] hover:text-[var(--ink)]"
              >
                <span>{copy.sandboxPastePlain}</span>
                <span class="kbd-cap">⌥↵</span>
              </button>
            </div>

            <button
              type="button"
              onclick={() => triggerPaste(activeClip, false)}
              class="inline-flex items-center gap-1.5 rounded-lg bg-[var(--primary)] px-3.5 py-1.5 text-xs font-medium text-[var(--primary-fg)] transition hover:opacity-90"
            >
              <span>{copy.sandboxPasteDefault}</span>
              <span class="font-mono text-[10px] opacity-70">↵</span>
            </button>
          </div>
        {/if}
      </div>
    </div>

    {#if toastText}
      <div
        role="status"
        aria-live="polite"
        class="pointer-events-none absolute bottom-4 left-1/2 -translate-x-1/2 rounded-full bg-[var(--toast-bg)] px-4 py-1.5 text-xs font-medium text-[var(--toast-fg)] shadow-lg"
      >
        {toastText}
      </div>
    {/if}
  </div>
</div>
