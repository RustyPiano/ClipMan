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

export function decodeClipText(item: ClipItem, emptyContent: string, decodeFailed: string) {
  if (item.contentType !== 'text') return '';
  if (!item.content) return emptyContent;

  const text = decodeBase64Text(item.content);
  if (text === null) return decodeFailed;

  return text;
}

/**
 * Decode the backend's JSON path list, with newline-separated legacy support.
 * Returns [] for non-files clips or when decoding fails.
 */
export function decodeFilePaths(item: ClipItem): string[] {
  if (item.contentType !== 'files') return [];
  if (!item.content) return [];

  const text = decodeBase64Text(item.content);
  if (text === null) return [];

  try {
    const paths: unknown = JSON.parse(text);
    if (Array.isArray(paths) && paths.every((path) => typeof path === 'string')) return paths;
  } catch {
    /* Legacy records used newline-separated paths. */
  }
  return text.split('\n').filter((line) => line.length > 0);
}

function decodeBase64Text(content: string) {
  try {
    const binaryString = atob(content);
    const bytes = new Uint8Array(binaryString.length);
    for (let i = 0; i < binaryString.length; i++) {
      bytes[i] = binaryString.charCodeAt(i);
    }
    return new TextDecoder().decode(bytes);
  } catch (error) {
    console.error('Failed to decode text content:', error);
    return null;
  }
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
