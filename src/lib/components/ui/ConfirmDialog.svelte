<script lang="ts">
  import type { Attachment } from 'svelte/attachments';
  import Button from './Button.svelte';
  import { confirmStore } from '$lib/stores/confirm.svelte';
  import { i18n } from '$lib/i18n';

  const t = $derived(i18n.t);
  const ACCEPT_ID = 'confirm-dialog-accept';
  const TITLE_ID = 'confirm-dialog-title';
  const MESSAGE_ID = 'confirm-dialog-message';

  // Move focus to the confirm button when the dialog opens so Enter/Space work
  // and focus leaves whatever was behind it (e.g. the QuickBar search box).
  $effect(() => {
    if (!confirmStore.open) return;
    globalThis.document?.getElementById(ACCEPT_ID)?.focus();
  });

  function handleWindowKeydown(event: KeyboardEvent) {
    if (!confirmStore.open || event.key !== 'Escape') return;
    event.preventDefault();
    event.stopPropagation();
    confirmStore.cancel();
  }

  // Keep Tab focus within the dialog's buttons so it can't escape to the inert
  // UI behind the modal. Wired as a native listener (not a template handler) to
  // avoid a spurious a11y warning on the container.
  const trapFocus: Attachment = (element) => {
    const handler = (event: Event) => {
      const e = event as KeyboardEvent;
      if (e.key !== 'Tab') return;
      const focusables = element.querySelectorAll<globalThis.HTMLElement>('button');
      if (focusables.length === 0) return;
      const first = focusables[0];
      const last = focusables[focusables.length - 1];
      const active = globalThis.document.activeElement;
      if (e.shiftKey && active === first) {
        e.preventDefault();
        last.focus();
      } else if (!e.shiftKey && active === last) {
        e.preventDefault();
        first.focus();
      }
    };
    element.addEventListener('keydown', handler);
    return () => element.removeEventListener('keydown', handler);
  };
</script>

<svelte:window onkeydown={handleWindowKeydown} />

{#if confirmStore.open && confirmStore.options}
  <div
    class="fixed inset-0 z-[60] flex items-center justify-center bg-background/80 p-4 backdrop-blur-sm animate-in fade-in duration-150"
  >
    <div
      {@attach trapFocus}
      class="w-full max-w-md space-y-4 rounded-lg border border-border bg-card p-6 text-card-foreground shadow-lg animate-in zoom-in-95 duration-200"
      role="alertdialog"
      aria-modal="true"
      aria-labelledby={TITLE_ID}
      aria-describedby={MESSAGE_ID}
    >
      <h3 id={TITLE_ID} class="text-lg font-semibold">{confirmStore.options.title}</h3>
      <p id={MESSAGE_ID} class="text-sm text-muted-foreground">{confirmStore.options.message}</p>
      <div class="flex justify-end gap-3 pt-2">
        <Button variant="outline" onclick={() => confirmStore.cancel()}>
          {confirmStore.options.cancelLabel ?? t.cancel}
        </Button>
        <Button
          id={ACCEPT_ID}
          variant={confirmStore.options.destructive ? 'destructive' : 'default'}
          onclick={() => confirmStore.confirm()}
        >
          {confirmStore.options.confirmLabel ?? t.confirm}
        </Button>
      </div>
    </div>
  </div>
{/if}
