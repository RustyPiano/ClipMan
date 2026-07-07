<script lang="ts">
  import Card from '$lib/components/ui/Card.svelte';
  import Button from '$lib/components/ui/Button.svelte';
  import Switch from '$lib/components/ui/Switch.svelte';
  import Input from '$lib/components/ui/Input.svelte';
  import { Trash2, X, Plus } from 'lucide-svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { i18n } from '$lib/i18n';
  import { toastStore } from '$lib/stores/toast.svelte';
  import { confirmStore } from '$lib/stores/confirm.svelte';
  import type { Settings } from '$lib/types';

  let { settings = $bindable() } = $props<{
    settings: Settings;
  }>();

  const t = $derived(i18n.t);

  let clearing = $state(false);

  async function clearNonPinnedHistory() {
    const confirmed = await confirmStore.ask({
      title: t.clearNonPinned,
      message: t.confirmClearHistory,
      confirmLabel: t.clear,
      destructive: true,
    });
    if (!confirmed) {
      return;
    }

    try {
      clearing = true;
      await invoke('clear_non_pinned_history');
    } catch (err) {
      console.error('Failed to clear non-pinned history:', err);
      toastStore.add(`${t.clearFailed}: ${String(err)}`, 'error');
    } finally {
      clearing = false;
    }
  }

  // --- Ignored apps (SPEC-4 §3) ---
  // `ignoredApps` is optional on Settings so this component doesn't depend on
  // every caller's default-settings literal already knowing about it; it is
  // always populated once the real settings load from the backend.
  const ignoredApps = $derived(settings.ignoredApps ?? []);

  let newIgnoredApp = $state('');

  function addIgnoredApp() {
    const trimmed = newIgnoredApp.trim();
    if (!trimmed) {
      return;
    }
    const alreadyListed = ignoredApps.some(
      (app: string) => app.toLowerCase() === trimmed.toLowerCase()
    );
    if (!alreadyListed) {
      settings.ignoredApps = [...ignoredApps, trimmed];
    }
    newIgnoredApp = '';
  }

  function removeIgnoredApp(app: string) {
    settings.ignoredApps = ignoredApps.filter((existing: string) => existing !== app);
  }

  function handleIgnoredAppKeydown(event: KeyboardEvent) {
    if (event.key === 'Enter') {
      event.preventDefault();
      addIgnoredApp();
    }
  }

  // --- Capture size limits (SPEC-3 §5 UI) ---
  const BYTES_PER_MB = 1_000_000;
  const DEFAULT_MAX_TEXT_BYTES = 2_000_000;
  const DEFAULT_MAX_IMAGE_DIMENSION = 4096;

  const maxTextMb = $derived(
    Math.round(((settings.maxTextBytes ?? DEFAULT_MAX_TEXT_BYTES) / BYTES_PER_MB) * 100) / 100
  );

  function updateMaxTextMb(raw: string) {
    const value = Number.parseFloat(raw);
    if (!Number.isFinite(value) || value < 0) {
      return;
    }
    settings.maxTextBytes = Math.round(value * BYTES_PER_MB);
  }

  function updateMaxImageDimension(raw: string) {
    const value = Number.parseInt(raw, 10);
    if (!Number.isFinite(value) || value < 0) {
      return;
    }
    settings.maxImageDimension = value;
  }
</script>

<div class="space-y-6 animate-in fade-in slide-in-from-bottom-4 duration-300">
  <div>
    <h2 class="text-lg font-semibold mb-1">{t.settingsClipboard}</h2>
    <p class="text-sm text-muted-foreground">
      {t.maxHistoryItemsDesc}
    </p>
  </div>

  <Card class="p-6 space-y-6">
    <div class="space-y-4">
      <div class="flex justify-between">
        <div class="space-y-0.5">
          <label for="max-items" class="text-sm font-medium">{t.maxHistoryItems}</label>
          <p class="text-xs text-muted-foreground">
            {t.maxHistoryItemsDesc}
          </p>
        </div>
        <span class="text-sm font-bold text-primary">{settings.maxHistoryItems}</span>
      </div>
      <input
        id="max-items"
        type="range"
        min="50"
        max="500"
        step="50"
        bind:value={settings.maxHistoryItems}
        class="w-full accent-primary h-2 bg-muted rounded-lg appearance-none cursor-pointer"
      />
    </div>

    <div class="flex items-center justify-between">
      <div class="space-y-0.5">
        <label for="auto-paste" class="text-sm font-medium cursor-pointer">
          {t.autoPaste}
        </label>
        <p class="text-xs text-muted-foreground">
          {t.autoPasteDesc}
        </p>
      </div>
      <Switch id="auto-paste" bind:checked={settings.autoPaste} />
    </div>

    <div class="flex items-center justify-between">
      <div class="space-y-0.5">
        <label for="ignore-concealed" class="text-sm font-medium cursor-pointer">
          {t.ignoreConcealed}
        </label>
        <p class="text-xs text-muted-foreground">
          {t.ignoreConcealedDesc}
        </p>
      </div>
      <Switch id="ignore-concealed" bind:checked={settings.ignoreConcealed} />
    </div>

    <div class="flex items-center justify-between">
      <div class="space-y-0.5">
        <label for="skip-secrets" class="text-sm font-medium cursor-pointer">
          {t.skipSecrets}
        </label>
        <p class="text-xs text-muted-foreground">
          {t.skipSecretsDesc}
        </p>
      </div>
      <Switch
        id="skip-secrets"
        checked={settings.skipSecrets ?? true}
        onchange={(event: Event) => {
          settings.skipSecrets = (event.currentTarget as HTMLInputElement).checked;
        }}
      />
    </div>

    <div class="pt-4 border-t border-border space-y-4">
      <div class="flex items-center justify-between gap-4">
        <div class="space-y-0.5">
          <label for="max-text-bytes" class="text-sm font-medium">{t.maxTextBytes}</label>
          <p class="text-xs text-muted-foreground">
            {t.maxTextBytesDesc}
          </p>
        </div>
        <input
          id="max-text-bytes"
          type="number"
          min="0"
          step="0.1"
          value={maxTextMb}
          oninput={(event) => updateMaxTextMb(event.currentTarget.value)}
          class="w-24 h-9 rounded-md border border-input bg-transparent px-3 py-1 text-sm shadow-sm text-right focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring"
        />
      </div>

      <div class="flex items-center justify-between gap-4">
        <div class="space-y-0.5">
          <label for="max-image-dimension" class="text-sm font-medium">{t.maxImageDimension}</label>
          <p class="text-xs text-muted-foreground">
            {t.maxImageDimensionDesc}
          </p>
        </div>
        <input
          id="max-image-dimension"
          type="number"
          min="0"
          step="1"
          value={settings.maxImageDimension ?? DEFAULT_MAX_IMAGE_DIMENSION}
          oninput={(event) => updateMaxImageDimension(event.currentTarget.value)}
          class="w-24 h-9 rounded-md border border-input bg-transparent px-3 py-1 text-sm shadow-sm text-right focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring"
        />
      </div>
    </div>

    <div class="pt-4 border-t border-border space-y-3">
      <div class="space-y-0.5">
        <span class="text-sm font-medium">{t.ignoredApps}</span>
        <p class="text-xs text-muted-foreground">
          {t.ignoredAppsDesc}
        </p>
      </div>

      <div class="flex gap-2">
        <Input
          bind:value={newIgnoredApp}
          placeholder={t.ignoredAppsPlaceholder}
          onkeydown={handleIgnoredAppKeydown}
          class="flex-1"
        />
        <Button type="button" variant="secondary" onclick={addIgnoredApp} class="gap-1.5 shrink-0">
          <Plus class="h-4 w-4" />
          {t.addIgnoredApp}
        </Button>
      </div>

      {#if ignoredApps.length > 0}
        <ul class="space-y-1.5">
          {#each ignoredApps as app (app)}
            <li
              class="flex items-center justify-between gap-2 rounded-md border border-border bg-muted/40 px-3 py-1.5 text-sm"
            >
              <span class="truncate">{app}</span>
              <button
                type="button"
                aria-label={t.removeIgnoredApp}
                onclick={() => removeIgnoredApp(app)}
                class="text-muted-foreground hover:text-destructive transition-colors shrink-0"
              >
                <X class="h-3.5 w-3.5" />
              </button>
            </li>
          {/each}
        </ul>
      {:else}
        <p class="text-xs text-muted-foreground">{t.noIgnoredApps}</p>
      {/if}
    </div>

    <div class="pt-4 border-t border-border">
      <div class="flex items-center justify-between">
        <div class="space-y-0.5">
          <span class="text-sm font-medium">{t.clear}</span>
          <p class="text-xs text-muted-foreground">
            {t.confirmClearHistory}
          </p>
        </div>
        <Button
          type="button"
          variant="destructive"
          onclick={clearNonPinnedHistory}
          disabled={clearing}
          class="gap-2"
        >
          <Trash2 class="h-4 w-4" />
          {clearing ? t.loading : t.clear}
        </Button>
      </div>
    </div>
  </Card>
</div>
