import { isMac } from './platform';

export function formatShortcutKey(key: string): string {
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

export function formatShortcut(shortcut: string): string[] {
  if (!shortcut) return [];
  return shortcut.split('+').map(formatShortcutKey);
}

export function shortcutLabel(shortcut: string): string {
  return formatShortcut(shortcut).join(isMac ? '' : '+');
}
