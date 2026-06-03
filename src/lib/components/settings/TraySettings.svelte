<script lang="ts">
  import Card from '$lib/components/ui/Card.svelte';
  import { i18n } from '$lib/i18n';
  import type { Settings } from '$lib/types';

  let { settings = $bindable() } = $props<{
    settings: Settings;
  }>();

  const t = $derived(i18n.t);
</script>

<div class="space-y-6 animate-in fade-in slide-in-from-bottom-4 duration-300">
  <div>
    <h2 class="text-lg font-semibold mb-1">{t.settingsTray}</h2>
    <p class="text-sm text-muted-foreground">
      {t.trayTextLengthDesc}
    </p>
  </div>

  <Card class="p-6 space-y-6">
    <div class="space-y-4">
      <div class="flex justify-between">
        <label for="tray-text-length" class="text-sm font-medium">{t.trayTextLength}</label>
        <span class="text-sm font-bold text-primary">{settings.trayTextLength}</span>
      </div>
      <input
        id="tray-text-length"
        type="range"
        min="10"
        max="100"
        step="5"
        bind:value={settings.trayTextLength}
        class="w-full accent-primary h-2 bg-muted rounded-lg appearance-none cursor-pointer"
      />
      <p class="text-xs text-muted-foreground">
        {t.trayTextLengthDesc}
      </p>
    </div>

    <div class="space-y-4">
      <div class="flex justify-between">
        <label for="max-pinned" class="text-sm font-medium">{t.maxPinnedInTray}</label>
        <span class="text-sm font-bold text-primary">{settings.maxPinnedInTray}</span>
      </div>
      <input
        id="max-pinned"
        type="range"
        min="0"
        max="10"
        step="1"
        bind:value={settings.maxPinnedInTray}
        class="w-full accent-primary h-2 bg-muted rounded-lg appearance-none cursor-pointer"
      />
      <p class="text-xs text-muted-foreground">
        {t.maxPinnedInTrayDesc}
      </p>
    </div>

    <div class="space-y-4">
      <div class="flex justify-between">
        <label for="max-recent" class="text-sm font-medium">{t.maxRecentInTray}</label>
        <span class="text-sm font-bold text-primary">{settings.maxRecentInTray}</span>
      </div>
      <input
        id="max-recent"
        type="range"
        min="0"
        max="30"
        step="1"
        bind:value={settings.maxRecentInTray}
        class="w-full accent-primary h-2 bg-muted rounded-lg appearance-none cursor-pointer"
      />
      <p class="text-xs text-muted-foreground">
        {t.maxRecentInTrayDesc}
      </p>
    </div>
  </Card>
</div>
