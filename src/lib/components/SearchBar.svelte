<script lang="ts">
import { clipboardStore } from '$lib/stores/clipboard.svelte';

let { placeholder = '搜索剪切板历史...' } = $props();

let inputValue = $state('');

function handleInput(event: Event) {
  const target = event.target as HTMLInputElement;
  inputValue = target.value;
  clipboardStore.search(inputValue);
}

function clearSearch() {
  inputValue = '';
  clipboardStore.search('');
}
</script>

<div class="search-bar">
  <div class="search-input-wrapper">
    <svg
      class="search-icon"
      xmlns="http://www.w3.org/2000/svg"
      viewBox="0 0 20 20"
      fill="currentColor"
    >
      <path
        fill-rule="evenodd"
        d="M9 3.5a5.5 5.5 0 100 11 5.5 5.5 0 000-11zM2 9a7 7 0 1112.452 4.391l3.328 3.329a.75.75 0 11-1.06 1.06l-3.329-3.328A7 7 0 012 9z"
        clip-rule="evenodd"
      />
    </svg>

    <input
      type="text"
      value={inputValue}
      oninput={handleInput}
      {placeholder}
      class="search-input"
    />

    {#if inputValue}
      <button
        onclick={clearSearch}
        class="clear-button"
        aria-label="清除搜索"
      >
        <svg
          xmlns="http://www.w3.org/2000/svg"
          viewBox="0 0 20 20"
          fill="currentColor"
        >
          <path
            d="M6.28 5.22a.75.75 0 00-1.06 1.06L8.94 10l-3.72 3.72a.75.75 0 101.06 1.06L10 11.06l3.72 3.72a.75.75 0 101.06-1.06L11.06 10l3.72-3.72a.75.75 0 00-1.06-1.06L10 8.94 6.28 5.22z"
          />
        </svg>
      </button>
    {/if}
  </div>
</div>

<style>
  .search-bar {
    padding: 1rem;
    border-bottom: 1px solid #e5e7eb;
  }

  .search-input-wrapper {
    position: relative;
    display: flex;
    align-items: center;
  }

  .search-icon {
    position: absolute;
    left: 0.75rem;
    width: 1.25rem;
    height: 1.25rem;
    color: #9ca3af;
    pointer-events: none;
  }

  .search-input {
    width: 100%;
    padding: 0.5rem 2.5rem 0.5rem 2.5rem;
    border: 1px solid #e5e7eb;
    border-radius: 0.5rem;
    font-size: 0.875rem;
    transition: all 0.15s ease;
  }

  .search-input:focus {
    outline: none;
    border-color: #3b82f6;
    box-shadow: 0 0 0 3px rgba(59, 130, 246, 0.1);
  }

  .clear-button {
    position: absolute;
    right: 0.75rem;
    padding: 0.25rem;
    background: none;
    border: none;
    cursor: pointer;
    color: #9ca3af;
    transition: color 0.15s ease;
  }

  .clear-button:hover {
    color: #6b7280;
  }

  .clear-button svg {
    width: 1.25rem;
    height: 1.25rem;
  }
</style>
