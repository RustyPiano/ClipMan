import { expect, test } from 'bun:test';

// toast store 使用 $state；测试直接导入未编译的 .svelte.ts，把 rune 换成普通函数
Object.defineProperty(globalThis, '$state', { configurable: true, value: <T>(value: T) => value });

const pending: Array<(icon: string | null) => void> = [];
Object.defineProperty(globalThis, 'window', {
  configurable: true,
  value: {
    __TAURI_INTERNALS__: {
      invoke: () => new Promise((resolve) => pending.push(resolve)),
      transformCallback: () => 1,
      unregisterCallback: () => {},
    },
  },
});

const { appIconStore } = await import('../../src/lib/stores/app-icons.svelte');

test('an icon saved while an older lookup is in flight is not overwritten', async () => {
  expect(appIconStore.get('Safari')).toBeNull();
  expect(pending).toHaveLength(1);

  // 后端保存图标并发出 app-icon-saved，此时第一次读取还没返回
  appIconStore.invalidate('Safari');
  appIconStore.get('Safari');
  expect(pending).toHaveLength(2);

  pending[1]('data:image/png;base64,new');
  await Promise.resolve();
  pending[0](null);
  await Promise.resolve();
  await Promise.resolve();

  expect(appIconStore.get('Safari')).toBe('data:image/png;base64,new');
  expect(pending).toHaveLength(2);
});
