<script lang="ts">
  import { onMount, tick } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import { clipboardStore } from '$lib/stores/clipboard.svelte';
  import { selectionStore, type QuickBarPanel } from '$lib/stores/selection.svelte';
  import { appIconStore } from '$lib/stores/app-icons.svelte';
  import { confirmStore } from '$lib/stores/confirm.svelte';
  import { toastStore } from '$lib/stores/toast.svelte';
  import { i18n } from '$lib/i18n';
  import { isMac } from '$lib/utils/platform';
  import { SEARCH_INPUT_ID, ONBOARDING_START_ID, ROW_HEIGHT_REM } from '$lib/constants';
  import type { ClipFilter, PasteMode } from '$lib/types';
  import SearchBar from '$lib/components/SearchBar.svelte';
  import ClipboardItem from '$lib/components/ClipboardItem.svelte';
  import ClipPreview from '$lib/components/ClipPreview.svelte';
  import NewFavoriteForm from '$lib/components/NewFavoriteForm.svelte';
  import PermissionCheck from '$lib/components/PermissionCheck.svelte';
  import Onboarding from '$lib/components/Onboarding.svelte';
  import Button from '$lib/components/ui/Button.svelte';
  import {
    ClipboardList,
    Loader2,
    MoreHorizontal,
    PanelRight,
    Star,
    Plus,
    Search,
    Settings,
    Trash2,
  } from 'lucide-svelte';

  const t = $derived(i18n.t);
  const onboardingOpen = $derived(!clipboardStore.onboardingCompleted);
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

  const filterOptions = $derived<{ id: ClipFilter; label: string }[]>([
    { id: 'all', label: t.filterAll },
    { id: 'text', label: t.filterText },
    { id: 'image', label: t.filterImage },
    { id: 'files', label: t.filterFiles },
    { id: 'link', label: t.filterLink },
  ]);

  let resultsScroller: HTMLDivElement | undefined = $state();
  let actionsOpen = $state(false);
  let actionsContainer: HTMLDivElement | undefined = $state();

  let hoverSelectArmed = false;
  let pointerX = 0;
  let pointerY = 0;
  let disarmedX = 0;
  let disarmedY = 0;

  let scrollTop = $state(0);
  let viewportHeight = $state(440);
  let viewportWidth = $state(window.innerWidth);
  let previewEnabled = $state(localStorage.getItem('preview-enabled') !== 'false');
  const showPreview = $derived(previewEnabled && viewportWidth >= 620 && !!selectedItem);
  let rowHeight = $state(ROW_HEIGHT_REM * 16);
  const OVERSCAN = 5;
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
  const selectedAllPinned = $derived(
    selectionStore.selectedIds.size > 0 &&
      displayItems
        .filter((item) => selectionStore.selectedIds.has(item.id))
        .every((item) => item.isPinned)
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

  let lastSelectedIndex = 0;
  $effect(() => {
    if (selectedItem?.id === selectionStore.selectedId) lastSelectedIndex = selectedIndex;
  });
  $effect(() => {
    if (selectionStore.selectedId && selectedItem?.id !== selectionStore.selectedId) {
      selectionStore.setSelectedIndex(lastSelectedIndex, displayItems);
      revealSelection();
    }
  });

  function focusSearch() {
    const target = clipboardStore.onboardingCompleted ? SEARCH_INPUT_ID : ONBOARDING_START_ID;
    document.getElementById(target)?.focus({ preventScroll: true });
  }

  function openSettings(tab?: 'about') {
    invoke('open_settings_window', { tab }).catch((error) =>
      toastStore.add(String(error), 'error')
    );
  }

  function resetPanel(panel: QuickBarPanel) {
    selectionStore.reset(panel);
    clipboardStore.syncQuickbarPanel(panel);
    if (clipboardStore.filter !== 'all') clipboardStore.setFilter('all');
    actionsOpen = false;
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

  function handlePointerMove(event: PointerEvent) {
    pointerX = event.clientX;
    pointerY = event.clientY;
    if (hoverSelectArmed) return;
    if (Math.hypot(pointerX - disarmedX, pointerY - disarmedY) > 6) {
      hoverSelectArmed = true;
    }
  }

  function disarmHover() {
    hoverSelectArmed = false;
    disarmedX = pointerX;
    disarmedY = pointerY;
  }

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
    const selected = [...selectionStore.selectedIds];
    if (slot === undefined && selected.length >= 2) {
      await clipboardStore.useSelectedClips(mode);
    } else {
      const item =
        slot === undefined && selected.length === 1
          ? displayItems.find((clip) => clip.id === selected[0])
          : displayItems[slot ?? selectionStore.index(displayItems)];
      if (item) await clipboardStore.useClip(item, mode, { plain });
    }
  }

  async function clearHistory() {
    actionsOpen = false;
    await clipboardStore.clearNonPinned();
    focusSearch();
  }

  function openNewPinned() {
    actionsOpen = false;
    selectionStore.panel = 'pinned';
    clipboardStore.syncQuickbarPanel('pinned');
    clipboardStore.newFavoriteOpen = true;
  }

  function handleWindowPointerDown(event: PointerEvent) {
    pointerX = event.clientX;
    pointerY = event.clientY;
    if (actionsOpen && actionsContainer && !actionsContainer.contains(event.target as Node)) {
      actionsOpen = false;
    }
  }

  function handleWindowFocusIn(event: FocusEvent) {
    if (actionsOpen && actionsContainer && !actionsContainer.contains(event.target as Node)) {
      actionsOpen = false;
    }
  }

  function handleKey(event: KeyboardEvent) {
    if (event.defaultPrevented || event.isComposing || event.keyCode === 229 || confirmStore.open)
      return;
    if (!clipboardStore.onboardingCompleted) {
      if (event.key === 'Escape') {
        event.preventDefault();
        void clipboardStore.hideQuickbar();
      }
      return;
    }
    const target = event.target as HTMLElement;
    if (target.closest('[data-row-editor]')) return;
    if (actionsOpen && event.key === 'Escape') {
      event.preventDefault();
      actionsOpen = false;
      focusSearch();
      return;
    }
    if (event.key === 'Escape') {
      event.preventDefault();
      selectionStore.cancelUse();
      if (clipboardStore.newFavoriteOpen) {
        clipboardStore.resetNewFavoriteDraft();
        focusSearch();
      } else if (selectionStore.selectedIds.size) {
        selectionStore.clearSelection();
      } else if (clipboardStore.searchQuery.trim() || clipboardStore.filter !== 'all') {
        clipboardStore.setFilter('all');
        void clipboardStore.clearSearch();
        focusSearch();
      } else {
        void clipboardStore.hideQuickbar();
      }
      return;
    }
    const input = target.id === SEARCH_INPUT_ID;
    if (!input && target.closest('button, a, summary, input, textarea, select')) return;
    const mod = event.metaKey || event.ctrlKey;
    const switchPanelShortcut =
      (mod && (event.key === '[' || event.key === ']')) ||
      (event.ctrlKey && event.key === 'Tab') ||
      (event.altKey &&
        (event.key === 'ArrowLeft' || event.key === 'ArrowRight') &&
        !clipboardStore.searchQuery);

    if (switchPanelShortcut) {
      event.preventDefault();
      resetPanel(selectionStore.panel === 'recent' ? 'pinned' : 'recent');
    } else if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
      event.preventDefault();
      disarmHover();
      if (mod && event.shiftKey && selectionStore.panel === 'pinned' && selectedItem) {
        void clipboardStore
          .reorderPinned(selectedItem.id, event.key === 'ArrowUp' ? 'up' : 'down')
          .then(revealSelection)
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
    } else if (mod && event.key.toLowerCase() === 'p' && !clipboardStore.isSearchPending) {
      event.preventDefault();
      if (selectionStore.selectedIds.size > 0) {
        void clipboardStore.batchTogglePin([...selectionStore.selectedIds]);
      } else if (selectedItem) {
        void clipboardStore.togglePin(selectedItem.id);
      }
    } else if (
      mod &&
      ['Backspace', 'Delete'].includes(event.key) &&
      !clipboardStore.isSearchPending
    ) {
      event.preventDefault();
      if (selectionStore.selectedIds.size > 0) {
        void clipboardStore.batchDeleteItems([...selectionStore.selectedIds]);
      } else if (selectedItem) {
        void clipboardStore.deleteItem(selectedItem.id);
      }
    } else if (!input && !mod && !event.altKey && event.key.length === 1) {
      event.preventDefault();
      focusSearch();
      const search = document.getElementById(SEARCH_INPUT_ID) as HTMLInputElement;
      search.setRangeText(event.key, search.selectionStart ?? 0, search.selectionEnd ?? 0, 'end');
      search.dispatchEvent(new Event('input', { bubbles: true }));
    }
  }

  onMount(() => {
    document.documentElement.classList.add('quickbar-window');
    void clipboardStore.initialize();
    focusSearch();
    const unlisteners: (() => void)[] = [];
    let disposed = false;
    for (const subscription of [
      listen<{ panel?: QuickBarPanel }>('quickbar-opened', (event) => {
        const panel = event.payload?.panel === 'pinned' ? 'pinned' : 'recent';
        if (clipboardStore.editingClipId) {
          const targetId = clipboardStore.editingClipId;
          const inPinned = clipboardStore.pinnedItems.some((item) => item.id === targetId);
          const inRecent = clipboardStore.recentItems.some((item) => item.id === targetId);
          if ((panel === 'pinned' && inPinned) || (panel === 'recent' && (inRecent || inPinned))) {
            selectionStore.panel = panel;
            selectionStore.selectedId = targetId;
            clipboardStore.syncQuickbarPanel(panel);
            actionsOpen = false;
            return;
          }
        }
        if (clipboardStore.newFavoriteOpen && panel === 'pinned') {
          selectionStore.panel = 'pinned';
          clipboardStore.syncQuickbarPanel('pinned');
          actionsOpen = false;
          return;
        }
        resetPanel(panel);
      }),
      listen<string>('app-icon-saved', (event) => appIconStore.invalidate(event.payload)),
      listen('quickbar-hidden', () => {
        scrollTop = 0;
        actionsOpen = false;
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
  onpointermove={handlePointerMove}
  onpointerdown={handleWindowPointerDown}
  onfocusin={handleWindowFocusIn}
/>

<div class="quickbar-panel relative flex h-screen flex-col overflow-hidden rounded-xl">
  {#if clipboardStore.onboardingCompleted}
    <PermissionCheck />
  {:else}
    <Onboarding onDone={() => void tick().then(focusSearch)} />
  {/if}

  <header class="qb-header flex flex-none items-center gap-2 px-3 py-1.5" inert={onboardingOpen}>
    <div class="min-w-0 flex-1">
      <SearchBar
        activeId={selectedVisible ? selectedItem?.id : undefined}
        expanded={displayItems.length > 0}
      />
    </div>
    <div class="qb-tabs flex h-8 flex-none gap-0.5 rounded-lg p-0.5" aria-label={t.switchPanel}>
      <button
        type="button"
        class="rounded-md px-2.5 text-xs font-medium"
        aria-pressed={selectionStore.panel === 'recent'}
        onclick={() => resetPanel('recent')}
      >
        {t.history}
      </button>
      <button
        type="button"
        class="flex items-center gap-1.5 rounded-md px-2.5 text-xs font-medium"
        aria-pressed={selectionStore.panel === 'pinned'}
        onclick={() => resetPanel('pinned')}
      >
        <span>{t.pinned}</span>
        {#if clipboardStore.pinnedItems.length > 0}
          <span
            class="text-[11px] font-normal tabular-nums text-muted-foreground"
            data-pinned-count
          >
            {clipboardStore.pinnedItems.length}
          </span>
        {/if}
      </button>
    </div>
  </header>

  <div
    class="qb-filter-bar flex flex-none items-center justify-between gap-2 px-3 py-1.5"
    inert={onboardingOpen}
  >
    {#if selectionStore.selectedIds.size > 0}
      <div class="flex min-w-0 flex-1 items-center gap-2 text-xs">
        <span class="font-medium text-foreground">
          {i18n.format(t.selectedCount, { n: selectionStore.selectedIds.size })}
        </span>
        {#if selectionStore.selectedIds.size >= 2}
          <button
            type="button"
            class="rounded-md bg-primary px-2 py-0.5 text-[11px] font-medium text-primary-foreground hover:opacity-90"
            onmousedown={(event) => event.preventDefault()}
            onclick={() => void clipboardStore.useSelectedClips('default')}
          >
            {t.mergePaste}
          </button>
        {/if}
        <button
          type="button"
          class="inline-flex items-center gap-1 rounded-md px-2 py-0.5 text-[11px] font-medium text-muted-foreground hover:bg-accent hover:text-foreground"
          onmousedown={(event) => event.preventDefault()}
          onclick={() => void clipboardStore.batchTogglePin([...selectionStore.selectedIds])}
        >
          <Star class="h-3 w-3 {selectedAllPinned ? 'qb-pin-text fill-current' : ''}" />
          {selectedAllPinned ? t.batchUnpin : t.batchPin}
        </button>
        <button
          type="button"
          class="inline-flex items-center gap-1 rounded-md px-2 py-0.5 text-[11px] font-medium text-destructive hover:bg-destructive/10"
          onmousedown={(event) => event.preventDefault()}
          onclick={() => void clipboardStore.batchDeleteItems([...selectionStore.selectedIds])}
        >
          <Trash2 class="h-3 w-3" />
          {t.deleteSelected}
        </button>
      </div>
      <button
        type="button"
        class="rounded-md px-2 py-0.5 text-[11px] text-muted-foreground hover:bg-accent hover:text-foreground"
        onmousedown={(event) => event.preventDefault()}
        onclick={() => selectionStore.clearSelection()}
      >
        {t.cancel}
      </button>
    {:else}
      <div class="flex items-center gap-2" role="group" aria-label={t.filterByType}>
        {#each filterOptions as opt (opt.id)}
          <button
            type="button"
            class="qb-filter-pill px-1.5 py-0.5 text-[11px] font-medium"
            aria-pressed={clipboardStore.filter === opt.id}
            onmousedown={(event) => event.preventDefault()}
            onclick={() => {
              clipboardStore.setFilter(opt.id);
              scrollTop = 0;
              if (resultsScroller) resultsScroller.scrollTop = 0;
            }}
          >
            {opt.label}
          </button>
        {/each}
      </div>
      {#if selectionStore.panel === 'pinned'}
        <button
          type="button"
          class="inline-flex items-center gap-1 rounded-md px-2 py-0.5 text-[11px] font-medium text-muted-foreground hover:bg-accent hover:text-foreground"
          onmousedown={(event) => event.preventDefault()}
          onclick={openNewPinned}
        >
          <Plus class="h-3 w-3" />
          {t.newPinned}
        </button>
      {/if}
    {/if}
  </div>

  {#if clipboardStore.capturePaused || clipboardStore.useNotice}
    <p
      class="border-b border-border px-4 py-1.5 text-xs text-muted-foreground"
      role="status"
      inert={onboardingOpen}
    >
      {clipboardStore.useNotice || t.capturePausedNotice}
    </p>
  {/if}

  <div class="flex min-h-0 flex-1" inert={onboardingOpen}>
    <main class="flex min-w-0 flex-1 flex-col py-1.5">
      {#if clipboardStore.newFavoriteOpen}
        <NewFavoriteForm
          onCreated={(id) => {
            selectionStore.selectedId = id;
            focusSearch();
          }}
          onCancel={focusSearch}
        />
      {/if}

      {#if clipboardStore.isLoading && !displayItems.length}
        <div class="flex flex-1 items-center justify-center gap-2 text-sm text-muted-foreground">
          <Loader2 class="h-4 w-4 animate-spin" />{t.loading}
        </div>
      {:else if !displayItems.length}
        <div
          class="flex flex-1 flex-col items-center justify-center gap-2.5 p-5 text-center text-sm text-muted-foreground"
        >
          {#if !displayError && clipboardStore.filter === 'all' && !clipboardStore.activeSearchQuery && selectionStore.panel === 'recent'}
            <ClipboardList class="h-7 w-7 opacity-60" />
          {:else}
            <Search class="h-7 w-7 opacity-60" />
          {/if}
          {#if displayError}
            <p role="alert">{displayError}</p>
            <Button
              variant="outline"
              size="sm"
              onclick={() =>
                clipboardStore.activeSearchQuery
                  ? clipboardStore.search(clipboardStore.activeSearchQuery)
                  : clipboardStore.loadHistory()}>{t.recheck}</Button
            >
          {:else if clipboardStore.filter !== 'all'}
            <p>{t.noFilterResults}</p>
            <Button variant="ghost" size="sm" onclick={() => clipboardStore.setFilter('all')}
              >{t.showAll}</Button
            >
          {:else if !clipboardStore.activeSearchQuery && selectionStore.panel === 'recent'}
            <p class="font-medium text-foreground">{t.noClipboardHistory}</p>
            <p class="-mt-1 text-xs">{t.emptyHistoryHint}</p>
          {:else}
            <p>
              {clipboardStore.activeSearchQuery ? t.noSearchResults : t.noPinnedItems}
            </p>
            {#if selectionStore.panel === 'pinned'}
              <div class="flex items-center gap-2">
                <Button variant="outline" size="sm" onclick={openNewPinned}>
                  <Plus class="mr-1 h-3.5 w-3.5" />{t.newPinned}
                </Button>
                {#if clipboardStore.recentDisplayItems.length}
                  <Button variant="ghost" size="sm" onclick={() => resetPanel('recent')}
                    >{t.history}</Button
                  >
                {/if}
              </div>
            {/if}
          {/if}
        </div>
      {:else}
        <div
          id="clipboard-results"
          role="grid"
          aria-rowcount={displayItems.length}
          aria-colcount="3"
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
              showReorder={selectionStore.panel === 'pinned' && !clipboardStore.activeSearchQuery}
              canMoveUp={position > 0}
              canMoveDown={position < displayItems.length - 1}
              searchQuery={clipboardStore.activeSearchQuery}
              slotLabel={position < 9 ? `${modifier}${position + 1}` : null}
              {position}
              onSelect={() => select(position)}
              onToggleSelect={() => selectionStore.toggleSelected(item.id)}
              onExtendSelect={() => selectionStore.extendTo(item.id, displayItems)}
              onHover={() => {
                if (hoverSelectArmed && !clipboardStore.isEditing) select(position);
              }}
              onUse={() => clipboardStore.useClip(item)}
              onReorder={(direction) =>
                void clipboardStore
                  .reorderPinned(item.id, direction)
                  .catch((error) => toastStore.add(String(error), 'error'))}
            />
          {/each}
          <div
            style:height={`${(displayItems.length - endIndex) * rowHeight}px`}
            aria-hidden="true"
          ></div>
        </div>
      {/if}
      {#if clipboardStore.searchHasMore}
        <p class="px-3 py-1 text-xs text-muted-foreground" role="status">{t.searchLimit}</p>
      {/if}
    </main>

    {#if showPreview}
      <aside class="qb-preview w-[41%] flex-none overflow-hidden">
        <ClipPreview item={selectedItem} />
      </aside>
    {/if}
  </div>

  <footer
    class="qb-footer relative flex flex-none items-center justify-between gap-3 px-3.5 py-2 text-[11px] text-muted-foreground"
    inert={onboardingOpen}
  >
    <div class="min-w-0 truncate" aria-live="polite">
      {#if clipboardStore.isUsing}
        {t.loading}
      {:else if selectionStore.selectedIds.size === 1}
        {i18n.format(t.selectedCount, { n: 1 })} · ↵ {clipboardStore.autoPaste ? t.paste : t.copy}
      {:else if selectionStore.selectedIds.size >= 2}
        {i18n.format(t.selectedCount, { n: selectionStore.selectedIds.size })} · ↵ {t.mergePasteHint}
        {#if skippedImages}
          · {i18n.format(t.mergeImagesSkipped, { n: skippedImages })}
        {/if}
      {:else}
        <span class="inline-flex items-center gap-3">
          <span><kbd class="kbd-keycap">↵</kbd> {clipboardStore.autoPaste ? t.paste : t.copy}</span>
          <span
            ><kbd class="kbd-keycap">{modifier}↵</kbd>
            {clipboardStore.autoPaste ? t.copy : t.paste}</span
          >
          <span><kbd class="kbd-keycap">{modifier}P</kbd> {t.pin}</span>
          <span><kbd class="kbd-keycap">Esc</kbd> {t.close}</span>
        </span>
      {/if}
    </div>

    <div class="flex flex-none items-center gap-1">
      {#if clipboardStore.updateVersion}
        <button
          type="button"
          class="rounded-md px-1.5 py-0.5 font-medium text-primary hover:bg-accent"
          onmousedown={(event) => event.preventDefault()}
          onclick={() => openSettings('about')}
        >
          {i18n.format(t.newVersionBadge, { version: clipboardStore.updateVersion })}
        </button>
      {/if}
      <span class="px-1.5 tabular-nums"
        >{displayItems.length}{clipboardStore.searchHasMore ? '+' : ''}</span
      >
      <button
        type="button"
        class="qb-footer-icon flex h-7 w-7 items-center justify-center rounded-md"
        title={t.previewToggle}
        aria-label={t.previewToggle}
        aria-pressed={previewEnabled}
        onmousedown={(event) => event.preventDefault()}
        onclick={() => {
          previewEnabled = !previewEnabled;
          localStorage.setItem('preview-enabled', String(previewEnabled));
        }}
      >
        <PanelRight class="h-4 w-4" />
      </button>

      <div bind:this={actionsContainer} data-actions class="relative">
        <button
          type="button"
          class="qb-footer-icon flex h-7 w-7 items-center justify-center rounded-md"
          title={t.actions}
          aria-label={t.actions}
          aria-expanded={actionsOpen}
          aria-haspopup="menu"
          onclick={() => (actionsOpen = !actionsOpen)}
        >
          <MoreHorizontal class="h-4 w-4" />
        </button>

        {#if actionsOpen}
          <div
            role="menu"
            class="qb-popover-panel absolute bottom-9 right-0 z-30 flex w-64 flex-col gap-1 rounded-xl border border-border bg-card p-2 text-card-foreground shadow-xl"
          >
            <button
              type="button"
              role="menuitem"
              class="flex w-full items-center gap-2 rounded-lg px-2.5 py-1.5 text-left text-xs font-medium text-foreground transition-colors hover:bg-accent"
              onclick={openNewPinned}
            >
              <Plus class="h-3.5 w-3.5 text-primary" />
              {t.newPinnedTitle}
            </button>
            <button
              type="button"
              role="menuitem"
              class="flex w-full items-center gap-2 rounded-lg px-2.5 py-1.5 text-left text-xs font-medium text-destructive transition-colors hover:bg-destructive/10"
              disabled={clipboardStore.isUsing || clipboardStore.isSearchPending}
              onclick={clearHistory}
            >
              <Trash2 class="h-3.5 w-3.5" />
              {t.clearNonPinned}
            </button>
            <div
              class="mt-1 border-t border-border px-2 pt-2 text-[11px] leading-relaxed text-muted-foreground"
            >
              <div
                class="mb-1 text-[10px] font-semibold uppercase tracking-wide text-foreground/70"
              >
                {t.shortcutsTitle}
              </div>
              <div class="grid grid-cols-[auto_1fr] gap-x-2 gap-y-0.5">
                <span class="font-mono text-foreground/80">{modifier}1–9</span><span>{t.slot}</span>
                <span class="font-mono text-foreground/80">⌥↵</span><span
                  >{clipboardStore.takesPlainText ? t.pasteRich : t.pastePlain}</span
                >
                <span class="font-mono text-foreground/80">{modifier}[ / ]</span><span
                  >{t.switchPanel}</span
                >
                <span class="font-mono text-foreground/80">⇧↑↓</span><span>{t.multiSelectHint}</span
                >
                <span class="font-mono text-foreground/80">{modifier}⇧↑↓</span><span
                  >{t.reorder}</span
                >
                <span class="font-mono text-foreground/80">{modifier}⌫</span><span>{t.delete}</span>
              </div>
            </div>
          </div>
        {/if}
      </div>

      <button
        type="button"
        class="qb-footer-icon flex h-7 w-7 items-center justify-center rounded-md"
        title={t.settings}
        aria-label={t.settings}
        onclick={() => openSettings()}
      >
        <Settings class="h-4 w-4" />
      </button>
    </div>
  </footer>
</div>
