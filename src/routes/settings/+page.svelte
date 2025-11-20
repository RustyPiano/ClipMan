<script lang="ts">
    import { invoke } from "@tauri-apps/api/core";
    import { onMount } from "svelte";
    import { router } from "$lib/stores/router.svelte";
    import Button from "$lib/components/ui/Button.svelte";
    import Input from "$lib/components/ui/Input.svelte";
    import Card from "$lib/components/ui/Card.svelte";
    import {
        ChevronLeft,
        Keyboard,
        History,
        Info,
        Loader2,
        RefreshCw,
        Download,
        Save,
        RotateCcw,
    } from "lucide-svelte";

    interface Settings {
        globalShortcut: string;
        maxHistoryItems: number;
        autoCleanup: boolean;
        trayTextLength: number;
        storeOriginalImage: boolean;
        maxPinnedInTray: number;
        maxRecentInTray: number;
        customDataPath: string | null;
    }

    interface UpdateInfo {
        available: boolean;
        current_version: string;
        latest_version?: string;
        body?: string;
        date?: string;
    }

    let settings = $state<Settings>({
        globalShortcut: "CommandOrControl+Shift+V",
        maxHistoryItems: 100,
        autoCleanup: true,
        trayTextLength: 50,
        storeOriginalImage: false,
        maxPinnedInTray: 5,
        maxRecentInTray: 20,
        customDataPath: null,
    });

    let loading = $state(true);
    let saving = $state(false);
    let message = $state("");

    // 更新相关状态
    let updateInfo = $state<UpdateInfo | null>(null);
    let checkingUpdate = $state(false);
    let installingUpdate = $state(false);
    let updateMessage = $state("");

    // 数据位置相关状态
    let currentDataPath = $state("");
    let changingDataPath = $state(false);
    let showMigrationDialog = $state(false);
    let newDataPath = $state("");
    let deleteOldData = $state(true);

    onMount(async () => {
        await loadSettings();
    });

    async function loadSettings() {
        try {
            loading = true;
            settings = await invoke<Settings>("get_settings");
        } catch (err) {
            console.error("Failed to load settings:", err);
            const errorMsg = err instanceof Error ? err.message : String(err);
            message = "加载设置失败: " + errorMsg;
        } finally {
            loading = false;
        }
    }

    async function saveSettings() {
        try {
            saving = true;
            message = "";
            await invoke("update_settings", { settings });
            message = "设置已保存！";
            setTimeout(() => (message = ""), 3000);
        } catch (err) {
            console.error("Failed to save settings:", err);
            const errorMsg = err instanceof Error ? err.message : String(err);
            message = "保存失败: " + errorMsg;
        } finally {
            saving = false;
        }
    }

    // 常用热键预设
    const shortcutPresets = [
        {
            label: "Ctrl/Cmd + Shift + V (默认)",
            value: "CommandOrControl+Shift+V",
        },
        { label: "Ctrl/Cmd + Alt + V", value: "CommandOrControl+Alt+V" },
        { label: "Ctrl/Cmd + Shift + C", value: "CommandOrControl+Shift+C" },
        { label: "Alt + V", value: "Alt+V" },
        { label: "Ctrl/Cmd + `", value: "CommandOrControl+`" },
    ];

    // 检查更新
    async function checkForUpdates() {
        try {
            checkingUpdate = true;
            updateMessage = "";
            updateInfo = await invoke<UpdateInfo>("check_for_updates");

            if (updateInfo.available) {
                updateMessage = `发现新版本 ${updateInfo.latest_version}！`;
            } else {
                updateMessage = "当前已是最新版本";
            }
        } catch (err) {
            console.error("Failed to check for updates:", err);
            const errStr = String(err);
            updateMessage = "检查更新失败: " + errStr;
            if (errStr.includes("Not Found") || errStr.includes("404")) {
                updateMessage =
                    "检查更新失败: 未找到更新信息 (可能是尚未发布新版本)";
            }
        } finally {
            checkingUpdate = false;
        }
    }

    // 安装更新
    async function installUpdate() {
        if (!updateInfo?.available) return;

        try {
            installingUpdate = true;
            updateMessage = "正在下载并安装更新...";
            await invoke("install_update");
            updateMessage = "更新安装成功！应用将重启。";
        } catch (err) {
            console.error("Failed to install update:", err);
            const errorMsg = err instanceof Error ? err.message : String(err);
            updateMessage = "安装更新失败: " + errorMsg;
            installingUpdate = false;
        }
    }

    // 加载当前数据路径
    async function loadDataPath() {
        try {
            currentDataPath = await invoke<string>("get_current_data_path");
        } catch (err) {
            console.error("Failed to load data path:", err);
        }
    }

    // 选择新的数据存储位置
    async function changeDataLocation() {
        try {
            changingDataPath = true;
            const selectedPath = await invoke<string | null>(
                "choose_data_folder",
            );

            if (selectedPath) {
                newDataPath = selectedPath;
                showMigrationDialog = true;
            }
        } catch (err) {
            console.error("Failed to choose folder:", err);
            message = "选择文件夹失败: " + String(err);
        } finally {
            changingDataPath = false;
        }
    }

    // 确认迁移数据
    async function confirmMigration() {
        try {
            changingDataPath = true;
            message = "正在迁移数据...";

            await invoke("migrate_data_location", {
                newPath: newDataPath,
                deleteOld: deleteOldData,
            });

            message = "数据迁移成功！";
            showMigrationDialog = false;
            await loadDataPath();
            await loadSettings();

            setTimeout(() => (message = ""), 3000);
        } catch (err) {
            console.error("Failed to migrate data:", err);
            message = "数据迁移失败: " + String(err);
        } finally {
            changingDataPath = false;
        }
    }

    // 打开数据文件夹
    async function openDataFolder() {
        if (currentDataPath) {
            await invoke("open", { path: currentDataPath });
        }
    }

    // 初始化时加载数据路径
    onMount(async () => {
        await loadSettings();
        await loadDataPath();
    });
</script>

<div class="min-h-screen bg-background text-foreground p-6 overflow-y-auto">
    <div class="max-w-2xl mx-auto space-y-6">
        <header class="flex items-center gap-4 pb-4 border-b border-border">
            <Button variant="ghost" size="sm" onclick={() => router.goHome()}>
                <ChevronLeft class="h-4 w-4 mr-1" /> 返回
            </Button>
            <div>
                <h1 class="text-2xl font-bold">设置</h1>
                <p class="text-sm text-muted-foreground">
                    配置 ClipMan 的行为和快捷键
                </p>
            </div>
        </header>

        {#if loading}
            <div class="flex justify-center py-12 text-muted-foreground">
                <Loader2 class="h-6 w-6 animate-spin mr-2" /> 加载中...
            </div>
        {:else}
            <form
                onsubmit={(e) => {
                    e.preventDefault();
                    saveSettings();
                }}
                class="space-y-6"
            >
                <!-- 全局热键设置 -->
                <Card class="p-6 space-y-4">
                    <div>
                        <h2
                            class="text-lg font-semibold flex items-center gap-2"
                        >
                            <Keyboard class="h-5 w-5" /> 全局热键
                        </h2>
                        <p class="text-sm text-muted-foreground mt-1">
                            设置打开 ClipMan 窗口的快捷键。Mac 上 Ctrl
                            会自动替换为 Cmd。
                        </p>
                    </div>

                    <div class="space-y-2">
                        <label for="shortcut-input" class="text-sm font-medium"
                            >自定义快捷键</label
                        >
                        <Input
                            id="shortcut-input"
                            type="text"
                            bind:value={settings.globalShortcut}
                            placeholder="例如: CommandOrControl+Shift+V"
                        />
                    </div>

                    <div class="space-y-2">
                        <span class="text-sm font-medium">快速选择</span>
                        <div class="flex flex-wrap gap-2">
                            {#each shortcutPresets as preset}
                                <Button
                                    type="button"
                                    variant={settings.globalShortcut ===
                                    preset.value
                                        ? "default"
                                        : "outline"}
                                    size="sm"
                                    onclick={() =>
                                        (settings.globalShortcut =
                                            preset.value)}
                                >
                                    {preset.label}
                                </Button>
                            {/each}
                        </div>
                    </div>
                </Card>

                <!-- 历史记录设置 -->
                <Card class="p-6 space-y-4">
                    <h2 class="text-lg font-semibold flex items-center gap-2">
                        <History class="h-5 w-5" /> 历史记录
                    </h2>

                    <div class="space-y-2">
                        <div class="flex justify-between">
                            <label for="max-items" class="text-sm font-medium"
                                >最大历史条目数</label
                            >
                            <span class="text-sm font-bold text-primary"
                                >{settings.maxHistoryItems}</span
                            >
                        </div>
                        <input
                            id="max-items"
                            type="range"
                            min="50"
                            max="500"
                            step="50"
                            bind:value={settings.maxHistoryItems}
                            class="w-full accent-primary h-2 bg-muted rounded-lg appearance-none cursor-pointer"
                        />
                        <p class="text-xs text-muted-foreground text-right">
                            范围: 50 - 500 条
                        </p>
                    </div>

                    <div class="flex items-center gap-2">
                        <input
                            type="checkbox"
                            id="auto-cleanup"
                            bind:checked={settings.autoCleanup}
                            class="w-4 h-4 rounded border-input text-primary focus:ring-ring"
                        />
                        <label
                            for="auto-cleanup"
                            class="text-sm font-medium cursor-pointer"
                        >
                            自动清理超出限制的历史记录
                        </label>
                    </div>
                </Card>

                <!-- 显示设置 -->
                <Card class="p-6 space-y-4">
                    <h2 class="text-lg font-semibold flex items-center gap-2">
                        <svg
                            xmlns="http://www.w3.org/2000/svg"
                            width="20"
                            height="20"
                            viewBox="0 0 24 24"
                            fill="none"
                            stroke="currentColor"
                            stroke-width="2"
                            stroke-linecap="round"
                            stroke-linejoin="round"
                            ><rect
                                width="18"
                                height="18"
                                x="3"
                                y="3"
                                rx="2"
                            /><path d="M7 7h10" /><path d="M7 12h10" /><path
                                d="M7 17h10"
                            /></svg
                        >
                        显示设置
                    </h2>

                    <div class="space-y-2">
                        <div class="flex justify-between">
                            <label
                                for="tray-text-length"
                                class="text-sm font-medium"
                                >托盘菜单文本长度</label
                            >
                            <span class="text-sm font-bold text-primary"
                                >{settings.trayTextLength} 字符</span
                            >
                        </div>
                        <input
                            id="tray-text-length"
                            type="range"
                            min="15"
                            max="80"
                            step="5"
                            bind:value={settings.trayTextLength}
                            class="w-full accent-primary h-2 bg-muted rounded-lg appearance-none cursor-pointer"
                        />
                        <p class="text-xs text-muted-foreground">
                            根据个人喜好调整，建议 30-50 字符
                        </p>
                    </div>

                    <div class="space-y-2">
                        <div class="flex justify-between">
                            <label
                                for="max-pinned-tray"
                                class="text-sm font-medium"
                                >托盘菜单最多置顶项</label
                            >
                            <span class="text-sm font-bold text-primary"
                                >{settings.maxPinnedInTray} 条</span
                            >
                        </div>
                        <input
                            id="max-pinned-tray"
                            type="range"
                            min="3"
                            max="10"
                            step="1"
                            bind:value={settings.maxPinnedInTray}
                            class="w-full accent-primary h-2 bg-muted rounded-lg appearance-none cursor-pointer"
                        />
                        <p class="text-xs text-muted-foreground">
                            控制托盘菜单中显示的置顶项数量 (3-10)
                        </p>
                    </div>

                    <div class="space-y-2">
                        <div class="flex justify-between">
                            <label
                                for="max-recent-tray"
                                class="text-sm font-medium"
                                >托盘菜单最多最近项</label
                            >
                            <span class="text-sm font-bold text-primary"
                                >{settings.maxRecentInTray} 条</span
                            >
                        </div>
                        <input
                            id="max-recent-tray"
                            type="range"
                            min="10"
                            max="50"
                            step="5"
                            bind:value={settings.maxRecentInTray}
                            class="w-full accent-primary h-2 bg-muted rounded-lg appearance-none cursor-pointer"
                        />
                        <p class="text-xs text-muted-foreground">
                            控制托盘菜单中显示的最近项数量
                            (10-50)，数量过多可能导致菜单超出屏幕
                        </p>
                    </div>

                    <div class="flex items-start gap-2">
                        <input
                            type="checkbox"
                            id="store-original-image"
                            bind:checked={settings.storeOriginalImage}
                            class="w-4 h-4 mt-0.5 rounded border-input text-primary focus:ring-ring"
                        />
                        <label
                            for="store-original-image"
                            class="text-sm font-medium cursor-pointer flex-1"
                        >
                            <div>保存高质量图片</div>
                            <p
                                class="text-xs text-muted-foreground font-normal mt-1"
                            >
                                开启后保存最大 2048px 的高质量图片（约
                                200-500KB/张）<br />
                                关闭则保存 256x256 缩略图（约 50KB/张，节省空间）
                            </p>
                        </label>
                    </div>
                </Card>

                <!-- 数据存储位置 -->
                <Card class="p-6 space-y-4">
                    <h2 class="text-lg font-semibold flex items-center gap-2">
                        <svg
                            xmlns="http://www.w3.org/2000/svg"
                            width="20"
                            height="20"
                            viewBox="0 0 24 24"
                            fill="none"
                            stroke="currentColor"
                            stroke-width="2"
                            stroke-linecap="round"
                            stroke-linejoin="round"
                            ><path
                                d="M20 20a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.9a2 2 0 0 1-1.69-.9L9.6 3.9A2 2 0 0 0 7.93 3H4a2 2 0 0 0-2 2v13a2 2 0 0 0 2 2Z"
                            /></svg
                        >
                        数据存储位置
                    </h2>

                    <div class="space-y-2">
                        <label class="text-sm font-medium">当前位置:</label>
                        <div class="flex gap-2">
                            <Input
                                value={currentDataPath}
                                readonly
                                class="flex-1 text-xs"
                            />
                            <Button
                                type="button"
                                variant="outline"
                                size="sm"
                                onclick={openDataFolder}
                            >
                                打开
                            </Button>
                        </div>
                        <p class="text-xs text-muted-foreground">
                            数据库、加密密钥和设置文件的存储位置
                        </p>
                    </div>

                    <div class="space-y-2">
                        <Button
                            type="button"
                            variant="secondary"
                            onclick={changeDataLocation}
                            disabled={changingDataPath}
                        >
                            {#if changingDataPath}
                                <Loader2 class="h-4 w-4 animate-spin mr-2" /> 处理中...
                            {:else}
                                更改存储位置...
                            {/if}
                        </Button>
                        <p class="text-xs text-muted-foreground">
                            更改后会自动迁移现有数据到新位置
                        </p>
                    </div>
                </Card>

                <!-- 关于和更新 -->
                <Card class="p-6 space-y-4">
                    <h2 class="text-lg font-semibold flex items-center gap-2">
                        <Info class="h-5 w-5" /> 关于和更新
                    </h2>

                    <div class="space-y-4">
                        {#if updateInfo}
                            <div class="space-y-2">
                                <p class="text-sm">
                                    <strong>当前版本：</strong>
                                    <span
                                        class="bg-muted px-2 py-0.5 rounded text-xs font-mono"
                                        >{updateInfo.current_version}</span
                                    >
                                </p>
                                {#if updateInfo.available && updateInfo.latest_version}
                                    <p class="text-sm">
                                        <strong>最新版本：</strong>
                                        <span
                                            class="bg-green-100 dark:bg-green-900 text-green-800 dark:text-green-100 px-2 py-0.5 rounded text-xs font-mono font-bold"
                                            >{updateInfo.latest_version}</span
                                        >
                                    </p>
                                    {#if updateInfo.body}
                                        <div
                                            class="mt-2 p-3 bg-muted/50 rounded border border-border text-sm"
                                        >
                                            <strong class="block mb-1"
                                                >更新内容：</strong
                                            >
                                            <pre
                                                class="whitespace-pre-wrap font-sans text-muted-foreground">{updateInfo.body}</pre>
                                        </div>
                                    {/if}
                                {/if}
                            </div>
                        {:else}
                            <p class="text-sm text-muted-foreground italic">
                                点击下方按钮检查更新
                            </p>
                        {/if}

                        <div class="flex gap-2">
                            <Button
                                type="button"
                                variant="secondary"
                                onclick={checkForUpdates}
                                disabled={checkingUpdate || installingUpdate}
                            >
                                {#if checkingUpdate}
                                    <Loader2
                                        class="h-4 w-4 animate-spin mr-2"
                                    /> 检查中...
                                {:else}
                                    <RefreshCw class="h-4 w-4 mr-2" /> 检查更新
                                {/if}
                            </Button>

                            {#if updateInfo?.available}
                                <Button
                                    type="button"
                                    class="bg-green-600 hover:bg-green-700 text-white"
                                    onclick={installUpdate}
                                    disabled={installingUpdate}
                                >
                                    {#if installingUpdate}
                                        <Loader2
                                            class="h-4 w-4 animate-spin mr-2"
                                        /> 安装中...
                                    {:else}
                                        <Download class="h-4 w-4 mr-2" /> 安装更新
                                    {/if}
                                </Button>
                            {/if}
                        </div>

                        {#if updateMessage}
                            <div
                                class="p-3 rounded text-sm
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

                <!-- 按钮组 -->
                <div class="flex gap-4 pt-4">
                    <Button type="submit" class="flex-1" disabled={saving}>
                        {#if saving}
                            <Loader2 class="h-4 w-4 animate-spin mr-2" /> 保存中...
                        {:else}
                            <Save class="h-4 w-4 mr-2" /> 保存设置
                        {/if}
                    </Button>
                    <Button
                        type="button"
                        variant="secondary"
                        onclick={loadSettings}
                    >
                        <RotateCcw class="h-4 w-4 mr-2" /> 重置
                    </Button>
                </div>

                {#if message}
                    <div
                        class="p-4 rounded-md text-sm font-medium text-center
                        {message.includes('失败')
                            ? 'bg-destructive/10 text-destructive'
                            : 'bg-green-100 dark:bg-green-900/30 text-green-800 dark:text-green-200'}"
                    >
                        {message}
                    </div>
                {/if}
            </form>
        {/if}
    </div>
</div>

<!-- 数据迁移确认对话框 -->
{#if showMigrationDialog}
    <div
        class="fixed inset-0 bg-black/50 flex items-center justify-center z-50"
    >
        <div
            class="bg-background border border-border rounded-lg p-6 max-w-lg w-full mx-4 space-y-4"
        >
            <h3 class="text-lg font-semibold">确认数据迁移</h3>

            <div class="space-y-2 text-sm">
                <div>
                    <span class="text-muted-foreground">当前位置:</span>
                    <p
                        class="font-mono text-xs bg-muted p-2 rounded mt-1 break-all"
                    >
                        {currentDataPath}
                    </p>
                </div>
                <div>
                    <span class="text-muted-foreground">新位置:</span>
                    <p
                        class="font-mono text-xs bg-muted p-2 rounded mt-1 break-all"
                    >
                        {newDataPath}
                    </p>
                </div>
            </div>

            <div
                class="flex items-start gap-2 p-3 bg-yellow-100 dark:bg-yellow-900/30 rounded"
            >
                <svg
                    xmlns="http://www.w3.org/2000/svg"
                    width="20"
                    height="20"
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="currentColor"
                    stroke-width="2"
                    stroke-linecap="round"
                    stroke-linejoin="round"
                    class="text-yellow-600 dark:text-yellow-400 flex-shrink-0"
                    ><path
                        d="m21.73 18-8-14a2 2 0 0 0-3.48 0l-8 14A2 2 0 0 0 4 21h16a2 2 0 0 0 1.73-3Z"
                    /><line x1="12" x2="12" y1="9" y2="13" /><line
                        x1="12"
                        x2="12.01"
                        y1="17"
                        y2="17"
                    /></svg
                >
                <p class="text-sm text-yellow-800 dark:text-yellow-200">
                    数据迁移过程中，剪贴板监控将暂时停止。迁移完成后会自动恢复。
                </p>
            </div>

            <div class="flex items-center gap-2">
                <input
                    type="checkbox"
                    id="delete-old-data"
                    bind:checked={deleteOldData}
                    class="w-4 h-4 rounded border-input text-primary focus:ring-ring"
                />
                <label
                    for="delete-old-data"
                    class="text-sm font-medium cursor-pointer"
                >
                    迁移完成后删除旧位置的数据
                </label>
            </div>

            <div class="flex gap-2 justify-end">
                <Button
                    type="button"
                    variant="outline"
                    onclick={() => (showMigrationDialog = false)}
                    disabled={changingDataPath}
                >
                    取消
                </Button>
                <Button
                    type="button"
                    onclick={confirmMigration}
                    disabled={changingDataPath}
                >
                    {#if changingDataPath}
                        <Loader2 class="h-4 w-4 animate-spin mr-2" /> 迁移中...
                    {:else}
                        确认迁移
                    {/if}
                </Button>
            </div>
        </div>
    </div>
{/if}
