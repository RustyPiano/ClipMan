import { expect, test } from 'bun:test';
import { scan, violations } from '../scripts/lint-comments.js';

test('distinguishes comments from data in the repository languages', () => {
  for (const [path, source, expected] of [
    ['code.rs', 'let s = r#"// literal"#; /* old */', ['/* old */']],
    ['code.swift', 'let s = "https://example.org" // why', ['// why']],
    ['code.ts', 'const s = /[/][/]/; // why', ['// why']],
    [
      'view.svelte',
      '<script lang="ts">\n// script\n</script>\n<!-- markup -->\n<style>/* css */</style>',
      ['// script', '<!-- markup -->', '/* css */'],
    ],
    ['test.py', '#!/usr/bin/env python3\ns = "#" # why', ['# why']],
    ['test.sh', "cat <<'EOF'\n# text\nEOF\necho ok # why", ['# why']],
    ['data.toml', 's = "#" # why', ['# why']],
    ['tsconfig.json', '{"glob":"**/*" /* why */}', ['/* why */']],
    [
      '.github/workflows/test.yml',
      'name: "#" # top\njobs:\n  a:\n    steps:\n      - run: |\n          echo "#" # body',
      ['# top', '# body'],
    ],
    ['metadata.yaml', '#!/usr/bin/info', ['#!/usr/bin/info']],
    ['.gitignore', '\\#name\n# why', ['# why']],
  ]) {
    expect(scan(path, source).map((comment) => comment.text.trim())).toEqual(expected);
  }
});

test('a baseline entry allows one exact occurrence and must be removed when it disappears', () => {
  const old = { path: 'a.rs', text: '// old' };
  expect(violations([{ ...old, line: 25 }], [old])).toEqual([]);
  for (const found of [[], [old, old], [{ ...old, path: 'b.rs' }], [{ ...old, text: '// new' }]]) {
    expect(violations(found, [old]).length).toBeGreaterThan(0);
  }
});
