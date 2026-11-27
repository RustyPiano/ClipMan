import eslint from '@eslint/js';
import tseslint from '@typescript-eslint/eslint-plugin';
import tsparser from '@typescript-eslint/parser';
import svelte from 'eslint-plugin-svelte';
import prettier from 'eslint-config-prettier';
import globals from 'globals';

const DIRECTIVE_COMMENT =
  /^\s*(?:eslint|global\s|globals\s|exported\s|@ts-|prettier-ignore|svelte-ignore|\/\s*<reference\b)/;

const noComments = {
  meta: {
    type: 'suggestion',
    messages: {
      comment:
        '代码里不写注释。删除它；不明显的设计决定写进 docs/adr/，权宜实现提 GitHub Issue（流程见 /delete-comment）。',
    },
  },
  create(context) {
    const { sourceCode } = context;
    const report = (loc, text) => {
      if (!DIRECTIVE_COMMENT.test(text)) context.report({ loc, messageId: 'comment' });
    };
    return {
      Program() {
        for (const comment of sourceCode.getAllComments()) {
          if (comment.type !== 'Shebang') report(comment.loc, comment.value);
        }
        const style = sourceCode.parserServices.getStyleContext?.();
        if (!style || style.status === 'no-style-element') return;
        if (style.status !== 'success') {
          throw new Error(`${context.filename}: <style> 无法解析（${style.status}）`);
        }
        style.sourceAst.walkComments((comment) => {
          const { start, end } = comment.source;
          report(
            {
              start: { line: start.line, column: start.column - 1 },
              end: { line: end.line, column: end.column },
            },
            comment.text
          );
        });
      },
      SvelteHTMLComment(node) {
        report(node.loc, node.value);
      },
    };
  },
};

const svelteRuneGlobals = {
  $bindable: 'readonly',
  $derived: 'readonly',
  $effect: 'readonly',
  $props: 'readonly',
  $state: 'readonly',
};

export default [
  {
    ignores: [
      'dist/',
      'website/dist/',
      'node_modules/',
      'src-tauri/target/',
      'src-tauri/gen/',
      '.svelte-kit/',
      '.claude/',
      '.agents/',
      'test-results/',
      'playwright-report/',
    ],
  },
  eslint.configs.recommended,
  ...svelte.configs['flat/base'],
  prettier,
  {
    languageOptions: {
      globals: {
        ...globals.browser,
        ...svelteRuneGlobals,
      },
    },
  },
  {
    files: ['**/*.ts'],
    ignores: ['**/*.svelte.ts'],
    languageOptions: {
      parser: tsparser,
      parserOptions: {
        ecmaVersion: 'latest',
        sourceType: 'module',
      },
    },
  },
  {
    files: ['**/*.svelte', '**/*.svelte.ts', '**/*.svelte.js'],
    languageOptions: {
      parserOptions: {
        parser: tsparser,
        extraFileExtensions: ['.svelte'],
      },
    },
  },
  {
    plugins: {
      local: { rules: { 'no-comments': noComments } },
    },
    rules: {
      'local/no-comments': 'error',
    },
  },
  {
    files: ['website/src/**'],
    languageOptions: {
      globals: { __RELEASE_VERSION__: 'readonly' },
    },
  },
  {
    files: ['tests/**', 'e2e/**', '*.config.{js,ts}'],
    languageOptions: {
      globals: globals.node,
    },
  },
  {
    files: ['**/*.ts', '**/*.svelte', '**/*.svelte.js'],
    plugins: {
      '@typescript-eslint': tseslint,
    },
    rules: {
      '@typescript-eslint/no-unused-vars': [
        'warn',
        { argsIgnorePattern: '^_', caughtErrorsIgnorePattern: '^_' },
      ],
      'no-unused-vars': 'off',
    },
  },
];
