import type { Theme } from '$lib/types';

const THEMES: readonly string[] = ['light', 'dark', 'light-pink', 'system'];

function isTheme(value: string | null): value is Theme {
  return value !== null && THEMES.includes(value);
}

function createThemeStore() {
  const stored = localStorage.getItem('theme');
  let theme = $state<Theme>(isTheme(stored) ? stored : 'system');

  // Sync theme across windows via storage events
  window.addEventListener('storage', (event) => {
    if (event.key === 'theme' && isTheme(event.newValue)) {
      theme = event.newValue;
    }
  });

  return {
    get current() {
      return theme;
    },
    setTheme: (newTheme: Theme) => {
      theme = newTheme;
    },
  };
}

export const themeStore = createThemeStore();
