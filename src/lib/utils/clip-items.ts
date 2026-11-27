import type { ClipFilter, ClipItem } from '$lib/types';

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

export function encodeClipTextPreview(text: string): string {
  const encoded = new TextEncoder().encode(text);
  let bytes = encoded;
  if (encoded.length > 4096) {
    let end = 4096;
    while (end > 0 && (encoded[end] & 0xc0) === 0x80) {
      end -= 1;
    }
    bytes = encoded.subarray(0, end);
  }
  let binary = '';
  for (let i = 0; i < bytes.length; i += 0x8000) {
    binary += String.fromCharCode(...bytes.subarray(i, i + 0x8000));
  }
  return btoa(binary);
}

export function isSingleUrl(text: string): boolean {
  return /^\s*https?:\/\/\S+\s*$/.test(text);
}

export function looksLikeCode(text: string): boolean {
  if (isSingleUrl(text)) return false;
  return (
    /^\s*([{[<$]|\/\/|#!|import\s|export\s|const\s|let\s|fn\s|def\s|class\s|SELECT\s)/i.test(
      text
    ) || /(&&|\|\||=>|::|;\s*$|^(\t| {2,})\S)/m.test(text)
  );
}

export interface ParsedColor {
  css: string;
  hex: string;
  rgb: string;
  hsl: string;
}

export function parseColorValue(raw: string): ParsedColor | null {
  const text = raw.trim();
  if (!text || text.length > 64) return null;

  const hexMatch = /^#([0-9a-f]{3}|[0-9a-f]{4}|[0-9a-f]{6}|[0-9a-f]{8})$/i.exec(text);
  if (hexMatch) {
    const hex = hexMatch[1];
    let r: number;
    let g: number;
    let b: number;
    let a = 1;
    if (hex.length === 3 || hex.length === 4) {
      r = parseInt(hex[0] + hex[0], 16);
      g = parseInt(hex[1] + hex[1], 16);
      b = parseInt(hex[2] + hex[2], 16);
      if (hex.length === 4) a = parseInt(hex[3] + hex[3], 16) / 255;
    } else {
      r = parseInt(hex.slice(0, 2), 16);
      g = parseInt(hex.slice(2, 4), 16);
      b = parseInt(hex.slice(4, 6), 16);
      if (hex.length === 8) a = parseInt(hex.slice(6, 8), 16) / 255;
    }
    return formatColor(r, g, b, a);
  }

  const rgbMatch =
    /^rgba?\(\s*(\d{1,3})\s*[,\s]\s*(\d{1,3})\s*[,\s]\s*(\d{1,3})(?:\s*[,/]\s*([0-9.]+%?))?\s*\)$/i.exec(
      text
    );
  if (rgbMatch) {
    const r = Number(rgbMatch[1]);
    const g = Number(rgbMatch[2]);
    const b = Number(rgbMatch[3]);
    if (r > 255 || g > 255 || b > 255) return null;
    const a = parseAlpha(rgbMatch[4]);
    if (a === null) return null;
    return formatColor(r, g, b, a);
  }

  const hslMatch =
    /^hsla?\(\s*(\d{1,3}(?:\.\d+)?)\s*[,\s]\s*(\d{1,3}(?:\.\d+)?)%\s*[,\s]\s*(\d{1,3}(?:\.\d+)?)%(?:\s*[,/]\s*([0-9.]+%?))?\s*\)$/i.exec(
      text
    );
  if (hslMatch) {
    const h = Number(hslMatch[1]) % 360;
    const s = Number(hslMatch[2]);
    const l = Number(hslMatch[3]);
    if (s > 100 || l > 100) return null;
    const a = parseAlpha(hslMatch[4]);
    if (a === null) return null;
    const [r, g, b] = hslToRgb(h, s / 100, l / 100);
    return formatColor(r, g, b, a);
  }

  return null;
}

function parseAlpha(raw?: string): number | null {
  if (raw === undefined) return 1;
  const val = raw.endsWith('%') ? Number(raw.slice(0, -1)) / 100 : Number(raw);
  if (Number.isNaN(val) || val < 0 || val > 1) return null;
  return val;
}

function hslToRgb(h: number, s: number, l: number): [number, number, number] {
  const c = (1 - Math.abs(2 * l - 1)) * s;
  const x = c * (1 - Math.abs(((h / 60) % 2) - 1));
  const m = l - c / 2;
  const [r, g, b] =
    h < 60
      ? [c, x, 0]
      : h < 120
        ? [x, c, 0]
        : h < 180
          ? [0, c, x]
          : h < 240
            ? [0, x, c]
            : h < 300
              ? [x, 0, c]
              : [c, 0, x];
  return [Math.round((r + m) * 255), Math.round((g + m) * 255), Math.round((b + m) * 255)];
}

function rgbToHsl(r: number, g: number, b: number): [number, number, number] {
  const rn = r / 255;
  const gn = g / 255;
  const bn = b / 255;
  const max = Math.max(rn, gn, bn);
  const min = Math.min(rn, gn, bn);
  const d = max - min;
  const l = (max + min) / 2;
  let h = 0;
  let s = 0;
  if (d !== 0) {
    s = d / (1 - Math.abs(2 * l - 1));
    if (max === rn) h = ((gn - bn) / d) % 6;
    else if (max === gn) h = (bn - rn) / d + 2;
    else h = (rn - gn) / d + 4;
    h = Math.round(h * 60);
    if (h < 0) h += 360;
  }
  return [h, Math.round(s * 100), Math.round(l * 100)];
}

function formatColor(r: number, g: number, b: number, a: number): ParsedColor {
  const toHex = (n: number) => n.toString(16).padStart(2, '0').toUpperCase();
  const alphaRounded = Math.round(a * 100) / 100;
  const hasAlpha = alphaRounded < 1;
  const hex = hasAlpha
    ? `#${toHex(r)}${toHex(g)}${toHex(b)}${toHex(Math.round(a * 255))}`
    : `#${toHex(r)}${toHex(g)}${toHex(b)}`;
  const rgb = hasAlpha ? `rgba(${r}, ${g}, ${b}, ${alphaRounded})` : `rgb(${r}, ${g}, ${b})`;
  const [h, s, l] = rgbToHsl(r, g, b);
  const hsl = hasAlpha ? `hsla(${h}, ${s}%, ${l}%, ${alphaRounded})` : `hsl(${h}, ${s}%, ${l}%)`;
  return { css: rgb, hex, rgb, hsl };
}

export function matchesClipFilter(item: ClipItem, filter: ClipFilter): boolean {
  if (filter === 'all') return true;
  if (filter === 'image') return item.contentType === 'image';
  if (filter === 'files') return item.contentType === 'files';
  if (filter === 'text') return item.contentType === 'text';
  if (filter === 'link')
    return item.contentType === 'text' && isSingleUrl(decodeClipText(item, ''));
  return true;
}

export interface TextSegment {
  text: string;
  match: boolean;
}

function escapeRegExp(value: string): string {
  return value.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
}

export function highlightSegments(text: string, query: string): TextSegment[] {
  const needle = query.trim();
  if (!needle) return [{ text, match: false }];
  const regex = new RegExp(escapeRegExp(needle), 'gi');
  const segments: TextSegment[] = [];
  let cursor = 0;
  for (const match of text.matchAll(regex)) {
    const index = match.index ?? -1;
    if (index < 0 || !match[0].length) continue;
    if (index > cursor) {
      segments.push({ text: text.slice(cursor, index), match: false });
    }
    segments.push({ text: match[0], match: true });
    cursor = index + match[0].length;
  }
  if (cursor < text.length) {
    segments.push({ text: text.slice(cursor), match: false });
  }
  return segments.length ? segments : [{ text, match: false }];
}

export function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(2)} MB`;
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
