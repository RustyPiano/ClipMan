<script lang="ts">
  import { onDestroy, onMount } from 'svelte';
  import { clipboardStore } from '$lib/stores/clipboard.svelte';
  import { i18n } from '$lib/i18n';
  import Input from '$lib/components/ui/Input.svelte';
  import Button from '$lib/components/ui/Button.svelte';
  import { Loader2, Search, X } from 'lucide-svelte';
  import { listen } from '@tauri-apps/api/event';
  import { hasTauriRuntime } from '$lib/utils/tauri';

  const SEARCH_DEBOUNCE_MS = 120;
  const SEARCH_INPUT_ID = 'quickbar-search';

  const t = $derived(i18n.t);

  let debounceTimer: ReturnType<typeof setTimeout>;
  let isComposing = false;
  let discardComposition = false;
  let unlistenHidden: (() => void) | undefined;
  let unlistenOpened: (() => void) | undefined;
  let isDestroyed = false;

  function runSearch(query: string) {
    clearTimeout(debounceTimer);
    debounceTimer = setTimeout(() => {
      void clipboardStore.search(query);
    }, SEARCH_DEBOUNCE_MS);
  }

  function applySearchInput(value: string) {
    if (!value.trim()) {
      clearTimeout(debounceTimer);
      void clipboardStore.clearSearch({ showLoading: false });
      return;
    }

    clipboardStore.setSearchQuery(value);
    runSearch(value);
  }

  function handleInput(event: Event) {
    if (discardComposition) {
      return;
    }
    const target = event.target as HTMLInputElement;
    if (isComposing) {
      clearTimeout(debounceTimer);
      clipboardStore.setSearchDraft(target.value);
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
    clearTimeout(debounceTimer);
    void clipboardStore.clearSearch({ showLoading: false });

    const input = document.getElementById(SEARCH_INPUT_ID);
    if (input instanceof HTMLInputElement) {
      input.focus();
    }
  }

  onMount(async () => {
    const input = document.getElementById(SEARCH_INPUT_ID);
    if (input instanceof HTMLInputElement) {
      input.focus();
    }

    if (hasTauriRuntime()) {
      const uHidden = await listen('quickbar-hidden', () => {
        const wasComposing = isComposing;
        isComposing = false;
        discardComposition = wasComposing;
        clearTimeout(debounceTimer);
      });
      if (isDestroyed) {
        uHidden();
      } else {
        unlistenHidden = uHidden;
      }

      const uOpened = await listen('quickbar-opened', () => {
        discardComposition = false;
      });
      if (isDestroyed) {
        uOpened();
      } else {
        unlistenOpened = uOpened;
      }
    }
  });

  onDestroy(() => {
    isDestroyed = true;
    clearTimeout(debounceTimer);
    unlistenHidden?.();
    unlistenOpened?.();
  });


</script>

<div class="relative w-full">
  <div
    class="pointer-events-none absolute left-3 top-1/2 -translate-y-1/2 text-muted-foreground/40 transition-colors"
  >
    {#if clipboardStore.isSearchPending}
      <Loader2 class="h-4 w-4 animate-spin stroke-[2]" />
    {:else}
      <Search class="h-4 w-4 stroke-[2]" />
    {/if}
  </div>

  <Input
    id={SEARCH_INPUT_ID}
    type="text"
    placeholder={t.searchPlaceholder}
    value={clipboardStore.searchQuery}
    oninput={handleInput}
    oncompositionstart={handleCompositionStart}
    oncompositionend={handleCompositionEnd}
    class="h-10 border-transparent bg-transparent pl-9 pr-10 text-[14px] font-medium placeholder:text-muted-foreground/35 shadow-none transition-colors focus-visible:ring-0 select-none"
  />

  {#if clipboardStore.searchQuery}
    <Button
      variant="ghost"
      size="icon"
      class="absolute right-1 top-1/2 -translate-y-1/2 h-6 w-6 rounded-md text-muted-foreground/50 hover:text-foreground hover:bg-secondary/65 active:scale-90 transition-all duration-150"
      onclick={clearSearch}
      title={t.clear}
    >
      <X class="h-3.5 w-3.5 animate-in zoom-in-50 duration-150" />
    </Button>
  {/if}
</div>
