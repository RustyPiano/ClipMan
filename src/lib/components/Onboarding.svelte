<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { Check, ClipboardCopy, Keyboard, MousePointerClick } from 'lucide-svelte';
  import Button from './ui/Button.svelte';
  import { i18n } from '$lib/i18n';
  import { isMac } from '$lib/utils/platform';
  import { formatShortcut } from '$lib/utils/shortcut';
  import { ONBOARDING_START_ID } from '$lib/constants';
  import { clipboardStore } from '$lib/stores/clipboard.svelte';
  import { toastStore } from '$lib/stores/toast.svelte';

  let { onDone }: { onDone: () => void } = $props();

  const t = $derived(i18n.t);
  const shortcutKeys = $derived(formatShortcut(clipboardStore.globalShortcut));
  const modifier = isMac ? '⌘' : 'Ctrl';

  let hasAccessibility = $state(true);

  async function checkAccessibility() {
    if (!isMac) return;
    try {
      hasAccessibility = await invoke<boolean>('check_accessibility_permission');
    } catch (error) {
      toastStore.add(String(error), 'error');
    }
  }

  async function grantAccess() {
    try {
      await invoke('open_accessibility_settings');
    } catch (error) {
      toastStore.add(String(error), 'error');
    }
  }

  async function finish() {
    await clipboardStore.completeOnboarding();
    if (clipboardStore.onboardingCompleted) onDone();
  }

  function focusOnMount(node: HTMLElement) {
    node.focus();
    void checkAccessibility();
    invoke('show_quickbar').catch((error) => toastStore.add(String(error), 'error'));
  }
</script>

<svelte:window onfocus={checkAccessibility} />

<section
  class="absolute inset-0 z-40 flex flex-col overflow-y-auto rounded-xl bg-card text-card-foreground"
  aria-labelledby="onboarding-title"
>
  <div class="m-auto w-full max-w-md px-8 py-8">
    <h1 id="onboarding-title" class="text-lg font-semibold">{t.welcomeTitle}</h1>
    <p class="mt-1.5 text-sm text-muted-foreground">{t.welcomeIntro}</p>

    <ol class="mt-6 space-y-5">
      <li class="flex gap-3">
        <Keyboard class="mt-0.5 h-4 w-4 flex-none text-muted-foreground" />
        <div class="min-w-0 flex-1">
          <h2 class="text-sm font-medium">{t.welcomeShortcutTitle}</h2>
          <p class="mt-0.5 text-xs leading-relaxed text-muted-foreground">
            {isMac ? t.welcomeShortcutDescMac : t.welcomeShortcutDescOther}
          </p>
          <div class="mt-2 flex items-center gap-1">
            {#each shortcutKeys as key, index (index)}
              <kbd class="kbd-keycap">{key}</kbd>
            {/each}
          </div>
        </div>
      </li>

      {#if isMac}
        <li class="flex gap-3">
          <MousePointerClick class="mt-0.5 h-4 w-4 flex-none text-muted-foreground" />
          <div class="min-w-0 flex-1">
            <h2 class="text-sm font-medium">{t.welcomeAccessibilityTitle}</h2>
            <p class="mt-0.5 text-xs leading-relaxed text-muted-foreground">
              {t.welcomeAccessibilityDesc}
            </p>
          </div>
          <div class="flex-none self-center">
            {#if hasAccessibility}
              <span
                class="inline-flex items-center gap-1 text-xs text-muted-foreground"
                role="status"
              >
                <Check class="h-3.5 w-3.5" />{t.welcomeAccessibilityGranted}
              </span>
            {:else}
              <Button variant="outline" size="sm" onclick={grantAccess}>{t.grantAccess}</Button>
            {/if}
          </div>
        </li>
      {/if}

      <li class="flex gap-3">
        <ClipboardCopy class="mt-0.5 h-4 w-4 flex-none text-muted-foreground" />
        <div class="min-w-0 flex-1">
          <h2 class="text-sm font-medium">{t.welcomeTryTitle}</h2>
          <p class="mt-0.5 text-xs leading-relaxed text-muted-foreground">
            {i18n.format(t.welcomeTryDesc, { mod: modifier })}
          </p>
        </div>
      </li>
    </ol>

    <Button id={ONBOARDING_START_ID} class="mt-7 w-full" onclick={finish} {@attach focusOnMount}>
      {t.getStarted}
    </Button>
  </div>
</section>
