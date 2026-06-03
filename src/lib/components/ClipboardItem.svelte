<script lang="ts">
  import { onDestroy } from "svelte";
  import { clipboardStore } from "$lib/stores/clipboard.svelte";
  import type { ClipItem } from "$lib/stores/clipboard.svelte";
  import { i18n } from "$lib/i18n";
  import Card from "./ui/Card.svelte";
  import Button from "./ui/Button.svelte";
  import {
    Copy,
    Check,
    Pin,
    Trash2,
    FileText,
    Image as ImageIcon,
    File,
  } from "lucide-svelte";

  interface Props {
    item: ClipItem;
  }

  let { item }: Props = $props();

  const t = $derived(i18n.t);

  // UI State
  let isCopied = $state(false);
  let copyTimeout: ReturnType<typeof setTimeout>;

  // Derived: Decode text content
  const decodedText = $derived.by(() => {
    if (item.contentType !== "text") return "";

    const content = item.content;
    if (!content || (typeof content === "string" && content.length === 0)) {
      return t.emptyContent;
    }

    try {
      // Content is now a base64 string from backend
      if (typeof content === "string") {
        const binaryString = atob(content);
        const bytes = new Uint8Array(binaryString.length);
        for (let i = 0; i < binaryString.length; i++) {
          bytes[i] = binaryString.charCodeAt(i);
        }
        return new TextDecoder().decode(bytes);
      }
      return t.decodeFailed;
    } catch (e) {
      console.error("Failed to decode text content:", e);
      return t.decodeFailed;
    }
  });

  // For images: content is already a data URL from backend, use directly
  const imageDataUrl = $derived(
    item.contentType === "image" && typeof item.content === "string"
      ? item.content
      : "",
  );

  function formatTime(timestamp: number): string {
    const date = new Date(timestamp * 1000);
    const now = new Date();
    const diff = now.getTime() - date.getTime();

    // Less than 1 minute
    if (diff < 60000) {
      return t.justNow;
    }

    // Less than 1 hour
    if (diff < 3600000) {
      return i18n.format(t.minutesAgo, { n: Math.floor(diff / 60000) });
    }

    // Less than 24 hours
    if (diff < 86400000) {
      return i18n.format(t.hoursAgo, { n: Math.floor(diff / 3600000) });
    }

    // Otherwise show date
    return date.toLocaleDateString(i18n.locale === 'zh-CN' ? 'zh-CN' : 'en-US');
  }

  async function handleCopy() {
    try {
      await clipboardStore.copyToClipboard(item);
      isCopied = true;
      clearTimeout(copyTimeout);
      copyTimeout = setTimeout(() => {
        isCopied = false;
      }, 2000);
    } catch (_error) {
      // Error handled in store
    }
  }

  onDestroy(() => {
    clearTimeout(copyTimeout);
  });
</script>

<div
  class="group relative transition-all duration-200 ease-in-out hover:scale-[1.01]"
  role="listitem"
>
  <Card
    class="overflow-hidden border-l-4 transition-colors duration-200 {item.isPinned
      ? 'border-l-primary bg-primary/5'
      : 'border-l-transparent hover:border-l-primary/50 hover:bg-muted/30'}"
  >
    <div class="p-3 flex gap-3">
      <!-- Content Type Icon -->
      <div class="flex-none pt-1 text-muted-foreground">
        {#if item.contentType === "text"}
          <FileText class="h-4 w-4" />
        {:else if item.contentType === "image"}
          <ImageIcon class="h-4 w-4" />
        {:else}
          <File class="h-4 w-4" />
        {/if}
      </div>

      <!-- Main Content -->
      <div class="flex-1 min-w-0">
        {#if item.contentType === "text"}
          <p
            class="text-sm text-foreground line-clamp-3 break-all font-mono leading-relaxed selection:bg-primary/20"
          >
            {decodedText}
          </p>
        {:else if item.contentType === "image"}
          <div
            class="relative rounded-md overflow-hidden border border-border bg-muted/50 max-h-32 w-fit group/image"
          >
            {#if imageDataUrl}
              <img
                src={imageDataUrl}
                alt="Clipboard content"
                class="max-w-full h-auto object-contain max-h-32 transition-transform duration-300 group-hover/image:scale-105"
                loading="lazy"
              />
            {:else}
              <div
                class="flex items-center justify-center w-20 h-20 text-xs text-muted-foreground"
              >
                {t.loading}
              </div>
            {/if}
          </div>
        {:else}
          <div
            class="flex items-center gap-2 p-2 rounded bg-muted/50 text-sm text-muted-foreground"
          >
            <File class="h-4 w-4" />
            <span class="italic">{t.binaryFileData}</span>
          </div>
        {/if}

        <div class="mt-2 flex items-center justify-between">
          <span class="text-xs text-muted-foreground font-medium opacity-70">
            {formatTime(item.timestamp)}
          </span>

          <!-- Actions (visible on hover) -->
          <div
            class="flex items-center gap-1 opacity-0 group-hover:opacity-100 transition-all duration-200 translate-y-1 group-hover:translate-y-0"
          >
            <Button
              variant="ghost"
              size="icon"
              class="h-7 w-7 hover:text-primary hover:bg-primary/10"
              title={t.copy}
              onclick={handleCopy}
            >
              {#if isCopied}
                <Check class="h-3.5 w-3.5 text-green-500" />
              {:else}
                <Copy class="h-3.5 w-3.5" />
              {/if}
            </Button>

            <Button
              variant="ghost"
              size="icon"
              class="h-7 w-7 {item.isPinned
                ? 'text-primary'
                : 'text-muted-foreground hover:text-primary hover:bg-primary/10'}"
              title={item.isPinned ? t.unpin : t.pin}
              onclick={async () => {
                await clipboardStore.togglePin(item.id);
              }}
            >
              <Pin class="h-3.5 w-3.5 {item.isPinned ? 'fill-current' : ''}" />
            </Button>

            <Button
              variant="ghost"
              size="icon"
              class="h-7 w-7 text-muted-foreground hover:text-destructive hover:bg-destructive/10"
              title={t.delete}
              onclick={async () => {
                await clipboardStore.deleteItem(item.id);
              }}
            >
              <Trash2 class="h-3.5 w-3.5" />
            </Button>
          </div>
        </div>
      </div>
    </div>
  </Card>
</div>
