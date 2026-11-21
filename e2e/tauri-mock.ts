import { expect, test, type Page } from '@playwright/test';

type CommandHandler = (cmd: string, args: any) => unknown;
export type Call = { cmd: string; args: any };

function installTauriMock(label: string, createHandler: () => CommandHandler) {
  const w = window as any;
  const callbacks = new Map<number, (event: unknown) => void>();
  const listeners = new Map<string, number[]>();
  let nextId = 0;
  w.calls = [];
  w.__TAURI_EVENT_PLUGIN_INTERNALS__ = { unregisterListener: () => {} };
  w.emitTestEvent = (event: string, payload: unknown = {}) => {
    for (const handler of listeners.get(event) ?? [])
      callbacks.get(handler)?.({ event, payload, id: handler });
  };
  const handleCommand = createHandler();
  w.__TAURI_INTERNALS__ = {
    metadata: { currentWindow: { label }, currentWebview: { label } },
    transformCallback: (callback: (event: unknown) => void) => {
      callbacks.set(++nextId, callback);
      return nextId;
    },
    unregisterCallback: (id: number) => callbacks.delete(id),
    invoke: async (cmd: string, args: any = {}) => {
      w.calls.push({ cmd, args });
      if (cmd === 'plugin:event|listen') {
        listeners.set(args.event, [...(listeners.get(args.event) ?? []), args.handler]);
        return args.handler;
      }
      if (cmd === 'plugin:event|emit' || cmd === 'plugin:event|emit_to') {
        w.emitTestEvent(args.event, args.payload);
        return null;
      }
      return handleCommand(cmd, args);
    },
  };
}

export function useTauriMock(label: string, createHandler: () => CommandHandler) {
  test.beforeEach(async ({ page }) => {
    await page.addInitScript({
      content: `(${installTauriMock})(${JSON.stringify(label)}, ${createHandler});`,
    });
  });
  test.afterEach(async ({ page }) => {
    expect(await page.pageErrors({ filter: 'all' })).toEqual([]);
  });
}

export function calls(page: Page, ...cmds: string[]): Promise<Call[]> {
  return page.evaluate(
    (cmds) => (window as any).calls.filter((call: Call) => cmds.includes(call.cmd)),
    cmds
  );
}
