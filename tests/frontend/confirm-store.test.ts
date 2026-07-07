import { describe, expect, test } from 'bun:test';

const state = (<T>(value: T) => value) as (<T>(value: T) => T) & {
  raw: <T>(value: T) => T;
};
state.raw = <T>(value: T) => value;
// Mock the $state rune as a global callable (see clipboard-store.test.ts).
Object.defineProperty(globalThis, '$state', { configurable: true, value: state });

const { confirmStore } = await import('../../src/lib/stores/confirm.svelte');

describe('confirm store', () => {
  test('resolves true when confirmed and closes', async () => {
    const pending = confirmStore.ask({ title: 't', message: 'm' });
    expect(confirmStore.open).toBe(true);
    confirmStore.confirm();
    expect(await pending).toBe(true);
    expect(confirmStore.open).toBe(false);
  });

  test('resolves false when cancelled', async () => {
    const pending = confirmStore.ask({ title: 't', message: 'm' });
    confirmStore.cancel();
    expect(await pending).toBe(false);
  });

  test('asking again resolves the previous pending request as cancelled', async () => {
    const first = confirmStore.ask({ title: 'a', message: 'm' });
    const second = confirmStore.ask({ title: 'b', message: 'm' });
    expect(await first).toBe(false);
    confirmStore.confirm();
    expect(await second).toBe(true);
  });
});
