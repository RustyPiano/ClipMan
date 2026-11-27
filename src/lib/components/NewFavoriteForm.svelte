<script lang="ts">
  import { clipboardStore } from '$lib/stores/clipboard.svelte';
  import { i18n } from '$lib/i18n';
  import Button from '$lib/components/ui/Button.svelte';
  import { Check, X } from 'lucide-svelte';

  let {
    onCreated,
    onCancel,
  }: {
    onCreated: (id: string) => void;
    onCancel: () => void;
  } = $props();

  const t = $derived(i18n.t);
  let saving = $state(false);

  function cancel() {
    clipboardStore.resetNewFavoriteDraft();
    onCancel();
  }

  function handleKeydown(event: KeyboardEvent) {
    if (event.isComposing || event.keyCode === 229) return;
    if (event.key === 'Escape') {
      event.preventDefault();
      cancel();
    } else if ((event.metaKey || event.ctrlKey) && event.key === 'Enter') {
      event.preventDefault();
      void submit();
    }
  }

  async function submit(event?: Event) {
    event?.preventDefault();
    if (saving || !clipboardStore.newFavoriteContent.trim()) return;
    saving = true;
    try {
      const created = await clipboardStore.createPinnedClip(
        clipboardStore.newFavoriteContent,
        clipboardStore.newFavoriteLabel
      );
      if (created) {
        clipboardStore.resetNewFavoriteDraft();
        onCreated(created.id);
      }
    } finally {
      saving = false;
    }
  }
</script>

<form
  data-row-editor
  class="mx-2 mb-2 flex flex-col gap-2 rounded-lg border border-border bg-card p-3 shadow-xs"
  onsubmit={submit}
>
  <div class="flex items-center justify-between">
    <span class="text-xs font-semibold text-foreground">{t.newPinnedTitle}</span>
    <button
      type="button"
      class="rounded p-0.5 text-muted-foreground hover:bg-accent hover:text-foreground"
      aria-label={t.cancel}
      onclick={cancel}
    >
      <X class="h-3.5 w-3.5" />
    </button>
  </div>
  <input
    type="text"
    class="h-8 rounded-md border border-input bg-background px-2.5 text-xs text-foreground"
    placeholder={t.newPinnedLabelPlaceholder}
    bind:value={clipboardStore.newFavoriteLabel}
    disabled={saving}
    onkeydown={handleKeydown}
  />
  <textarea
    rows="3"
    class="resize-none rounded-md border border-input bg-background p-2.5 text-xs leading-relaxed text-foreground"
    placeholder={t.newPinnedContentPlaceholder}
    bind:value={clipboardStore.newFavoriteContent}
    disabled={saving}
    {@attach (el) => el.focus()}
    onkeydown={handleKeydown}></textarea>
  <div class="flex justify-end gap-1.5">
    <Button variant="ghost" size="sm" disabled={saving} onclick={cancel}>{t.cancel}</Button>
    <Button type="submit" size="sm" disabled={saving || !clipboardStore.newFavoriteContent.trim()}>
      <Check class="mr-1 h-3.5 w-3.5" />{t.save}
    </Button>
  </div>
</form>
