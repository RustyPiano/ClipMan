<script lang="ts">
  import { toastStore } from '$lib/stores/toast.svelte';
  import { fly } from 'svelte/transition';
  import { Check, AlertCircle, Info, Star } from 'lucide-svelte';
</script>

<div
  class="pointer-events-none fixed bottom-11 left-1/2 z-[9999] flex -translate-x-1/2 flex-col items-center gap-1.5"
  role="status"
  aria-live="polite"
  aria-atomic="true"
>
  {#each toastStore.toasts as toast (toast.id)}
    <div
      class="qb-toast-pill flex items-center justify-center gap-1.5 rounded-full px-3.5 py-1.5 text-xs font-medium shadow-lg"
      in:fly={{ y: 8, duration: 150 }}
      out:fly={{ y: -6, duration: 130 }}
    >
      {#if toast.type === 'success'}
        <Check class="h-3.5 w-3.5 flex-none text-emerald-400" />
      {:else if toast.type === 'error'}
        <AlertCircle class="h-3.5 w-3.5 flex-none text-rose-400" />
      {:else if toast.type === 'pin'}
        <Star class="h-3.5 w-3.5 flex-none fill-current text-amber-400" />
      {:else}
        <Info class="h-3.5 w-3.5 flex-none text-zinc-400" />
      {/if}
      <span>{toast.message}</span>
    </div>
  {/each}
</div>
