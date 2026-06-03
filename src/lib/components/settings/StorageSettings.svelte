<script lang="ts">
  import Card from '$lib/components/ui/Card.svelte';
  import Button from '$lib/components/ui/Button.svelte';
  import { Loader2, FolderOpen } from 'lucide-svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { i18n } from '$lib/i18n';
  import type { Settings } from '$lib/types';

  let {
    settings = $bindable(),
    currentDataPath,
    changingDataPath,
    changeDataLocation,
  } = $props<{
    settings: Settings;
    currentDataPath: string;
    changingDataPath: boolean;
    changeDataLocation: () => void;
  }>();

  const t = $derived(i18n.t);

  async function openDataFolder() {
    if (!currentDataPath) {
      return;
    }

    try {
      await invoke('open_folder', { path: currentDataPath });
    } catch (err) {
      console.error('Failed to open folder:', err);
    }
  }
</script>

<div class="space-y-6 animate-in fade-in slide-in-from-bottom-4 duration-300">
  <div>
    <h2 class="text-lg font-semibold mb-1">{t.settingsStorage}</h2>
    <p class="text-sm text-muted-foreground">{t.dataLocationDesc}</p>
  </div>

  <Card class="p-6 space-y-6">
    <div class="space-y-2">
      <div class="flex items-center justify-between">
        <label for="data-path" class="text-sm font-medium">{t.currentLocation}</label>
        <Button
          type="button"
          variant="ghost"
          size="sm"
          onclick={openDataFolder}
          class="gap-1 h-7 text-xs"
        >
          <FolderOpen class="h-3.5 w-3.5" />
          {t.openFolder}
        </Button>
      </div>
      <div class="p-3 bg-muted rounded-md text-sm font-mono break-all border border-border">
        {currentDataPath || t.loading}
      </div>
      <p class="text-xs text-muted-foreground">
        {t.dataLocationDesc}
      </p>
    </div>

    <div class="pt-2 border-t border-border">
      <div class="flex items-center justify-between">
        <div class="space-y-0.5">
          <span class="text-sm font-medium">{t.changeLocation}</span>
          <p class="text-xs text-muted-foreground">
            {t.dataLocationDesc}
          </p>
        </div>
        <Button
          type="button"
          variant="secondary"
          onclick={changeDataLocation}
          disabled={changingDataPath}
        >
          {#if changingDataPath}
            <Loader2 class="h-4 w-4 animate-spin mr-2" /> {t.loading}
          {:else}
            {t.changeLocation}
          {/if}
        </Button>
      </div>
    </div>
  </Card>
</div>
