import type { Toast } from '$lib/types';

const TOAST_DURATION_MS = 2000;

class ToastStore {
  toasts = $state<Toast[]>([]);
  private counter = 0;

  add(message: string, type: Toast['type'] = 'info') {
    const id = ++this.counter;
    const toast = { id, message, type };
    this.toasts.push(toast);

    setTimeout(() => {
      this.toasts = this.toasts.filter((t) => t.id !== id);
    }, TOAST_DURATION_MS);
  }
}

export const toastStore = new ToastStore();
