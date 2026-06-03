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
  import type { ClipItem, PasteMode, ReorderDirection } from '$lib/types';
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
    Loader2,
    Heart,
  } from 'lucide-svelte';
  import { flip } from 'svelte/animate';

  const SEARCH_INPUT_ID = 'quickbar-search';

  interface QuickBarOpenedPayload {
    panel?: QuickBarPanel;
  }

  const t = $derived(i18n.t);
  const displayItems = $derived(
    selectionStore.panel === 'pinned'
      ? clipboardStore.pinnedDisplayItems
      : clipboardStore.recentDisplayItems
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
    return opposite ? 'opposite' : 'default';
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

  async function reorderSelectedPinned(direction: ReorderDirection) {
    if (selectionStore.panel !== 'pinned') return;

    const item = getSelectedItem();
    if (!item?.isPinned) return;

    await clipboardStore.reorderPinned(item.id, direction);

    const nextIndex = clipboardStore.pinnedDisplayItems.findIndex(
      (pinnedItem) => pinnedItem.id === item.id
    );
    if (nextIndex >= 0) {
      selectionStore.setSelectedIndex(nextIndex, clipboardStore.pinnedDisplayItems.length);
    }
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

    if (
      selectionStore.panel === 'pinned' &&
      hasModifier &&
      event.shiftKey &&
      (event.key === 'ArrowDown' || event.key === 'ArrowUp')
    ) {
      event.preventDefault();
      void reorderSelectedPinned(event.key === 'ArrowUp' ? 'up' : 'down');
      return;
    }

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

    // Mark this window so app.css can make the body transparent, letting the
    // rounded floating panel + shadow show. Settings window never gets this.
    document.documentElement.classList.add('quickbar-window');

    let unlistenQuickbarOpened: (() => void) | undefined;

    void listen<QuickBarOpenedPayload>('quickbar-opened', (event) => {
      selectionStore.reset(event.payload?.panel === 'pinned' ? 'pinned' : 'recent');
      void (async () => {
        await clipboardStore.clearSearch();
        await clipboardStore.refreshSettings();
        focusSearchInput();
      })();
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
      document.documentElement.classList.remove('quickbar-window');
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
  <div class="flex h-screen flex-col p-2 text-foreground">
    <div
      class="flex h-full flex-col overflow-hidden rounded-2xl border border-border/60 bg-background/95 shadow-2xl backdrop-blur-xl"
    >
      <PermissionCheck />
      <Toast />

      <!-- Spotlight-style search row -->
      <div class="flex flex-none items-center gap-2 border-b border-border/60 px-2.5 py-1.5">
        <div class="min-w-0 flex-1">
          <SearchBar />
        </div>
        <div class="flex flex-none rounded-lg bg-muted p-0.5 text-xs font-medium" role="tablist">
          <button
            role="tab"
            aria-selected={selectionStore.panel === 'recent'}
            aria-controls="clipboard-content"
            tabindex={selectionStore.panel === 'recent' ? 0 : -1}
            class="rounded-md px-2.5 py-1 transition-all {selectionStore.panel === 'recent'
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
            class="rounded-md px-2.5 py-1 transition-all {selectionStore.panel === 'pinned'
              ? 'bg-background text-foreground shadow-sm'
              : 'text-muted-foreground hover:text-foreground'}"
            onclick={() => switchPanel('pinned')}
          >
            {t.pinned}<span class="ml-1 opacity-60">{clipboardStore.pinnedItems.length}</span>
          </button>
        </div>
      </div>

      <!-- Results -->
      <main id="clipboard-content" class="flex min-h-0 flex-1 flex-col overflow-hidden">
        {#if clipboardStore.isLoading}
          <div class="flex h-full flex-col items-center justify-center gap-2 text-muted-foreground">
            <Loader2 class="h-6 w-6 animate-spin" />
            <p class="text-sm">{t.loading}</p>
          </div>
        {:else if displayItems.length === 0}
          <div
            class="flex h-full flex-col items-center justify-center gap-1.5 p-6 text-center text-muted-foreground"
          >
            {#if selectionStore.panel === 'pinned'}
              <Pin class="h-8 w-8 opacity-20" />
              <p class="text-sm font-medium">{t.noPinnedItems}</p>
              <p class="text-xs opacity-70">{t.noPinnedItemsHint}</p>
            {:else}
              <ClipboardList class="h-8 w-8 opacity-20" />
              <p class="text-sm font-medium">{t.noClipboardHistory}</p>
              <p class="text-xs opacity-70">{t.noClipboardHistoryHint}</p>
            {/if}
          </div>
        {:else}
          <div class="flex-1 space-y-1 overflow-y-auto p-2">
            {#each displayItems as item, index (item.id)}
              <div animate:flip={{ duration: 200 }}>
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

      <!-- Footer: shortcut hints + quick actions -->
      <div
        class="flex flex-none items-center justify-between gap-2 border-t border-border/60 bg-muted/30 px-2.5 py-1 text-[11px] text-muted-foreground"
      >
        <div class="flex min-w-0 items-center gap-2.5 overflow-hidden">
          <span class="flex flex-none items-center gap-1">
            <kbd
              class="rounded border border-border bg-background px-1 font-sans text-[10px] leading-4"
              >↵</kbd
            >
            {t.paste}
          </span>
          <span class="flex flex-none items-center gap-1">
            <kbd
              class="rounded border border-border bg-background px-1 font-sans text-[10px] leading-4"
              >⌘↵</kbd
            >
            {t.copy}
          </span>
          <span class="flex flex-none items-center gap-1">
            <kbd
              class="rounded border border-border bg-background px-1 font-sans text-[10px] leading-4"
              >Tab</kbd
            >
            {t.switchPanel}
          </span>
          <span class="flex flex-none items-center gap-1">
            <kbd
              class="rounded border border-border bg-background px-1 font-sans text-[10px] leading-4"
              >esc</kbd
            >
            {t.close}
          </span>
        </div>

        <div class="flex flex-none items-center gap-0.5">
          <span class="mr-1 tabular-nums opacity-70">{displayItems.length}</span>
          <Button
            variant="ghost"
            size="icon"
            class="h-6 w-6 text-muted-foreground hover:text-destructive"
            title={t.clearNonPinned}
            onclick={clearHistory}
          >
            <Trash2 class="h-3.5 w-3.5" />
          </Button>
          <Button
            variant="ghost"
            size="icon"
            class="h-6 w-6 text-muted-foreground hover:text-foreground"
            title={t.switchTheme}
            onclick={() => themeStore.toggle()}
          >
            {#if themeStore.current === 'light'}
              <Sun class="h-3.5 w-3.5" />
            {:else if themeStore.current === 'dark'}
              <Moon class="h-3.5 w-3.5" />
            {:else if themeStore.current === 'light-pink'}
              <Heart class="h-3.5 w-3.5" />
            {:else}
              <Monitor class="h-3.5 w-3.5" />
            {/if}
          </Button>
          <Button
            variant="ghost"
            size="icon"
            class="h-6 w-6 text-muted-foreground hover:text-foreground"
            title={t.settings}
            onclick={() => void openSettingsWindow()}
          >
            <Settings class="h-3.5 w-3.5" />
          </Button>
        </div>
      </div>
    </div>
  </div>
{/if}
