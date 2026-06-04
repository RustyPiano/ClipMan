<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { hasTauriRuntime } from '$lib/utils/tauri';
  import { i18n } from '$lib/i18n';
  import Button from './ui/Button.svelte';
  import { AlertTriangle, RefreshCw } from 'lucide-svelte';

  const t = $derived(i18n.t);

  let hasPermission = $state(true);
  let isChecking = $state(true);
  let errorMessage = $state('');

  async function checkPermission() {
    isChecking = true;
    errorMessage = '';
    try {
      const res = await invoke<string>('check_clipboard_permission');
      hasPermission = res === 'granted';
      if (!hasPermission) {
        errorMessage = res;
      }
    } catch (e) {
      console.error('Failed to check permission:', e);
      // The command only throws when the clipboard can't be created (not a permission
      // denial), so don't block the UI — assume granted.
      hasPermission = true;
    } finally {
      isChecking = false;
    }
  }

  onMount(() => {
    if (!hasTauriRuntime()) {
      isChecking = false;
      return;
    }

    checkPermission();

    // Re-check when window gains focus
    window.addEventListener('focus', checkPermission);
    return () => {
      window.removeEventListener('focus', checkPermission);
    };
  });
</script>

{#if !hasPermission}
  <div
    class="bg-amber-50 dark:bg-amber-900/20 border-l-4 border-amber-500 p-4 mb-4 mx-4 rounded-r shadow-sm"
  >
    <div class="flex items-start">
      <div class="flex-shrink-0">
        <AlertTriangle class="h-5 w-5 text-amber-500" />
      </div>
      <div class="ml-3 w-full">
        <h3 class="text-sm font-medium text-amber-800 dark:text-amber-200">
          {t.clipboardAccessTitle}
        </h3>
        <div class="mt-2 text-sm text-amber-700 dark:text-amber-300">
          <p>{t.clipboardAccessDesc}</p>
          <p class="mt-2">{t.clipboardAccessHint}</p>
        </div>

        <details class="mt-3">
          <summary
            class="text-xs text-amber-600 dark:text-amber-400 cursor-pointer hover:underline"
          >
            {t.details}
          </summary>
          {#if errorMessage}
            <p
              class="mt-2 p-2 bg-red-50 dark:bg-red-900/20 text-red-800 dark:text-red-200 rounded text-xs font-mono break-all"
            >
              {t.errorLabel}: {errorMessage}
            </p>
          {/if}
        </details>

        <div class="mt-4">
          <Button
            onclick={checkPermission}
            disabled={isChecking}
            class="bg-amber-500 hover:bg-amber-600 text-white border-none"
            size="sm"
          >
            <RefreshCw class="h-4 w-4 mr-2 {isChecking ? 'animate-spin' : ''}" />
            {isChecking ? t.checking : t.recheck}
          </Button>
        </div>
      </div>
    </div>
  </div>
{/if}
