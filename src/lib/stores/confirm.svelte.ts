export interface ConfirmOptions {
  title: string;
  message: string;
  confirmLabel?: string;
  cancelLabel?: string;
  /** Style the confirm button as destructive (for clear/reset/delete). */
  destructive?: boolean;
}

/**
 * Promise-based confirmation dialog, replacing the native `confirm()` which is
 * jarring inside the frameless QuickBar. A single <ConfirmDialog /> mounted at
 * the app root renders whatever is asked here.
 *
 *   if (await confirmStore.ask({ title, message, destructive: true })) { ... }
 */
class ConfirmStore {
  open = $state(false);
  options = $state<ConfirmOptions | null>(null);
  #resolve: ((value: boolean) => void) | null = null;

  ask(options: ConfirmOptions): Promise<boolean> {
    // Resolve any dialog already in flight as cancelled before replacing it.
    this.#settle(false);
    this.options = options;
    this.open = true;
    return new Promise<boolean>((resolve) => {
      this.#resolve = resolve;
    });
  }

  confirm() {
    this.#settle(true);
  }

  cancel() {
    this.#settle(false);
  }

  #settle(value: boolean) {
    if (!this.#resolve) return;
    const resolve = this.#resolve;
    this.#resolve = null;
    this.open = false;
    resolve(value);
  }
}

export const confirmStore = new ConfirmStore();
