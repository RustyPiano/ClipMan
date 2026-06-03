<script lang="ts">
  import Card from '$lib/components/ui/Card.svelte';
  import { themeStore } from '$lib/stores/theme.svelte';
  import { i18n, type Locale } from '$lib/i18n';
  import { Monitor, Moon, Sun, Heart, Globe } from 'lucide-svelte';
  import type { Settings } from '$lib/types';

  let { settings = $bindable() } = $props<{
    settings: Settings;
  }>();

  const t = $derived(i18n.t);

  const themes = [
    { value: 'light' as const, icon: Sun, labelKey: 'themeLight' as const },
    { value: 'dark' as const, icon: Moon, labelKey: 'themeDark' as const },
    { value: 'light-pink' as const, icon: Heart, labelKey: 'themePink' as const },
    { value: 'system' as const, icon: Monitor, labelKey: 'themeSystem' as const },
  ];

  const languages: { value: Locale; label: string }[] = [
    { value: 'zh-CN', label: '简体中文' },
    { value: 'en', label: 'English' },
  ];

  function getThemeLabel(key: 'themeLight' | 'themeDark' | 'themePink' | 'themeSystem'): string {
    return t[key];
  }
</script>

<div class="space-y-6 animate-in fade-in slide-in-from-bottom-4 duration-300">
  <div>
    <h2 class="text-lg font-semibold mb-1">{t.settingsAppearance}</h2>
    <p class="text-sm text-muted-foreground">{t.themeMode}</p>
  </div>

  <Card class="p-6 space-y-6">
    <!-- Theme Selection -->
    <div class="space-y-3">
      <span class="text-sm font-medium" id="theme-label">{t.themeMode}</span>
      <div
        class="grid grid-cols-2 sm:grid-cols-4 gap-4"
        role="radiogroup"
        aria-labelledby="theme-label"
      >
        {#each themes as theme (theme.value)}
          <button
            class="flex flex-col items-center gap-3 p-4 rounded-lg border-2 transition-all hover:bg-muted/50
                        {themeStore.current === theme.value
              ? 'border-primary bg-primary/5'
              : 'border-transparent bg-muted/20'}"
            onclick={() => themeStore.setTheme(theme.value)}
            role="radio"
            aria-checked={themeStore.current === theme.value}
          >
            <div
              class="p-2 rounded-full {themeStore.current === theme.value
                ? 'bg-primary text-primary-foreground'
                : 'bg-background text-muted-foreground'}"
            >
              <theme.icon class="h-5 w-5" />
            </div>
            <span class="text-sm font-medium">{getThemeLabel(theme.labelKey)}</span>
          </button>
        {/each}
      </div>
    </div>

    <!-- Language Selection -->
    <div class="space-y-3 pt-4 border-t border-border">
      <span class="text-sm font-medium" id="lang-label">{t.language}</span>
      <div class="grid grid-cols-2 gap-4" role="radiogroup" aria-labelledby="lang-label">
        {#each languages as lang (lang.value)}
          <button
            class="flex items-center gap-3 p-3 rounded-lg border-2 transition-all hover:bg-muted/50
                        {settings.locale === lang.value
              ? 'border-primary bg-primary/5'
              : 'border-transparent bg-muted/20'}"
            onclick={() => {
              settings.locale = lang.value;
            }}
            role="radio"
            aria-checked={settings.locale === lang.value}
          >
            <div
              class="p-2 rounded-full {settings.locale === lang.value
                ? 'bg-primary text-primary-foreground'
                : 'bg-background text-muted-foreground'}"
            >
              <Globe class="h-4 w-4" />
            </div>
            <span class="text-sm font-medium">{lang.label}</span>
          </button>
        {/each}
      </div>
    </div>
  </Card>
</div>
