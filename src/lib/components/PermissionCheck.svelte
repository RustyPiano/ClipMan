<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import { hasTauriRuntime } from '$lib/utils/tauri';
  import { i18n } from '$lib/i18n';
  import Button from './ui/Button.svelte';
  import { AlertTriangle, RefreshCw, Settings } from 'lucide-svelte';

  const t = $derived(i18n.t);

  const isMac =
    typeof navigator !== 'undefined' && navigator.platform.toUpperCase().includes('MAC');

  let hasPermission = $state(true);
  let isChecking = $state(true);
  let errorMessage = $state('');

  // Accessibility permission (macOS only). Auto-paste simulates Cmd+V, which
  // requires it; the grant is commonly lost after an app update.
  let hasAccessibility = $state(true);
  let isCheckingAccessibility = $state(false);

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

  async function checkAccessibility() {
    if (!isMac) return;
    isCheckingAccessibility = true;
    try {
      hasAccessibility = await invoke<boolean>('check_accessibility_permission');
    } catch (e) {
      console.error('Failed to check accessibility permission:', e);
      // Don't block the UI if the check itself fails.
      hasAccessibility = true;
    } finally {
      isCheckingAccessibility = false;
    }
  }

  async function openAccessibilitySettings() {
    try {
      await invoke('open_accessibility_settings');
    } catch (e) {
      console.error('Failed to open accessibility settings:', e);
    }
  }

  onMount(() => {
    if (!hasTauriRuntime()) {
      isChecking = false;
      return;
    }

    checkPermission();
    checkAccessibility();

    // Re-check when the window regains focus (e.g. after the user toggles the
    // permission in System Settings and comes back).
    const onFocus = () => {
      checkPermission();
      checkAccessibility();
    };
    window.addEventListener('focus', onFocus);

    // The backend emits this the moment an auto-paste is blocked by a missing
    // permission, so the banner is already showing next time the QuickBar opens.
    let unlisten: (() => void) | undefined;
    if (isMac) {
      listen('accessibility-permission-required', () => {
        hasAccessibility = false;
      }).then((fn) => {
        unlisten = fn;
      });
    }

    return () => {
      window.removeEventListener('focus', onFocus);
      unlisten?.();
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

{#if isMac && !hasAccessibility}
  <div
    class="bg-amber-50 dark:bg-amber-900/20 border-l-4 border-amber-500 p-4 mb-4 mx-4 rounded-r shadow-sm"
  >
    <div class="flex items-start">
      <div class="flex-shrink-0">
        <AlertTriangle class="h-5 w-5 text-amber-500" />
      </div>
      <div class="ml-3 w-full">
        <h3 class="text-sm font-medium text-amber-800 dark:text-amber-200">
          {t.accessibilityTitle}
        </h3>
        <div class="mt-2 text-sm text-amber-700 dark:text-amber-300">
          <p>{t.accessibilityDesc}</p>
          <p class="mt-2">{t.accessibilityHint}</p>
        </div>

        <div class="mt-4 flex flex-wrap gap-2">
          <Button
            onclick={openAccessibilitySettings}
            class="bg-amber-500 hover:bg-amber-600 text-white border-none"
            size="sm"
          >
            <Settings class="h-4 w-4 mr-2" />
            {t.openSettings}
          </Button>
          <Button
            onclick={checkAccessibility}
            disabled={isCheckingAccessibility}
            variant="outline"
            size="sm"
          >
            <RefreshCw class="h-4 w-4 mr-2 {isCheckingAccessibility ? 'animate-spin' : ''}" />
            {isCheckingAccessibility ? t.checking : t.recheck}
          </Button>
        </div>
      </div>
    </div>
  </div>
{/if}
