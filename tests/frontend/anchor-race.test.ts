import { beforeEach, describe, expect, test } from 'bun:test';

// Mock the $state rune as a global callable (the selection store is imported
// raw, not compiled) — same pattern as tests/selection.test.ts.
Object.defineProperty(globalThis, '$state', {
  configurable: true,
  value: <T>(value: T) => value,
});

const { selectionStore, clampIndex } = await import('../../src/lib/stores/selection.svelte');

// The anchor-race (REVIEW-2026-07-07 #10) lives in +page.svelte's
// resetPanelAndReveal + anchor $effect timing. The real component can't be
// mounted under bun (no DOM harness), so we reproduce the flushSync ordering
// at the logic level — exactly what the spec asks for ("store/逻辑层面模拟
// flushSync 时序即可"). The mirror below is a line-for-line transcription of
// the relevant component code so the test genuinely characterizes the race.

type Item = { id: string };

class QuickBarModel {
  private recentItems: Item[];
  private pinnedItems: Item[];
  anchoredId: string | null = null;
  private lastSelectionQuery: string;
  private activeSearchQuery: string;
  scrollTop = 0;
  // Whether the one-line fix (null the anchor after reset, before flushSync)
  // is applied. Lets one harness demonstrate both the buggy and fixed ordering.
  private readonly clearAnchorOnReset: boolean;

  constructor(opts: {
    recent: Item[];
    pinned: Item[];
    query?: string;
    clearAnchorOnReset: boolean;
  }) {
    this.recentItems = opts.recent;
    this.pinnedItems = opts.pinned;
    this.activeSearchQuery = opts.query ?? '';
    this.lastSelectionQuery = this.activeSearchQuery;
    this.clearAnchorOnReset = opts.clearAnchorOnReset;
  }

  // Mirror of the `displayItems` $derived (panel-driven).
  private get displayItems(): Item[] {
    return selectionStore.panel === 'pinned' ? this.pinnedItems : this.recentItems;
  }

  // Mirror of the anchor $effect body (+page.svelte lines 102-134). A panel
  // switch mutates displayItems (a tracked dep), so flushSync re-runs this.
  private runAnchorEffect() {
    const query = this.activeSearchQuery;
    const items = this.displayItems;

    if (query !== this.lastSelectionQuery) {
      this.lastSelectionQuery = query;
      selectionStore.selectedIndex = 0;
      this.anchoredId = items[0]?.id ?? null;
      this.scrollTop = 0;
      return;
    }

    if (this.anchoredId === null) return;

    const currentIndex = selectionStore.selectedIndex;
    const nextIndex = items.findIndex((item) => item.id === this.anchoredId);
    if (nextIndex >= 0) {
      if (nextIndex !== currentIndex) {
        selectionStore.selectedIndex = nextIndex;
      }
    } else {
      this.anchoredId = items[clampIndex(currentIndex, items.length)]?.id ?? null;
    }
  }

  // Mirror of anchorToSelection (lines 168-170).
  private anchorToSelection() {
    this.anchoredId = this.displayItems[selectionStore.selectedIndex]?.id ?? null;
  }

  // Mirror of selectIndex (lines 172-175): user highlights a row.
  select(index: number) {
    selectionStore.setSelectedIndex(index, this.displayItems.length);
    this.anchorToSelection();
  }

  // Mirror of resetPanelAndReveal (lines 195-207). flushSync() => the pending
  // anchor effect runs; scrollItemIntoView(0) lands the viewport on row 0.
  resetPanelAndReveal(panel: 'recent' | 'pinned') {
    selectionStore.reset(panel);
    if (this.clearAnchorOnReset) {
      // The fix: drop the stale anchor before the effect can act on it.
      this.anchoredId = null;
    }
    this.runAnchorEffect(); // flushSync()
    this.anchorToSelection();
    this.scrollTop = 0; // scrollItemIntoView(0)
  }
}

describe('anchor-race (#10): a panel reset must not be overridden by a stale anchor', () => {
  beforeEach(() => {
    selectionStore.reset('recent');
  });

  test('reproduces the race: the anchor effect drags the reset selection back to the stale row', () => {
    const model = new QuickBarModel({
      recent: [{ id: 'a' }, { id: 'b' }, { id: 'c' }, { id: 'd' }],
      // 'c' is also pinned, at a non-zero index in the pinned panel.
      pinned: [{ id: 'x' }, { id: 'c' }, { id: 'y' }],
      clearAnchorOnReset: false,
    });

    // User highlights recent row 2 ('c'); the anchor follows the highlight.
    model.select(2);
    expect(selectionStore.selectedIndex).toBe(2);
    expect(model.anchoredId).toBe('c');

    // Switch to the pinned panel (Tab / tab click -> resetPanelAndReveal).
    model.resetPanelAndReveal('pinned');

    // Without the fix the flushSync'd anchor effect restores the selection to
    // 'c' (pinned index 1) even though reset() had just set it to 0 and the
    // viewport was scrolled to row 0 -> highlight offscreen, Enter pastes an
    // invisible row.
    expect(selectionStore.selectedIndex).toBe(1);
    expect(model.scrollTop).toBe(0);
    // The defining symptom: the highlighted row is not the revealed top row.
    expect(selectionStore.selectedIndex).not.toBe(0);
  });

  test('fixed: clearing the anchor before flushSync keeps the reset on row 0', () => {
    const model = new QuickBarModel({
      recent: [{ id: 'a' }, { id: 'b' }, { id: 'c' }, { id: 'd' }],
      pinned: [{ id: 'x' }, { id: 'c' }, { id: 'y' }],
      clearAnchorOnReset: true,
    });

    model.select(2);
    expect(selectionStore.selectedIndex).toBe(2);

    model.resetPanelAndReveal('pinned');

    // Reset wins: selection and viewport agree on the top row, and the anchor
    // is re-established on the new top row so later live reorders still track.
    expect(selectionStore.selectedIndex).toBe(0);
    expect(model.scrollTop).toBe(0);
    expect(model.anchoredId).toBe('x');
  });
});
