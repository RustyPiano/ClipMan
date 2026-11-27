let supportsDisplayTransition: boolean | undefined;

function canTransitionDisplay(): boolean {
  if (supportsDisplayTransition !== undefined) return supportsDisplayTransition;
  if (!window.CSS?.supports?.('transition-behavior', 'allow-discrete') || !document.body) {
    return false;
  }
  const probe = document.createElement('div');
  probe.style.cssText = 'transition: display 1s allow-discrete !important; display: block;';
  document.body.appendChild(probe);
  getComputedStyle(probe).display;
  probe.style.display = 'none';
  supportsDisplayTransition = getComputedStyle(probe).display === 'block';
  probe.remove();
  return supportsDisplayTransition;
}

function supportsTopLayerExit(): boolean {
  return Boolean(window.CSS?.supports?.('overlay', 'auto') && canTransitionDisplay());
}

export async function closeTopLayer(dialog: HTMLDialogElement): Promise<void> {
  if (!dialog.open) return;
  if (!supportsTopLayerExit()) {
    dialog.setAttribute('data-closing', '');
    const animations = dialog.getAnimations({ subtree: true });
    if (animations.length > 0) {
      await Promise.race([
        Promise.allSettled(animations.map((a) => a.finished)),
        new Promise((resolve) => setTimeout(resolve, 600)),
      ]);
    }
    if (!dialog.hasAttribute('data-closing')) return;
    dialog.removeAttribute('data-closing');
  }
  dialog.close();
}

export function bindDialogDismiss(dialog: HTMLDialogElement): () => void {
  const onCancel = (event: Event) => {
    event.preventDefault();
    void closeTopLayer(dialog);
  };
  dialog.addEventListener('cancel', onCancel);

  const supportsClosedBy = 'closedBy' in HTMLDialogElement.prototype;
  const onClick = (event: MouseEvent) => {
    if (supportsClosedBy || event.target !== dialog) return;
    const rect = dialog.getBoundingClientRect();
    const inside =
      rect.top <= event.clientY &&
      event.clientY <= rect.top + rect.height &&
      rect.left <= event.clientX &&
      event.clientX <= rect.left + rect.width;
    if (!inside) {
      void closeTopLayer(dialog);
    }
  };
  dialog.addEventListener('click', onClick);

  return () => {
    dialog.removeEventListener('cancel', onCancel);
    dialog.removeEventListener('click', onClick);
  };
}
