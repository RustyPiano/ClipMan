<script lang="ts">
  import Card from '$lib/components/ui/Card.svelte';
  import Button from '$lib/components/ui/Button.svelte';
  import { Loader2, FolderOpen } from 'lucide-svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { i18n } from '$lib/i18n';
  import { toastStore } from '$lib/stores/toast.svelte';

  let {
    currentDataPath,
    loadingDataPath,
    dataPathError,
    loadDataPath,
    changingDataPath,
    changeDataLocation,
  } = $props<{
    currentDataPath: string;
    loadingDataPath: boolean;
    dataPathError: string;
    loadDataPath: () => void;
    changingDataPath: boolean;
    changeDataLocation: () => void;
  }>();

  const t = $derived(i18n.t);

  async function openDataFolder() {
    try {
      await invoke('open_folder', { path: currentDataPath });
    } catch (err) {
      toastStore.add(String(err), 'error');
    }
  }
</script>

<div class="space-y-6 duration-300">
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
          disabled={loadingDataPath || !!dataPathError || !currentDataPath}
          class="gap-1 h-7 text-xs"
        >
          <FolderOpen class="h-3.5 w-3.5" />
          {t.openFolder}
        </Button>
      </div>
      {#if dataPathError}
        <p role="alert" class="text-sm text-destructive">{dataPathError}</p>
        <Button variant="outline" onclick={loadDataPath}>{t.recheck}</Button>
      {:else}
        <div class="p-3 bg-muted rounded-md text-sm font-mono break-all border border-border">
          {loadingDataPath ? t.loading : currentDataPath}
        </div>
      {/if}
    </div>

    <div class="pt-2 border-t border-border">
      <div class="flex items-center justify-between">
        <div class="space-y-0.5">
          <span class="text-sm font-medium">{t.changeLocation}</span>
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
