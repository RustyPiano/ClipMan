<script lang="ts">
  import { onMount } from 'svelte';
  import { clipboardStore } from '$lib/stores/clipboard.svelte';
  import { i18n } from '$lib/i18n';
  import { Loader2, Search, X } from 'lucide-svelte';
  import { listen } from '@tauri-apps/api/event';
  import { SEARCH_INPUT_ID } from '$lib/constants';

  let { activeId, expanded }: { activeId?: string; expanded: boolean } = $props();
  const SEARCH_DEBOUNCE_MS = 30;

  const t = $derived(i18n.t);

  let debounceTimer: ReturnType<typeof setTimeout>;
  let isComposing = false;
  let discardComposition = false;

  function applySearchInput(value: string) {
    if (!value.trim()) {
      clearTimeout(debounceTimer);
      void clipboardStore.clearSearch();
      return;
    }

    clipboardStore.setSearchQuery(value);
    clearTimeout(debounceTimer);
    debounceTimer = setTimeout(() => {
      void clipboardStore.search(value);
    }, SEARCH_DEBOUNCE_MS);
  }

  function handleInput(event: Event) {
    if (discardComposition) {
      return;
    }
    const target = event.target as HTMLInputElement;
    if (isComposing) {
      clearTimeout(debounceTimer);
      clipboardStore.setSearchQuery(target.value, false);
      return;
    }

    applySearchInput(target.value);
  }

  function handleCompositionStart() {
    discardComposition = false;
    isComposing = true;
    clearTimeout(debounceTimer);
  }

  function handleCompositionEnd(event: Event) {
    isComposing = false;
    if (discardComposition) {
      return;
    }
    applySearchInput((event.target as HTMLInputElement).value);
  }

  function clearSearch() {
    applySearchInput('');
    document.getElementById(SEARCH_INPUT_ID)?.focus();
  }

  onMount(() => {
    const unlisteners: (() => void)[] = [];
    let disposed = false;
    for (const subscription of [
      listen('quickbar-hidden', () => {
        discardComposition = isComposing;
        isComposing = false;
        clearTimeout(debounceTimer);
      }),
      listen('quickbar-opened', () => {
        discardComposition = false;
      }),
    ])
      void subscription.then((stop) => (disposed ? stop() : unlisteners.push(stop)));
    return () => {
      disposed = true;
      clearTimeout(debounceTimer);
      unlisteners.forEach((stop) => stop());
    };
  });
</script>

<div class="relative flex w-full items-center">
  <div class="pointer-events-none flex flex-none items-center pl-1 pr-2.5 text-muted-foreground/70">
    {#if clipboardStore.isSearchPending}
      <Loader2 class="h-4 w-4 animate-spin" />
    {:else}
      <Search class="h-4 w-4" />
    {/if}
  </div>

  <input
    id={SEARCH_INPUT_ID}
    type="text"
    role="combobox"
    aria-haspopup="grid"
    aria-label={t.searchPlaceholder}
    aria-controls={expanded ? 'clipboard-results' : undefined}
    aria-expanded={expanded}
    aria-activedescendant={activeId ? `clip-item-${activeId}` : undefined}
    aria-autocomplete="list"
    placeholder={t.searchPlaceholder}
    value={clipboardStore.searchQuery}
    oninput={handleInput}
    oncompositionstart={handleCompositionStart}
    oncompositionend={handleCompositionEnd}
    class="qb-search h-10 w-full pr-7 text-[14px]"
  />

  {#if clipboardStore.searchQuery}
    <button
      type="button"
      class="absolute right-0.5 flex h-6 w-6 items-center justify-center rounded-md text-muted-foreground/60 hover:bg-accent hover:text-foreground"
      onclick={clearSearch}
      title={t.clear}
      aria-label={t.clear}
    >
      <X class="h-3.5 w-3.5" />
    </button>
  {/if}
</div>
