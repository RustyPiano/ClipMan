<script lang="ts">
import { invoke } from '@tauri-apps/api/core';
import { onMount } from 'svelte';
import { router } from '$lib/stores/router.svelte';

interface Settings {
    globalShortcut: string;
    maxHistoryItems: number;
    autoCleanup: boolean;
}

let settings = $state<Settings>({
    globalShortcut: 'CommandOrControl+Shift+V',
    maxHistoryItems: 100,
    autoCleanup: true
});

let loading = $state(true);
let saving = $state(false);
let message = $state('');

onMount(async () => {
    await loadSettings();
});

async function loadSettings() {
    try {
        loading = true;
        settings = await invoke<Settings>('get_settings');
    } catch (err) {
        console.error('Failed to load settings:', err);
        message = '加载设置失败: ' + err;
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
        message = '保存失败: ' + err;
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
</script>

<div class="settings-page">
    <header>
        <div class="header-top">
            <button class="back-btn" onclick={() => router.goHome()}>← 返回</button>
            <h1>⚙️ 设置</h1>
        </div>
        <p class="subtitle">配置 ClipMan 的行为和快捷键</p>
    </header>

    {#if loading}
        <div class="loading">加载中...</div>
    {:else}
        <form onsubmit={(e) => { e.preventDefault(); saveSettings(); }}>
            <!-- 全局热键设置 -->
            <section class="setting-section">
                <h2>🔥 全局热键</h2>
                <p class="description">
                    设置打开 ClipMan 窗口的快捷键。<br>
                    <small>Mac 上 Ctrl 会自动替换为 Cmd</small>
                </p>

                <div class="form-group">
                    <label for="shortcut-input">自定义快捷键：</label>
                    <input
                        id="shortcut-input"
                        type="text"
                        bind:value={settings.globalShortcut}
                        placeholder="例如: CommandOrControl+Shift+V"
                    />
                </div>

                <div class="form-group">
                    <label>快速选择：</label>
                    <div class="preset-buttons">
                        {#each shortcutPresets as preset}
                            <button
                                type="button"
                                class="preset-btn"
                                class:active={settings.globalShortcut === preset.value}
                                onclick={() => settings.globalShortcut = preset.value}
                            >
                                {preset.label}
                            </button>
                        {/each}
                    </div>
                </div>
            </section>

            <!-- 历史记录设置 -->
            <section class="setting-section">
                <h2>📜 历史记录</h2>

                <div class="form-group">
                    <label for="max-items">
                        最大历史条目数：
                        <span class="value">{settings.maxHistoryItems}</span>
                    </label>
                    <input
                        id="max-items"
                        type="range"
                        min="50"
                        max="500"
                        step="50"
                        bind:value={settings.maxHistoryItems}
                    />
                    <small>范围: 50 - 500 条</small>
                </div>

                <div class="form-group checkbox">
                    <label>
                        <input
                            type="checkbox"
                            bind:checked={settings.autoCleanup}
                        />
                        自动清理超出限制的历史记录
                    </label>
                </div>
            </section>

            <!-- 按钮组 -->
            <div class="actions">
                <button type="submit" class="btn-primary" disabled={saving}>
                    {saving ? '保存中...' : '💾 保存设置'}
                </button>
                <button type="button" class="btn-secondary" onclick={loadSettings}>
                    🔄 重置
                </button>
            </div>

            {#if message}
                <div class="message" class:error={message.includes('失败')}>
                    {message}
                </div>
            {/if}
        </form>
    {/if}
</div>

<style>
.settings-page {
    max-width: 700px;
    margin: 0 auto;
    padding: 2rem;
    font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Oxygen, Ubuntu, Cantarell, sans-serif;
}

header {
    margin-bottom: 2rem;
    border-bottom: 2px solid #e0e0e0;
    padding-bottom: 1rem;
}

.header-top {
    display: flex;
    align-items: center;
    gap: 1rem;
    margin-bottom: 0.5rem;
}

.back-btn {
    padding: 0.5rem 1rem;
    border: 1px solid #ddd;
    background: white;
    border-radius: 4px;
    cursor: pointer;
    font-size: 0.9rem;
    transition: all 0.2s;
}

.back-btn:hover {
    background: #f0f0f0;
    border-color: #999;
}

h1 {
    margin: 0;
    font-size: 2rem;
    color: #333;
}

.subtitle {
    margin: 0.5rem 0 0 0;
    color: #666;
    font-size: 0.95rem;
}

.loading {
    text-align: center;
    padding: 3rem;
    color: #666;
}

.setting-section {
    background: #f8f9fa;
    border-radius: 8px;
    padding: 1.5rem;
    margin-bottom: 1.5rem;
}

.setting-section h2 {
    margin: 0 0 0.5rem 0;
    font-size: 1.3rem;
    color: #444;
}

.description {
    margin: 0 0 1rem 0;
    color: #666;
    font-size: 0.9rem;
}

.form-group {
    margin-bottom: 1.5rem;
}

.form-group:last-child {
    margin-bottom: 0;
}

label {
    display: block;
    margin-bottom: 0.5rem;
    font-weight: 500;
    color: #555;
}

.value {
    color: #007bff;
    font-weight: 600;
}

input[type="text"],
input[type="range"] {
    width: 100%;
    padding: 0.6rem;
    border: 1px solid #ddd;
    border-radius: 4px;
    font-size: 0.95rem;
    box-sizing: border-box;
}

input[type="text"]:focus {
    outline: none;
    border-color: #007bff;
    box-shadow: 0 0 0 3px rgba(0, 123, 255, 0.1);
}

input[type="range"] {
    padding: 0;
    cursor: pointer;
}

.preset-buttons {
    display: flex;
    flex-wrap: wrap;
    gap: 0.5rem;
}

.preset-btn {
    padding: 0.5rem 1rem;
    border: 1px solid #ddd;
    background: white;
    border-radius: 4px;
    cursor: pointer;
    font-size: 0.85rem;
    transition: all 0.2s;
}

.preset-btn:hover {
    border-color: #007bff;
    background: #f0f8ff;
}

.preset-btn.active {
    border-color: #007bff;
    background: #007bff;
    color: white;
}

.checkbox label {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    font-weight: normal;
}

.checkbox input[type="checkbox"] {
    width: auto;
    cursor: pointer;
}

.actions {
    display: flex;
    gap: 1rem;
    margin-top: 2rem;
}

.btn-primary,
.btn-secondary {
    padding: 0.75rem 1.5rem;
    border: none;
    border-radius: 6px;
    font-size: 1rem;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.2s;
}

.btn-primary {
    background: #007bff;
    color: white;
    flex: 1;
}

.btn-primary:hover:not(:disabled) {
    background: #0056b3;
}

.btn-primary:disabled {
    opacity: 0.5;
    cursor: not-allowed;
}

.btn-secondary {
    background: #6c757d;
    color: white;
}

.btn-secondary:hover {
    background: #545b62;
}

.message {
    margin-top: 1rem;
    padding: 1rem;
    border-radius: 4px;
    background: #d4edda;
    color: #155724;
    border: 1px solid #c3e6cb;
}

.message.error {
    background: #f8d7da;
    color: #721c24;
    border-color: #f5c6cb;
}

small {
    display: block;
    margin-top: 0.3rem;
    color: #888;
    font-size: 0.85rem;
}
</style>
