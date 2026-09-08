import { expect, test, type Page } from '@playwright/test';

const pageErrors = new WeakMap<Page, string[]>();

test.afterEach(async ({ page }) => {
  expect(pageErrors.get(page)).toEqual([]);
});

test.beforeEach(async ({ page }) => {
  const errors: string[] = [];
  pageErrors.set(page, errors);
  page.on('pageerror', (error) => errors.push(error.message));
  await page.addInitScript(() => {
    const w = window as any;
    localStorage.setItem('locale', 'en');
    w.calls = [];
    w.failLoad = false;
    w.failSave = false;
    const callbacks = new Map<number, (event: any) => void>();
    const listeners = new Map<string, number[]>();
    let callbackId = 0;
    w.settings = {
      globalShortcut: 'CommandOrControl+Shift+V',
      pinnedShortcut: null,
      autoPaste: true,
      ignoreConcealed: true,
      maxHistoryItems: 321,
      trayTextLength: 70,
      maxPinnedInTray: 5,
      maxRecentInTray: 20,
      customDataPath: '/qa/data',
      enableAutostart: false,
      locale: 'en',
      ignoredApps: [],
      skipSecrets: true,
      maxTextBytes: 2000000,
      maxImageDimension: 4096,
      capturePaused: true,
    };
    w.__TAURI_INTERNALS__ = {
      metadata: { currentWindow: { label: 'settings' }, currentWebview: { label: 'settings' } },
      transformCallback: (fn: (event: any) => void) => {
        callbacks.set(++callbackId, fn);
        return callbackId;
      },
      unregisterCallback: (id: number) => {
        callbacks.delete(id);
      },
      invoke: async (cmd: string, args: any = {}) => {
        w.calls.push({ cmd, args });
        if (cmd === 'plugin:event|listen') {
          listeners.set(args.event, [...(listeners.get(args.event) ?? []), args.handler]);
          return args.handler;
        }
        if (cmd === 'plugin:event|emit' || cmd === 'plugin:event|emit_to') {
          for (const id of listeners.get(args.event) ?? [])
            callbacks.get(id)?.({ event: args.event, payload: args.payload, id });
          return null;
        }
        if (cmd === 'disable_global_shortcut' && w.holdDisable)
          return new Promise<void>((resolve) => {
            w.finishDisable = resolve;
          });
        if (cmd === 'get_settings') {
          if (w.failLoad) throw new Error('load unavailable');
          return w.settings;
        }
        if (cmd === 'get_current_data_path') return '/qa/data';
        if (cmd === 'update_settings') {
          if (w.failSave) throw new Error('save unavailable');
          return {
            settings: args.settings ?? {
              ...w.settings,
              maxHistoryItems: 100,
              pinnedShortcut: null,
            },
            warning: null,
          };
        }
        return null;
      },
    };
  });
});

test('failed settings load cannot expose or save invented defaults; retry reads backend', async ({
  page,
}) => {
  await page.addInitScript(() => {
    (window as any).failLoad = true;
  });
  await page.goto('/');
  await expect(page.getByRole('button', { name: 'Save', exact: true })).toBeDisabled();
  await expect(page.locator('#pinned-shortcut-input')).toHaveCount(0);
  await page.evaluate(() => {
    (window as any).failLoad = false;
  });
  await page.getByRole('button', { name: 'Re-check', exact: true }).click();
  await expect(page.getByRole('button', { name: 'Save', exact: true })).toBeEnabled();
  expect(
    await page.evaluate(() => (window as any).calls.filter((c: any) => c.cmd === 'update_settings'))
  ).toEqual([]);
});

test('reset delegates defaults to backend and keeps draft intact on failure', async ({ page }) => {
  await page.goto('/');
  const shortcut = page.locator('#pinned-shortcut-input');
  await shortcut.fill('CommandOrControl+Shift+P');
  await page.evaluate(() => {
    (window as any).failSave = true;
  });
  await page.getByRole('button', { name: 'Reset', exact: true }).click();
  await page.getByRole('alertdialog').getByRole('button', { name: 'Reset', exact: true }).click();
  await expect(page.getByRole('button', { name: 'Save', exact: true })).toBeEnabled();
  await expect(shortcut).toHaveValue('CommandOrControl+Shift+P');
  await page.evaluate(() => {
    (window as any).failSave = false;
  });
  await page.getByRole('button', { name: 'Reset', exact: true }).click();
  await page.getByRole('alertdialog').getByRole('button', { name: 'Reset', exact: true }).click();
  await expect(shortcut).toHaveValue('');
  expect(
    await page.evaluate(() =>
      (window as any).calls.filter((c: any) => c.cmd === 'update_settings').map((c: any) => c.args)
    )
  ).toEqual([{ settings: null }, { settings: null }]);
});

test('leaving settings restores shortcuts even while disable is pending', async ({ page }) => {
  await page.goto('/');
  await page.evaluate(() => {
    (window as any).holdDisable = true;
  });
  await page.getByRole('button', { name: 'Record', exact: true }).click();
  await expect(page.getByRole('button', { name: 'Cancel', exact: true })).toBeVisible();
  await page.getByRole('button', { name: 'Close', exact: true }).click();
  expect(
    await page.evaluate(() =>
      (window as any).calls.some((c: any) => c.cmd === 'enable_global_shortcut')
    )
  ).toBe(false);
  await page.evaluate(() => (window as any).finishDisable());
  await expect
    .poll(() =>
      page.evaluate(
        () => (window as any).calls.filter((c: any) => c.cmd === 'enable_global_shortcut').length
      )
    )
    .toBe(1);
  await expect(page.getByRole('button', { name: 'Record', exact: true })).toBeVisible();
});

test('native hidden event and blur restore a recording only once', async ({ page }) => {
  await page.goto('/');
  await page.getByRole('button', { name: 'Record', exact: true }).click();
  await page.evaluate(async () => {
    await (window as any).__TAURI_INTERNALS__.invoke('plugin:event|emit', {
      event: 'settings-hidden',
    });
    window.dispatchEvent(new Event('blur'));
  });
  await expect
    .poll(() =>
      page.evaluate(
        () => (window as any).calls.filter((c: any) => c.cmd === 'enable_global_shortcut').length
      )
    )
    .toBe(1);
});

test('native radios support arrow navigation and saved locale updates document language', async ({
  page,
}) => {
  await page.goto('/');
  await page.getByRole('button', { name: 'Appearance', exact: true }).click();
  const english = page.getByRole('radio', { name: 'English', exact: true });
  await english.focus();
  await page.keyboard.press('ArrowLeft');
  await expect(page.getByRole('radio', { name: '简体中文', exact: true })).toBeChecked();
  await page.getByRole('button', { name: 'Save', exact: true }).click();
  await expect(page.locator('html')).toHaveAttribute('lang', 'zh-CN');
});
