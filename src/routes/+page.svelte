<script lang="ts">
  import { onMount } from 'svelte';
  import type { Attachment } from 'svelte/attachments';
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { clipboardStore } from '$lib/stores/clipboard.svelte';
  import { selectionStore, type QuickBarPanel } from '$lib/stores/selection.svelte';
  import { themeStore } from '$lib/stores/theme.svelte';
  import { confirmStore } from '$lib/stores/confirm.svelte';
  import { toastStore } from '$lib/stores/toast.svelte';
  import { i18n } from '$lib/i18n';
  import { isMac } from '$lib/utils/platform';
  import { SEARCH_INPUT_ID, ROW_HEIGHT_REM } from '$lib/constants';
  import type { ClipItem, PasteMode } from '$lib/types';
  import SearchBar from '$lib/components/SearchBar.svelte';
  import ClipboardItem from '$lib/components/ClipboardItem.svelte';
  import ClipPreview from '$lib/components/ClipPreview.svelte';
  import SettingsPage from './settings/+page.svelte';
  import PermissionCheck from '$lib/components/PermissionCheck.svelte';
  import Toast from '$lib/components/Toast.svelte';
  import ConfirmDialog from '$lib/components/ui/ConfirmDialog.svelte';
  import Button from '$lib/components/ui/Button.svelte';
  import { Settings, PanelRight, MoreHorizontal, Loader2, Search } from 'lucide-svelte';

  const isSettingsWindow = getCurrentWindow().label === 'settings';
  $effect(() => {
    document.documentElement.lang = i18n.locale;
  });
  const t = $derived(i18n.t);
  const modifier = isMac ? '⌘' : 'Ctrl';
  const displayItems = $derived(
    selectionStore.panel === 'pinned'
      ? clipboardStore.pinnedDisplayItems
      : clipboardStore.recentDisplayItems
  );
  const selectedIndex = $derived(selectionStore.index(displayItems));
  const selectedItem = $derived(displayItems[selectedIndex]);
  const displayError = $derived(
    clipboardStore.activeSearchQuery.trim()
      ? clipboardStore.searchError
      : clipboardStore.historyError
  );
  let resultsScroller: HTMLDivElement | undefined = $state();
  let actions: HTMLDetailsElement | undefined = $state();
  let hoverSelectArmed = false;
  let scrollTop = $state(0);
  let viewportHeight = $state(480);
  let viewportWidth = $state(window.innerWidth);
  let previewEnabled = $state(localStorage.getItem('preview-enabled') !== 'false');
  const showPreview = $derived(previewEnabled && viewportWidth >= 620 && !!selectedItem);
  // One shared rem-based height keeps CSS, keyboard reveal and virtualization in sync.
  let rowHeight = $state(ROW_HEIGHT_REM * 16);
  const OVERSCAN = 4;
  const startIndex = $derived(
    Math.max(0, Math.min(displayItems.length - 1, Math.floor(scrollTop / rowHeight) - OVERSCAN))
  );
  const endIndex = $derived(
    Math.min(displayItems.length, Math.ceil((scrollTop + viewportHeight) / rowHeight) + OVERSCAN)
  );
  const visibleItems = $derived(displayItems.slice(startIndex, endIndex));
  const selectedVisible = $derived(selectedIndex >= startIndex && selectedIndex < endIndex);
  const skippedImages = $derived(
    displayItems.filter(
      (item) => selectionStore.selectedIds.has(item.id) && item.contentType === 'image'
    ).length
  );

  let lastQuery = clipboardStore.activeSearchQuery;
  $effect(() => {
    const query = clipboardStore.activeSearchQuery;
    if (query !== lastQuery) {
      lastQuery = query;
      scrollTop = 0;
      if (resultsScroller) resultsScroller.scrollTop = 0;
    }
  });

  $effect(() => {
    if (selectionStore.selectedId && selectedItem?.id !== selectionStore.selectedId) {
      selectionStore.setSelectedIndex(0, displayItems);
      revealSelection();
    }
  });

  function focusSearch() {
    document.getElementById(SEARCH_INPUT_ID)?.focus({ preventScroll: true });
  }

  function resetPanel(panel: QuickBarPanel) {
    selectionStore.reset(panel);
    if (panel === 'recent') clipboardStore.resetRecentPagination();
    if (resultsScroller) resultsScroller.scrollTop = 0;
    scrollTop = 0;
    focusSearch();
  }

  function revealSelection() {
    if (!resultsScroller) return;
    const top = selectionStore.index(displayItems) * rowHeight;
    const next = Math.max(top + rowHeight - viewportHeight, Math.min(scrollTop, top));
    resultsScroller.scrollTop = next;
    scrollTop = next;
  }

  function select(index: number) {
    selectionStore.setSelectedIndex(index, displayItems);
  }

  function observeScroller(element: HTMLDivElement) {
    const observer = new ResizeObserver(() => {
      viewportHeight = element.clientHeight;
      rowHeight = parseFloat(getComputedStyle(document.documentElement).fontSize) * ROW_HEIGHT_REM;
    });
    observer.observe(element);
    observer.observe(document.documentElement);
    return () => observer.disconnect();
  }

  function handleScroll() {
    if (!resultsScroller) return;
    scrollTop = resultsScroller.scrollTop;
    if (
      selectionStore.panel === 'recent' &&
      resultsScroller.scrollHeight - scrollTop < viewportHeight * 3
    ) {
      void clipboardStore.loadMoreRecent();
    }
  }

  // `plain` is the explicit override (⌥Enter); undefined follows the
  // paste-format mode inside the store's useClip.
  async function useSelection(mode: PasteMode = 'default', plain?: boolean, slot?: number) {
    if (clipboardStore.isUsing) return;
    const revision = selectionStore.beginUse();
    const query = clipboardStore.searchQuery;
    if (clipboardStore.isSearchPending) {
      let timer: ReturnType<typeof setTimeout>;
      const ready = await Promise.race([
        clipboardStore.search(query).then(() => true),
        new Promise<boolean>((resolve) => {
          timer = setTimeout(() => resolve(false), 1500);
        }),
      ]).finally(() => clearTimeout(timer));
      if (!ready) {
        toastStore.add(t.searchPendingHint, 'info');
        return;
      }
      if (
        clipboardStore.isSearchPending ||
        clipboardStore.searchError ||
        clipboardStore.activeSearchQuery !== query
      )
        return;
    }
    if (!selectionStore.isCurrentUse(revision)) return;
    if (slot === undefined && selectionStore.selectedIds.size >= 2) {
      await clipboardStore.useSelectedClips(mode);
    } else {
      const item = displayItems[slot ?? selectionStore.index(displayItems)];
      if (item) await clipboardStore.useClip(item, mode, { plain });
    }
  }

  async function useItem(item: ClipItem) {
    if (clipboardStore.isSearchPending) return;
    await clipboardStore.useClip(item);
  }

  async function clearHistory() {
    if (
      await confirmStore.ask({
        title: t.clearNonPinned,
        message: t.confirmClearHistory,
        confirmLabel: t.clear,
        destructive: true,
      })
    ) {
      try {
        await clipboardStore.clearNonPinned();
      } catch (error) {
        toastStore.add(String(error), 'error');
      }
    }
    focusSearch();
  }

  function handleKey(event: KeyboardEvent) {
    if (isSettingsWindow || event.defaultPrevented || event.isComposing || confirmStore.open)
      return;
    const target = event.target as HTMLElement;
    if (target.closest('[data-row-editor]')) return;
    if (target.closest('[data-actions]')) {
      if (event.key === 'Escape') {
        event.preventDefault();
        if (actions) actions.open = false;
        focusSearch();
      }
      return;
    }
    if (event.key === 'Escape') {
      event.preventDefault();
      selectionStore.cancelUse();
      if (selectionStore.selectedIds.size) selectionStore.clearSelection();
      else void clipboardStore.hideQuickbar();
      return;
    }
    const input = target.id === SEARCH_INPUT_ID;
    if (!input && target.closest('button, a, summary, input, textarea, select')) return;
    const mod = event.metaKey || event.ctrlKey;
    if (event.altKey && (event.key === 'ArrowLeft' || event.key === 'ArrowRight')) {
      event.preventDefault();
      resetPanel(selectionStore.panel === 'recent' ? 'pinned' : 'recent');
    } else if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
      event.preventDefault();
      hoverSelectArmed = false;
      if (mod && event.shiftKey && selectionStore.panel === 'pinned' && selectedItem) {
        void clipboardStore
          .reorderPinned(selectedItem.id, event.key === 'ArrowUp' ? 'up' : 'down')
          .catch((error) => toastStore.add(String(error), 'error'));
      } else {
        const delta = event.key === 'ArrowDown' ? 1 : -1;
        if (event.shiftKey && !mod) selectionStore.extendSelection(delta, displayItems);
        else selectionStore.move(delta, displayItems);
        revealSelection();
        if (selectionStore.panel === 'recent' && selectedIndex >= displayItems.length - 10) {
          void clipboardStore.loadMoreRecent();
        }
      }
    } else if (event.key === 'Enter') {
      event.preventDefault();
      if (!event.repeat)
        void useSelection(
          mod ? 'opposite' : 'default',
          event.altKey ? !clipboardStore.takesPlainText : undefined
        );
    } else if (mod && /^[1-9]$/.test(event.key)) {
      event.preventDefault();
      if (!event.repeat) void useSelection('default', undefined, Number(event.key) - 1);
    } else if (
      mod &&
      event.key.toLowerCase() === 'p' &&
      selectedItem &&
      !clipboardStore.isSearchPending
    ) {
      event.preventDefault();
      void clipboardStore.togglePin(selectedItem.id);
    } else if (
      mod &&
      ['Backspace', 'Delete'].includes(event.key) &&
      selectedItem &&
      !clipboardStore.isSearchPending
    ) {
      event.preventDefault();
      void clipboardStore.deleteItem(selectedItem.id);
    } else if (!input && !mod && !event.altKey && event.key.length === 1) {
      event.preventDefault();
      focusSearch();
      const search = document.getElementById(SEARCH_INPUT_ID) as HTMLInputElement;
      search.setRangeText(event.key, search.selectionStart ?? 0, search.selectionEnd ?? 0, 'end');
      search.dispatchEvent(new Event('input', { bubbles: true }));
    }
  }

  function syncTheme(theme: typeof themeStore.current): Attachment {
    return () => {
      const root = document.documentElement;
      const media = matchMedia('(prefers-color-scheme: dark)');

      const apply = () => {
        const isDark = theme === 'dark' || (theme === 'system' && media.matches);

        root.classList.remove('dark', 'light-pink');

        if (theme === 'light-pink') {
          root.classList.add('light-pink');
        } else if (isDark) {
          root.classList.add('dark');
        }
      };

      apply();
      localStorage.setItem('theme', theme);

      // In 'system' mode the effective theme tracks the OS appearance, so follow
      // live light/dark switches; fixed modes need no listener. The attachment is
      // re-run whenever themeStore.current changes, so this cleanup detaches the
      // listener as soon as the user leaves 'system'.
      if (theme === 'system') {
        media.addEventListener('change', apply);
        return () => media.removeEventListener('change', apply);
      }
    };
  }

  onMount(() => {
    if (isSettingsWindow) return;
    document.documentElement.classList.add('quickbar-window');
    void clipboardStore.initialize();
    focusSearch();
    const unlisteners: (() => void)[] = [];
    let disposed = false;
    for (const subscription of [
      listen<{ panel?: QuickBarPanel }>('quickbar-opened', (event) => {
        resetPanel(event.payload?.panel === 'pinned' ? 'pinned' : 'recent');
      }),
      listen('quickbar-hidden', () => {
        scrollTop = 0;
        if (actions) actions.open = false;
      }),
    ])
      void subscription.then((stop) => (disposed ? stop() : unlisteners.push(stop)));
    return () => {
      disposed = true;
      unlisteners.forEach((stop) => stop());
      document.documentElement.classList.remove('quickbar-window');
    };
  });
</script>

<svelte:window
  onkeydowncapture={handleKey}
  onresize={() => (viewportWidth = window.innerWidth)}
  onpointermove={() => (hoverSelectArmed = true)}
/>
<Toast />
<ConfirmDialog />
{#if isSettingsWindow}
  <div class="contents" {@attach syncTheme(themeStore.current)}><SettingsPage /></div>
{:else}
  <div
    class="quickbar-panel flex h-screen flex-col overflow-hidden rounded-xl"
    {@attach syncTheme(themeStore.current)}
  >
    <PermissionCheck />
    <header class="qb-header flex flex-none items-center gap-3 px-3 py-3">
      <div class="min-w-0 flex-1">
        <SearchBar
          activeId={selectedVisible ? selectedItem?.id : undefined}
          expanded={displayItems.length > 0}
        />
      </div>
      <div class="qb-tabs flex flex-none gap-0.5 rounded-lg p-0.5" aria-label={t.switchPanel}>
        {#each ['recent', 'pinned'] as panel (panel)}
          <button
            class="rounded-md px-3 py-1.5 text-xs font-medium"
            aria-pressed={selectionStore.panel === panel}
            onclick={() => resetPanel(panel as QuickBarPanel)}
          >
            {panel === 'recent' ? t.history : t.pinned}
          </button>
        {/each}
      </div>
    </header>
    {#if clipboardStore.capturePaused || clipboardStore.useNotice}
      <p class="border-b border-border px-4 py-2 text-xs text-muted-foreground" role="status">
        {clipboardStore.useNotice || t.capturePausedNotice}
      </p>
    {/if}
    <div class="flex min-h-0 flex-1">
      <main class="flex min-w-0 flex-1 flex-col py-2">
        {#if clipboardStore.isLoading && !displayItems.length}
          <div class="flex flex-1 items-center justify-center gap-2 text-muted-foreground">
            <Loader2 class="h-5 w-5 animate-spin" />{t.loading}
          </div>
        {:else if !displayItems.length}
          <div
            class="flex flex-1 flex-col items-center justify-center gap-3 p-5 text-center text-sm text-muted-foreground"
          >
            <Search class="h-7 w-7" />
            {#if displayError}
              <p role="alert">{displayError}</p>
              <Button
                variant="outline"
                onclick={() =>
                  clipboardStore.activeSearchQuery
                    ? clipboardStore.search(clipboardStore.activeSearchQuery)
                    : clipboardStore.loadHistory()}>{t.recheck}</Button
              >
            {:else}
              <p>
                {clipboardStore.activeSearchQuery
                  ? t.noSearchResults
                  : selectionStore.panel === 'pinned'
                    ? t.noPinnedItems
                    : t.noClipboardHistory}
              </p>
              {#if selectionStore.panel === 'pinned' && clipboardStore.recentDisplayItems.length}
                <Button variant="ghost" onclick={() => resetPanel('recent')}>{t.history}</Button>
              {/if}
            {/if}
          </div>
        {:else}
          <div
            id="clipboard-results"
            role="grid"
            aria-rowcount={displayItems.length}
            aria-colcount="2"
            aria-label={t.history}
            aria-multiselectable="true"
            aria-busy={clipboardStore.isSearchPending}
            bind:this={resultsScroller}
            {@attach observeScroller}
            class="min-h-0 flex-1 overflow-y-auto px-2"
            onscroll={handleScroll}
          >
            <div style:height={`${startIndex * rowHeight}px`} aria-hidden="true"></div>
            {#each visibleItems as item, index (item.id)}
              {@const position = startIndex + index}
              <ClipboardItem
                {item}
                selected={position === selectedIndex}
                multiSelected={selectionStore.selectedIds.has(item.id)}
                multiSelecting={selectionStore.selectedIds.size > 0}
                slotNumber={position < 9 ? position + 1 : null}
                {position}
                onSelect={() => select(position)}
                onToggleSelect={() => selectionStore.toggleSelected(item.id)}
                onHover={() => {
                  if (hoverSelectArmed) select(position);
                }}
                onUse={() => useItem(item)}
              />
            {/each}
            <div
              style:height={`${(displayItems.length - endIndex) * rowHeight}px`}
              aria-hidden="true"
            ></div>
          </div>
        {/if}
        {#if clipboardStore.searchHasMore}<p
            class="px-3 py-1 text-xs text-muted-foreground"
            role="status"
          >
            {t.searchLimit}
          </p>{/if}
      </main>
      {#if showPreview}<aside class="qb-preview w-[40%] flex-none overflow-hidden">
          <ClipPreview item={selectedItem} />
        </aside>{/if}
    </div>
    <footer
      class="qb-footer relative flex flex-none items-center justify-between gap-3 px-4 py-2 text-[11px] text-muted-foreground"
    >
      <div class="min-w-0 truncate" aria-live="polite">
        {#if clipboardStore.isUsing}{t.loading}
        {:else if selectionStore.selectedIds.size >= 2}
          {i18n.format(t.selectedCount, { n: selectionStore.selectedIds.size })} · ↵ {t.mergePasteHint}
          {#if skippedImages}
            · {i18n.format(t.mergeImagesSkipped, { n: skippedImages })}{/if}
        {:else}<span class="inline-flex items-center gap-3">
            <span
              ><kbd class="kbd-keycap">↵</kbd> {clipboardStore.autoPaste ? t.paste : t.copy}</span
            >
            <span
              ><kbd class="kbd-keycap">{modifier}↵</kbd>
              {clipboardStore.autoPaste ? t.copy : t.paste}</span
            >
            <span><kbd class="kbd-keycap">⇧↑↓</kbd> {t.multiSelectHint}</span>
            <span><kbd class="kbd-keycap">Esc</kbd> {t.close}</span>
          </span>{/if}
      </div>
      <div class="flex flex-none items-center gap-1">
        <span class="px-1 tabular-nums"
          >{displayItems.length}{clipboardStore.searchHasMore ? '+' : ''}</span
        >
        <Button
          variant="ghost"
          size="icon"
          title={t.previewToggle}
          aria-pressed={previewEnabled}
          onclick={() => {
            previewEnabled = !previewEnabled;
            localStorage.setItem('preview-enabled', String(previewEnabled));
          }}><PanelRight class="h-4 w-4" /></Button
        >
        <details bind:this={actions} data-actions class="relative">
          <summary
            class="flex h-9 w-9 cursor-pointer list-none items-center justify-center rounded-md hover:bg-muted"
            aria-label={t.actions}><MoreHorizontal class="h-4 w-4" /></summary
          >
          <fieldset
            disabled={clipboardStore.isUsing || clipboardStore.isSearchPending}
            class="absolute bottom-11 right-0 z-20 flex w-64 flex-col gap-1 rounded-lg border border-border bg-background p-2 shadow-lg"
          >
            <Button variant="ghost" onclick={clearHistory}>{t.clearNonPinned}</Button>
            <p class="border-t border-border px-2 pt-2 text-[11px] leading-relaxed">
              {modifier}1–9 {t.slot}<br />⌥↵ {clipboardStore.takesPlainText
                ? t.pasteRich
                : t.pastePlain}<br />Alt ←/→ {t.switchPanel}<br />{modifier}⇧↑↓ {t.reorder}
            </p>
          </fieldset>
        </details>
        <Button
          variant="ghost"
          size="icon"
          title={t.settings}
          onclick={() =>
            invoke('open_settings_window').catch((error) => toastStore.add(String(error), 'error'))}
          ><Settings class="h-4 w-4" /></Button
        >
      </div>
    </footer>
  </div>
{/if}
