import type { Theme } from '$lib/types';
export type { Theme } from '$lib/types';

const browser = typeof window !== 'undefined';

function createThemeStore() {
  let theme = $state<Theme>('system');

  // Initialize from localStorage if available
  if (browser) {
    const stored = localStorage.getItem('theme') as Theme;
    if (stored && ['light', 'dark', 'light-pink', 'system'].includes(stored)) {
      theme = stored;
    }

    // Sync theme across windows via storage events
    window.addEventListener('storage', (event) => {
      if (event.key === 'theme' && event.newValue) {
        if (['light', 'dark', 'light-pink', 'system'].includes(event.newValue)) {
          theme = event.newValue as Theme;
        }
      }
    });
  }

  // Effect removed - handled in component

  return {
    get current() {
      return theme;
    },
    setTheme: (newTheme: Theme) => {
      theme = newTheme;
    },
    toggle: () => {
      const order: Theme[] = ['light', 'dark', 'light-pink', 'system'];
      const idx = order.indexOf(theme);
      theme = order[(idx + 1) % order.length];
    },
  };
}

export const themeStore = createThemeStore();
