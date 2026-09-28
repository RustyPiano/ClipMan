import { SvelteSet } from 'svelte/reactivity';

export type QuickBarPanel = 'recent' | 'pinned';
type Item = { id: string };

function clampIndex(index: number, itemCount: number) {
  return Math.max(0, Math.min(index, itemCount - 1));
}

class SelectionStore {
  panel = $state<QuickBarPanel>('recent');
  selectedId = $state<string | null>(null);
  selectedIds = new SvelteSet<string>();
  private useRevision = 0;

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
    const count = items.length;
    this.setSelectedIndex(count ? (this.index(items) + delta + count) % count : 0, items);
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
  }

  clearSelection() {
    this.selectedIds.clear();
  }

  cancelUse() {
    this.useRevision += 1;
  }
  beginUse() {
    return ++this.useRevision;
  }
  isCurrentUse(revision: number) {
    return revision === this.useRevision;
  }
}

export const selectionStore = new SelectionStore();
