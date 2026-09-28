<script lang="ts">
  import Button from './Button.svelte';
  import { confirmStore } from '$lib/stores/confirm.svelte';
  import { i18n } from '$lib/i18n';

  const t = $derived(i18n.t);
  const ACCEPT_ID = 'confirm-dialog-accept';
  const TITLE_ID = 'confirm-dialog-title';
  const MESSAGE_ID = 'confirm-dialog-message';
  let dialog: HTMLDialogElement;

  // showModal 让背景不可交互、支持 Esc 取消；打开时把焦点放到确认按钮上。
  $effect(() => {
    if (confirmStore.open) {
      dialog.showModal();
      document.getElementById(ACCEPT_ID)!.focus();
    } else if (dialog.open) {
      dialog.close();
    }
  });
</script>

<dialog
  bind:this={dialog}
  role="alertdialog"
  aria-labelledby={TITLE_ID}
  aria-describedby={MESSAGE_ID}
  oncancel={() => confirmStore.cancel()}
  class="m-auto w-[calc(100%-2rem)] max-w-md space-y-4 rounded-lg border border-border bg-card p-6 text-card-foreground shadow-lg backdrop:bg-background/80 backdrop:backdrop-blur-sm"
>
  {#if confirmStore.options}
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
  {/if}
</dialog>
