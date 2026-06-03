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

  // Tabs with icons - labels are computed from i18n
  const tabConfig = [
    { id: 'general' as const, icon: SettingsIcon },
    { id: 'appearance' as const, icon: Palette },
    { id: 'clipboard' as const, icon: ClipboardList },
    { id: 'tray' as const, icon: Menu },
    { id: 'storage' as const, icon: Database },
    { id: 'about' as const, icon: Info },
  ];

  function getTabLabel(id: SettingsTab): string {
    const labels: Record<SettingsTab, string> = {
      general: t.settingsGeneral,
      appearance: t.settingsAppearance,
      clipboard: t.settingsClipboard,
      tray: t.settingsTray,
      storage: t.settingsStorage,
      about: t.settingsAbout,
    };
    return labels[id];
  }
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
        {getTabLabel(tab.id)}
      </button>
    {/each}
  </nav>
</aside>
