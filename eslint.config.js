import eslint from '@eslint/js';
import tseslint from '@typescript-eslint/eslint-plugin';
import tsparser from '@typescript-eslint/parser';
import svelte from 'eslint-plugin-svelte';
import prettier from 'eslint-config-prettier';

export default [
  eslint.configs.recommended,
  prettier,
  {
    files: ['**/*.ts', '**/*.svelte'],
    languageOptions: {
      parser: tsparser,
      parserOptions: {
        extraFileExtensions: ['.svelte'],
      },
    },
    plugins: {
      '@typescript-eslint': tseslint,
    },
    rules: {
      '@typescript-eslint/no-unused-vars': ['warn', { argsIgnorePattern: '^_' }],
      'no-unused-vars': 'off',
    },
  },
  {
    files: ['**/*.svelte'],
    plugins: {
      svelte,
    },
    processor: 'svelte/svelte',
  },
  {
    ignores: ['dist/', 'node_modules/', 'src-tauri/target/', '.svelte-kit/'],
  },
];
