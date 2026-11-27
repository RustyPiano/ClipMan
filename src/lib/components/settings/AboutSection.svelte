<script lang="ts">
  import Card from '$lib/components/ui/Card.svelte';
  import Button from '$lib/components/ui/Button.svelte';
  import Switch from '$lib/components/ui/Switch.svelte';
  import { Loader2, RefreshCw, Download, RotateCw, ClipboardCopy, FolderOpen } from 'lucide-svelte';
  import { getVersion } from '@tauri-apps/api/app';
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import { onMount } from 'svelte';
  import { i18n } from '$lib/i18n';
  import { toastStore } from '$lib/stores/toast.svelte';
  import appIcon from '../../../../src-tauri/icons/128x128.png';
  import type { Settings, UpdateInfo, UpdateProgress } from '$lib/types';

  let { settings = $bindable() } = $props<{ settings: Settings }>();

  const t = $derived(i18n.t);

  let currentVersion = $state('');
  let status = $state<UpdateInfo | null>(null);
  let checking = $state(false);
  let installing = $state(false);
  let progress = $state<UpdateProgress | null>(null);
  let message = $state('');

  const progressPercent = $derived(
    progress?.total ? Math.min(100, Math.round((progress.downloaded / progress.total) * 100)) : null
  );

  onMount(() => {
    void loadStatus();
    const unlisteners: (() => void)[] = [];
    let disposed = false;
    for (const subscription of [
      listen<UpdateInfo>('update-status', (event) => (status = event.payload)),
      listen<UpdateProgress>('update-progress', (event) => (progress = event.payload)),
    ])
      void subscription.then((stop) => (disposed ? stop() : unlisteners.push(stop)));
    return () => {
      disposed = true;
      unlisteners.forEach((stop) => stop());
    };
  });

  async function loadStatus() {
    try {
      [currentVersion, status] = await Promise.all([
        getVersion(),
        invoke<UpdateInfo>('get_update_status'),
      ]);
    } catch (err) {
      toastStore.add(String(err), 'error');
    }
  }

  async function checkForUpdates() {
    checking = true;
    message = '';
    try {
      status = await invoke<UpdateInfo>('check_for_updates');
      if (!status.available) message = t.noUpdateAvailable;
    } catch (err) {
      message = `${t.checkUpdateFailed}: ${String(err)}`;
    } finally {
      checking = false;
    }
  }

  async function installUpdate() {
    installing = true;
    progress = null;
    message = '';
    try {
      await invoke('install_update');
      status = await invoke<UpdateInfo>('get_update_status');
    } catch (err) {
      message = `${t.installUpdateFailed}: ${String(err)}`;
    } finally {
      installing = false;
    }
  }

  async function restart() {
    try {
      await invoke('restart_app');
    } catch (err) {
      toastStore.add(String(err), 'error');
    }
  }

  async function copyDiagnostics() {
    try {
      const text = await invoke<string>('get_diagnostics');
      await invoke('copy_plain_text', { text });
      toastStore.add(t.diagnosticsCopied, 'success');
    } catch (err) {
      toastStore.add(`${t.operationFailed}: ${String(err)}`, 'error');
    }
  }

  async function openLogs() {
    try {
      await invoke('open_log_folder');
    } catch (err) {
      toastStore.add(`${t.operationFailed}: ${String(err)}`, 'error');
    }
  }
</script>

<div class="space-y-6">
  <h2 class="text-lg font-semibold">{t.settingsAbout}</h2>

  <Card class="space-y-5 p-6">
    <div class="flex items-center gap-4">
      <img src={appIcon} alt="" class="h-12 w-12" />
      <div>
        <h3 class="text-base font-semibold">{t.appName}</h3>
        <p class="text-sm text-muted-foreground">{t.appTagline}</p>
        <p class="mt-1 flex items-center gap-2 text-xs text-muted-foreground">
          {#if currentVersion}<span class="font-mono">v{currentVersion}</span>{/if}
          <a
            href="https://github.com/RustyPiano/ClipLeaf"
            target="_blank"
            class="text-primary hover:underline">GitHub</a
          >
        </p>
      </div>
    </div>

    <div class="space-y-3 border-t border-border pt-5">
      {#if status?.available && status.latestVersion}
        <div class="flex items-center justify-between text-sm">
          <span class="font-medium">{t.updateAvailable}</span>
          <span class="font-mono">{status.latestVersion}</span>
        </div>
        {#if status.body}
          <div class="max-h-48 overflow-y-auto rounded-md border border-border bg-muted/40 p-3">
            <p class="mb-1 text-xs font-medium text-muted-foreground">{t.releaseNotes}</p>
            <p class="whitespace-pre-wrap text-sm">{status.body}</p>
          </div>
        {/if}
      {/if}

      {#if installing}
        <div class="space-y-1.5" role="status">
          <p class="text-xs text-muted-foreground">
            {progressPercent === null
              ? t.downloadingUpdate
              : i18n.format(t.downloadProgress, { percent: progressPercent })}
          </p>
          <div class="h-1.5 overflow-hidden rounded-full bg-muted">
            <div
              class="h-full bg-primary transition-[width]"
              style:width={progressPercent === null ? '30%' : `${progressPercent}%`}
            ></div>
          </div>
        </div>
      {/if}

      {#if status?.readyToRestart}
        <div class="flex items-center justify-between gap-3 text-sm" role="status">
          <span>{t.updateInstalled}</span>
          <Button size="sm" class="gap-1.5" onclick={restart}>
            <RotateCw class="h-3.5 w-3.5" />{t.restartNow}
          </Button>
        </div>
      {:else}
        <div class="flex gap-2">
          <Button
            variant="secondary"
            class="flex-1 gap-2"
            onclick={checkForUpdates}
            disabled={checking || installing}
          >
            {#if checking}
              <Loader2 class="h-4 w-4 animate-spin" /> {t.checking}
            {:else}
              <RefreshCw class="h-4 w-4" /> {t.checkUpdate}
            {/if}
          </Button>
          {#if status?.available}
            <Button class="flex-1 gap-2" onclick={installUpdate} disabled={installing}>
              {#if installing}
                <Loader2 class="h-4 w-4 animate-spin" /> {t.installing}
              {:else}
                <Download class="h-4 w-4" /> {t.installUpdate}
              {/if}
            </Button>
          {/if}
        </div>
      {/if}

      {#if message}
        <p class="text-center text-sm text-muted-foreground" role="status">{message}</p>
      {/if}
    </div>

    <div class="flex items-center justify-between gap-4 border-t border-border pt-5">
      <div class="space-y-0.5">
        <label for="auto-check-updates" class="cursor-pointer text-sm font-medium">
          {t.autoCheckUpdates}
        </label>
        <p class="text-xs text-muted-foreground">{t.autoCheckUpdatesDesc}</p>
      </div>
      <Switch id="auto-check-updates" bind:checked={settings.autoCheckUpdates} />
    </div>
  </Card>

  <Card class="space-y-3 p-6">
    <div class="space-y-0.5">
      <span class="text-sm font-medium">{t.diagnostics}</span>
      <p class="text-xs text-muted-foreground">{t.diagnosticsDesc}</p>
    </div>
    <div class="flex gap-2">
      <Button variant="outline" size="sm" class="gap-1.5" onclick={copyDiagnostics}>
        <ClipboardCopy class="h-3.5 w-3.5" />{t.copyDiagnostics}
      </Button>
      <Button variant="outline" size="sm" class="gap-1.5" onclick={openLogs}>
        <FolderOpen class="h-3.5 w-3.5" />{t.openLogs}
      </Button>
    </div>
  </Card>
</div>
