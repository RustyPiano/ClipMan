<script lang="ts">
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { themeStore } from '$lib/stores/theme.svelte';
  import { i18n } from '$lib/i18n';
  import QuickBar from '$lib/components/QuickBar.svelte';
  import SettingsPage from './settings/+page.svelte';
  import Toast from '$lib/components/Toast.svelte';
  import ConfirmDialog from '$lib/components/ui/ConfirmDialog.svelte';

  const isSettingsWindow = getCurrentWindow().label === 'settings';

  $effect(() => {
    document.documentElement.lang = i18n.locale;
  });

  $effect(() => {
    const theme = themeStore.current;
    const root = document.documentElement;
    const media = matchMedia('(prefers-color-scheme: dark)');

    const apply = () => {
      const isDark = theme === 'dark' || (theme === 'system' && media.matches);
      root.classList.remove('dark', 'light-pink');
      if (theme === 'light-pink') {
        root.classList.add('light-pink');
      } else if (isDark) {
        root.classList.add('dark');
      }
    };

    apply();
    localStorage.setItem('theme', theme);

    if (theme === 'system') {
      media.addEventListener('change', apply);
      return () => media.removeEventListener('change', apply);
    }
  });
</script>

<Toast />
<ConfirmDialog />
{#if isSettingsWindow}
  <SettingsPage />
{:else}
  <QuickBar />
{/if}
