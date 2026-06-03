export type QuickBarPanel = 'recent' | 'pinned';

function clampIndex(index: number, itemCount: number) {
  if (itemCount <= 0) return 0;
  return Math.min(Math.max(index, 0), itemCount - 1);
}

class SelectionStore {
  panel = $state<QuickBarPanel>('recent');
  selectedIndex = $state(0);

  selectPanel(panel: QuickBarPanel) {
    if (this.panel !== panel) {
      this.panel = panel;
      this.selectedIndex = 0;
    }
  }

  reset(panel: QuickBarPanel = 'recent') {
    this.panel = panel;
    this.selectedIndex = 0;
  }

  togglePanel() {
    this.panel = this.panel === 'recent' ? 'pinned' : 'recent';
    this.selectedIndex = 0;
  }

  setSelectedIndex(index: number, itemCount: number) {
    this.selectedIndex = clampIndex(index, itemCount);
  }

  move(delta: number, itemCount: number) {
    this.selectedIndex = clampIndex(this.selectedIndex + delta, itemCount);
  }

  selectSlot(slotNumber: number, itemCount: number) {
    this.selectedIndex = clampIndex(slotNumber - 1, itemCount);
  }

  clamp(itemCount: number) {
    this.selectedIndex = clampIndex(this.selectedIndex, itemCount);
  }
}

export const selectionStore = new SelectionStore();
