export interface Toast {
    id: number;
    message: string;
    type: 'success' | 'error' | 'info';
}

const TOAST_DURATION_MS = 2000;

class ToastStore {
    toasts = $state<Toast[]>([]);
    private counter = 0;

    add(message: string, type: 'success' | 'error' | 'info' = 'info') {
        const id = ++this.counter;
        const toast = { id, message, type };
        this.toasts.push(toast);

        // Auto remove after configured duration
        setTimeout(() => {
            this.remove(id);
        }, TOAST_DURATION_MS);
    }

    remove(id: number) {
        this.toasts = this.toasts.filter((t) => t.id !== id);
    }
}

export const toastStore = new ToastStore();
