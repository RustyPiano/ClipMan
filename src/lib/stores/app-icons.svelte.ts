import { invoke } from '@tauri-apps/api/core';
import { SvelteMap } from 'svelte/reactivity';
import { toastStore } from './toast.svelte';

/** 按应用名缓存来源应用图标（data URL）；null 表示后端还没有这个应用的图标。 */
class AppIconStore {
  private icons = new SvelteMap<string, string | null>();
  private loading = new Set<string>();

  get(name: string): string | null {
    if (!this.icons.has(name) && !this.loading.has(name)) void this.load(name);
    return this.icons.get(name) ?? null;
  }

  /** 后端保存了新图标（`app-icon-saved`）后重新读取。 */
  invalidate(name: string) {
    this.icons.delete(name);
  }

  private async load(name: string) {
    this.loading.add(name);
    try {
      this.icons.set(name, await invoke<string | null>('get_app_icon', { name }));
    } catch (error) {
      // 记为无图标，避免每次渲染重复请求和提示；错误已经显示给用户
      this.icons.set(name, null);
      toastStore.add(String(error), 'error');
    } finally {
      this.loading.delete(name);
    }
  }
}

export const appIconStore = new AppIconStore();
