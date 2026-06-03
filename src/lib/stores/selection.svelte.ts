export type QuickBarPanel = 'recent' | 'pinned';

function clampIndex(index: number, itemCount: number) {
  if (itemCount <= 0) return 0;
  return Math.min(Math.max(index, 0), itemCount - 1);
}

function wrapIndex(index: number, itemCount: number) {
  if (itemCount <= 0) return 0;
  return ((index % itemCount) + itemCount) % itemCount;
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

  clamp(itemCount: number) {
    this.selectedIndex = clampIndex(this.selectedIndex, itemCount);
  }

  move(delta: number, itemCount: number) {
    this.selectedIndex = wrapIndex(clampIndex(this.selectedIndex, itemCount) + delta, itemCount);
  }

  selectSlot(slotNumber: number, itemCount: number) {
    this.selectedIndex = clampIndex(slotNumber - 1, itemCount);
  }
}

export const selectionStore = new SelectionStore();
