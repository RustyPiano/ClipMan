<script lang="ts">
  import Card from '$lib/components/ui/Card.svelte';
  import Input from '$lib/components/ui/Input.svelte';
  import Button from '$lib/components/ui/Button.svelte';
  import Switch from '$lib/components/ui/Switch.svelte';
  import { onDestroy } from 'svelte';
  import { Keyboard, X } from 'lucide-svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { i18n } from '$lib/i18n';
  import { isMac } from '$lib/utils/platform';
  import type { Settings } from '$lib/types';

  let { settings = $bindable() } = $props<{
    settings: Settings;
  }>();

  const t = $derived(i18n.t);

  // Dynamic shortcut presets based on OS
  const shortcutPresets = [
    {
      label: isMac ? '⌘⇧V' : 'Ctrl+Shift+V',
      value: 'CommandOrControl+Shift+V',
    },
    {
      label: isMac ? '⌘⇧C' : 'Ctrl+Shift+C',
      value: 'CommandOrControl+Shift+C',
    },
    {
      label: isMac ? '⌥V' : 'Alt+V',
      value: 'Alt+V',
    },
  ];

  const pinnedShortcutPresets = [
    {
      label: isMac ? '⌘⇧P' : 'Ctrl+Shift+P',
      value: 'CommandOrControl+Shift+P',
    },
    {
      label: isMac ? '⌘⌥V' : 'Ctrl+Alt+V',
      value: 'CommandOrControl+Alt+V',
    },
  ];

  interface ShortcutKey {
    id: string;
    label: string;
  }

  // Convert Tauri shortcut to display format
  function formatShortcut(shortcut: string): ShortcutKey[] {
    if (!shortcut) return [];

    const keys = shortcut.split('+');
    return keys.map((key, index) => ({
      id: `${index}:${key}`,
      label: formatShortcutKey(key),
    }));
  }

  function formatShortcutKey(key: string): string {
    switch (key) {
      case 'CommandOrControl':
        return isMac ? '⌘' : 'Ctrl';
      case 'Command':
        return '⌘';
      case 'Control':
        return isMac ? '⌃' : 'Ctrl';
      case 'Alt':
        return isMac ? '⌥' : 'Alt';
      case 'Option':
        return '⌥';
      case 'Shift':
        return isMac ? '⇧' : 'Shift';
      default:
        return key;
    }
  }

  // Keyboard recording state
  let isRecording = $state(false);
  let recordedKeys = $state<ShortcutKey[]>([]);
  let recordingWarning = $state('');
  let recordingTimeout: ReturnType<typeof setTimeout> | undefined;
  const formattedKeys = $derived(formatShortcut(settings.globalShortcut));

  async function startRecording() {
    try {
      // Disable global shortcut to prevent triggering during recording
      await invoke('disable_global_shortcut');
      isRecording = true;
      recordedKeys = [];
      recordingWarning = '';
    } catch (err) {
      console.error('Failed to disable shortcut:', err);
      recordingWarning = t.disableHotkeyFailed;
    }
  }

  async function stopRecording() {
    clearTimeout(recordingTimeout);
    recordingTimeout = undefined;

    try {
      // Re-enable global shortcut
      if (isRecording) {
        await invoke('enable_global_shortcut');
      }
    } catch (err) {
      console.error('Failed to re-enable shortcut:', err);
    } finally {
      isRecording = false;
      recordedKeys = [];
      recordingWarning = '';
    }
  }

  function handleKeyDown(event: KeyboardEvent) {
    if (!isRecording) return;

    event.preventDefault();
    event.stopPropagation();

    // ESC to cancel
    if (event.key === 'Escape') {
      void stopRecording();
      return;
    }

    // Build key combination
    const keys: string[] = [];

    // Add modifiers
    if (event.metaKey || event.ctrlKey) {
      keys.push('CommandOrControl');
    }
    if (event.shiftKey) {
      keys.push('Shift');
    }
    if (event.altKey) {
      keys.push('Alt');
    }

    // Add main key (ignore pure modifiers)
    const mainKey = event.key.toUpperCase();
    if (
      mainKey !== 'META' &&
      mainKey !== 'CONTROL' &&
      mainKey !== 'SHIFT' &&
      mainKey !== 'ALT' &&
      mainKey !== 'ESCAPE' &&
      mainKey.length === 1
    ) {
      keys.push(mainKey);

      // Build the shortcut string
      const shortcut = keys.join('+');

      // Check if it's the same as current shortcut
      if (shortcut === settings.globalShortcut) {
        recordingWarning = t.alreadyCurrentHotkey;
        // Auto-close warning after 2 seconds
        recordingTimeout = setTimeout(() => {
          void stopRecording();
        }, 2000);
      } else {
        // Set the new shortcut
        settings.globalShortcut = shortcut;
        void stopRecording();
      }
    } else {
      // Just show modifiers while waiting for main key
      recordedKeys = keys.map((key, index) => ({
        id: `${index}:${key}`,
        label: formatShortcutKey(key),
      }));
      recordingWarning = '';
    }
  }

  function handlePinnedShortcutInput(event: Event) {
    const value = (event.currentTarget as HTMLInputElement).value.trim();
    settings.pinnedShortcut = value.length > 0 ? value : null;
  }

  onDestroy(() => {
    clearTimeout(recordingTimeout);
    if (isRecording) {
      void invoke('enable_global_shortcut');
    }
  });
</script>

<svelte:window onkeydown={handleKeyDown} />

<div class="space-y-6 animate-in fade-in slide-in-from-bottom-4 duration-300">
  <div>
    <h2 class="text-lg font-semibold mb-1">{t.settingsGeneral}</h2>
    <p class="text-sm text-muted-foreground">{t.globalHotkeyDesc}</p>
  </div>

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
          <Button type="button" variant="outline" size="sm" onclick={startRecording} class="gap-2">
            <Keyboard class="h-4 w-4" />
            {t.recording}
          </Button>
        {:else}
          <Button type="button" variant="outline" size="sm" onclick={stopRecording} class="gap-2">
            <X class="h-4 w-4" />
            {t.cancel}
          </Button>
        {/if}
      </div>

      <!-- Keyboard key display or recording prompt -->
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
                {#each recordedKeys as key (key.id)}
                  <kbd
                    class="inline-flex items-center justify-center min-w-[2rem] h-8 px-2.5 text-sm font-semibold
                                               bg-gradient-to-b from-background to-muted
                                               border border-border rounded-md shadow-sm
                                               text-foreground"
                  >
                    {key.label}
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
            {#each formattedKeys as key, index (key.id)}
              <kbd
                class="inline-flex items-center justify-center min-w-[2rem] h-8 px-2.5 text-sm font-semibold
                                       bg-gradient-to-b from-background to-muted
                                       border border-border rounded-md shadow-sm
                                       text-foreground"
              >
                {key.label}
              </kbd>
              {#if index < formattedKeys.length - 1}
                <span class="text-muted-foreground text-sm">+</span>
              {/if}
            {/each}
          </div>
        {/if}
      </div>

      <!-- Preset shortcuts -->
      <div class="flex flex-wrap gap-2 pt-1">
        <span class="text-xs text-muted-foreground self-center">{t.commonHotkeys}</span>
        {#each shortcutPresets as preset (preset.value)}
          <Button
            type="button"
            variant={settings.globalShortcut === preset.value ? 'default' : 'outline'}
            size="sm"
            onclick={() => (settings.globalShortcut = preset.value)}
          >
            {preset.label}
          </Button>
        {/each}
      </div>

      <!-- Advanced: Manual input -->
      <details class="group">
        <summary
          class="cursor-pointer text-xs text-muted-foreground hover:text-foreground transition-colors select-none"
        >
          {t.advancedManualInput}
        </summary>
        <div class="mt-2">
          <Input
            id="shortcut-input"
            type="text"
            bind:value={settings.globalShortcut}
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
          type="text"
          value={settings.pinnedShortcut ?? ''}
          oninput={handlePinnedShortcutInput}
          placeholder="CommandOrControl+Shift+P"
          class="text-sm font-mono"
        />
        <Button
          type="button"
          variant="outline"
          onclick={() => (settings.pinnedShortcut = null)}
          disabled={!settings.pinnedShortcut}
        >
          {t.clear}
        </Button>
      </div>

      <div class="flex flex-wrap gap-2 pt-1">
        <span class="text-xs text-muted-foreground self-center">{t.commonHotkeys}</span>
        {#each pinnedShortcutPresets as preset (preset.value)}
          <Button
            type="button"
            variant={settings.pinnedShortcut === preset.value ? 'default' : 'outline'}
            size="sm"
            onclick={() => (settings.pinnedShortcut = preset.value)}
          >
            {preset.label}
          </Button>
        {/each}
      </div>
    </div>
  </Card>
</div>
