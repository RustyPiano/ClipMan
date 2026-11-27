<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import { onMount } from 'svelte';
  import { i18n } from '$lib/i18n';
  import { toastStore } from '$lib/stores/toast.svelte';
  import { confirmStore } from '$lib/stores/confirm.svelte';
  import Button from '$lib/components/ui/Button.svelte';
  import { Loader2 } from 'lucide-svelte';
  import { open } from '@tauri-apps/plugin-dialog';
  import type { Settings, SettingsTab, SettingsUpdateResult } from '$lib/types';

  import Sidebar from '$lib/components/settings/Sidebar.svelte';
  import GeneralSettings from '$lib/components/settings/GeneralSettings.svelte';
  import ClipboardSettings from '$lib/components/settings/ClipboardSettings.svelte';
  import StorageSettings from '$lib/components/settings/StorageSettings.svelte';
  import AboutSection from '$lib/components/settings/AboutSection.svelte';
  import AppearanceSettings from '$lib/components/settings/AppearanceSettings.svelte';

  const SETTINGS_TABS: SettingsTab[] = ['general', 'appearance', 'clipboard', 'storage', 'about'];
  const SAVE_DELAY_MS = 300;

  const t = $derived(i18n.t);

  let settings = $state<Settings | null>(null);
  let savedSnapshot = '';
  let loading = $state(true);
  let saving = $state(false);
  let saveTimer: ReturnType<typeof setTimeout> | undefined;
  let saveAgain = false;
  let resetQueued = false;

  let currentDataPath = $state('');
  let loadingDataPath = $state(true);
  let dataPathError = $state('');
  let changingDataPath = $state(false);
  let newDataPath = $state('');
  let deleteOldData = $state(false);
  let migrationDialog: HTMLDialogElement;

  const MIGRATION_DIALOG_TITLE_ID = 'migration-dialog-title';
  const MIGRATION_DIALOG_DESCRIPTION_ID = 'migration-dialog-description';
  const MIGRATION_CANCEL_ID = 'migration-dialog-cancel';

  let activeTab = $state<SettingsTab>('general');

  onMount(() => {
    void loadSettings();
    void loadDataPath();
    const unlisteners: (() => void)[] = [];
    let disposed = false;
    for (const subscription of [
      listen<string>('settings-open-tab', (event) => {
        const tab = SETTINGS_TABS.find((candidate) => candidate === event.payload);
        if (tab) activeTab = tab;
      }),
      listen('settings-hidden', () => void flushSave()),
    ])
      void subscription.then((stop) => (disposed ? stop() : unlisteners.push(stop)));
    return () => {
      disposed = true;
      unlisteners.forEach((stop) => stop());
    };
  });

  $effect(() => {
    if (!settings) return;
    const snapshot = JSON.stringify(settings);
    if (snapshot === savedSnapshot) return;
    clearTimeout(saveTimer);
    saveTimer = setTimeout(() => void flushSave(), SAVE_DELAY_MS);
  });

  async function loadSettings() {
    try {
      loading = true;
      const loaded = await invoke<Settings>('get_settings');
      savedSnapshot = JSON.stringify(loaded);
      settings = loaded;
      i18n.setLocale(loaded.locale);
    } catch (err) {
      toastStore.add(`${t.loadSettingsFailed}: ${String(err)}`, 'error');
    } finally {
      loading = false;
    }
  }

  async function loadDataPath() {
    loadingDataPath = true;
    dataPathError = '';
    try {
      currentDataPath = await invoke<string>('get_current_data_path');
    } catch (err) {
      dataPathError = String(err);
    } finally {
      loadingDataPath = false;
    }
  }

  async function flushSave(reset = false) {
    clearTimeout(saveTimer);
    if (!settings) return;
    if (saving) {
      saveAgain = true;
      resetQueued ||= reset;
      return;
    }
    const request = reset ? null : $state.snapshot(settings);
    const requestSnapshot = JSON.stringify(request);
    if (!reset && requestSnapshot === savedSnapshot) return;
    saving = true;
    try {
      const result = await invoke<SettingsUpdateResult>('update_settings', { settings: request });
      savedSnapshot = JSON.stringify(result.settings);
      if (reset || JSON.stringify(settings) === requestSnapshot) settings = result.settings;
      i18n.setLocale(result.settings.locale);
      if (result.warning) toastStore.add(result.warning, 'info');
      if (reset) toastStore.add(t.saved, 'success');
    } catch (err) {
      toastStore.add(`${t.saveSettingsFailed}: ${String(err)}`, 'error');
      settings = JSON.parse(savedSnapshot);
    } finally {
      saving = false;
      if (saveAgain) {
        const queuedReset = resetQueued;
        saveAgain = false;
        resetQueued = false;
        void flushSave(queuedReset);
      }
    }
  }

  async function resetSettings() {
    const confirmed = await confirmStore.ask({
      title: t.resetSettings,
      message: t.confirmResetSettings,
      confirmLabel: t.reset,
      destructive: true,
    });
    if (!confirmed) return;

    await flushSave(true);
  }

  async function changeDataLocation() {
    try {
      const selected = await open({
        directory: true,
        multiple: false,
        title: t.selectDataLocation,
      });

      if (selected && typeof selected === 'string') {
        newDataPath = selected;
        deleteOldData = false;
        migrationDialog.showModal();
        document.getElementById(MIGRATION_CANCEL_ID)!.focus();
      }
    } catch (err) {
      console.error('Failed to select directory:', err);
      toastStore.add(t.selectDirectoryFailed, 'error');
    }
  }

  async function confirmMigration() {
    try {
      changingDataPath = true;
      migrationDialog.close();

      const warning = await invoke<string | null>('migrate_data_location', {
        newPath: newDataPath,
        deleteOld: deleteOldData,
      });

      toastStore.add(t.migrationSuccess, 'success');
      if (warning) {
        toastStore.add(warning, 'info');
      }
    } catch (err) {
      toastStore.add(`${t.migrationFailed}: ${String(err)}`, 'error');
    } finally {
      await loadDataPath();
      changingDataPath = false;
    }
  }
</script>

<div class="flex h-screen overflow-hidden bg-background text-foreground">
  <Sidebar bind:activeTab {saving} />

  <main inert={changingDataPath} class="flex-1 overflow-y-auto px-8 py-7">
    {#if loading}
      <div class="flex h-full items-center justify-center">
        <Loader2 class="h-6 w-6 animate-spin text-muted-foreground" />
      </div>
    {:else if settings}
      <div class="mx-auto max-w-2xl">
        {#if activeTab === 'general'}
          <GeneralSettings bind:settings {resetSettings} />
        {:else if activeTab === 'clipboard'}
          <ClipboardSettings bind:settings />
        {:else if activeTab === 'appearance'}
          <AppearanceSettings bind:settings />
        {:else if activeTab === 'storage'}
          <StorageSettings
            {currentDataPath}
            {loadingDataPath}
            {dataPathError}
            {loadDataPath}
            {changingDataPath}
            {changeDataLocation}
          />
        {:else if activeTab === 'about'}
          <AboutSection bind:settings />
        {/if}
      </div>
    {:else}
      <div class="flex h-full flex-col items-center justify-center gap-3">
        <p>{t.loadSettingsFailed}</p>
        <Button onclick={loadSettings}>{t.recheck}</Button>
      </div>
    {/if}
  </main>
</div>

<dialog
  bind:this={migrationDialog}
  aria-labelledby={MIGRATION_DIALOG_TITLE_ID}
  aria-describedby={MIGRATION_DIALOG_DESCRIPTION_ID}
  class="m-auto w-[calc(100%-2rem)] max-w-md space-y-4 rounded-lg border border-border bg-card p-6 text-card-foreground shadow-lg backdrop:bg-background/80 backdrop:backdrop-blur-sm"
>
  <h3 id={MIGRATION_DIALOG_TITLE_ID} class="text-lg font-semibold">{t.confirmMigration}</h3>
  <p id={MIGRATION_DIALOG_DESCRIPTION_ID} class="text-sm text-muted-foreground">
    {t.migratingTo} <br />
    <span class="font-mono bg-muted px-1 rounded">{newDataPath}</span>
  </p>

  <div class="flex items-center space-x-2">
    <input
      type="checkbox"
      id="delete-old"
      bind:checked={deleteOldData}
      class="rounded border-input"
    />
    <label for="delete-old" class="text-sm font-medium">{t.deleteOldData}</label>
  </div>

  <div class="flex justify-end gap-3 pt-2">
    <Button id={MIGRATION_CANCEL_ID} variant="outline" onclick={() => migrationDialog.close()}>
      {t.cancel}
    </Button>
    <Button onclick={confirmMigration} disabled={changingDataPath}>{t.startMigration}</Button>
  </div>
</dialog>
