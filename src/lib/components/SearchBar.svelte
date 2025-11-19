<script lang="ts">
  import { onDestroy } from 'svelte';
  import { clipboardStore } from '$lib/stores/clipboard.svelte';
  import Input from '$lib/components/ui/Input.svelte';
  import Button from '$lib/components/ui/Button.svelte';
  import { Search, X } from 'lucide-svelte';

  const SEARCH_DEBOUNCE_MS = 300;

  let searchQuery = $state('');
  let debounceTimer: ReturnType<typeof setTimeout>;

  function handleInput(e: Event) {
    const target = e.target as HTMLInputElement;
    searchQuery = target.value;
    
    clearTimeout(debounceTimer);
    debounceTimer = setTimeout(() => {
      clipboardStore.search(searchQuery);
    }, SEARCH_DEBOUNCE_MS);
  }

  function clearSearch() {
    searchQuery = '';
    clipboardStore.search('');
  }

  // Cleanup debounce timer on component destroy
  onDestroy(() => {
    clearTimeout(debounceTimer);
  });
</script>

<div class="relative w-full max-w-md mx-auto">
  <div class="absolute left-3 top-1/2 -translate-y-1/2 text-muted-foreground pointer-events-none">
    <Search class="h-4 w-4" />
  </div>
  
  <Input
    type="text"
    placeholder="搜索剪切板内容..."
    value={searchQuery}
    oninput={handleInput}
    class="pl-9 pr-8 bg-muted/50 border-transparent focus:bg-background focus:border-input transition-all"
  />

  {#if searchQuery}
    <Button
      variant="ghost"
      size="icon"
      class="absolute right-1 top-1 h-7 w-7 text-muted-foreground hover:text-foreground"
      onclick={clearSearch}
      title="清除搜索"
    >
      <X class="h-4 w-4" />
    </Button>
  {/if}
</div>
