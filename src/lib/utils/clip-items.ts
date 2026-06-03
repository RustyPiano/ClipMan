import type { ClipItem } from '$lib/types';

const MAX_DECODE_CACHE_SIZE = 1000;

const decodedTextCache = new Map<string, { content: string; text: string }>();

export function comparePinOrder(a: ClipItem, b: ClipItem) {
  const aOrder = a.pinOrder ?? Number.MAX_SAFE_INTEGER;
  const bOrder = b.pinOrder ?? Number.MAX_SAFE_INTEGER;
  return aOrder - bOrder || b.timestamp - a.timestamp;
}

export function decodeClipText(item: ClipItem, emptyContent: string, decodeFailed: string) {
  if (item.contentType !== 'text') return '';
  if (!item.content) return emptyContent;

  const cached = decodedTextCache.get(item.id);
  if (cached?.content === item.content) {
    return cached.text;
  }

  const text = decodeBase64Text(item.content);
  if (text === null) return decodeFailed;

  decodedTextCache.set(item.id, { content: item.content, text });

  if (decodedTextCache.size > MAX_DECODE_CACHE_SIZE) {
    const oldestKey = decodedTextCache.keys().next().value;
    if (oldestKey) decodedTextCache.delete(oldestKey);
  }

  return text;
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
    recentItems: [incoming, ...recentWithoutIncoming].slice(0, maxHistoryItems),
    pinnedItems: pinnedWithoutIncoming,
  };
}
