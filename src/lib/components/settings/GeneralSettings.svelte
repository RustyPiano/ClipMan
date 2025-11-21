<script lang="ts">
    import Card from "$lib/components/ui/Card.svelte";
    import Input from "$lib/components/ui/Input.svelte";
    import Button from "$lib/components/ui/Button.svelte";
    import { Keyboard, X } from "lucide-svelte";
    import { invoke } from "@tauri-apps/api/core";

    interface Settings {
        globalShortcut: string;
        enableAutostart: boolean;
        [key: string]: any;
    }

    let { settings = $bindable() } = $props<{
        settings: Settings;
    }>();

    // Detect OS
    const isMac = navigator.platform.toUpperCase().indexOf("MAC") >= 0;

    // Dynamic shortcut presets based on OS
    const shortcutPresets = [
        {
            label: isMac ? "⌘⇧V" : "Ctrl+Shift+V",
            value: "CommandOrControl+Shift+V",
        },
        {
            label: isMac ? "⌘⇧C" : "Ctrl+Shift+C",
            value: "CommandOrControl+Shift+C",
        },
        {
            label: isMac ? "⌥V" : "Alt+V",
            value: "Alt+V",
        },
    ];

    // Convert Tauri shortcut to display format
    function formatShortcut(shortcut: string): string[] {
        if (!shortcut) return [];

        const keys = shortcut.split("+");
        return keys.map((key) => {
            switch (key) {
                case "CommandOrControl":
                    return isMac ? "⌘" : "Ctrl";
                case "Command":
                    return "⌘";
                case "Control":
                    return isMac ? "⌃" : "Ctrl";
                case "Alt":
                    return isMac ? "⌥" : "Alt";
                case "Option":
                    return "⌥";
                case "Shift":
                    return isMac ? "⇧" : "Shift";
                default:
                    return key;
            }
        });
    }

    // Keyboard recording state
    let isRecording = $state(false);
    let recordedKeys = $state<string[]>([]);
    let recordingWarning = $state("");

    async function startRecording() {
        try {
            // Disable global shortcut to prevent triggering during recording
            await invoke("disable_global_shortcut");
            isRecording = true;
            recordedKeys = [];
            recordingWarning = "";
        } catch (err) {
            console.error("Failed to disable shortcut:", err);
            recordingWarning = "无法禁用快捷键";
        }
    }

    async function stopRecording() {
        try {
            // Re-enable global shortcut
            if (isRecording) {
                await invoke("enable_global_shortcut");
            }
        } catch (err) {
            console.error("Failed to re-enable shortcut:", err);
        } finally {
            isRecording = false;
            recordedKeys = [];
            recordingWarning = "";
        }
    }

    function handleKeyDown(event: KeyboardEvent) {
        if (!isRecording) return;

        event.preventDefault();
        event.stopPropagation();

        // ESC to cancel
        if (event.key === "Escape") {
            stopRecording();
            return;
        }

        // Build key combination
        const keys: string[] = [];

        // Add modifiers
        if (event.metaKey || event.ctrlKey) {
            keys.push("CommandOrControl");
        }
        if (event.shiftKey) {
            keys.push("Shift");
        }
        if (event.altKey) {
            keys.push("Alt");
        }

        // Add main key (ignore pure modifiers)
        const mainKey = event.key.toUpperCase();
        if (
            mainKey !== "META" &&
            mainKey !== "CONTROL" &&
            mainKey !== "SHIFT" &&
            mainKey !== "ALT" &&
            mainKey !== "ESCAPE" &&
            mainKey.length === 1
        ) {
            keys.push(mainKey);

            // Build the shortcut string
            const shortcut = keys.join("+");

            // Check if it's the same as current shortcut
            if (shortcut === settings.globalShortcut) {
                recordingWarning = "这已经是当前快捷键了";
                // Auto-close warning after 2 seconds
                setTimeout(() => {
                    stopRecording();
                }, 2000);
            } else {
                // Set the new shortcut
                settings.globalShortcut = shortcut;
                stopRecording();
            }
        } else {
            // Just show modifiers while waiting for main key
            recordedKeys = keys.map((k) => formatShortcut(k + "+X")[0]);
            recordingWarning = "";
        }
    }

    $effect(() => {
        formattedKeys = formatShortcut(settings.globalShortcut);
    });

    let formattedKeys = $state<string[]>(
        formatShortcut(settings.globalShortcut),
    );
</script>

<svelte:window onkeydown={handleKeyDown} />

<div class="space-y-6 animate-in fade-in slide-in-from-bottom-4 duration-300">
    <div>
        <h2 class="text-lg font-semibold mb-1">常规</h2>
        <p class="text-sm text-muted-foreground">应用的基本行为设置</p>
    </div>

    <Card class="p-6 space-y-6">
        <div class="flex items-center justify-between">
            <div class="space-y-0.5">
                <label
                    for="enable-autostart"
                    class="text-sm font-medium cursor-pointer"
                >
                    开机自启动
                </label>
                <p class="text-xs text-muted-foreground">
                    系统启动时自动运行 ClipMan
                </p>
            </div>
            <input
                id="enable-autostart"
                type="checkbox"
                bind:checked={settings.enableAutostart}
                class="w-11 h-6 appearance-none rounded-full relative cursor-pointer transition-colors
                       before:content-[''] before:absolute before:top-1 before:left-1 before:w-4 before:h-4 before:bg-white checked:before:bg-primary-foreground before:rounded-full before:transition-transform
                       checked:before:translate-x-5"
                style:background-color={settings.enableAutostart
                    ? "var(--primary)"
                    : "var(--muted)"}
            />
        </div>

        <div class="space-y-3">
            <div class="flex items-center justify-between">
                <div class="space-y-1">
                    <label for="shortcut-input" class="text-sm font-medium"
                        >全局热键</label
                    >
                    <p class="text-xs text-muted-foreground">
                        设置打开 ClipMan 窗口的快捷键
                    </p>
                </div>
                {#if !isRecording}
                    <Button
                        type="button"
                        variant="outline"
                        size="sm"
                        onclick={startRecording}
                        class="gap-2"
                    >
                        <Keyboard class="h-4 w-4" />
                        录入
                    </Button>
                {:else}
                    <Button
                        type="button"
                        variant="outline"
                        size="sm"
                        onclick={stopRecording}
                        class="gap-2"
                    >
                        <X class="h-4 w-4" />
                        取消
                    </Button>
                {/if}
            </div>

            <!-- Keyboard key display or recording prompt -->
            <div
                class={`flex items-center gap-2 p-3 rounded-lg border transition-all duration-200 ${
                    isRecording
                        ? "bg-primary/10 border-primary animate-pulse"
                        : "bg-muted/30 border-border"
                }`}
            >
                {#if isRecording}
                    <div class="flex-1 text-center">
                        {#if recordingWarning}
                            <div
                                class="flex items-center justify-center gap-2 text-sm text-orange-600 dark:text-orange-400"
                            >
                                <span>⚠️</span>
                                <span>{recordingWarning}</span>
                            </div>
                        {:else if recordedKeys.length > 0}
                            <div
                                class="flex items-center justify-center gap-1.5"
                            >
                                {#each recordedKeys as key}
                                    <kbd
                                        class="inline-flex items-center justify-center min-w-[2rem] h-8 px-2.5 text-sm font-semibold
                                               bg-gradient-to-b from-background to-muted
                                               border border-border rounded-md shadow-sm
                                               text-foreground"
                                    >
                                        {key}
                                    </kbd>
                                    <span class="text-muted-foreground text-sm"
                                        >+</span
                                    >
                                {/each}
                                <span
                                    class="text-muted-foreground text-sm animate-pulse"
                                    >?</span
                                >
                            </div>
                        {:else}
                            <div
                                class="flex items-center justify-center gap-2 text-sm text-muted-foreground"
                            >
                                <Keyboard class="h-4 w-4" />
                                <span>按下快捷键组合...</span>
                                <span class="text-xs">(ESC 取消)</span>
                            </div>
                        {/if}
                    </div>
                {:else}
                    <div class="flex items-center gap-1.5">
                        {#each formattedKeys as key, index}
                            <kbd
                                class="inline-flex items-center justify-center min-w-[2rem] h-8 px-2.5 text-sm font-semibold
                                       bg-gradient-to-b from-background to-muted
                                       border border-border rounded-md shadow-sm
                                       text-foreground"
                            >
                                {key}
                            </kbd>
                            {#if index < formattedKeys.length - 1}
                                <span class="text-muted-foreground text-sm"
                                    >+</span
                                >
                            {/if}
                        {/each}
                    </div>
                {/if}
            </div>

            <!-- Preset shortcuts -->
            <div class="flex flex-wrap gap-2 pt-1">
                <span class="text-xs text-muted-foreground self-center"
                    >常用快捷键:</span
                >
                {#each shortcutPresets as preset}
                    <Button
                        type="button"
                        variant={settings.globalShortcut === preset.value
                            ? "default"
                            : "outline"}
                        size="sm"
                        onclick={() => (settings.globalShortcut = preset.value)}
                    >
                        {preset.label}
                    </Button>
                {/each}
            </div>

            <!-- Advanced: Manual input -->
            <details class="group">
                <summary
                    class="cursor-pointer text-xs text-muted-foreground hover:text-foreground transition-colors select-none"
                >
                    高级：手动输入...
                </summary>
                <div class="mt-2">
                    <Input
                        id="shortcut-input"
                        type="text"
                        bind:value={settings.globalShortcut}
                        placeholder="例如: CommandOrControl+Shift+V"
                        class="text-sm font-mono"
                    />
                </div>
            </details>
        </div>
    </Card>
</div>
