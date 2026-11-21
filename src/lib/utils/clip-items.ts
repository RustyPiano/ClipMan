import type { ClipItem } from '$lib/types';

export function comparePinOrder(a: ClipItem, b: ClipItem) {
  const aOrder = a.pinOrder ?? Number.MAX_SAFE_INTEGER;
  const bOrder = b.pinOrder ?? Number.MAX_SAFE_INTEGER;
  return aOrder - bOrder || b.timestamp - a.timestamp;
}

interface DisplayItemsOptions {
  activeSearchQuery: string;
  searchResults: readonly ClipItem[];
  recentItems: readonly ClipItem[];
  pinnedItems: readonly ClipItem[];
}

export function getRecentDisplayItems({
  activeSearchQuery,
  searchResults,
  recentItems,
  pinnedItems,
}: DisplayItemsOptions) {
  const items = activeSearchQuery.trim()
    ? searchResults
    : mergeItemsById([...recentItems, ...pinnedItems]);

  return [...items].sort(compareTimestampDesc);
}

export function getPinnedDisplayItems({
  activeSearchQuery,
  searchResults,
  pinnedItems,
}: DisplayItemsOptions) {
  const items = activeSearchQuery.trim()
    ? searchResults.filter((item) => item.isPinned)
    : pinnedItems;

  return [...items].sort(comparePinOrder);
}

function compareTimestampDesc(a: ClipItem, b: ClipItem) {
  return b.timestamp - a.timestamp || (a.id < b.id ? 1 : a.id > b.id ? -1 : 0);
}

function mergeItemsById(items: readonly ClipItem[]) {
  const merged = new Map<string, ClipItem>();
  for (const item of items) {
    merged.set(item.id, item);
  }
  return [...merged.values()];
}

export function decodeClipText(item: ClipItem, emptyContent: string) {
  if (item.contentType !== 'text') return '';
  if (!item.content) return emptyContent;
  return decodeBase64Text(item.content);
}

export function looksLikeCode(text: string): boolean {
  return (
    /^\s*(https?:\/\/|[{[<$]|\/\/)/.test(text) || /(&&|\|\||=>|::|;\s*$|^(\t| {2,})\S)/m.test(text)
  );
}

export interface ClipTimeLabels {
  justNow: string;
  minutesAgo: (n: number) => string;
  yesterday: string;
}

export function formatClipTime(ms: number, now: number, locale: string, labels: ClipTimeLabels) {
  const age = Math.max(0, now - ms);
  if (age < 60_000) return labels.justNow;
  if (age < 3_600_000) return labels.minutesAgo(Math.floor(age / 60_000));
  const date = new Date(ms);
  const time = date.toLocaleTimeString(locale, { timeStyle: 'short' });
  const today = new Date(now);
  const dayStart = (d: Date) => new Date(d.getFullYear(), d.getMonth(), d.getDate()).getTime();
  const days = Math.round((dayStart(today) - dayStart(date)) / 86_400_000);
  if (days === 0) return time;
  if (days === 1) return `${labels.yesterday} ${time}`;
  return date.toLocaleDateString(
    locale,
    date.getFullYear() === today.getFullYear()
      ? { month: 'short', day: 'numeric' }
      : { year: 'numeric', month: 'short', day: 'numeric' }
  );
}

function lastSeparator(path: string): number {
  return Math.max(path.lastIndexOf('/'), path.lastIndexOf('\\'));
}

export function fileBasename(path: string): string {
  const trimmed = path.replace(/[\\/]+$/, '');
  const slash = lastSeparator(trimmed);
  return slash >= 0 ? trimmed.slice(slash + 1) : trimmed;
}

export function fileDirname(path: string): string {
  const trimmed = path.replace(/[\\/]+$/, '');
  const slash = lastSeparator(trimmed);
  return slash > 0 ? trimmed.slice(0, slash) : '';
}

export function looksLikeDirectory(path: string): boolean {
  if (path.endsWith('/') || path.endsWith('\\')) return true;
  return !fileBasename(path).includes('.');
}

export function decodeFilePaths(item: ClipItem): string[] {
  if (item.contentType !== 'files') return [];
  return JSON.parse(decodeBase64Text(item.content)) as string[];
}

function decodeBase64Text(content: string) {
  return new TextDecoder().decode(Uint8Array.from(atob(content), (char) => char.charCodeAt(0)));
}

interface ApplyClipboardChangedOptions {
  recentItems: readonly ClipItem[];
  pinnedItems: readonly ClipItem[];
  incoming: ClipItem;
  maxHistoryItems: number;
}

export function applyClipboardChanged({
  recentItems,
  pinnedItems,
  incoming,
  maxHistoryItems,
}: ApplyClipboardChangedOptions) {
  const recentWithoutIncoming = recentItems.filter((item) => item.id !== incoming.id);
  const pinnedWithoutIncoming = pinnedItems.filter((item) => item.id !== incoming.id);

  if (incoming.isPinned) {
    return {
      recentItems: recentWithoutIncoming,
      pinnedItems: [...pinnedWithoutIncoming, incoming].sort(comparePinOrder),
    };
  }

  return {
    recentItems: [incoming, ...recentWithoutIncoming]
      .sort(compareTimestampDesc)
      .slice(0, maxHistoryItems),
    pinnedItems: pinnedWithoutIncoming,
  };
}
