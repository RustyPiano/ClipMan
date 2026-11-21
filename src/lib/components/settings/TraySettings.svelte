<script lang="ts">
  import Card from '$lib/components/ui/Card.svelte';
  import { i18n } from '$lib/i18n';
  import type { Settings } from '$lib/types';

  let { settings = $bindable() } = $props<{
    settings: Settings;
  }>();

  const t = $derived(i18n.t);
  const sliders = [
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
</script>

<div class="space-y-6 duration-300">
  <h2 class="text-lg font-semibold">{t.settingsTray}</h2>

  <Card class="p-6 space-y-6">
    {#each sliders as slider (slider.id)}
      <div class="space-y-4">
        <div class="flex justify-between">
          <label for={slider.id} class="text-sm font-medium">{t[slider.labelKey]}</label>
          <span class="text-sm font-bold text-primary">{settings[slider.field]}</span>
        </div>
        <input
          id={slider.id}
          type="range"
          min={slider.min}
          max={slider.max}
          step={slider.step}
          bind:value={settings[slider.field]}
          class="w-full accent-primary h-2 bg-muted rounded-lg appearance-none cursor-pointer"
        />
        <p class="text-xs text-muted-foreground">
          {t[slider.descKey]}
        </p>
      </div>
    {/each}
  </Card>
</div>
