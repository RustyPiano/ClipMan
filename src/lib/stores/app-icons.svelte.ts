import { invoke } from '@tauri-apps/api/core';
import { SvelteMap } from 'svelte/reactivity';
import { toastStore } from './toast.svelte';

/** 按应用名缓存来源应用图标（data URL）；null 表示后端没有这个应用的图标，或读取失败。 */
class AppIconStore {
  private icons = new SvelteMap<string, string | null>();
  // 每个应用名正在进行的请求；invalidate 删除它，之后返回的旧结果不会写入缓存
  private requests = new Map<string, symbol>();

  get(name: string): string | null {
    if (!this.icons.has(name) && !this.requests.has(name)) void this.load(name);
    return this.icons.get(name) ?? null;
  }

  /** 后端保存了新图标（`app-icon-saved`）后重新读取。 */
  invalidate(name: string) {
    this.requests.delete(name);
    this.icons.delete(name);
  }

  private async load(name: string) {
    const request = Symbol(name);
    this.requests.set(name, request);
    let icon: string | null = null;
    try {
      icon = await invoke<string | null>('get_app_icon', { name });
    } catch (error) {
      // 缓存为 null，不在每次渲染时重复请求和提示
      toastStore.add(String(error), 'error');
    }
    if (this.requests.get(name) !== request) return;
    this.requests.delete(name);
    this.icons.set(name, icon);
  }
}

export const appIconStore = new AppIconStore();
