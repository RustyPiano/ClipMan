export type ThemeChoice = 'system' | 'paper' | 'ink' | 'petal';
export type ResolvedTheme = Exclude<ThemeChoice, 'system'>;

const STORAGE_KEY = 'clipleaf-theme';

function isThemeChoice(value: string | null): value is ResolvedTheme {
  return value === 'paper' || value === 'ink' || value === 'petal';
}

function readStoredTheme(): ThemeChoice {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    return isThemeChoice(raw) ? raw : 'system';
  } catch {
    return 'system';
  }
}

function applyThemeToDom(choice: ThemeChoice): void {
  const meta = document.querySelector<HTMLMetaElement>('meta[name="color-scheme"]');
  const root = document.documentElement;
  if (choice === 'system') {
    if (meta) meta.content = 'light dark';
    delete root.dataset.theme;
    return;
  }
  if (choice === 'ink') {
    if (meta) meta.content = 'dark';
    root.dataset.theme = 'ink';
    return;
  }
  if (meta) meta.content = 'light';
  root.dataset.theme = choice;
}

class ThemeStore {
  choice = $state<ThemeChoice>('system');
  systemDark = $state(false);

  resolved = $derived<ResolvedTheme>(
    this.choice === 'system' ? (this.systemDark ? 'ink' : 'paper') : this.choice
  );

  init(): () => void {
    const media = window.matchMedia('(prefers-color-scheme: dark)');
    this.systemDark = media.matches;
    this.choice = readStoredTheme();
    applyThemeToDom(this.choice);

    const onMedia = (event: MediaQueryListEvent) => {
      this.systemDark = event.matches;
    };
    const onStorage = (event: StorageEvent) => {
      if (event.key !== STORAGE_KEY) return;
      this.choice = readStoredTheme();
      applyThemeToDom(this.choice);
    };

    media.addEventListener('change', onMedia);
    window.addEventListener('storage', onStorage);
    return () => {
      media.removeEventListener('change', onMedia);
      window.removeEventListener('storage', onStorage);
    };
  }

  set(next: ThemeChoice): void {
    this.choice = next;
    applyThemeToDom(next);
    try {
      if (next === 'system') {
        localStorage.removeItem(STORAGE_KEY);
      } else {
        localStorage.setItem(STORAGE_KEY, next);
      }
    } catch {
      return;
    }
  }
}

export const themeStore = new ThemeStore();
