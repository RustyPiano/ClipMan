<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { clipboardStore } from '$lib/stores/clipboard.svelte';
  import { router } from '$lib/stores/router.svelte';
  import { selectionStore, type QuickBarPanel } from '$lib/stores/selection.svelte';
  import { themeStore } from '$lib/stores/theme.svelte';
  import { i18n } from '$lib/i18n';
  import type { ClipItem, PasteMode } from '$lib/types';
  import SearchBar from '$lib/components/SearchBar.svelte';
  import ClipboardItem from '$lib/components/ClipboardItem.svelte';
  import SettingsPage from './settings/+page.svelte';
  import PermissionCheck from '$lib/components/PermissionCheck.svelte';
  import Toast from '$lib/components/Toast.svelte';
  import Button from '$lib/components/ui/Button.svelte';
  import {
    Sun,
    Moon,
    Monitor,
    Settings,
    Trash2,
    Pin,
    ClipboardList,
    FileText,
    Image as ImageIcon,
    Loader2,
    Heart,
  } from 'lucide-svelte';
  import { flip } from 'svelte/animate';

  const SEARCH_INPUT_ID = 'quickbar-search';

  const t = $derived(i18n.t);
  const displayItems = $derived(
    selectionStore.panel === 'pinned'
      ? clipboardStore.pinnedDisplayItems
      : clipboardStore.recentDisplayItems,
  );

  function isTextInput(element: Element | null) {
    return (
      element instanceof HTMLInputElement ||
      element instanceof HTMLTextAreaElement ||
      element instanceof HTMLSelectElement ||
      element?.hasAttribute('contenteditable')
    );
  }

  function focusSearchInput() {
    const input = document.getElementById(SEARCH_INPUT_ID);
    if (input instanceof HTMLInputElement) {
      input.focus();
      return input;
    }

    return null;
  }

  function typeIntoSearch(event: KeyboardEvent) {
    const input = focusSearchInput();
    if (!input) return;

    const start = input.selectionStart ?? input.value.length;
    const end = input.selectionEnd ?? input.value.length;
    const nextValue = `${input.value.slice(0, start)}${event.key}${input.value.slice(end)}`;
    const nextPosition = start + event.key.length;

    input.value = nextValue;
    input.setSelectionRange(nextPosition, nextPosition);
    input.dispatchEvent(new Event('input', { bubbles: true }));
  }

  function switchPanel(panel: QuickBarPanel) {
    selectionStore.selectPanel(panel);
    focusSearchInput();
  }

  function getSelectedItem() {
    return displayItems[selectionStore.selectedIndex];
  }

  function modeForDefault(opposite = false): PasteMode {
    const defaultMode: PasteMode = clipboardStore.autoPaste ? 'paste' : 'copy';
    if (!opposite) return defaultMode;
    return defaultMode === 'paste' ? 'copy' : 'paste';
  }

  async function useItem(item: ClipItem | undefined, mode: PasteMode = modeForDefault()) {
    if (!item) return;
    await clipboardStore.useClip(item, mode);
  }

  async function useSelectedItem(opposite = false) {
    await useItem(getSelectedItem(), modeForDefault(opposite));
  }

  async function useSlot(slotNumber: number) {
    const item = displayItems[slotNumber - 1];
    if (!item) return;

    selectionStore.selectSlot(slotNumber, displayItems.length);
    await useItem(item);
  }

  async function toggleSelectedPin() {
    const item = getSelectedItem();
    if (!item) return;
    await clipboardStore.togglePin(item.id);
  }

  async function deleteSelectedItem() {
    const item = getSelectedItem();
    if (!item) return;
    await clipboardStore.deleteItem(item.id);
  }

  async function clearHistory() {
    if (confirm(t.confirmClearHistory)) {
      await clipboardStore.clearNonPinned();
    }
  }

  async function openSettingsWindow() {
    try {
      await invoke('open_settings_window');
    } catch (error) {
      console.error('[ERROR] Failed to open settings window:', error);
    }
  }

  function handleQuickBarKeydown(event: KeyboardEvent) {
    if (router.currentRoute !== 'home' || event.defaultPrevented || event.isComposing) return;

    const activeElement = document.activeElement;
    const activeTextInput = isTextInput(activeElement);
    const hasModifier = event.metaKey || event.ctrlKey;

    if (event.key === 'ArrowDown') {
      event.preventDefault();
      selectionStore.move(1, displayItems.length);
      return;
    }

    if (event.key === 'ArrowUp') {
      event.preventDefault();
      selectionStore.move(-1, displayItems.length);
      return;
    }

    if (event.key === 'Tab') {
      event.preventDefault();
      selectionStore.togglePanel();
      focusSearchInput();
      return;
    }

    if (event.key === 'Enter') {
      event.preventDefault();
      void useSelectedItem(hasModifier);
      return;
    }

    if (event.key === 'Escape') {
      event.preventDefault();
      void clipboardStore.hideQuickbar();
      return;
    }

    if (hasModifier && event.key.toLowerCase() === 'p') {
      event.preventDefault();
      void toggleSelectedPin();
      return;
    }

    if (hasModifier && (event.key === 'Delete' || event.key === 'Backspace')) {
      event.preventDefault();
      void deleteSelectedItem();
      return;
    }

    if (!hasModifier && !event.altKey && /^[1-9]$/.test(event.key)) {
      event.preventDefault();
      void useSlot(Number(event.key));
      return;
    }

    if (!activeTextInput && !hasModifier && !event.altKey && event.key.length === 1) {
      event.preventDefault();
      typeIntoSearch(event);
    }
  }

  onMount(() => {
    if (getCurrentWindow().label === 'settings') {
      router.goToSettings();
      return;
    }

    let unlistenQuickbarOpened: (() => void) | undefined;

    void listen('quickbar-opened', () => {
      selectionStore.reset('recent');
      void clipboardStore.refreshSettings();
      focusSearchInput();
    }).then((unlisten) => {
      unlistenQuickbarOpened = unlisten;
    });

    selectionStore.reset('recent');
    void clipboardStore.refreshSettings();
    focusSearchInput();
    window.addEventListener('keydown', handleQuickBarKeydown);

    return () => {
      window.removeEventListener('keydown', handleQuickBarKeydown);
      unlistenQuickbarOpened?.();
    };
  });

  $effect(() => {
    selectionStore.clamp(displayItems.length);
  });

  $effect(() => {
    const selectedItem = getSelectedItem();
    if (!selectedItem || typeof window === 'undefined') return;

    const frame = requestAnimationFrame(() => {
      document.getElementById(`clip-item-${selectedItem.id}`)?.scrollIntoView({
        block: 'nearest',
      });
    });

    return () => cancelAnimationFrame(frame);
  });

  $effect(() => {
    const theme = themeStore.current;
    const root = document.documentElement;
    const isDark =
      theme === 'dark' ||
      (theme === 'system' && window.matchMedia('(prefers-color-scheme: dark)').matches);

    root.classList.remove('dark', 'light-pink');

    if (theme === 'light-pink') {
      root.classList.add('light-pink');
    } else if (isDark) {
      root.classList.add('dark');
    }

    localStorage.setItem('theme', theme);
  });
</script>

{#if router.currentRoute === 'settings'}
  <SettingsPage />
{:else}
  <div class="flex h-screen flex-col overflow-hidden bg-background text-foreground">
    <PermissionCheck />
    <Toast />

    <header class="sticky top-0 z-10 flex-none border-b border-border bg-muted/30 p-4 backdrop-blur-sm">
      <div class="mb-4 flex items-center justify-between">
        <h1 class="text-2xl font-bold tracking-tight">{t.appName}</h1>
        <div class="flex items-center gap-2">
          <Button
            variant="ghost"
            size="icon"
            onclick={() => themeStore.toggle()}
            title={t.switchTheme}
          >
            {#if themeStore.current === 'light'}
              <Sun class="h-4 w-4" />
            {:else if themeStore.current === 'dark'}
              <Moon class="h-4 w-4" />
            {:else if themeStore.current === 'light-pink'}
              <Heart class="h-4 w-4" />
            {:else}
              <Monitor class="h-4 w-4" />
            {/if}
          </Button>
          <Button
            variant="ghost"
            size="icon"
            title={t.settings}
            onclick={() => void openSettingsWindow()}
          >
            <Settings class="h-4 w-4" />
          </Button>
        </div>
      </div>

      <div class="mb-4 flex items-center gap-2">
        <div class="flex rounded-lg bg-muted p-1" role="tablist">
          <button
            role="tab"
            aria-selected={selectionStore.panel === 'recent'}
            aria-controls="clipboard-content"
            tabindex={selectionStore.panel === 'recent' ? 0 : -1}
            class="rounded-md px-4 py-1.5 text-sm font-medium transition-all {selectionStore.panel ===
            'recent'
              ? 'bg-background text-foreground shadow-sm'
              : 'text-muted-foreground hover:text-foreground'}"
            onclick={() => switchPanel('recent')}
          >
            {t.history}
          </button>
          <button
            role="tab"
            aria-selected={selectionStore.panel === 'pinned'}
            aria-controls="clipboard-content"
            tabindex={selectionStore.panel === 'pinned' ? 0 : -1}
            class="rounded-md px-4 py-1.5 text-sm font-medium transition-all {selectionStore.panel ===
            'pinned'
              ? 'bg-background text-foreground shadow-sm'
              : 'text-muted-foreground hover:text-foreground'}"
            onclick={() => switchPanel('pinned')}
          >
            {t.pinned}
            <span class="ml-1 text-xs opacity-70">({clipboardStore.pinnedItems.length})</span>
          </button>
        </div>

        <div class="ml-auto">
          <Button
            variant="ghost"
            size="icon"
            title={t.clearNonPinned}
            onclick={clearHistory}
            class="text-muted-foreground hover:text-destructive"
          >
            <Trash2 class="h-4 w-4" />
          </Button>
        </div>
      </div>

      <SearchBar />
    </header>

    <main id="clipboard-content" class="flex flex-1 flex-col overflow-hidden bg-background">
      {#if clipboardStore.isLoading}
        <div class="flex h-full flex-col items-center justify-center text-muted-foreground">
          <Loader2 class="mb-2 h-8 w-8 animate-spin" />
          <p>{t.loading}</p>
        </div>
      {:else if displayItems.length === 0}
        <div class="flex h-full flex-col items-center justify-center p-4 text-center text-muted-foreground">
          {#if selectionStore.panel === 'pinned'}
            <Pin class="mb-4 h-12 w-12 opacity-20" />
            <p class="font-medium">{t.noPinnedItems}</p>
            <p class="mt-1 text-sm opacity-70">{t.noPinnedItemsHint}</p>
          {:else}
            <ClipboardList class="mb-4 h-12 w-12 opacity-20" />
            <p class="font-medium">{t.noClipboardHistory}</p>
            <p class="mt-1 text-sm opacity-70">{t.noClipboardHistoryHint}</p>
            <div class="mx-auto mt-8 max-w-xs space-y-1 rounded-lg bg-muted/50 p-4 text-left text-xs">
              <p class="mb-2 font-semibold opacity-70">{t.statistics}</p>
              <div class="flex justify-between">
                <span>{t.total}:</span> <span>{clipboardStore.items.length}</span>
              </div>
              <div class="flex justify-between">
                <span>{t.text}:</span>
                <span>{clipboardStore.items.filter((i) => i.contentType === 'text').length}</span>
              </div>
              <div class="flex justify-between">
                <span>{t.image}:</span>
                <span>{clipboardStore.items.filter((i) => i.contentType === 'image').length}</span>
              </div>
            </div>
          {/if}
        </div>
      {:else}
        <div
          class="z-10 flex flex-none items-center justify-between border-b border-border/50 bg-background/95 px-4 py-2 text-xs text-muted-foreground backdrop-blur"
        >
          <span>{t.showing} {displayItems.length} {t.items}</span>
          <div class="flex items-center gap-2">
            <span class="flex items-center gap-1">
              <FileText class="h-3 w-3" />
              {displayItems.filter((i) => i.contentType === 'text').length}
            </span>
            <span class="flex items-center gap-1">
              <ImageIcon class="h-3 w-3" />
              {displayItems.filter((i) => i.contentType === 'image').length}
            </span>
          </div>
        </div>

        <div class="flex-1 space-y-2 overflow-y-auto p-4 pb-8">
          {#each displayItems as item, index (item.id)}
            <div animate:flip={{ duration: 300 }}>
              <ClipboardItem
                {item}
                selected={index === selectionStore.selectedIndex}
                slotNumber={index < 9 ? index + 1 : null}
                onSelect={() => selectionStore.setSelectedIndex(index, displayItems.length)}
                onUse={() => useItem(item)}
              />
            </div>
          {/each}
        </div>
      {/if}
    </main>
  </div>
{/if}
