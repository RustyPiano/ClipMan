<script lang="ts">
import { toastStore } from '$lib/stores/toast.svelte';
import { fly } from 'svelte/transition';
import { CheckCircle, AlertCircle } from 'lucide-svelte';
</script>

<div class="fixed bottom-8 left-1/2 -translate-x-1/2 flex flex-col gap-2 z-[9999] pointer-events-none">
  {#each toastStore.toasts as toast (toast.id)}
    <div
      class="px-4 py-2 rounded-full text-sm font-medium shadow-lg flex items-center gap-2 justify-center
             {toast.type === 'success' ? 'bg-emerald-500 text-white' : 'bg-red-500 text-white'}"
      in:fly={{ y: 10, duration: 200 }}
      out:fly={{ y: -10, duration: 200 }}
    >
      {#if toast.type === 'success'}
        <CheckCircle class="h-4 w-4" />
      {:else}
        <AlertCircle class="h-4 w-4" />
      {/if}
      {toast.message}
    </div>
  {/each}
</div>
