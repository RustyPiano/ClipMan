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
  // ⇧↑/↓ 从锚点扩展到光标；单独勾选的行会成为新的锚点
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
    const count = items.length;
    this.setSelectedIndex(count ? (this.index(items) + delta + count) % count : 0, items);
  }

  /** 光标移动一行，并把多选设为锚点到光标之间的连续范围（到边界不回绕）。 */
  extendSelection(delta: number, items: readonly Item[]) {
    const from = this.index(items);
    if (!this.selectedIds.size || !items.some((item) => item.id === this.anchorId))
      this.anchorId = items[from]?.id ?? null;
    this.setSelectedIndex(from + delta, items);
    const anchor = items.findIndex((item) => item.id === this.anchorId);
    const cursor = this.index(items);
    this.selectedIds.clear();
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
