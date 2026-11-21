import { SvelteSet } from 'svelte/reactivity';
import { RequestSequencer } from '$lib/utils/request-sequencer';

export type QuickBarPanel = 'recent' | 'pinned';
type Item = { id: string };

function clampIndex(index: number, itemCount: number) {
  return Math.max(0, Math.min(index, itemCount - 1));
}

class SelectionStore {
  panel = $state<QuickBarPanel>('recent');
  selectedId = $state<string | null>(null);
  selectedIds = new SvelteSet<string>();
  private uses = new RequestSequencer();
  private anchorId: string | null = null;

  index(items: readonly Item[]) {
    return Math.max(
      0,
      items.findIndex((item) => item.id === this.selectedId)
    );
  }

  setSelectedIndex(index: number, items: readonly Item[]) {
    this.cancelUse();
    this.selectedId = items[clampIndex(index, items.length)]?.id ?? null;
  }

  move(delta: number, items: readonly Item[]) {
    this.clearSelection();
    const count = items.length;
    this.setSelectedIndex(count ? (this.index(items) + delta + count) % count : 0, items);
  }

  extendSelection(delta: number, items: readonly Item[]) {
    const from = this.index(items);
    if (!this.hasAnchor(items)) this.anchorId = items[from]?.id ?? null;
    this.setSelectedIndex(from + delta, items);
    this.selectRange(items);
  }

  extendTo(id: string, items: readonly Item[]) {
    this.setSelectedIndex(
      items.findIndex((item) => item.id === id),
      items
    );
    if (this.hasAnchor(items)) return this.selectRange(items);
    this.selectedIds.clear();
    this.toggleSelected(id);
  }

  private hasAnchor(items: readonly Item[]) {
    return this.selectedIds.size > 0 && items.some((item) => item.id === this.anchorId);
  }

  private selectRange(items: readonly Item[]) {
    const anchor = items.findIndex((item) => item.id === this.anchorId);
    const cursor = this.index(items);
    this.selectedIds.clear();
    if (anchor === cursor) return;
    for (const item of items.slice(Math.min(anchor, cursor), Math.max(anchor, cursor) + 1))
      this.selectedIds.add(item.id);
  }

  reset(panel: QuickBarPanel = 'recent') {
    this.panel = panel;
    this.resetSelection();
    this.cancelUse();
  }

  resetSelection() {
    this.selectedId = null;
    this.clearSelection();
  }

  toggleSelected(id: string) {
    this.cancelUse();
    if (!this.selectedIds.delete(id)) this.selectedIds.add(id);
    this.anchorId = id;
  }

  clearSelection() {
    this.selectedIds.clear();
  }

  cancelUse() {
    this.uses.next();
  }
  beginUse() {
    return this.uses.next();
  }
  isCurrentUse(revision: number) {
    return this.uses.isCurrent(revision);
  }
}

export const selectionStore = new SelectionStore();
