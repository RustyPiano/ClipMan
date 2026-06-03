import eslint from '@eslint/js';
import tseslint from '@typescript-eslint/eslint-plugin';
import tsparser from '@typescript-eslint/parser';
import svelte from 'eslint-plugin-svelte';
import prettier from 'eslint-config-prettier';

const browserGlobals = {
  alert: 'readonly',
  atob: 'readonly',
  cancelAnimationFrame: 'readonly',
  clearTimeout: 'readonly',
  confirm: 'readonly',
  console: 'readonly',
  document: 'readonly',
  Element: 'readonly',
  Event: 'readonly',
  HTMLInputElement: 'readonly',
  HTMLSelectElement: 'readonly',
  HTMLTextAreaElement: 'readonly',
  KeyboardEvent: 'readonly',
  localStorage: 'readonly',
  MouseEvent: 'readonly',
  navigator: 'readonly',
  requestAnimationFrame: 'readonly',
  setTimeout: 'readonly',
  TextDecoder: 'readonly',
  window: 'readonly',
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
    ignores: ['dist/', 'node_modules/', 'src-tauri/target/', '.svelte-kit/'],
  },
  eslint.configs.recommended,
  ...svelte.configs['flat/base'],
  prettier,
  {
    languageOptions: {
      globals: {
        ...browserGlobals,
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
  {
    files: ['**/*.svelte', '**/*.svelte.ts', '**/*.svelte.js'],
    languageOptions: {
      parserOptions: {
        parser: tsparser,
        extraFileExtensions: ['.svelte'],
      },
    },
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
