<script lang="ts">
  import { Settings as SettingsIcon, ClipboardList, Database, Info, Palette } from 'lucide-svelte';
  import { i18n } from '$lib/i18n';
  import type { SettingsTab } from '$lib/types';

  let { activeTab = $bindable(), saving } = $props<{
    activeTab: SettingsTab;
    saving: boolean;
  }>();

  const t = $derived(i18n.t);

  const tabConfig = [
    { id: 'general', labelKey: 'settingsGeneral', icon: SettingsIcon },
    { id: 'appearance', labelKey: 'settingsAppearance', icon: Palette },
    { id: 'clipboard', labelKey: 'settingsClipboard', icon: ClipboardList },
    { id: 'storage', labelKey: 'settingsStorage', icon: Database },
    { id: 'about', labelKey: 'settingsAbout', icon: Info },
  ] as const;
</script>

<aside class="flex w-52 flex-none flex-col border-r border-border bg-muted/30 px-3 py-5">
  <nav class="space-y-0.5">
    {#each tabConfig as tab (tab.id)}
      <button
        type="button"
        aria-current={activeTab === tab.id ? 'page' : undefined}
        class={[
          'flex w-full items-center gap-2.5 rounded-md px-2.5 py-1.5 text-sm transition-colors',
          activeTab === tab.id
            ? 'bg-accent font-medium text-foreground'
            : 'text-muted-foreground hover:bg-accent/60 hover:text-foreground',
        ]}
        onclick={() => (activeTab = tab.id)}
      >
        <tab.icon class="h-4 w-4" />
        {t[tab.labelKey]}
      </button>
    {/each}
  </nav>
  <p class="mt-auto px-2.5 text-[11px] text-muted-foreground" role="status">
    {saving ? t.saving : t.settingsAutoSaved}
  </p>
</aside>
