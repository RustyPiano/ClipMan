<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import { i18n } from '$lib/i18n';
  import { isMac } from '$lib/utils/platform';
  import { toastStore } from '$lib/stores/toast.svelte';
  import Button from './ui/Button.svelte';
  import { AlertTriangle, RefreshCw, Settings } from 'lucide-svelte';

  const t = $derived(i18n.t);

  let hasPermission = $state(true);
  let isChecking = $state(true);
  let errorMessage = $state('');

  let hasAccessibility = $state(true);
  let isCheckingAccessibility = $state(false);

  async function checkPermission() {
    isChecking = true;
    errorMessage = '';
    try {
      await invoke('check_clipboard_permission');
      hasPermission = true;
    } catch (e) {
      hasPermission = false;
      errorMessage = String(e);
    } finally {
      isChecking = false;
    }
  }

  async function checkAccessibility() {
    if (!isMac) return;
    isCheckingAccessibility = true;
    try {
      hasAccessibility = await invoke<boolean>('check_accessibility_permission');
    } finally {
      isCheckingAccessibility = false;
    }
  }

  async function openAccessibilitySettings() {
    try {
      await invoke('open_accessibility_settings');
    } catch (e) {
      toastStore.add(String(e), 'error');
    }
  }

  onMount(() => {
    checkPermission();
    checkAccessibility();

    const onFocus = () => {
      checkPermission();
      checkAccessibility();
    };
    window.addEventListener('focus', onFocus);

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
  <div class="mx-3 mt-3 rounded-lg border border-border bg-muted/40 px-3.5 py-3">
    <div class="flex items-start gap-3">
      <AlertTriangle class="mt-0.5 h-4 w-4 flex-none text-amber-500/90" />
      <div class="min-w-0 flex-1 text-[13px] leading-relaxed">
        <p>
          <span class="font-semibold text-foreground">{t.clipboardAccessTitle}</span>
          <span class="text-muted-foreground"> · {t.clipboardAccessDesc}</span>
        </p>
        <p class="mt-0.5 text-muted-foreground">{t.clipboardAccessHint}</p>
        {#if errorMessage}
          <details class="mt-1.5">
            <summary class="cursor-pointer text-xs text-muted-foreground hover:text-foreground">
              {t.details}
            </summary>
            <p
              class="mt-1.5 rounded-md bg-muted px-2 py-1.5 font-mono text-xs break-all text-destructive"
            >
              {t.errorLabel}: {errorMessage}
            </p>
          </details>
        {/if}
      </div>
      <Button
        onclick={checkPermission}
        disabled={isChecking}
        variant="outline"
        size="sm"
        class="flex-none"
      >
        <RefreshCw class="mr-1.5 h-3.5 w-3.5 {isChecking ? 'animate-spin' : ''}" />
        {isChecking ? t.checking : t.recheck}
      </Button>
    </div>
  </div>
{/if}

{#if isMac && !hasAccessibility}
  <div class="mx-3 mt-3 rounded-lg border border-border bg-muted/40 px-3.5 py-3">
    <div class="flex items-start gap-3">
      <AlertTriangle class="mt-0.5 h-4 w-4 flex-none text-amber-500/90" />
      <div class="min-w-0 flex-1 text-[13px] leading-relaxed">
        <p>
          <span class="font-semibold text-foreground">{t.accessibilityTitle}</span>
          <span class="text-muted-foreground"> · {t.accessibilityDesc}</span>
        </p>
        <p class="mt-0.5 text-muted-foreground">{t.accessibilityHint}</p>
      </div>
      <div class="flex flex-none items-center gap-1.5">
        <Button onclick={openAccessibilitySettings} size="sm">
          <Settings class="mr-1.5 h-3.5 w-3.5" />
          {t.openSettings}
        </Button>
        <Button
          onclick={checkAccessibility}
          disabled={isCheckingAccessibility}
          variant="outline"
          size="sm"
        >
          <RefreshCw class="mr-1.5 h-3.5 w-3.5 {isCheckingAccessibility ? 'animate-spin' : ''}" />
          {isCheckingAccessibility ? t.checking : t.recheck}
        </Button>
      </div>
    </div>
  </div>
{/if}
