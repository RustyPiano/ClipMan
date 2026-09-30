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

  /** 不按 Shift 的 ↑/↓：与访达一致，移动光标并清空多选。 */
  move(delta: number, items: readonly Item[]) {
    this.clearSelection();
    const count = items.length;
    this.setSelectedIndex(count ? (this.index(items) + delta + count) % count : 0, items);
  }

  /** 光标移动一行，并把多选设为锚点到光标之间的连续范围（到边界不回绕）；范围缩回一行时取消多选。 */
  extendSelection(delta: number, items: readonly Item[]) {
    const from = this.index(items);
    if (!this.hasAnchor(items)) this.anchorId = items[from]?.id ?? null;
    this.setSelectedIndex(from + delta, items);
    this.selectRange(items);
  }

  /** ⇧ 点击：还没有多选时只勾选这一行并作为锚点，否则选中锚点到这一行的连续范围。 */
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
