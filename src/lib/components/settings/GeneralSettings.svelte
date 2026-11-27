<script lang="ts">
  import Card from '$lib/components/ui/Card.svelte';
  import Input from '$lib/components/ui/Input.svelte';
  import Button from '$lib/components/ui/Button.svelte';
  import Switch from '$lib/components/ui/Switch.svelte';
  import { onDestroy, onMount } from 'svelte';
  import { listen } from '@tauri-apps/api/event';
  import { toastStore } from '$lib/stores/toast.svelte';
  import { Keyboard, X } from 'lucide-svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { i18n } from '$lib/i18n';
  import { isMac } from '$lib/utils/platform';
  import { formatShortcut, formatShortcutKey } from '$lib/utils/shortcut';
  import type { Settings } from '$lib/types';

  let { settings = $bindable(), resetSettings } = $props<{
    settings: Settings;
    resetSettings: () => void;
  }>();

  const traySliders = [
    {
      id: 'tray-text-length',
      field: 'trayTextLength',
      labelKey: 'trayTextLength',
      descKey: 'trayTextLengthDesc',
      min: 10,
      max: 200,
      step: 5,
    },
    {
      id: 'max-pinned',
      field: 'maxPinnedInTray',
      labelKey: 'maxPinnedInTray',
      descKey: 'maxPinnedInTrayDesc',
      min: 0,
      max: 50,
      step: 1,
    },
    {
      id: 'max-recent',
      field: 'maxRecentInTray',
      labelKey: 'maxRecentInTray',
      descKey: 'maxRecentInTrayDesc',
      min: 0,
      max: 100,
      step: 1,
    },
  ] as const;

  const t = $derived(i18n.t);

  const shortcutPresets = ['CommandOrControl+Shift+V', 'CommandOrControl+Shift+C', 'Alt+V'];
  const pinnedShortcutPresets = ['CommandOrControl+Shift+P', 'CommandOrControl+Alt+V'];

  let isRecording = $state(false);
  let recordedKeys = $state<string[]>([]);
  let recordingWarning = $state('');
  let recordingTimeout: ReturnType<typeof setTimeout> | undefined;
  const formattedKeys = $derived(formatShortcut(settings.globalShortcut));

  let shortcutTask = Promise.resolve();
  let recordingRevision = 0;

  function startRecording() {
    if (isRecording) return shortcutTask;
    const revision = ++recordingRevision;
    isRecording = true;
    recordedKeys = [];
    recordingWarning = '';
    shortcutTask = shortcutTask
      .then(() => invoke<void>('disable_global_shortcut'))
      .catch((error) => {
        if (revision === recordingRevision) {
          isRecording = false;
        }
        toastStore.add(String(error), 'error');
      });
    return shortcutTask;
  }

  function stopRecording() {
    clearTimeout(recordingTimeout);
    recordingTimeout = undefined;
    if (!isRecording) return shortcutTask;
    recordingRevision++;
    isRecording = false;
    recordedKeys = [];
    recordingWarning = '';
    shortcutTask = shortcutTask
      .then(() => invoke<void>('enable_global_shortcut'))
      .catch((error) => {
        toastStore.add(String(error), 'error');
      });
    return shortcutTask;
  }

  onMount(() => {
    let disposed = false;
    let unlisten: (() => void) | undefined;
    void listen('settings-hidden', () => {
      void stopRecording();
    }).then((stop) => {
      if (disposed) stop();
      else unlisten = stop;
    });
    return () => {
      disposed = true;
      unlisten?.();
    };
  });

  function handleKeyDown(event: KeyboardEvent) {
    if (!isRecording) return;

    event.preventDefault();
    event.stopPropagation();

    if (event.key === 'Escape') {
      void stopRecording();
      return;
    }

    const keys: string[] = [];

    if (event.metaKey || event.ctrlKey) {
      keys.push('CommandOrControl');
    }
    if (event.shiftKey) {
      keys.push('Shift');
    }
    if (event.altKey) {
      keys.push('Alt');
    }

    const mainKey = event.key.toUpperCase();
    if (mainKey.length === 1) {
      keys.push(mainKey);

      const shortcut = keys.join('+');

      if (shortcut === settings.globalShortcut) {
        recordingWarning = t.alreadyCurrentHotkey;
        recordingTimeout = setTimeout(() => {
          void stopRecording();
        }, 2000);
      } else {
        settings.globalShortcut = shortcut;
        void stopRecording();
      }
    } else {
      recordedKeys = keys.map(formatShortcutKey);
      recordingWarning = '';
    }
  }

  function handlePinnedShortcutChange(event: Event) {
    const value = (event.currentTarget as HTMLInputElement).value.trim();
    settings.pinnedShortcut = value.length > 0 ? value : null;
  }

  onDestroy(() => {
    void stopRecording();
  });
</script>

<svelte:window
  onkeydown={handleKeyDown}
  onblur={() => {
    void stopRecording();
  }}
/>

<div class="space-y-6">
  <h2 class="text-lg font-semibold">{t.settingsGeneral}</h2>

  <Card class="p-6 space-y-6">
    <div class="flex items-center justify-between">
      <div class="space-y-0.5">
        <label for="enable-autostart" class="text-sm font-medium cursor-pointer">
          {t.autostart}
        </label>
        <p class="text-xs text-muted-foreground">
          {t.autostartDesc}
        </p>
      </div>
      <Switch id="enable-autostart" bind:checked={settings.enableAutostart} />
    </div>

    <div class="space-y-3">
      <div class="flex items-center justify-between">
        <div class="space-y-1">
          <label for="shortcut-input" class="text-sm font-medium">{t.globalHotkey}</label>
          <p class="text-xs text-muted-foreground">
            {t.globalHotkeyDesc}
          </p>
        </div>
        {#if !isRecording}
          <Button variant="outline" size="sm" onclick={startRecording} class="gap-2">
            <Keyboard class="h-4 w-4" />
            {t.recording}
          </Button>
        {:else}
          <Button variant="outline" size="sm" onclick={stopRecording} class="gap-2">
            <X class="h-4 w-4" />
            {t.cancel}
          </Button>
        {/if}
      </div>

      <div
        class={`flex items-center gap-2 p-3 rounded-lg border transition-all duration-200 ${
          isRecording ? 'bg-primary/10 border-primary animate-pulse' : 'bg-muted/30 border-border'
        }`}
      >
        {#if isRecording}
          <div class="flex-1 text-center">
            {#if recordingWarning}
              <div
                class="flex items-center justify-center gap-2 text-sm text-orange-600 dark:text-orange-400"
              >
                <span>⚠️</span>
                <span>{recordingWarning}</span>
              </div>
            {:else if recordedKeys.length > 0}
              <div class="flex items-center justify-center gap-1.5">
                {#each recordedKeys as label (label)}
                  <kbd
                    class="inline-flex items-center justify-center min-w-[2rem] h-8 px-2.5 text-sm font-semibold
                                               bg-gradient-to-b from-background to-muted
                                               border border-border rounded-md shadow-sm
                                               text-foreground"
                  >
                    {label}
                  </kbd>
                  <span class="text-muted-foreground text-sm">+</span>
                {/each}
                <span class="text-muted-foreground text-sm animate-pulse">?</span>
              </div>
            {:else}
              <div class="flex items-center justify-center gap-2 text-sm text-muted-foreground">
                <Keyboard class="h-4 w-4" />
                <span>{t.recordingHint}</span>
                <span class="text-xs">(ESC {t.cancel})</span>
              </div>
            {/if}
          </div>
        {:else}
          <div class="flex items-center gap-1.5">
            {#each formattedKeys as label, index (index)}
              <kbd
                class="inline-flex items-center justify-center min-w-[2rem] h-8 px-2.5 text-sm font-semibold
                                       bg-gradient-to-b from-background to-muted
                                       border border-border rounded-md shadow-sm
                                       text-foreground"
              >
                {label}
              </kbd>
              {#if index < formattedKeys.length - 1}
                <span class="text-muted-foreground text-sm">+</span>
              {/if}
            {/each}
          </div>
        {/if}
      </div>

      <div class="flex flex-wrap gap-2 pt-1">
        <span class="text-xs text-muted-foreground self-center">{t.commonHotkeys}</span>
        {#each shortcutPresets as preset (preset)}
          <Button
            variant={settings.globalShortcut === preset ? 'default' : 'outline'}
            size="sm"
            onclick={() => (settings.globalShortcut = preset)}
          >
            {formatShortcut(preset).join(isMac ? '' : '+')}
          </Button>
        {/each}
      </div>

      <details class="group">
        <summary
          class="cursor-pointer text-xs text-muted-foreground hover:text-foreground transition-colors select-none"
        >
          {t.advancedManualInput}
        </summary>
        <div class="mt-2">
          <Input
            id="shortcut-input"
            value={settings.globalShortcut}
            onchange={(event) => (settings.globalShortcut = event.currentTarget.value.trim())}
            placeholder="CommandOrControl+Shift+V"
            class="text-sm font-mono"
          />
        </div>
      </details>
    </div>

    <div class="space-y-3 border-t border-border pt-6">
      <div class="space-y-1">
        <label for="pinned-shortcut-input" class="text-sm font-medium">{t.pinnedShortcut}</label>
        <p class="text-xs text-muted-foreground">
          {t.pinnedShortcutDesc}
        </p>
      </div>

      <div class="flex gap-2">
        <Input
          id="pinned-shortcut-input"
          value={settings.pinnedShortcut ?? ''}
          onchange={handlePinnedShortcutChange}
          placeholder="CommandOrControl+Shift+P"
          class="text-sm font-mono"
        />
        <Button
          variant="outline"
          onclick={() => (settings.pinnedShortcut = null)}
          disabled={!settings.pinnedShortcut}
        >
          {t.clear}
        </Button>
      </div>

      <div class="flex flex-wrap gap-2 pt-1">
        <span class="text-xs text-muted-foreground self-center">{t.commonHotkeys}</span>
        {#each pinnedShortcutPresets as preset (preset)}
          <Button
            variant={settings.pinnedShortcut === preset ? 'default' : 'outline'}
            size="sm"
            onclick={() => (settings.pinnedShortcut = preset)}
          >
            {formatShortcut(preset).join(isMac ? '' : '+')}
          </Button>
        {/each}
      </div>
    </div>
  </Card>

  <section class="space-y-3">
    <h3 class="text-sm font-semibold">{t.settingsTray}</h3>
    <Card class="space-y-6 p-6">
      {#each traySliders as slider (slider.id)}
        <div class="space-y-3">
          <div class="flex justify-between">
            <div class="space-y-0.5">
              <label for={slider.id} class="text-sm font-medium">{t[slider.labelKey]}</label>
              <p class="text-xs text-muted-foreground">{t[slider.descKey]}</p>
            </div>
            <span class="text-sm font-semibold tabular-nums">{settings[slider.field]}</span>
          </div>
          <input
            id={slider.id}
            type="range"
            min={slider.min}
            max={slider.max}
            step={slider.step}
            bind:value={settings[slider.field]}
            class="h-2 w-full cursor-pointer appearance-none rounded-lg bg-muted accent-primary"
          />
        </div>
      {/each}
    </Card>
  </section>

  <Card class="flex items-center justify-between gap-4 p-6">
    <div class="space-y-0.5">
      <span class="text-sm font-medium">{t.resetSettings}</span>
      <p class="text-xs text-muted-foreground">{t.resetSettingsDesc}</p>
    </div>
    <Button variant="outline" onclick={resetSettings}>{t.reset}</Button>
  </Card>
</div>
