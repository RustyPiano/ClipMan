<script lang="ts">
import { invoke } from '@tauri-apps/api/core';
import { onMount } from 'svelte';
import { router } from '$lib/stores/router.svelte';

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
        updateMessage = '检查更新失败: ' + err;
        if (err.toString().includes('Not Found') || err.toString().includes('404')) {
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
        updateMessage = '安装更新失败: ' + err;
        installingUpdate = false;
    }
}
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
                    <span class="form-label">快速选择：</span>
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

            <!-- 关于和更新 -->
            <section class="setting-section">
                <h2>ℹ️ 关于和更新</h2>

                <div class="update-info">
                    <div class="version-info">
                        {#if updateInfo}
                            <p>
                                <strong>当前版本：</strong>
                                <span class="version">{updateInfo.current_version}</span>
                            </p>
                            {#if updateInfo.available && updateInfo.latest_version}
                                <p>
                                    <strong>最新版本：</strong>
                                    <span class="version latest">{updateInfo.latest_version}</span>
                                </p>
                                {#if updateInfo.body}
                                    <div class="release-notes">
                                        <strong>更新内容：</strong>
                                        <pre>{updateInfo.body}</pre>
                                    </div>
                                {/if}
                            {/if}
                        {:else}
                            <p class="hint">点击下方按钮检查更新</p>
                        {/if}
                    </div>

                    <div class="update-actions">
                        <button
                            type="button"
                            class="btn-update"
                            onclick={checkForUpdates}
                            disabled={checkingUpdate || installingUpdate}
                        >
                            {checkingUpdate ? '检查中...' : '🔍 检查更新'}
                        </button>

                        {#if updateInfo?.available}
                            <button
                                type="button"
                                class="btn-install"
                                onclick={installUpdate}
                                disabled={installingUpdate}
                            >
                                {installingUpdate ? '安装中...' : '⬇️ 安装更新'}
                            </button>
                        {/if}
                    </div>

                    {#if updateMessage}
                        <div
                            class="update-message"
                            class:error={updateMessage.includes('失败')}
                            class:success={updateMessage.includes('最新版本') || updateMessage.includes('成功')}
                        >
                            {updateMessage}
                        </div>
                    {/if}
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
    height: 100vh;
    overflow-y: auto;
    box-sizing: border-box;
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

label,
.form-label {
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

/* 更新相关样式 */
.update-info {
    display: flex;
    flex-direction: column;
    gap: 1rem;
}

.version-info {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
}

.version-info p {
    margin: 0;
    display: flex;
    align-items: center;
    gap: 0.5rem;
}

.version {
    font-family: 'Courier New', monospace;
    background: #e9ecef;
    padding: 0.2rem 0.5rem;
    border-radius: 3px;
    font-size: 0.9rem;
}

.version.latest {
    background: #d4edda;
    color: #155724;
    font-weight: 600;
}

.release-notes {
    margin-top: 0.5rem;
    padding: 0.75rem;
    background: white;
    border: 1px solid #ddd;
    border-radius: 4px;
}

.release-notes strong {
    display: block;
    margin-bottom: 0.5rem;
}

.release-notes pre {
    margin: 0;
    white-space: pre-wrap;
    word-wrap: break-word;
    font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Oxygen, Ubuntu, Cantarell, sans-serif;
    font-size: 0.85rem;
    line-height: 1.5;
    color: #555;
}

.hint {
    margin: 0;
    color: #888;
    font-style: italic;
}

.update-actions {
    display: flex;
    gap: 0.5rem;
    flex-wrap: wrap;
}

.btn-update,
.btn-install {
    padding: 0.6rem 1.2rem;
    border: none;
    border-radius: 6px;
    font-size: 0.95rem;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.2s;
    min-width: 120px;
    display: flex;
    justify-content: center;
    align-items: center;
}

.btn-update {
    background: #6c757d;
    color: white;
}

.btn-update:hover:not(:disabled) {
    background: #545b62;
}

.btn-install {
    background: #28a745;
    color: white;
}

.btn-install:hover:not(:disabled) {
    background: #218838;
}

.btn-update:disabled,
.btn-install:disabled {
    opacity: 0.5;
    cursor: not-allowed;
}

.update-message {
    padding: 0.75rem;
    border-radius: 4px;
    font-size: 0.9rem;
    background: #e9ecef;
    color: #495057;
}

.update-message.success {
    background: #d4edda;
    color: #155724;
    border: 1px solid #c3e6cb;
}

.update-message.error {
    background: #f8d7da;
    color: #721c24;
    border: 1px solid #f5c6cb;
}
</style>
