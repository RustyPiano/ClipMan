<script lang="ts">
  import { flushSync, onMount } from 'svelte';
  import type { Attachment } from 'svelte/attachments';
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { clipboardStore } from '$lib/stores/clipboard.svelte';
  import { router } from '$lib/stores/router.svelte';
  import { selectionStore, type QuickBarPanel } from '$lib/stores/selection.svelte';
  import { themeStore } from '$lib/stores/theme.svelte';
  import { confirmStore } from '$lib/stores/confirm.svelte';
  import { toastStore } from '$lib/stores/toast.svelte';
  import { i18n } from '$lib/i18n';
  import { hasTauriRuntime } from '$lib/utils/tauri';
  import type { ClipItem, PasteMode, ReorderDirection } from '$lib/types';
  import SearchBar from '$lib/components/SearchBar.svelte';
  import ClipboardItem from '$lib/components/ClipboardItem.svelte';
  import ClipPreview from '$lib/components/ClipPreview.svelte';
  import SettingsPage from './settings/+page.svelte';
  import PermissionCheck from '$lib/components/PermissionCheck.svelte';
  import Toast from '$lib/components/Toast.svelte';
  import ConfirmDialog from '$lib/components/ui/ConfirmDialog.svelte';
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
    Search,
  } from 'lucide-svelte';

  const initialSettingsCheck =
    typeof window !== 'undefined' &&
    (window as any).__TAURI_INTERNALS__?.metadata?.currentWindow?.label === 'settings';

  if (initialSettingsCheck) {
    router.goToSettings();
  }

  let isSettings = $state(initialSettingsCheck);
  let resultsScroller: Element | null = $state(null);

  const SEARCH_INPUT_ID = 'quickbar-search';
  const SCROLL_EDGE_PADDING = 6;
  const isMac =
    typeof navigator !== 'undefined' && navigator.platform.toUpperCase().includes('MAC');
  const shortcutModifierLabel = isMac ? '⌘' : 'Ctrl';

  interface QuickBarOpenedPayload {
    panel?: QuickBarPanel;
  }

  const t = $derived(i18n.t);
  const displayItems = $derived(
    selectionStore.panel === 'pinned'
      ? clipboardStore.pinnedDisplayItems
      : clipboardStore.recentDisplayItems
  );
  const selectedIndex = $derived(
    clampSelectedIndex(selectionStore.selectedIndex, displayItems.length)
  );
  const selectedItem = $derived(displayItems[selectedIndex]);

  // Master-detail: show the preview pane only when the window is wide enough and
  // there's an actual list to preview (loading/empty states span full width).
  const PREVIEW_MIN_VIEWPORT = 620;
  let viewportWidth = $state(typeof window !== 'undefined' ? window.innerWidth : 1024);
  const showPreview = $derived(
    viewportWidth >= PREVIEW_MIN_VIEWPORT && !clipboardStore.isLoading && displayItems.length > 0
  );

  // Hover-to-select is only honored after a genuine pointer move, so keyboard
  // navigation isn't hijacked when the list scrolls under a stationary cursor.
  let hoverSelectArmed = $state(true);

  $effect(() => {
    const clampedIndex = selectedIndex;
    if (clampedIndex !== selectionStore.selectedIndex) {
      selectionStore.selectedIndex = clampedIndex;
    }
  });

  function isTextInput(element: Element | null) {
    return (
      element instanceof HTMLInputElement ||
      element instanceof HTMLTextAreaElement ||
      element instanceof HTMLSelectElement ||
      element?.hasAttribute('contenteditable')
    );
  }

  function clampSelectedIndex(index: number, itemCount: number) {
    if (itemCount <= 0) return 0;
    return Math.min(Math.max(index, 0), itemCount - 1);
  }

  function movedIndex(delta: number, itemCount: number) {
    if (itemCount <= 0) return 0;

    const currentIndex = clampSelectedIndex(selectionStore.selectedIndex, itemCount);
    return (((currentIndex + delta) % itemCount) + itemCount) % itemCount;
  }

  function scrollItemIntoView(index: number) {
    const scroller = resultsScroller;
    const item = displayItems[index];
    if (!scroller || !item) return;
    if (!(scroller instanceof globalThis.HTMLElement)) return;

    const element = document.getElementById(`clip-item-${item.id}`);
    if (!(element instanceof globalThis.HTMLElement)) return;

    const scrollerRect = scroller.getBoundingClientRect();
    const elementRect = element.getBoundingClientRect();
    const topOverflow = scrollerRect.top + SCROLL_EDGE_PADDING - elementRect.top;
    const bottomOverflow = elementRect.bottom - (scrollerRect.bottom - SCROLL_EDGE_PADDING);

    if (topOverflow > 0) {
      scroller.scrollTop -= topOverflow;
    } else if (bottomOverflow > 0) {
      scroller.scrollTop += bottomOverflow;
    }
  }

  function selectIndexAndReveal(index: number, itemCount: number) {
    selectionStore.setSelectedIndex(index, itemCount);
    flushSync();
    scrollItemIntoView(selectionStore.selectedIndex);
  }

  function resetPanelAndReveal(panel: QuickBarPanel) {
    selectionStore.reset(panel);
    flushSync();
    scrollItemIntoView(0);
  }

  const attachResultsScroller: Attachment = (element) => {
    if (!(element instanceof globalThis.HTMLDivElement)) return;

    resultsScroller = element;

    return () => {
      if (resultsScroller === element) {
        resultsScroller = null;
      }
    };
  };

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
    if (selectionStore.panel !== panel) {
      resetPanelAndReveal(panel);
    }
    focusSearchInput();
  }

  function getSelectedItem() {
    return selectedItem;
  }

  function modeForDefault(opposite = false): PasteMode {
    return opposite ? 'opposite' : 'default';
  }

  async function useItem(item: ClipItem | undefined, mode: PasteMode = modeForDefault()) {
    if (!item) return;
    if (clipboardStore.isSearchPending) return;

    await clipboardStore.useClip(item, mode);
  }

  async function useSelectedItem(opposite = false) {
    await useItem(getSelectedItem(), modeForDefault(opposite));
  }

  async function useSlot(slotNumber: number) {
    if (clipboardStore.isSearchPending) return;

    const item = displayItems[slotNumber - 1];
    if (!item) return;

    selectIndexAndReveal(slotNumber - 1, displayItems.length);
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
      selectIndexAndReveal(nextIndex, clipboardStore.pinnedDisplayItems.length);
    }
  }

  async function clearHistory() {
    const confirmed = await confirmStore.ask({
      title: t.clearNonPinned,
      message: t.confirmClearHistory,
      confirmLabel: t.clear,
      destructive: true,
    });
    if (confirmed) {
      try {
        await clipboardStore.clearNonPinned();
      } catch (_error) {
        toastStore.add(t.clearFailed, 'error');
      }
    }
    focusSearchInput();
  }

  async function openSettingsWindow() {
    try {
      await invoke('open_settings_window');
    } catch (error) {
      console.error('[ERROR] Failed to open settings window:', error);
    }
  }

  function syncTheme(theme: typeof themeStore.current): Attachment {
    return () => {
      const root = globalThis.document.documentElement;
      const isDark =
        theme === 'dark' ||
        (theme === 'system' && globalThis.matchMedia('(prefers-color-scheme: dark)').matches);

      root.classList.remove('dark', 'light-pink');

      if (theme === 'light-pink') {
        root.classList.add('light-pink');
      } else if (isDark) {
        root.classList.add('dark');
      }

      globalThis.localStorage.setItem('theme', theme);
    };
  }

  function handleQuickBarKeydown(event: KeyboardEvent) {
    if (router.currentRoute !== 'home' || event.defaultPrevented || event.isComposing) return;
    if (confirmStore.open) return;

    const activeElement = document.activeElement;
    const activeTextInput = isTextInput(activeElement);
    const activeSearchInput =
      activeElement instanceof HTMLInputElement && activeElement.id === SEARCH_INPUT_ID;
    const hasModifier = event.metaKey || event.ctrlKey;

    if (activeTextInput && !activeSearchInput) return;

    if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
      event.preventDefault();
      hoverSelectArmed = false;

      if (selectionStore.panel === 'pinned' && hasModifier && event.shiftKey) {
        void reorderSelectedPinned(event.key === 'ArrowUp' ? 'up' : 'down');
      } else {
        const nextIndex = movedIndex(event.key === 'ArrowUp' ? -1 : 1, displayItems.length);
        selectIndexAndReveal(nextIndex, displayItems.length);
      }

      return;
    }

    if (event.key === 'Tab') {
      event.preventDefault();
      hoverSelectArmed = false;
      resetPanelAndReveal(selectionStore.panel === 'recent' ? 'pinned' : 'recent');
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

    if (event.key === 'Delete' || event.key === 'Backspace') {
      if (hasModifier) {
        event.preventDefault();
        void deleteSelectedItem();
      }

      return;
    }

    // Slot quick-paste lives on the modifier (⌘/Ctrl+1-9) so plain digits stay
    // available to type into the always-focused search box.
    if (hasModifier && !event.altKey && /^[1-9]$/.test(event.key)) {
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
    const tauriAvailable = hasTauriRuntime();

    if (isSettings || (tauriAvailable && getCurrentWindow().label === 'settings')) {
      isSettings = true;
      router.goToSettings();
      return;
    }

    // Transparent body so the rounded panel corners + shadow show (main window
    // only). The panel stays opaque — no frosted glass.
    document.documentElement.classList.add('quickbar-window');

    selectionStore.reset('recent');
    focusSearchInput();
    window.addEventListener('keydown', handleQuickBarKeydown, true);

    if (!tauriAvailable) {
      return () => {
        window.removeEventListener('keydown', handleQuickBarKeydown, true);
        document.documentElement.classList.remove('quickbar-window');
      };
    }

    let unlistenQuickbarOpened: (() => void) | undefined;

    void listen<QuickBarOpenedPayload>('quickbar-opened', (event) => {
      resetPanelAndReveal(event.payload?.panel === 'pinned' ? 'pinned' : 'recent');
      if (clipboardStore.searchQuery.trim()) {
        void clipboardStore.clearSearch({ showLoading: false });
      }
      void clipboardStore.refreshSettings();
      focusSearchInput();
    }).then((unlisten) => {
      unlistenQuickbarOpened = unlisten;
    });

    void clipboardStore.refreshSettings();

    return () => {
      window.removeEventListener('keydown', handleQuickBarKeydown, true);
      unlistenQuickbarOpened?.();
      document.documentElement.classList.remove('quickbar-window');
    };
  });
</script>

<svelte:window
  onresize={() => (viewportWidth = window.innerWidth)}
  onpointermove={() => (hoverSelectArmed = true)}
/>

<Toast />
<ConfirmDialog />

{#if isSettings || router.currentRoute === 'settings'}
  <div class="contents" {@attach syncTheme(themeStore.current)}>
    <SettingsPage />
  </div>
{:else}
  <div class="flex h-screen flex-col p-3" {@attach syncTheme(themeStore.current)}>
    <div class="quickbar-panel flex h-full min-h-0 flex-col overflow-hidden rounded-xl">
      <PermissionCheck />

      <!-- Spotlight-style search row -->
      <div
        class="flex flex-none items-center gap-2 border-b border-border/60 px-4 py-2.5 bg-transparent"
      >
        <div class="min-w-0 flex-1">
          <SearchBar />
        </div>

        <!-- Sliding Capsule Tab Switcher -->
        <div
          class="relative flex w-40 flex-none rounded-lg bg-muted/65 p-0.5 text-[11px] font-semibold border border-border/10 select-none"
          role="tablist"
        >
          <!-- Sliding pill background -->
          <div
            class="absolute top-0.5 bottom-0.5 left-0.5 rounded-md bg-background shadow-sm transition-all duration-300 ease-[cubic-bezier(0.16,1,0.3,1)]"
            style="width: calc(50% - 2px); transform: translateX({selectionStore.panel === 'pinned'
              ? '100%'
              : '0'});"
          ></div>

          <button
            role="tab"
            aria-selected={selectionStore.panel === 'recent'}
            aria-controls="clipboard-content"
            tabindex={selectionStore.panel === 'recent' ? 0 : -1}
            class="relative z-10 flex-1 py-1 rounded-md text-center cursor-pointer transition-colors duration-200 {selectionStore.panel ===
            'recent'
              ? 'text-foreground font-semibold'
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
            class="relative z-10 flex-1 py-1 rounded-md text-center cursor-pointer transition-colors duration-200 {selectionStore.panel ===
            'pinned'
              ? 'text-foreground font-semibold'
              : 'text-muted-foreground hover:text-foreground'}"
            onclick={() => switchPanel('pinned')}
          >
            {t.pinned}<span class="ml-1 opacity-60">{clipboardStore.pinnedItems.length}</span>
          </button>
        </div>
      </div>

      <!-- Results + Preview (master-detail) -->
      <div class="flex min-h-0 flex-1 overflow-hidden">
        <main
          id="clipboard-content"
          class="flex min-h-0 flex-1 flex-col overflow-hidden {showPreview
            ? 'border-r border-border/60'
            : ''}"
        >
          {#if clipboardStore.isLoading}
            <div
              class="flex h-full flex-col items-center justify-center gap-2 text-muted-foreground"
            >
              <Loader2 class="h-6 w-6 animate-spin" />
              <p class="text-sm">{t.loading}</p>
            </div>
          {:else if displayItems.length === 0}
            <div
              class="flex h-full flex-col items-center justify-center gap-1.5 p-6 text-center text-muted-foreground"
            >
              {#if clipboardStore.activeSearchQuery.trim()}
                <Search class="h-8 w-8 opacity-20" />
                <p class="text-sm font-medium">{t.noSearchResults}</p>
                <p class="text-xs opacity-70">{t.noSearchResultsHint}</p>
              {:else if selectionStore.panel === 'pinned'}
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
            <div
              {@attach attachResultsScroller}
              role="list"
              class="flex-1 space-y-1 overflow-y-auto p-2"
            >
              {#each displayItems as item, index (item.id)}
                <ClipboardItem
                  {item}
                  selected={index === selectedIndex}
                  slotNumber={index < 9 ? index + 1 : null}
                  onSelect={() => selectionStore.setSelectedIndex(index, displayItems.length)}
                  onHover={() => {
                    if (hoverSelectArmed)
                      selectionStore.setSelectedIndex(index, displayItems.length);
                  }}
                  onUse={() => useItem(item)}
                />
              {/each}
            </div>
          {/if}
        </main>

        {#if showPreview}
          <aside class="flex w-[42%] min-w-0 flex-none flex-col overflow-hidden">
            <ClipPreview item={selectedItem} />
          </aside>
        {/if}
      </div>

      <!-- Footer: shortcut hints + quick actions -->
      <div
        class="flex flex-none items-center justify-between gap-2 border-t border-border/60 bg-muted/15 px-2.5 py-1.5 text-[11px] text-muted-foreground animate-in fade-in duration-300"
      >
        <div class="flex min-w-0 items-center gap-3 overflow-hidden select-none">
          <span class="flex flex-none items-center gap-1.5">
            <kbd class="kbd-keycap text-[9px] min-w-4 h-4 scale-95">↵</kbd>
            {clipboardStore.autoPaste ? t.paste : t.copy}
          </span>
          <span class="flex flex-none items-center gap-1.5">
            <kbd class="kbd-keycap text-[9px] min-w-8 h-4 scale-95">{shortcutModifierLabel}↵</kbd>
            {clipboardStore.autoPaste ? t.copy : t.paste}
          </span>
          <span class="flex flex-none items-center gap-1.5">
            <kbd class="kbd-keycap text-[9px] min-w-10 h-4 scale-95">{shortcutModifierLabel}1-9</kbd
            >
            {t.slot}
          </span>
          <span class="flex flex-none items-center gap-1.5">
            <kbd class="kbd-keycap text-[9px] min-w-8 h-4 scale-95">{shortcutModifierLabel}P</kbd>
            {t.pin}
          </span>
          <span class="flex flex-none items-center gap-1.5">
            <kbd class="kbd-keycap text-[9px] min-w-8 h-4 scale-95">{shortcutModifierLabel}⌫</kbd>
            {t.delete}
          </span>
          {#if selectionStore.panel === 'pinned'}
            <span class="flex flex-none items-center gap-1.5">
              <kbd class="kbd-keycap text-[9px] min-w-12 h-4 scale-95"
                >{shortcutModifierLabel}⇧↑↓</kbd
              >
              {t.reorder}
            </span>
          {/if}
          <span class="flex flex-none items-center gap-1.5">
            <kbd class="kbd-keycap text-[9px] min-w-8 h-4 scale-95">Tab</kbd>
            {t.switchPanel}
          </span>
          <span class="flex flex-none items-center gap-1.5">
            <kbd class="kbd-keycap text-[9px] min-w-8 h-4 scale-95">esc</kbd>
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
