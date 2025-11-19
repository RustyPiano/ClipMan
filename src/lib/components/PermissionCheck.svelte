<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import Button from './ui/Button.svelte';
  import { AlertTriangle, RefreshCw } from 'lucide-svelte';

  let hasPermission = $state(true);
  let isChecking = $state(true);
  let errorMessage = $state('');

  async function checkPermission() {
    isChecking = true;
    errorMessage = '';
    try {
      // Call backend to check permission
      // Note: This command needs to be implemented in backend or use existing check
      // For now we'll assume it returns boolean if the command exists, or we catch error
      hasPermission = await invoke('check_clipboard_permission');
    } catch (e) {
      console.error('Failed to check permission:', e);
      // If command doesn't exist yet or fails, we might want to show error or assume true
      // For safety in this refactor, let's assume true unless explicitly denied, 
      // but if it's a specific permission error, we set hasPermission = false
      if (String(e).includes('denied')) {
          hasPermission = false;
          errorMessage = String(e);
      } else {
          // If the command is missing (during dev), don't block the UI
          hasPermission = true; 
      }
    } finally {
      isChecking = false;
    }
  }

  onMount(() => {
    checkPermission();
    
    // Re-check when window gains focus
    window.addEventListener('focus', checkPermission);
    return () => {
      window.removeEventListener('focus', checkPermission);
    };
  });
</script>

{#if !hasPermission}
    <div class="bg-amber-50 dark:bg-amber-900/20 border-l-4 border-amber-500 p-4 mb-4 mx-4 rounded-r shadow-sm">
        <div class="flex items-start">
            <div class="flex-shrink-0">
                <AlertTriangle class="h-5 w-5 text-amber-500" />
            </div>
            <div class="ml-3 w-full">
                <h3 class="text-sm font-medium text-amber-800 dark:text-amber-200">
                    需要剪贴板访问权限
                </h3>
                <div class="mt-2 text-sm text-amber-700 dark:text-amber-300">
                    <p>ClipMan 需要辅助功能权限来监听剪贴板变化。</p>
                    <ol class="list-decimal list-inside mt-2 space-y-1">
                        <li>打开 <strong>系统设置</strong> > <strong>隐私与安全性</strong></li>
                        <li>点击 <strong>辅助功能</strong></li>
                        <li>确保 <strong>ClipMan</strong> 已启用</li>
                    </ol>
                </div>
                
                <details class="mt-3">
                    <summary class="text-xs text-amber-600 dark:text-amber-400 cursor-pointer hover:underline">
                        查看详情
                    </summary>
                    {#if errorMessage}
                        <p class="mt-2 p-2 bg-red-50 dark:bg-red-900/20 text-red-800 dark:text-red-200 rounded text-xs font-mono break-all">
                            错误: {errorMessage}
                        </p>
                    {/if}
                </details>
                
                <div class="mt-4">
                    <Button 
                        onclick={checkPermission}
                        class="bg-amber-500 hover:bg-amber-600 text-white border-none"
                        size="sm"
                    >
                        <RefreshCw class="h-4 w-4 mr-2" />
                        重新检查
                    </Button>
                </div>
            </div>
        </div>
    </div>
{/if}
