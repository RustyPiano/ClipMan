<script lang="ts">
  import { toastStore } from '$lib/stores/toast.svelte';
  import { fly } from 'svelte/transition';
  import { CheckCircle, AlertCircle, Info } from 'lucide-svelte';
  import type { ToastType } from '$lib/types';

  // Each toast type gets its own colour so an `info` toast no longer borrows the
  // red error style (only `error` is red; `info` is neutral/blue).
  const toastStyles: Record<ToastType, string> = {
    success: 'bg-emerald-500 text-white',
    error: 'bg-red-500 text-white',
    info: 'bg-sky-500 text-white',
  };
</script>

<div
  class="fixed bottom-8 left-1/2 -translate-x-1/2 flex flex-col gap-2 z-[9999] pointer-events-none"
  role="status"
  aria-live="polite"
  aria-atomic="true"
>
  {#each toastStore.toasts as toast (toast.id)}
    <div
      class="px-4 py-2 rounded-full text-sm font-medium shadow-lg flex items-center gap-2 justify-center
             {toastStyles[toast.type]}"
      in:fly={{ y: 10, duration: 200 }}
      out:fly={{ y: -10, duration: 200 }}
    >
      {#if toast.type === 'success'}
        <CheckCircle class="h-4 w-4" />
      {:else if toast.type === 'error'}
        <AlertCircle class="h-4 w-4" />
      {:else}
        <Info class="h-4 w-4" />
      {/if}
      {toast.message}
    </div>
  {/each}
</div>
