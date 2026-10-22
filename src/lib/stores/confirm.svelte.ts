interface ConfirmOptions {
  title: string;
  message: string;
  confirmLabel?: string;
  cancelLabel?: string;
  destructive?: boolean;
}

class ConfirmStore {
  open = $state(false);
  options = $state<ConfirmOptions | null>(null);
  #resolve: ((value: boolean) => void) | null = null;

  ask(options: ConfirmOptions): Promise<boolean> {
    // 新请求替换对话框时，先取消上一个等待中的请求。
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
