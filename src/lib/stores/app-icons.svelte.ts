import { invoke } from '@tauri-apps/api/core';
import { SvelteMap } from 'svelte/reactivity';
import { toastStore } from './toast.svelte';

class AppIconStore {
  private icons = new SvelteMap<string, string | null>();
  private requests = new Map<string, symbol>();

  get(name: string): string | null {
    if (!this.icons.has(name) && !this.requests.has(name)) void this.load(name);
    return this.icons.get(name) ?? null;
  }

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
      toastStore.add(String(error), 'error');
    }
    if (this.requests.get(name) !== request) return;
    this.requests.delete(name);
    this.icons.set(name, icon);
  }
}

export const appIconStore = new AppIconStore();
