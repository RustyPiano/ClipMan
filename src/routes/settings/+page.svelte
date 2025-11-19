<script lang="ts">
import { invoke } from '@tauri-apps/api/core';
import { onMount } from 'svelte';
import { router } from '$lib/stores/router.svelte';
import Button from '$lib/components/ui/Button.svelte';
import Input from '$lib/components/ui/Input.svelte';
import Card from '$lib/components/ui/Card.svelte';
import { 
    ChevronLeft, 
    Keyboard, 
    History, 
    Info, 
    Loader2, 
    RefreshCw, 
    Download, 
    Save, 
    RotateCcw 
} from 'lucide-svelte';

interface Settings {
    globalShortcut: string;
    maxHistoryItems: number;
    autoCleanup: boolean;
}

interface UpdateInfo {
    available: boolean;
    current_version: string;
    latest_version?: string;
    body?: string;
    date?: string;
}

let settings = $state<Settings>({
    globalShortcut: 'CommandOrControl+Shift+V',
    maxHistoryItems: 100,
    autoCleanup: true
});

let loading = $state(true);
let saving = $state(false);
let message = $state('');

// 更新相关状态
let updateInfo = $state<UpdateInfo | null>(null);
let checkingUpdate = $state(false);
let installingUpdate = $state(false);
let updateMessage = $state('');

onMount(async () => {
    await loadSettings();
});

async function loadSettings() {
    try {
        loading = true;
        settings = await invoke<Settings>('get_settings');
    } catch (err) {
        console.error('Failed to load settings:', err);
        const errorMsg = err instanceof Error ? err.message : String(err);
        message = '加载设置失败: ' + errorMsg;
    } finally {
        loading = false;
    }
}

async function saveSettings() {
    try {
        saving = true;
        message = '';
        await invoke('update_settings', { settings });
        message = '设置已保存！';
        setTimeout(() => message = '', 3000);
    } catch (err) {
        console.error('Failed to save settings:', err);
        const errorMsg = err instanceof Error ? err.message : String(err);
        message = '保存失败: ' + errorMsg;
    } finally {
        saving = false;
    }
}

// 常用热键预设
const shortcutPresets = [
    { label: 'Ctrl/Cmd + Shift + V (默认)', value: 'CommandOrControl+Shift+V' },
    { label: 'Ctrl/Cmd + Alt + V', value: 'CommandOrControl+Alt+V' },
    { label: 'Ctrl/Cmd + Shift + C', value: 'CommandOrControl+Shift+C' },
    { label: 'Alt + V', value: 'Alt+V' },
    { label: 'Ctrl/Cmd + `', value: 'CommandOrControl+`' },
];

// 检查更新
async function checkForUpdates() {
    try {
        checkingUpdate = true;
        updateMessage = '';
        updateInfo = await invoke<UpdateInfo>('check_for_updates');

        if (updateInfo.available) {
            updateMessage = `发现新版本 ${updateInfo.latest_version}！`;
        } else {
            updateMessage = '当前已是最新版本';
        }
    } catch (err) {
        console.error('Failed to check for updates:', err);
        const errStr = String(err);
        updateMessage = '检查更新失败: ' + errStr;
        if (errStr.includes('Not Found') || errStr.includes('404')) {
            updateMessage = '检查更新失败: 未找到更新信息 (可能是尚未发布新版本)';
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
        updateMessage = '正在下载并安装更新...';
        await invoke('install_update');
        updateMessage = '更新安装成功！应用将重启。';
    } catch (err) {
        console.error('Failed to install update:', err);
        const errorMsg = err instanceof Error ? err.message : String(err);
        updateMessage = '安装更新失败: ' + errorMsg;
        installingUpdate = false;
    }
}
</script>

<div class="min-h-screen bg-background text-foreground p-6 overflow-y-auto">
    <div class="max-w-2xl mx-auto space-y-6">
        <header class="flex items-center gap-4 pb-4 border-b border-border">
            <Button variant="ghost" size="sm" onclick={() => router.goHome()}>
                <ChevronLeft class="h-4 w-4 mr-1" /> 返回
            </Button>
            <div>
                <h1 class="text-2xl font-bold">设置</h1>
                <p class="text-sm text-muted-foreground">配置 ClipMan 的行为和快捷键</p>
            </div>
        </header>

        {#if loading}
            <div class="flex justify-center py-12 text-muted-foreground">
                <Loader2 class="h-6 w-6 animate-spin mr-2" /> 加载中...
            </div>
        {:else}
            <form onsubmit={(e) => { e.preventDefault(); saveSettings(); }} class="space-y-6">
                <!-- 全局热键设置 -->
                <Card class="p-6 space-y-4">
                    <div>
                        <h2 class="text-lg font-semibold flex items-center gap-2">
                            <Keyboard class="h-5 w-5" /> 全局热键
                        </h2>
                        <p class="text-sm text-muted-foreground mt-1">
                            设置打开 ClipMan 窗口的快捷键。Mac 上 Ctrl 会自动替换为 Cmd。
                        </p>
                    </div>

                    <div class="space-y-2">
                        <label for="shortcut-input" class="text-sm font-medium">自定义快捷键</label>
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
                                    variant={settings.globalShortcut === preset.value ? 'default' : 'outline'}
                                    size="sm"
                                    onclick={() => settings.globalShortcut = preset.value}
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
                            <label for="max-items" class="text-sm font-medium">最大历史条目数</label>
                            <span class="text-sm font-bold text-primary">{settings.maxHistoryItems}</span>
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
                        <p class="text-xs text-muted-foreground text-right">范围: 50 - 500 条</p>
                    </div>

                    <div class="flex items-center gap-2">
                        <input
                            type="checkbox"
                            id="auto-cleanup"
                            bind:checked={settings.autoCleanup}
                            class="w-4 h-4 rounded border-input text-primary focus:ring-ring"
                        />
                        <label for="auto-cleanup" class="text-sm font-medium cursor-pointer">
                            自动清理超出限制的历史记录
                        </label>
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
                                    <span class="bg-muted px-2 py-0.5 rounded text-xs font-mono">{updateInfo.current_version}</span>
                                </p>
                                {#if updateInfo.available && updateInfo.latest_version}
                                    <p class="text-sm">
                                        <strong>最新版本：</strong>
                                        <span class="bg-green-100 dark:bg-green-900 text-green-800 dark:text-green-100 px-2 py-0.5 rounded text-xs font-mono font-bold">{updateInfo.latest_version}</span>
                                    </p>
                                    {#if updateInfo.body}
                                        <div class="mt-2 p-3 bg-muted/50 rounded border border-border text-sm">
                                            <strong class="block mb-1">更新内容：</strong>
                                            <pre class="whitespace-pre-wrap font-sans text-muted-foreground">{updateInfo.body}</pre>
                                        </div>
                                    {/if}
                                {/if}
                            </div>
                        {:else}
                            <p class="text-sm text-muted-foreground italic">点击下方按钮检查更新</p>
                        {/if}

                        <div class="flex gap-2">
                            <Button
                                type="button"
                                variant="secondary"
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
                                    class="bg-green-600 hover:bg-green-700 text-white"
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
                                class="p-3 rounded text-sm
                                {updateMessage.includes('失败') ? 'bg-red-100 dark:bg-red-900/30 text-red-800 dark:text-red-200' : 
                                 updateMessage.includes('最新版本') || updateMessage.includes('成功') ? 'bg-green-100 dark:bg-green-900/30 text-green-800 dark:text-green-200' : 
                                 'bg-muted text-muted-foreground'}"
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
                    <Button type="button" variant="secondary" onclick={loadSettings}>
                        <RotateCcw class="h-4 w-4 mr-2" /> 重置
                    </Button>
                </div>

                {#if message}
                    <div 
                        class="p-4 rounded-md text-sm font-medium text-center
                        {message.includes('失败') ? 'bg-destructive/10 text-destructive' : 'bg-green-100 dark:bg-green-900/30 text-green-800 dark:text-green-200'}"
                    >
                        {message}
                    </div>
                {/if}
            </form>
        {/if}
    </div>
</div>
