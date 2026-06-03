<script lang="ts">
  import Card from '$lib/components/ui/Card.svelte';
  import Button from '$lib/components/ui/Button.svelte';
  import MarkdownContent from '$lib/components/ui/MarkdownContent.svelte';
  import { Loader2, Info, RefreshCw, Download } from 'lucide-svelte';
  import { getVersion } from '@tauri-apps/api/app';
  import { onMount } from 'svelte';
  import { i18n } from '$lib/i18n';
  import type { UpdateInfo } from '$lib/types';

  let {
    updateInfo,
    checkingUpdate,
    installingUpdate,
    updateMessage,
    checkForUpdates,
    installUpdate,
  } = $props<{
    updateInfo: UpdateInfo | null;
    checkingUpdate: boolean;
    installingUpdate: boolean;
    updateMessage: string;
    checkForUpdates: () => void;
    installUpdate: () => void;
  }>();

  const t = $derived(i18n.t);

  let currentVersion = $state('');

  onMount(async () => {
    try {
      currentVersion = await getVersion();
    } catch (err) {
      console.error('Failed to get version:', err);
      currentVersion = '?';
    }
  });
</script>

<div class="space-y-6 animate-in fade-in slide-in-from-bottom-4 duration-300">
  <div>
    <h2 class="text-lg font-semibold mb-1">{t.settingsAbout}</h2>
    <p class="text-sm text-muted-foreground">{t.version}</p>
  </div>

  <Card class="p-6 space-y-6">
    <div class="flex items-center gap-4">
      <div class="h-12 w-12 bg-primary/10 rounded-xl flex items-center justify-center">
        <Info class="h-6 w-6 text-primary" />
      </div>
      <div>
        <h3 class="font-bold text-lg">{t.appName}</h3>
        <p class="text-sm text-muted-foreground">
          {i18n.locale === 'zh-CN' ? '高效的剪贴板管理工具' : 'Efficient clipboard manager'}
        </p>
        <div class="flex items-center gap-2 mt-1">
          {#if currentVersion}
            <span class="text-xs bg-muted px-2 py-0.5 rounded text-muted-foreground"
              >v{currentVersion}</span
            >
          {/if}
          <a
            href="https://github.com/RustyPiano/ClipMan"
            target="_blank"
            class="text-xs text-primary hover:underline">GitHub</a
          >
        </div>
      </div>
    </div>

    <div class="space-y-4 pt-4 border-t border-border">
      <div class="space-y-2">
        <div class="flex justify-between text-sm">
          <span class="text-muted-foreground">{t.version}</span>
          <span class="font-mono">{currentVersion || t.loading}</span>
        </div>

        {#if updateInfo?.available && updateInfo.latest_version}
          <div class="flex justify-between text-sm">
            <span class="text-muted-foreground">{t.updateAvailable}</span>
            <span class="font-mono font-bold text-green-600 dark:text-green-400"
              >{updateInfo.latest_version}</span
            >
          </div>

          {#if updateInfo.body}
            <div class="mt-3 p-3 bg-muted/50 rounded border border-border">
              <strong class="block mb-2 text-xs uppercase tracking-wider text-muted-foreground"
                >{i18n.locale === 'zh-CN' ? '更新内容' : 'Release Notes'}</strong
              >
              <MarkdownContent content={updateInfo.body} />
            </div>
          {/if}
        {:else if !updateInfo}
          <div class="text-center py-4 text-sm text-muted-foreground">
            {t.checkUpdate}
          </div>
        {/if}
      </div>

      <div class="flex gap-2 pt-2">
        <Button
          type="button"
          variant="secondary"
          class="flex-1"
          onclick={checkForUpdates}
          disabled={checkingUpdate || installingUpdate}
        >
          {#if checkingUpdate}
            <Loader2 class="h-4 w-4 animate-spin mr-2" /> {t.checking}
          {:else}
            <RefreshCw class="h-4 w-4 mr-2" /> {t.checkUpdate}
          {/if}
        </Button>

        {#if updateInfo?.available}
          <Button
            type="button"
            class="flex-1 !bg-green-600 !hover:bg-green-700 !text-white"
            onclick={installUpdate}
            disabled={installingUpdate}
          >
            {#if installingUpdate}
              <Loader2 class="h-4 w-4 animate-spin mr-2" /> {t.installing}
            {:else}
              <Download class="h-4 w-4 mr-2" /> {t.installUpdate}
            {/if}
          </Button>
        {/if}
      </div>

      {#if updateMessage}
        <div
          class="p-3 rounded text-sm text-center
                    {updateMessage.includes(t.noUpdateAvailable) || updateMessage.includes('✓')
            ? 'bg-green-100 dark:bg-green-900/30 text-green-800 dark:text-green-200'
            : 'bg-muted text-muted-foreground'}"
        >
          {updateMessage}
        </div>
      {/if}
    </div>
  </Card>
</div>
