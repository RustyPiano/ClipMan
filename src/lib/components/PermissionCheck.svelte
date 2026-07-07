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

<!-- Permission banners: quiet, token-driven cards that sit inside the panel
     and adapt to all three themes. The amber accent is confined to the icon so
     the banner reads as a notice, not an alarm. -->
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
            <p class="mt-1.5 rounded-md bg-muted px-2 py-1.5 font-mono text-xs break-all text-destructive">
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
