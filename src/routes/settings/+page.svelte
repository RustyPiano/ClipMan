<script lang="ts">
    import { invoke } from "@tauri-apps/api/core";
    import { onMount } from "svelte";
    import { router } from "$lib/stores/router.svelte";
    import { i18n } from "$lib/i18n";
    import Button from "$lib/components/ui/Button.svelte";
    import { ChevronLeft, Loader2, Save, RotateCcw } from "lucide-svelte";
    import { open } from "@tauri-apps/plugin-dialog";
    import type { Settings, UpdateInfo, SettingsTab } from "$lib/types";

    // Import modularized components
    import Sidebar from "$lib/components/settings/Sidebar.svelte";
    import GeneralSettings from "$lib/components/settings/GeneralSettings.svelte";
    import ClipboardSettings from "$lib/components/settings/ClipboardSettings.svelte";
    import TraySettings from "$lib/components/settings/TraySettings.svelte";
    import StorageSettings from "$lib/components/settings/StorageSettings.svelte";
    import AboutSection from "$lib/components/settings/AboutSection.svelte";
    import AppearanceSettings from "$lib/components/settings/AppearanceSettings.svelte";

    const t = $derived(i18n.t);

    let settings = $state<Settings>({
        globalShortcut: "CommandOrControl+Shift+V",
        maxHistoryItems: 100,
        autoCleanup: true,
        trayTextLength: 50,
        storeOriginalImage: false,
        maxPinnedInTray: 5,
        maxRecentInTray: 20,
        customDataPath: null,
        enableAutostart: false,
        locale: "zh-CN",
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

    // 侧边栏导航状态
    let activeTab = $state<SettingsTab>("general");

    onMount(async () => {
        // Load settings and data path in parallel for better performance
        await Promise.all([loadSettings(), loadDataPath()]);
    });

    async function loadSettings() {
        try {
            loading = true;
            settings = await invoke<Settings>("get_settings");
        } catch (err) {
            console.error("Failed to load settings:", err);
            const errorMsg = err instanceof Error ? err.message : String(err);
            message = `${t.loadSettingsFailed}: ${errorMsg}`;
        } finally {
            loading = false;
        }
    }

    async function loadDataPath() {
        try {
            currentDataPath = await invoke<string>("get_current_data_path");
        } catch (err) {
            console.error("Failed to load data path:", err);
            currentDataPath = "";
        }
    }

    async function saveSettings() {
        try {
            saving = true;
            message = "";
            await invoke("update_settings", { settings: settings });
            message = t.save + " ✓";
            setTimeout(() => (message = ""), 3000);
        } catch (err) {
            console.error("Failed to save settings:", err);
            const errorMsg = err instanceof Error ? err.message : String(err);
            message = `${t.saveSettingsFailed}: ${errorMsg}`;
        } finally {
            saving = false;
        }
    }

    async function resetSettings() {
        if (!confirm(t.confirmResetSettings)) return;

        try {
            settings = {
                globalShortcut: "CommandOrControl+Shift+V",
                maxHistoryItems: 100,
                autoCleanup: true,
                trayTextLength: 50,
                storeOriginalImage: false,
                maxPinnedInTray: 5,
                maxRecentInTray: 20,
                customDataPath: null,
                enableAutostart: false,
                locale: "zh-CN",
            };
            await saveSettings();
            message = t.reset + " ✓";
        } catch (err) {
            message = t.saveSettingsFailed;
        }
    }

    async function checkForUpdates() {
        try {
            checkingUpdate = true;
            updateMessage = "";
            updateInfo = await invoke<UpdateInfo>("check_for_updates");
            if (updateInfo.available) {
                updateMessage = `${t.updateAvailable}: ${updateInfo.latest_version}`;
            } else {
                updateMessage = t.noUpdateAvailable;
            }
        } catch (err) {
            console.error("Check update failed:", err);
            updateMessage = "检查更新失败";
        } finally {
            checkingUpdate = false;
        }
    }

    async function installUpdate() {
        if (!updateInfo?.available) return;

        try {
            installingUpdate = true;
            updateMessage = "正在下载并安装更新...";
            await invoke("install_update");
            updateMessage = "更新安装成功，请重启应用";
        } catch (err) {
            console.error("Install update failed:", err);
            updateMessage = "安装更新失败: " + String(err);
        } finally {
            installingUpdate = false;
        }
    }

    async function changeDataLocation() {
        try {
            const selected = await open({
                directory: true,
                multiple: false,
                title: "选择新的数据存储位置",
            });

            if (selected && typeof selected === "string") {
                newDataPath = selected;
                showMigrationDialog = true;
            }
        } catch (err) {
            console.error("Failed to select directory:", err);
            message = "选择目录失败";
        }
    }

    async function confirmMigration() {
        try {
            changingDataPath = true;
            showMigrationDialog = false;

            await invoke("migrate_data_location", {
                newPath: newDataPath,
                deleteOld: deleteOldData,
            });

            settings.customDataPath = newDataPath;
            await saveSettings();
            await loadDataPath();

            message = "数据迁移成功！";
            setTimeout(() => (message = ""), 3000);
        } catch (err) {
            console.error("Migration failed:", err);
            const errorMsg = err instanceof Error ? err.message : String(err);
            message = "迁移失败: " + errorMsg;
        } finally {
            changingDataPath = false;
        }
    }
</script>

<div
    class="h-screen flex flex-col bg-background text-foreground overflow-hidden"
>
    <!-- 顶部标题栏 -->
    <header
        class="flex-none flex items-center justify-between px-6 py-4 border-b border-border bg-background/95 backdrop-blur supports-[backdrop-filter]:bg-background/60 z-10"
    >
        <div class="flex items-center gap-4">
            <Button
                variant="ghost"
                size="icon"
                onclick={() => router.goHome()}
                class="hover:bg-muted rounded-full"
            >
                <ChevronLeft class="h-5 w-5" />
            </Button>
            <h1 class="text-xl font-bold tracking-tight">{t.settings}</h1>
        </div>

        <div class="flex items-center gap-2">
            <Button
                variant="outline"
                onclick={resetSettings}
                disabled={loading || saving}
                class="gap-2"
            >
                <RotateCcw class="h-4 w-4" />
                {t.reset}
            </Button>
            <Button
                onclick={saveSettings}
                disabled={loading || saving}
                class="gap-2 min-w-[100px]"
            >
                {#if saving}
                    <Loader2 class="h-4 w-4 animate-spin" />
                    {t.saving}
                {:else}
                    <Save class="h-4 w-4" />
                    {t.save}
                {/if}
            </Button>
        </div>
    </header>

    <div class="flex-1 flex overflow-hidden">
        <!-- 侧边栏导航 -->
        <Sidebar bind:activeTab />

        <!-- 主内容区域 -->
        <main class="flex-1 overflow-y-auto p-8 bg-muted/10">
            {#if loading}
                <div class="flex items-center justify-center h-full">
                    <Loader2 class="h-8 w-8 animate-spin text-primary" />
                </div>
            {:else}
                <div class="max-w-2xl mx-auto space-y-6">
                    {#if activeTab === "general"}
                        <GeneralSettings bind:settings />
                    {:else if activeTab === "clipboard"}
                        <ClipboardSettings bind:settings />
                    {:else if activeTab === "appearance"}
                        <AppearanceSettings />
                    {:else if activeTab === "tray"}
                        <TraySettings bind:settings />
                    {:else if activeTab === "storage"}
                        <StorageSettings
                            bind:settings
                            {currentDataPath}
                            {changingDataPath}
                            {changeDataLocation}
                        />
                    {:else if activeTab === "about"}
                        <AboutSection
                            {updateInfo}
                            {checkingUpdate}
                            {installingUpdate}
                            {updateMessage}
                            {checkForUpdates}
                            {installUpdate}
                        />
                    {/if}
                </div>
            {/if}
        </main>
    </div>

    {#if message}
        <div
            class="absolute bottom-6 right-6 p-4 rounded-md shadow-lg text-sm font-medium animate-in slide-in-from-bottom-4 fade-in duration-300 z-50
            {message.includes('✓')
                ? 'bg-primary text-primary-foreground'
                : 'bg-destructive text-destructive-foreground'}"
        >
            {message}
        </div>
    {/if}
</div>

<!-- 数据迁移确认对话框 -->
{#if showMigrationDialog}
    <div
        class="fixed inset-0 bg-background/80 backdrop-blur-sm z-50 flex items-center justify-center p-4"
    >
        <div
            class="bg-card text-card-foreground rounded-lg shadow-lg max-w-md w-full border border-border p-6 space-y-4 animate-in zoom-in-95 duration-200"
        >
            <h3 class="text-lg font-semibold">{t.confirmMigration}</h3>
            <p class="text-sm text-muted-foreground">
                {t.migratingTo} <br />
                <span class="font-mono bg-muted px-1 rounded"
                    >{newDataPath}</span
                >
            </p>

            <div class="flex items-center space-x-2">
                <input
                    type="checkbox"
                    id="delete-old"
                    bind:checked={deleteOldData}
                    class="rounded border-input"
                />
                <label for="delete-old" class="text-sm font-medium"
                    >{t.deleteOldData}</label
                >
            </div>

            <div class="flex justify-end gap-3 pt-2">
                <Button
                    variant="outline"
                    onclick={() => (showMigrationDialog = false)}
                >
                    {t.cancel}
                </Button>
                <Button onclick={confirmMigration}>{t.startMigration}</Button>
            </div>
        </div>
    </div>
{/if}
