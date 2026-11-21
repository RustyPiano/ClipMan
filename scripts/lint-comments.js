import { execFileSync } from 'node:child_process';
import { existsSync, readFileSync } from 'node:fs';
import { basename, extname } from 'node:path';
import { createHighlighter } from 'shiki';
import { LineCounter, parseDocument, visit } from 'yaml';
import baseline from './comment-baseline.json';

const scriptLanguages = new Set([
  'shellscript',
  'python',
  'javascript',
  'typescript',
  'swift',
  'rust',
]);
const languages = {
  rs: 'rust',
  swift: 'swift',
  sh: 'shellscript',
  py: 'python',
  ts: 'typescript',
  js: 'javascript',
  svelte: 'svelte',
  css: 'css',
  html: 'html',
  svg: 'xml',
  json: 'jsonc',
  toml: 'toml',
  yml: 'yaml',
  yaml: 'yaml',
};
function languageFor(path) {
  switch (basename(path)) {
    case 'Cargo.lock':
      return 'toml';
    case '.prettierrc':
      return 'jsonc';
    case '.gitignore':
      return 'gitignore';
    default:
      return languages[extname(path).slice(1)];
  }
}

const highlighter = await createHighlighter({
  langs: [...new Set(Object.values(languages)), 'powershell'],
  themes: [],
});

function tokenize(source, language, firstLine = 1) {
  const grammar = highlighter.getLanguage(language);
  const comments = [];
  let state;
  for (const [index, line] of source.split(/\r?\n/).entries()) {
    const { tokens, ruleStack } = grammar.tokenizeLine(line, state);
    state = ruleStack;
    if (index === 0 && scriptLanguages.has(language) && /^#!\s*\//.test(line)) continue;
    let previousEnd = -1;
    for (const token of tokens) {
      if (!token.scopes.some((scope) => scope.startsWith('comment.'))) continue;
      const end = Math.min(token.endIndex, line.length);
      if (end <= token.startIndex) continue;
      const content = line.slice(token.startIndex, end);
      if (token.startIndex === previousEnd) comments.at(-1).text += content;
      else comments.push({ line: firstLine + index, column: token.startIndex, text: content });
      previousEnd = end;
    }
  }
  return comments;
}

export function scan(path, source) {
  const language = languageFor(path);
  if (language === 'gitignore') {
    return source
      .split(/\r?\n/)
      .flatMap((text, i) => (text.startsWith('#') ? [{ line: i + 1, column: 0, text }] : []));
  }
  if (!language) return [];
  const comments = tokenize(source, language);
  if (!path.startsWith('.github/workflows/') || language !== 'yaml') return comments;
  const lineCounter = new LineCounter();
  const doc = parseDocument(source, { lineCounter });
  if (doc.errors.length) throw doc.errors[0];
  visit(doc, {
    Pair(_, pair, parents) {
      if (pair.key?.value !== 'run' || typeof pair.value?.value !== 'string') return;
      const node = pair.value;
      const position = lineCounter.linePos(node.range[0]);
      const block = node.type.startsWith('BLOCK_');
      const raw = source.slice(node.range[0], node.range[1]);
      const shell = parents.at(-1).get('shell') ?? 'bash';
      const body = block ? raw.slice(raw.indexOf('\n') + 1) : node.value;
      for (const comment of tokenize(
        body,
        /pwsh|powershell/.test(shell) ? 'powershell' : 'shellscript',
        position.line + Number(block)
      )) {
        if (!block) comment.column += position.col - 1 + Number(node.type.startsWith('QUOTE_'));
        comments.push(comment);
      }
    },
  });
  return comments;
}

export function violations(found, allowed) {
  const remaining = [...allowed];
  const errors = [];
  for (const { path, line, text } of found) {
    const index = remaining.findIndex((entry) => entry.path === path && entry.text === text);
    if (index === -1) errors.push(`${path}:${line}: remove comment: ${text}`);
    else remaining.splice(index, 1);
  }
  return errors.concat(
    remaining.map((entry) => `${entry.path}: remove obsolete comment-baseline entry: ${entry.text}`)
  );
}

if (import.meta.main) {
  const files = execFileSync('git', ['ls-files', '-co', '--exclude-standard', '-z'], {
    encoding: 'utf8',
  });
  const found = [];
  for (const path of new Set(files.split('\0').filter(Boolean))) {
    if (!languageFor(path)) continue;
    if (existsSync(path)) {
      for (const comment of scan(path, readFileSync(path, 'utf8')))
        found.push({ path, ...comment });
    }
  }
  if (process.argv.includes('--list')) console.log(JSON.stringify(found, null, 2));
  else {
    const errors = violations(found, baseline);
    if (errors.length) {
      console.error(errors.join('\n'));
      process.exitCode = 1;
    }
  }
}
