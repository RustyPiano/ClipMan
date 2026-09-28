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
      pasteFormat: 'original',
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
        if (cmd === 'plugin:dialog|open') return '/qa/new-data';
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
  // 语言来自后端设置；读取失败时界面保持默认中文。
  await expect(page.getByRole('button', { name: '保存', exact: true })).toBeDisabled();
  await expect(page.locator('#pinned-shortcut-input')).toHaveCount(0);
  await page.evaluate(() => {
    (window as any).failLoad = false;
  });
  await page.getByRole('button', { name: '重新检查', exact: true }).click();
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

test('paste format radios switch modes and persist the choice on save', async ({ page }) => {
  await page.goto('/');
  await page.getByRole('button', { name: 'Clipboard', exact: true }).click();
  const original = page.getByRole('radio', { name: 'Keep original formatting', exact: true });
  const globalPlain = page.getByRole('radio', { name: 'Always plain text', exact: true });
  await expect(original).toBeChecked();
  await original.focus();
  await page.keyboard.press('ArrowDown');
  await page.keyboard.press('ArrowDown');
  await expect(globalPlain).toBeChecked();
  await page.getByRole('button', { name: 'Save', exact: true }).click();
  await expect
    .poll(() =>
      page.evaluate(() => {
        const saves = (window as any).calls.filter(
          (c: any) => c.cmd === 'update_settings' && c.args.settings
        );
        return saves[saves.length - 1]?.args?.settings?.pasteFormat;
      })
    )
    .toBe('globalPlain');
});

// 模态对话框外的内容不可交互：连续按 Tab，焦点只会停在对话框内，或离开网页（落到 body）。
async function expectTabStaysInDialog(page: Page, browserName: string) {
  for (let i = 0; i < 4; i++) {
    await page.keyboard.press(browserName === 'webkit' ? 'Alt+Tab' : 'Tab');
    expect(
      await page.evaluate(
        () =>
          document.activeElement === document.body ||
          document.querySelector('dialog[open]')!.contains(document.activeElement)
      )
    ).toBe(true);
  }
}

test('confirmation dialog focuses its action and Escape cancels', async ({ page, browserName }) => {
  await page.goto('/');
  await page.getByRole('button', { name: 'Reset', exact: true }).click();
  const dialog = page.getByRole('alertdialog');
  await expect(dialog.getByRole('button', { name: 'Reset', exact: true })).toBeFocused();
  await expectTabStaysInDialog(page, browserName);
  await dialog.getByRole('button', { name: 'Cancel', exact: true }).focus();
  await page.keyboard.press('Escape');
  await expect(dialog).toHaveCount(0);
  expect(
    await page.evaluate(() => (window as any).calls.filter((c: any) => c.cmd === 'update_settings'))
  ).toEqual([]);
});

test('migration dialog keeps focus inside and Escape cancels', async ({ page, browserName }) => {
  await page.goto('/');
  await page.getByRole('button', { name: 'Storage', exact: true }).click();
  await page.getByRole('button', { name: 'Change location', exact: true }).click();
  const dialog = page.getByRole('dialog', { name: 'Confirm data migration' });
  await expect(dialog.getByRole('button', { name: 'Cancel', exact: true })).toBeFocused();
  await expectTabStaysInDialog(page, browserName);
  await dialog.getByRole('button', { name: 'Cancel', exact: true }).focus();
  await page.keyboard.press('Escape');
  await expect(dialog).toHaveCount(0);
  expect(
    await page.evaluate(() =>
      (window as any).calls.some((c: any) => c.cmd === 'migrate_data_location')
    )
  ).toBe(false);
});
