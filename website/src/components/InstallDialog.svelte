<script lang="ts">
  import { ShieldCheck, X } from 'lucide-svelte';
  import { COPY, type Locale } from '../lib/i18n';
  import { bindDialogDismiss, closeTopLayer } from '../lib/dialog';

  interface Props {
    locale: Locale;
    openSignal: number;
  }

  let { locale, openSignal }: Props = $props();
  let dialogEl = $state<HTMLDialogElement | null>(null);
  let lastSignal = 0;

  let copy = $derived(COPY[locale]);

  $effect(() => {
    if (openSignal > lastSignal && dialogEl && !dialogEl.open) {
      lastSignal = openSignal;
      dialogEl.showModal();
    }
  });

  function handleClose(): void {
    if (dialogEl) {
      void closeTopLayer(dialogEl);
    }
  }
</script>

<dialog
  bind:this={dialogEl}
  {@attach bindDialogDismiss}
  class="cl-dialog"
  aria-labelledby="install-dialog-title"
  {...{ closedby: 'any' } as Record<string, string>}
>
  <div class="p-6 text-left">
    <div class="flex items-start justify-between gap-4">
      <div class="flex items-center gap-2.5">
        <span
          class="flex h-9 w-9 shrink-0 items-center justify-center rounded-xl bg-[var(--accent-soft)] text-[var(--accent)]"
        >
          <ShieldCheck class="h-5 w-5" aria-hidden="true" />
        </span>
        <h2 id="install-dialog-title" class="text-base font-semibold text-[var(--ink)]">
          {copy.dialogTitle}
        </h2>
      </div>
      <button
        type="button"
        onclick={handleClose}
        aria-label={copy.dialogClose}
        class="flex h-8 w-8 items-center justify-center rounded-lg text-[var(--ink-muted)] transition hover:bg-[var(--surface-subtle)] hover:text-[var(--ink)]"
      >
        <X class="h-4 w-4" aria-hidden="true" />
      </button>
    </div>

    <p class="mt-3 text-sm leading-relaxed text-[var(--ink-muted)]">
      {copy.dialogIntro}
    </p>

    <div class="mt-4 space-y-3">
      <div class="rounded-xl border border-[var(--hairline)] bg-[var(--surface-subtle)]/70 p-4">
        <h3 class="text-sm font-medium text-[var(--ink)]">{copy.dialogStep1Title}</h3>
        <p class="mt-1.5 text-xs leading-relaxed text-[var(--ink-muted)]">
          {copy.dialogStep1Body}
        </p>
      </div>

      <div class="rounded-xl border border-[var(--hairline)] bg-[var(--surface-subtle)]/70 p-4">
        <h3 class="text-sm font-medium text-[var(--ink)]">{copy.dialogStep2Title}</h3>
        <p class="mt-1.5 text-xs leading-relaxed text-[var(--ink-muted)]">
          {copy.dialogStep2Body}
        </p>
      </div>
    </div>

    <div class="mt-5 flex justify-end">
      <button
        type="button"
        onclick={handleClose}
        class="rounded-lg bg-[var(--primary)] px-4 py-2 text-xs font-medium text-[var(--primary-fg)] transition hover:opacity-90"
      >
        {copy.dialogClose}
      </button>
    </div>
  </div>
</dialog>
