<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { onMount } from 'svelte';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { router } from '$lib/stores/router.svelte';
  import { i18n } from '$lib/i18n';
  import { toastStore } from '$lib/stores/toast.svelte';
  import { confirmStore } from '$lib/stores/confirm.svelte';
  import Button from '$lib/components/ui/Button.svelte';
  import { ChevronLeft, Loader2, Save, RotateCcw } from 'lucide-svelte';
  import { open } from '@tauri-apps/plugin-dialog';
  import type { Settings, UpdateInfo, SettingsTab } from '$lib/types';
  import { normalizeSettingsLocale } from '$lib/utils/settings';

  // Import modularized components
  import Sidebar from '$lib/components/settings/Sidebar.svelte';
  import GeneralSettings from '$lib/components/settings/GeneralSettings.svelte';
  import ClipboardSettings from '$lib/components/settings/ClipboardSettings.svelte';
  import TraySettings from '$lib/components/settings/TraySettings.svelte';
  import StorageSettings from '$lib/components/settings/StorageSettings.svelte';
  import AboutSection from '$lib/components/settings/AboutSection.svelte';
  import AppearanceSettings from '$lib/components/settings/AppearanceSettings.svelte';

  const t = $derived(i18n.t);

  let settings = $state<Settings>({
    globalShortcut: 'CommandOrControl+Shift+V',
    autoPaste: true,
    ignoreConcealed: true,
    pinnedShortcut: null,
    maxHistoryItems: 100,
    trayTextLength: 70,
    maxPinnedInTray: 5,
    maxRecentInTray: 20,
    customDataPath: null,
    enableAutostart: false,
    locale: 'zh-CN',
    ignoredApps: [],
    skipSecrets: true,
    maxTextBytes: 2000000,
    maxImageDimension: 4096,
    capturePaused: false,
  });

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
  let deleteOldData = $state(true);

  // 侧边栏导航状态
  let activeTab = $state<SettingsTab>('general');

  onMount(async () => {
    // Load settings and data path in parallel for better performance
    await Promise.all([loadSettings(), loadDataPath()]);
  });

  async function loadSettings({ preserveMessage = false }: { preserveMessage?: boolean } = {}) {
    try {
      loading = true;
      const data = await invoke<Settings>('get_settings');
      const normalized = normalizeSettingsLocale(data);
      if (normalized.needsSave) {
        console.warn(
          `[WARNING] Invalid locale loaded from backend: ${
            (data as Settings & { locale: unknown }).locale
          }, falling back to 'zh-CN'`
        );
      }
      settings = normalized.settings;
      if (settings.locale !== i18n.locale) {
        i18n.setLocale(settings.locale);
      }
      if (normalized.needsSave) {
        // If we corrected the locale, update backend settings immediately to fix the file on disk
        await invoke('update_settings', { settings: settings });
      }
    } catch (err) {
      console.error('Failed to load settings:', err);
      const errorMsg = err instanceof Error ? err.message : String(err);
      if (!preserveMessage) {
        toastStore.add(`${t.loadSettingsFailed}: ${errorMsg}`, 'error');
      }
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

  async function saveSettings() {
    try {
      saving = true;
      await invoke('update_settings', { settings: settings });
      i18n.setLocale(settings.locale);
      toastStore.add(t.saved, 'success');
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

    // saveSettings handles its own success/error toasts, so no try/catch here.
    settings = {
      globalShortcut: 'CommandOrControl+Shift+V',
      autoPaste: true,
      ignoreConcealed: true,
      pinnedShortcut: null,
      maxHistoryItems: 100,
      trayTextLength: 70,
      maxPinnedInTray: 5,
      maxRecentInTray: 20,
      customDataPath: null,
      enableAutostart: false,
      locale: 'zh-CN',
      ignoredApps: [],
      skipSecrets: true,
      maxTextBytes: 2000000,
      maxImageDimension: 4096,
      // Reset restores capture; the tray toggle owns this at runtime.
      capturePaused: false,
    };
    await saveSettings();
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

      await invoke('migrate_data_location', {
        newPath: newDataPath,
        deleteOld: deleteOldData,
      });

      toastStore.add(t.migrationSuccess, 'success');
    } catch (err) {
      console.error('Migration failed:', err);
      const errorMsg = err instanceof Error ? err.message : String(err);
      toastStore.add(`${t.migrationFailed}: ${errorMsg}`, 'error');
    } finally {
      await Promise.all([loadSettings({ preserveMessage: true }), loadDataPath()]);
      changingDataPath = false;
    }
  }

  async function handleBack() {
    try {
      const win = getCurrentWindow();
      if (win.label === 'settings') {
        await invoke('show_quickbar');
        await win.hide();
      } else {
        router.goHome();
      }
    } catch (err) {
      console.error('Failed to handle back navigation:', err);
      router.goHome();
    }
  }
</script>

<div class="h-screen flex flex-col bg-background text-foreground overflow-hidden">
  <!-- 顶部标题栏 -->
  <header
    class="flex-none flex items-center justify-between px-6 py-4 border-b border-border bg-background/95 backdrop-blur supports-[backdrop-filter]:bg-background/60 z-10"
  >
    <div class="flex items-center gap-4">
      <Button variant="ghost" size="icon" onclick={handleBack} class="hover:bg-muted rounded-full">
        <ChevronLeft class="h-5 w-5" />
      </Button>
      <h1 class="text-xl font-bold tracking-tight">{t.settings}</h1>
    </div>

    <div class="flex items-center gap-2">
      <Button
        variant="outline"
        onclick={resetSettings}
        disabled={loading || saving || changingDataPath}
        class="gap-2"
      >
        <RotateCcw class="h-4 w-4" />
        {t.reset}
      </Button>
      <Button
        onclick={saveSettings}
        disabled={loading || saving || changingDataPath}
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
    <main class="flex-1 overflow-y-auto p-8 bg-muted/10">
      {#if loading}
        <div class="flex items-center justify-center h-full">
          <Loader2 class="h-8 w-8 animate-spin text-primary" />
        </div>
      {:else}
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
      {/if}
    </main>
  </div>
</div>

<!-- 数据迁移确认对话框 -->
{#if showMigrationDialog}
  <div
    class="fixed inset-0 bg-background/80 backdrop-blur-sm z-50 flex items-center justify-center p-4"
  >
    <div
      class="bg-card text-card-foreground rounded-lg shadow-lg max-w-md w-full border border-border p-6 space-y-4 animate-in zoom-in-95 duration-200"
    >
      <h3 class="text-lg font-semibold">{t.confirmMigration}</h3>
      <p class="text-sm text-muted-foreground">
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
        <Button variant="outline" onclick={() => (showMigrationDialog = false)}>
          {t.cancel}
        </Button>
        <Button onclick={confirmMigration} disabled={changingDataPath}>{t.startMigration}</Button>
      </div>
    </div>
  </div>
{/if}
