<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { onMount } from 'svelte';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { i18n } from '$lib/i18n';
  import { toastStore } from '$lib/stores/toast.svelte';
  import { confirmStore } from '$lib/stores/confirm.svelte';
  import Button from '$lib/components/ui/Button.svelte';
  import { ChevronLeft, Loader2, Save, RotateCcw } from 'lucide-svelte';
  import { open } from '@tauri-apps/plugin-dialog';
  import type { Settings, UpdateInfo, SettingsTab } from '$lib/types';

  // Import modularized components
  import Sidebar from '$lib/components/settings/Sidebar.svelte';
  import GeneralSettings from '$lib/components/settings/GeneralSettings.svelte';
  import ClipboardSettings from '$lib/components/settings/ClipboardSettings.svelte';
  import TraySettings from '$lib/components/settings/TraySettings.svelte';
  import StorageSettings from '$lib/components/settings/StorageSettings.svelte';
  import AboutSection from '$lib/components/settings/AboutSection.svelte';
  import AppearanceSettings from '$lib/components/settings/AppearanceSettings.svelte';

  const t = $derived(i18n.t);

  let settings = $state<Settings | null>(null);

  let loading = $state(true);
  let saving = $state(false);

  // 更新相关状态
  let updateInfo = $state<UpdateInfo | null>(null);
  let checkingUpdate = $state(false);
  let installingUpdate = $state(false);
  let updateMessage = $state('');

  // 数据位置相关状态
  let currentDataPath = $state('');
  let changingDataPath = $state(false);
  let showMigrationDialog = $state(false);
  let newDataPath = $state('');
  let deleteOldData = $state(false);
  let migrationDialog: HTMLDialogElement;

  const MIGRATION_DIALOG_TITLE_ID = 'migration-dialog-title';
  const MIGRATION_DIALOG_DESCRIPTION_ID = 'migration-dialog-description';
  const MIGRATION_CANCEL_ID = 'migration-dialog-cancel';

  // 侧边栏导航状态
  let activeTab = $state<SettingsTab>('general');

  onMount(async () => {
    // Load settings and data path in parallel for better performance
    await Promise.all([loadSettings(), loadDataPath()]);
  });

  async function loadSettings() {
    try {
      loading = true;
      settings = await invoke<Settings>('get_settings');
      i18n.setLocale(settings.locale);
    } catch (err) {
      console.error('Failed to load settings:', err);
      const errorMsg = err instanceof Error ? err.message : String(err);
      toastStore.add(`${t.loadSettingsFailed}: ${errorMsg}`, 'error');
    } finally {
      loading = false;
    }
  }

  async function loadDataPath() {
    try {
      currentDataPath = await invoke<string>('get_current_data_path');
    } catch (err) {
      console.error('Failed to load data path:', err);
      currentDataPath = '';
    }
  }

  async function saveSettings(reset = false) {
    if (saving || !settings) return;
    try {
      saving = true;
      const result = await invoke<{ settings: Settings; warning: string | null }>(
        'update_settings',
        { settings: reset ? null : settings }
      );
      settings = result.settings;
      i18n.setLocale(settings.locale);
      toastStore.add(result.warning ?? t.saved, result.warning ? 'info' : 'success');
    } catch (err) {
      console.error('Failed to save settings:', err);
      const errorMsg = err instanceof Error ? err.message : String(err);
      toastStore.add(`${t.saveSettingsFailed}: ${errorMsg}`, 'error');
    } finally {
      saving = false;
    }
  }

  async function resetSettings() {
    const confirmed = await confirmStore.ask({
      title: t.reset,
      message: t.confirmResetSettings,
      confirmLabel: t.reset,
      destructive: true,
    });
    if (!confirmed) return;

    await saveSettings(true);
  }

  async function checkForUpdates() {
    try {
      checkingUpdate = true;
      updateMessage = '';
      updateInfo = await invoke<UpdateInfo>('check_for_updates');
      if (updateInfo.available) {
        updateMessage = `${t.updateAvailable}: ${updateInfo.latest_version}`;
      } else {
        updateMessage = t.noUpdateAvailable;
      }
    } catch (err) {
      console.error('Check update failed:', err);
      updateMessage = t.checkUpdateFailed;
    } finally {
      checkingUpdate = false;
    }
  }

  async function installUpdate() {
    if (!updateInfo?.available) return;

    try {
      installingUpdate = true;
      updateMessage = t.downloadingUpdate;
      await invoke('install_update');
      updateMessage = t.updateInstalled;
    } catch (err) {
      console.error('Install update failed:', err);
      updateMessage = `${t.installUpdateFailed}: ${String(err)}`;
    } finally {
      installingUpdate = false;
    }
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
        showMigrationDialog = true;
      }
    } catch (err) {
      console.error('Failed to select directory:', err);
      toastStore.add(t.selectDirectoryFailed, 'error');
    }
  }

  async function confirmMigration() {
    try {
      changingDataPath = true;
      showMigrationDialog = false;

      const warning = await invoke<string | null>('migrate_data_location', {
        newPath: newDataPath,
        deleteOld: deleteOldData,
      });

      toastStore.add(t.migrationSuccess, 'success');
      if (warning) {
        toastStore.add(warning, 'info');
      }
    } catch (err) {
      console.error('Migration failed:', err);
      const errorMsg = err instanceof Error ? err.message : String(err);
      toastStore.add(`${t.migrationFailed}: ${errorMsg}`, 'error');
    } finally {
      // 迁移只改变数据路径；不重新读取其他设置，保留用户尚未保存的修改。
      await loadDataPath();
      changingDataPath = false;
    }
  }

  function closeMigrationDialog() {
    showMigrationDialog = false;
  }

  // showModal 让背景不可交互、支持 Esc 取消；打开时把焦点放到取消按钮上。
  $effect(() => {
    if (showMigrationDialog) {
      migrationDialog.showModal();
      document.getElementById(MIGRATION_CANCEL_ID)!.focus();
    } else if (migrationDialog.open) {
      migrationDialog.close();
    }
  });

  async function handleBack() {
    try {
      const win = getCurrentWindow();
      await win.emit('settings-hidden');
      await invoke('show_quickbar');
      await win.hide();
    } catch (err) {
      console.error('Failed to handle back navigation:', err);
      toastStore.add(String(err), 'error');
    }
  }
</script>

<div class="h-screen flex flex-col bg-background text-foreground overflow-hidden">
  <!-- 顶部标题栏 -->
  <header
    class="flex-none flex items-center justify-between px-6 py-4 border-b border-border bg-background/95 backdrop-blur supports-[backdrop-filter]:bg-background/60 z-10"
  >
    <div class="flex items-center gap-4">
      <Button
        variant="ghost"
        size="icon"
        onclick={handleBack}
        aria-label={t.close}
        class="hover:bg-muted rounded-full"
      >
        <ChevronLeft class="h-5 w-5" />
      </Button>
      <h1 class="text-xl font-bold tracking-tight">{t.settings}</h1>
    </div>

    <div class="flex items-center gap-2">
      <Button
        variant="outline"
        onclick={resetSettings}
        disabled={!settings || loading || saving || changingDataPath}
        class="gap-2"
      >
        <RotateCcw class="h-4 w-4" />
        {t.reset}
      </Button>
      <Button
        onclick={() => saveSettings()}
        disabled={!settings || loading || saving || changingDataPath}
        class="gap-2 min-w-[100px]"
      >
        {#if saving}
          <Loader2 class="h-4 w-4 animate-spin" />
          {t.saving}
        {:else}
          <Save class="h-4 w-4" />
          {t.save}
        {/if}
      </Button>
    </div>
  </header>

  <div class="flex-1 flex overflow-hidden">
    <!-- 侧边栏导航 -->
    <Sidebar bind:activeTab />

    <!-- 主内容区域 -->
    <main inert={saving || changingDataPath} class="flex-1 overflow-y-auto p-8 bg-muted/10">
      {#if loading}
        <div class="flex items-center justify-center h-full">
          <Loader2 class="h-8 w-8 animate-spin text-primary" />
        </div>
      {:else if settings}
        <div class="max-w-2xl mx-auto space-y-6">
          {#if activeTab === 'general'}
            <GeneralSettings bind:settings />
          {:else if activeTab === 'clipboard'}
            <ClipboardSettings bind:settings />
          {:else if activeTab === 'appearance'}
            <AppearanceSettings bind:settings />
          {:else if activeTab === 'tray'}
            <TraySettings bind:settings />
          {:else if activeTab === 'storage'}
            <StorageSettings {currentDataPath} {changingDataPath} {changeDataLocation} />
          {:else if activeTab === 'about'}
            <AboutSection
              {updateInfo}
              {checkingUpdate}
              {installingUpdate}
              {updateMessage}
              {checkForUpdates}
              {installUpdate}
            />
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
</div>

<!-- 数据迁移确认对话框 -->
<dialog
  bind:this={migrationDialog}
  aria-labelledby={MIGRATION_DIALOG_TITLE_ID}
  aria-describedby={MIGRATION_DIALOG_DESCRIPTION_ID}
  oncancel={closeMigrationDialog}
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
    <Button id={MIGRATION_CANCEL_ID} variant="outline" onclick={closeMigrationDialog}>
      {t.cancel}
    </Button>
    <Button onclick={confirmMigration} disabled={changingDataPath}>{t.startMigration}</Button>
  </div>
</dialog>
