import { Lexer } from 'marked';
import type { Token, Tokens } from 'marked';

const URL_SAFE_PROTOCOLS = new Set(['http:', 'https:', 'mailto:']);

export function markdownBlockTokens(content: string) {
  return stripUnsafeTokens(Lexer.lex(content, { breaks: true, gfm: true }));
}

export function safeMarkdownUrl(value: string, attributeName: 'href' | 'src') {
  const trimmedValue = value.trim();
  return isSafeUrl(trimmedValue, attributeName) ? trimmedValue : null;
}

function stripUnsafeTokens(tokens: Token[]): Token[] {
  return tokens
    .filter((token) => token.type !== 'html' && token.type !== 'tag')
    .map((token) => stripNestedUnsafeTokens(token));
}

function stripNestedUnsafeTokens(token: Token): Token {
  if ('tokens' in token && Array.isArray(token.tokens)) {
    token.tokens = stripUnsafeTokens(token.tokens);
  }

  if (isListToken(token)) {
    token.items = token.items.map((item) => ({
      ...item,
      tokens: stripUnsafeTokens(item.tokens),
    }));
  } else if (isTableToken(token)) {
    token.header = stripTableCells(token.header);
    token.rows = token.rows.map(stripTableCells);
  }

  return token;
}

function stripTableCells(cells: Tokens.TableCell[]) {
  return cells.map((cell) => ({
    ...cell,
    tokens: stripUnsafeTokens(cell.tokens),
  }));
}

function isListToken(token: Token): token is Tokens.List {
  return token.type === 'list' && 'items' in token;
}

function isTableToken(token: Token): token is Tokens.Table {
  return token.type === 'table' && 'header' in token && 'rows' in token;
}

function isSafeUrl(value: string, attributeName: 'href' | 'src') {
  if (value.startsWith('#') || value.startsWith('/')) return true;

  try {
    const url = new globalThis.URL(value, 'https://clipman.local');
    if (attributeName === 'src') {
      return url.protocol === 'http:' || url.protocol === 'https:';
    }
    return URL_SAFE_PROTOCOLS.has(url.protocol);
  } catch (_error) {
    return false;
  }
}
