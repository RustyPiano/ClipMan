<script lang="ts">
  import { onDestroy, onMount } from 'svelte';
  import { clipboardStore } from '$lib/stores/clipboard.svelte';
  import { i18n } from '$lib/i18n';
  import Input from '$lib/components/ui/Input.svelte';
  import Button from '$lib/components/ui/Button.svelte';
  import { Search, X } from 'lucide-svelte';

  const SEARCH_DEBOUNCE_MS = 300;
  const SEARCH_INPUT_ID = 'quickbar-search';

  const t = $derived(i18n.t);

  let searchQuery = $state(clipboardStore.searchQuery);
  let debounceTimer: ReturnType<typeof setTimeout>;

  function runSearch(query: string) {
    clearTimeout(debounceTimer);
    debounceTimer = setTimeout(() => {
      void clipboardStore.search(query);
    }, SEARCH_DEBOUNCE_MS);
  }

  function handleInput(event: Event) {
    const target = event.target as HTMLInputElement;
    searchQuery = target.value;
    runSearch(searchQuery);
  }

  function clearSearch() {
    searchQuery = '';
    void clipboardStore.search('');

    const input = document.getElementById(SEARCH_INPUT_ID);
    if (input instanceof HTMLInputElement) {
      input.focus();
    }
  }

  onMount(() => {
    const input = document.getElementById(SEARCH_INPUT_ID);
    if (input instanceof HTMLInputElement) {
      input.focus();
    }
  });

  onDestroy(() => {
    clearTimeout(debounceTimer);
  });

  $effect(() => {
    if (clipboardStore.searchQuery === '') {
      clearTimeout(debounceTimer);
      searchQuery = '';
    }
  });
</script>

<div class="relative w-full">
  <div class="pointer-events-none absolute left-2 top-1/2 -translate-y-1/2 text-muted-foreground">
    <Search class="h-4 w-4" />
  </div>

  <Input
    id={SEARCH_INPUT_ID}
    type="text"
    placeholder={t.searchPlaceholder}
    value={searchQuery}
    oninput={handleInput}
    class="h-10 border-transparent bg-transparent pl-8 pr-8 text-base shadow-none transition-colors focus-visible:ring-0"
  />

  {#if searchQuery}
    <Button
      variant="ghost"
      size="icon"
      class="absolute right-1 top-1 h-7 w-7 text-muted-foreground hover:text-foreground"
      onclick={clearSearch}
      title={t.clear}
    >
      <X class="h-4 w-4" />
    </Button>
  {/if}
</div>
