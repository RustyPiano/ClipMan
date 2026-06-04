<script lang="ts">
  import Card from '$lib/components/ui/Card.svelte';
  import Button from '$lib/components/ui/Button.svelte';
  import Switch from '$lib/components/ui/Switch.svelte';
  import { Trash2 } from 'lucide-svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { i18n } from '$lib/i18n';
  import type { Settings } from '$lib/types';

  let { settings = $bindable() } = $props<{
    settings: Settings;
  }>();

  const t = $derived(i18n.t);

  let clearing = $state(false);

  async function clearNonPinnedHistory() {
    if (!confirm(t.confirmClearHistory)) {
      return;
    }

    try {
      clearing = true;
      await invoke('clear_non_pinned_history');
    } catch (err) {
      console.error('Failed to clear non-pinned history:', err);
      alert(t.copyFailed + ': ' + String(err));
    } finally {
      clearing = false;
    }
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
