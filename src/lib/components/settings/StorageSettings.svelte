<script lang="ts">
    import Card from "$lib/components/ui/Card.svelte";
    import Button from "$lib/components/ui/Button.svelte";
    import { Loader2, FolderOpen } from "lucide-svelte";
    import { invoke } from "@tauri-apps/api/core";

    interface Settings {
        customDataPath: string | null;
        [key: string]: any;
    }

    let {
        settings = $bindable(),
        currentDataPath,
        changingDataPath,
        changeDataLocation,
    } = $props<{
        settings: Settings;
        currentDataPath: string;
        changingDataPath: boolean;
        changeDataLocation: () => void;
    }>();

    // Detect OS for dynamic text
    const isMac = navigator.platform.toUpperCase().indexOf("MAC") >= 0;
    const isWindows = navigator.platform.toUpperCase().indexOf("WIN") >= 0;

    // Dynamic button text based on OS
    const openFolderText = isMac
        ? "在Finder中打开"
        : isWindows
          ? "在资源管理器中打开"
          : "打开文件夹";

    async function openDataFolder() {
        if (!currentDataPath || currentDataPath === "使用默认应用数据目录") {
            alert("数据路径未加载，请稍后重试");
            return;
        }

        try {
            await invoke("open_folder", { path: currentDataPath });
        } catch (err) {
            console.error("Failed to open folder:", err);
            alert("打开文件夹失败: " + String(err));
        }
    }
</script>

<div class="space-y-6 animate-in fade-in slide-in-from-bottom-4 duration-300">
    <div>
        <h2 class="text-lg font-semibold mb-1">数据存储</h2>
        <p class="text-sm text-muted-foreground">管理应用数据的存储位置</p>
    </div>

    <Card class="p-6 space-y-6">
        <div class="space-y-2">
            <div class="flex items-center justify-between">
                <label for="data-path" class="text-sm font-medium"
                    >当前存储位置</label
                >
                <Button
                    type="button"
                    variant="ghost"
                    size="sm"
                    onclick={openDataFolder}
                    class="gap-1 h-7 text-xs"
                >
                    <FolderOpen class="h-3.5 w-3.5" />
                    {openFolderText}
                </Button>
            </div>
            <div
                class="p-3 bg-muted rounded-md text-sm font-mono break-all border border-border"
            >
                {currentDataPath || "加载中..."}
            </div>
            <p class="text-xs text-muted-foreground">
                包含数据库、加密密钥等重要文件
            </p>
        </div>

        <div class="pt-2 border-t border-border">
            <div class="flex items-center justify-between">
                <div class="space-y-0.5">
                    <span class="text-sm font-medium">迁移数据</span>
                    <p class="text-xs text-muted-foreground">
                        将数据移动到新的存储位置
                    </p>
                </div>
                <Button
                    type="button"
                    variant="secondary"
                    onclick={changeDataLocation}
                    disabled={changingDataPath}
                >
                    {#if changingDataPath}
                        <Loader2 class="h-4 w-4 animate-spin mr-2" /> 处理中...
                    {:else}
                        更改位置...
                    {/if}
                </Button>
            </div>
        </div>
    </Card>
</div>
