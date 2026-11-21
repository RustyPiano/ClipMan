<script lang="ts">
  import {
    Settings as SettingsIcon,
    ClipboardList,
    Menu,
    Database,
    Info,
    Palette,
  } from 'lucide-svelte';
  import { i18n } from '$lib/i18n';
  import type { SettingsTab } from '$lib/types';

  let { activeTab = $bindable() } = $props<{
    activeTab: SettingsTab;
  }>();

  const t = $derived(i18n.t);

  const tabConfig = [
    { id: 'general', labelKey: 'settingsGeneral', icon: SettingsIcon },
    { id: 'appearance', labelKey: 'settingsAppearance', icon: Palette },
    { id: 'clipboard', labelKey: 'settingsClipboard', icon: ClipboardList },
    { id: 'tray', labelKey: 'settingsTray', icon: Menu },
    { id: 'storage', labelKey: 'settingsStorage', icon: Database },
    { id: 'about', labelKey: 'settingsAbout', icon: Info },
  ] as const;
</script>

<aside class="w-64 border-r border-border bg-muted/30 p-4 flex flex-col">
  <nav class="space-y-1">
    {#each tabConfig as tab (tab.id)}
      <button
        class="w-full flex items-center gap-3 px-3 py-2 text-sm font-medium rounded-md transition-colors
                {activeTab === tab.id
          ? 'bg-primary text-primary-foreground shadow-sm'
          : 'text-muted-foreground hover:bg-muted hover:text-foreground'}"
        onclick={() => (activeTab = tab.id)}
      >
        <tab.icon class="h-4 w-4" />
        {t[tab.labelKey]}
      </button>
    {/each}
  </nav>
</aside>
