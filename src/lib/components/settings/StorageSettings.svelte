<script lang="ts">
  import Card from '$lib/components/ui/Card.svelte';
  import Button from '$lib/components/ui/Button.svelte';
  import { Loader2, FolderOpen, Trash2, Upload, Download } from 'lucide-svelte';
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { open, save } from '@tauri-apps/plugin-dialog';
  import { i18n } from '$lib/i18n';
  import { toastStore } from '$lib/stores/toast.svelte';
  import { confirmStore } from '$lib/stores/confirm.svelte';
  import { clipboardStore } from '$lib/stores/clipboard.svelte';
  import { formatBytes } from '$lib/utils/format-bytes';
  import type { BackupFile, ImportResult, StorageUsage } from '$lib/types';

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
  const FAVORITES_FILTERS = [{ name: 'JSON', extensions: ['json'] }];

  let usage = $state<StorageUsage | null>(null);
  let backups = $state.raw<BackupFile[]>([]);
  let busy = $state(false);

  onMount(() => {
    void refresh();
  });

  async function refresh() {
    try {
      [usage, backups] = await Promise.all([
        invoke<StorageUsage>('get_storage_usage'),
        invoke<BackupFile[]>('list_old_backups'),
      ]);
    } catch (err) {
      toastStore.add(String(err), 'error');
    }
  }

  async function run(task: () => Promise<void>, failure: string) {
    if (busy) return;
    busy = true;
    try {
      await task();
    } catch (err) {
      toastStore.add(`${failure}: ${String(err)}`, 'error');
    } finally {
      busy = false;
      await refresh();
    }
  }

  async function openDataFolder() {
    try {
      await invoke('open_folder', { path: currentDataPath });
    } catch (err) {
      toastStore.add(String(err), 'error');
    }
  }

  function exportFavorites() {
    return run(async () => {
      const path = await save({
        defaultPath: 'clipleaf-favorites.json',
        filters: FAVORITES_FILTERS,
      });
      if (!path) return;
      const count = await invoke<number>('export_favorites', { path });
      toastStore.add(i18n.format(t.exportedFavorites, { n: count }), 'success');
    }, t.exportFavoritesFailed);
  }

  function importFavorites() {
    return run(async () => {
      const path = await open({ multiple: false, directory: false, filters: FAVORITES_FILTERS });
      if (typeof path !== 'string') return;
      const result = await invoke<ImportResult>('import_favorites', {
        path,
      });
      toastStore.add(
        i18n.format(t.importedFavorites, { n: result.imported, skipped: result.skipped }),
        'success'
      );
    }, t.importFavoritesFailed);
  }

  function deleteBackups() {
    return run(async () => {
      const confirmed = await confirmStore.ask({
        title: t.deleteOldBackups,
        message: t.confirmDeleteOldBackups,
        confirmLabel: t.delete,
        destructive: true,
      });
      if (!confirmed) return;
      await invoke('delete_old_backups');
      toastStore.add(t.oldBackupsDeleted, 'success');
    }, t.operationFailed);
  }

  function clearHistory() {
    return run(() => clipboardStore.clearNonPinned(), t.clearFailed);
  }
</script>

<div class="space-y-6">
  <h2 class="text-lg font-semibold">{t.settingsStorage}</h2>

  <Card class="space-y-5 p-6">
    <div class="space-y-2">
      <div class="flex items-center justify-between">
        <label for="data-path" class="text-sm font-medium">{t.currentLocation}</label>
        <Button
          variant="ghost"
          size="sm"
          onclick={openDataFolder}
          disabled={loadingDataPath || !!dataPathError || !currentDataPath}
          class="h-7 gap-1 text-xs"
        >
          <FolderOpen class="h-3.5 w-3.5" />
          {t.openFolder}
        </Button>
      </div>
      {#if dataPathError}
        <p role="alert" class="text-sm text-destructive">{dataPathError}</p>
        <Button variant="outline" onclick={loadDataPath}>{t.recheck}</Button>
      {:else}
        <div
          id="data-path"
          class="break-all rounded-md border border-border bg-muted p-3 font-mono text-sm"
        >
          {loadingDataPath ? t.loading : currentDataPath}
        </div>
      {/if}
      <div class="flex justify-end">
        <Button
          variant="secondary"
          size="sm"
          onclick={changeDataLocation}
          disabled={changingDataPath}
        >
          {#if changingDataPath}
            <Loader2 class="mr-2 h-4 w-4 animate-spin" /> {t.loading}
          {:else}
            {t.changeLocation}
          {/if}
        </Button>
      </div>
    </div>

    <div class="flex items-center justify-between gap-4 border-t border-border pt-5">
      <span class="text-sm font-medium">{t.storageUsage}</span>
      {#if usage}
        <div class="text-right text-xs text-muted-foreground">
          <p>
            {i18n.format(t.usageSummary, {
              count: usage.total,
              pinned: usage.pinned,
              images: usage.images,
            })}
          </p>
          <p>{i18n.format(t.databaseSize, { size: formatBytes(usage.databaseBytes) })}</p>
        </div>
      {/if}
    </div>
  </Card>

  <Card class="space-y-3 p-6">
    <div class="space-y-0.5">
      <span class="text-sm font-medium">{t.favoritesBackup}</span>
      <p class="text-xs text-muted-foreground">{t.favoritesBackupDesc}</p>
    </div>
    <div class="flex gap-2">
      <Button variant="outline" size="sm" class="gap-1.5" disabled={busy} onclick={exportFavorites}>
        <Upload class="h-3.5 w-3.5" />{t.exportFavorites}
      </Button>
      <Button variant="outline" size="sm" class="gap-1.5" disabled={busy} onclick={importFavorites}>
        <Download class="h-3.5 w-3.5" />{t.importFavorites}
      </Button>
    </div>
  </Card>

  <Card class="space-y-3 p-6">
    <div class="flex items-center justify-between gap-4">
      <div class="space-y-0.5">
        <span class="text-sm font-medium">{t.oldBackups}</span>
        <p class="text-xs text-muted-foreground">{t.oldBackupsDesc}</p>
      </div>
      <Button
        variant="outline"
        size="sm"
        disabled={busy || backups.length === 0}
        onclick={deleteBackups}
      >
        {t.deleteOldBackups}
      </Button>
    </div>
    {#if backups.length}
      <ul class="space-y-1 text-xs text-muted-foreground">
        {#each backups as backup (backup.name)}
          <li class="flex justify-between gap-3">
            <span class="truncate font-mono">{backup.name}</span>
            <span class="flex-none tabular-nums">{formatBytes(backup.bytes)}</span>
          </li>
        {/each}
      </ul>
    {:else}
      <p class="text-xs text-muted-foreground">{t.noOldBackups}</p>
    {/if}
  </Card>

  <Card class="flex items-center justify-between gap-4 p-6">
    <div class="space-y-0.5">
      <span class="text-sm font-medium">{t.clearHistoryTitle}</span>
      <p class="text-xs text-muted-foreground">{t.confirmClearHistory}</p>
    </div>
    <Button variant="destructive" size="sm" class="gap-1.5" disabled={busy} onclick={clearHistory}>
      <Trash2 class="h-3.5 w-3.5" />{t.clear}
    </Button>
  </Card>
</div>
