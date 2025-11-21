<script lang="ts">
    import Card from "$lib/components/ui/Card.svelte";
    import Button from "$lib/components/ui/Button.svelte";
    import { Loader2, Info, RefreshCw, Download } from "lucide-svelte";

    interface UpdateInfo {
        available: boolean;
        current_version: string;
        latest_version?: string;
        body?: string;
        date?: string;
    }

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
</script>

<div class="space-y-6 animate-in fade-in slide-in-from-bottom-4 duration-300">
    <div>
        <h2 class="text-lg font-semibold mb-1">关于</h2>
        <p class="text-sm text-muted-foreground">版本信息和软件更新</p>
    </div>

    <Card class="p-6 space-y-6">
        <div class="flex items-center gap-4">
            <div
                class="h-12 w-12 bg-primary/10 rounded-xl flex items-center justify-center"
            >
                <Info class="h-6 w-6 text-primary" />
            </div>
            <div>
                <h3 class="font-bold text-lg">ClipMan</h3>
                <p class="text-sm text-muted-foreground">
                    高效的剪贴板管理工具
                </p>
                <div class="flex items-center gap-2 mt-1">
                    {#if updateInfo}
                        <span
                            class="text-xs bg-muted px-2 py-0.5 rounded text-muted-foreground"
                            >v{updateInfo.current_version}</span
                        >
                    {/if}
                    <a
                        href="https://github.com/Kiaana/ClipMan"
                        target="_blank"
                        class="text-xs text-primary hover:underline"
                        >GitHub 仓库</a
                    >
                </div>
            </div>
        </div>

        <div class="space-y-4 pt-4 border-t border-border">
            {#if updateInfo}
                <div class="space-y-2">
                    <div class="flex justify-between text-sm">
                        <span class="text-muted-foreground">当前版本</span>
                        <span class="font-mono"
                            >{updateInfo.current_version}</span
                        >
                    </div>

                    {#if updateInfo.available && updateInfo.latest_version}
                        <div class="flex justify-between text-sm">
                            <span class="text-muted-foreground">最新版本</span>
                            <span
                                class="font-mono font-bold text-green-600 dark:text-green-400"
                                >{updateInfo.latest_version}</span
                            >
                        </div>

                        {#if updateInfo.body}
                            <div
                                class="mt-3 p-3 bg-muted/50 rounded border border-border text-sm"
                            >
                                <strong
                                    class="block mb-1 text-xs uppercase tracking-wider text-muted-foreground"
                                    >更新内容</strong
                                >
                                <pre
                                    class="whitespace-pre-wrap font-sans text-sm">{updateInfo.body}</pre>
                            </div>
                        {/if}
                    {/if}
                </div>
            {:else}
                <div class="text-center py-4 text-sm text-muted-foreground">
                    点击检查更新获取最新版本信息
                </div>
            {/if}

            <div class="flex gap-2 pt-2">
                <Button
                    type="button"
                    variant="secondary"
                    class="flex-1"
                    onclick={checkForUpdates}
                    disabled={checkingUpdate || installingUpdate}
                >
                    {#if checkingUpdate}
                        <Loader2 class="h-4 w-4 animate-spin mr-2" /> 检查中...
                    {:else}
                        <RefreshCw class="h-4 w-4 mr-2" /> 检查更新
                    {/if}
                </Button>

                {#if updateInfo?.available}
                    <Button
                        type="button"
                        class="flex-1 bg-green-600 hover:bg-green-700 text-white"
                        onclick={installUpdate}
                        disabled={installingUpdate}
                    >
                        {#if installingUpdate}
                            <Loader2 class="h-4 w-4 animate-spin mr-2" /> 安装中...
                        {:else}
                            <Download class="h-4 w-4 mr-2" /> 安装更新
                        {/if}
                    </Button>
                {/if}
            </div>

            {#if updateMessage}
                <div
                    class="p-3 rounded text-sm text-center
                    {updateMessage.includes('失败')
                        ? 'bg-red-100 dark:bg-red-900/30 text-red-800 dark:text-red-200'
                        : updateMessage.includes('最新版本') ||
                            updateMessage.includes('成功')
                          ? 'bg-green-100 dark:bg-green-900/30 text-green-800 dark:text-green-200'
                          : 'bg-muted text-muted-foreground'}"
                >
                    {updateMessage}
                </div>
            {/if}
        </div>
    </Card>
</div>
