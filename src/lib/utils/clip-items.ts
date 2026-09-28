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

/** Last separator position, accepting both POSIX and Windows separators. */
function lastSeparator(path: string): number {
  return Math.max(path.lastIndexOf('/'), path.lastIndexOf('\\'));
}

/** Pure string helpers (no disk access) for rendering Files clips. */
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

// A trailing separator or an extensionless basename reads as a directory.
export function looksLikeDirectory(path: string): boolean {
  if (path.endsWith('/') || path.endsWith('\\')) return true;
  return !fileBasename(path).includes('.');
}

/** 后端的文件列表内容是 base64 编码的 JSON 字符串数组。 */
export function decodeFilePaths(item: ClipItem): string[] {
  if (item.contentType !== 'files') return [];
  return JSON.parse(new TextDecoder().decode(base64Bytes(item.content))) as string[];
}

function base64Bytes(content: string) {
  return Uint8Array.from(atob(content), (char) => char.charCodeAt(0));
}

function decodeBase64Text(content: string) {
  try {
    return new TextDecoder().decode(base64Bytes(content));
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
