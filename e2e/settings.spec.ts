import { expect, test } from '@playwright/test';
import { calls, useTauriMock } from './tauri-mock';

useTauriMock('settings', () => {
  const w = window as any;
  w.failLoad = false;
  w.failSave = false;
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
  return async (cmd: string, args: any) => {
    if (cmd === 'disable_global_shortcut' && w.holdDisable)
      return new Promise<void>((resolve) => {
        w.finishDisable = resolve;
      });
    if (cmd === 'get_settings') {
      if (w.failLoad) throw new Error('load unavailable');
      return w.settings;
    }
    if (cmd === 'get_current_data_path') {
      if (w.failDataPath) throw new Error('data path unavailable');
      return '/qa/data';
    }
    if (cmd === 'open_folder' && w.failOpenFolder) throw new Error('folder unavailable');
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
  };
});

test('failed settings load cannot expose or save invented defaults; retry reads backend', async ({
  page,
}) => {
  await page.addInitScript(() => {
    (window as any).failLoad = true;
  });
  await page.goto('/');
  await expect(page.getByRole('button', { name: '保存', exact: true })).toBeDisabled();
  await expect(page.locator('#pinned-shortcut-input')).toHaveCount(0);
  await page.evaluate(() => {
    (window as any).failLoad = false;
  });
  await page.getByRole('button', { name: '重新检查', exact: true }).click();
  await expect(page.getByRole('button', { name: 'Save', exact: true })).toBeEnabled();
  expect(await calls(page, 'update_settings')).toEqual([]);
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
  expect((await calls(page, 'update_settings')).map((c) => c.args)).toEqual([
    { settings: null },
    { settings: null },
  ]);
});

test('data path errors allow retry and folder errors are shown', async ({ page }) => {
  await page.addInitScript(() => {
    (window as any).failDataPath = true;
  });
  await page.goto('/');
  await page.getByRole('button', { name: 'Storage', exact: true }).click();
  await expect(page.getByRole('alert')).toContainText('data path unavailable');
  const openFolder = page.getByRole('button', { name: 'Open folder', exact: true });
  await expect(openFolder).toBeDisabled();
  await page.evaluate(() => {
    (window as any).failDataPath = false;
    (window as any).failOpenFolder = true;
  });
  await page.getByRole('button', { name: 'Re-check', exact: true }).click();
  await expect(page.getByRole('alert')).toHaveCount(0);
  await expect(page.getByText('/qa/data', { exact: true })).toBeVisible();
  await openFolder.click();
  await expect(page.getByRole('status')).toContainText('folder unavailable');
});

test('size inputs keep typed values and invalid fields prevent saving', async ({ page }) => {
  await page.goto('/');
  await page.getByRole('button', { name: 'Clipboard', exact: true }).click();
  const textSize = page.locator('#max-text-bytes');
  const imageSize = page.locator('#max-image-dimension');
  const save = page.getByRole('button', { name: 'Save', exact: true });
  await textSize.fill('51');
  await save.click();
  await expect(textSize).toHaveValue('51');
  expect(
    await page.evaluate(() =>
      (window as any).calls.some((call: any) => call.cmd === 'update_settings')
    )
  ).toBe(false);
  await textSize.fill('1.5');
  await imageSize.fill('12');
  await expect(imageSize).toHaveValue('12');
  await imageSize.fill('');
  await save.click();
  expect(
    await page.evaluate(() =>
      (window as any).calls.some((call: any) => call.cmd === 'update_settings')
    )
  ).toBe(false);
  await imageSize.fill('0');
  await save.click();
  expect(
    await page.evaluate(() => {
      const call = (window as any).calls.find((entry: any) => entry.cmd === 'update_settings');
      return [call.args.settings.maxTextBytes, call.args.settings.maxImageDimension];
    })
  ).toEqual([1500000, 0]);
});

test('shortcut recording ignores a modifier alone and records the main key', async ({ page }) => {
  await page.goto('/');
  await page.getByRole('button', { name: 'Record', exact: true }).click();
  await page.keyboard.down('Control');
  await expect(page.getByRole('button', { name: 'Cancel', exact: true })).toBeVisible();
  await page.keyboard.press('k');
  await page.keyboard.up('Control');
  await expect(page.getByRole('button', { name: 'Record', exact: true })).toBeVisible();
  await expect(page.locator('#shortcut-input')).toHaveValue('CommandOrControl+K');
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
  await expect.poll(async () => (await calls(page, 'enable_global_shortcut')).length).toBe(1);
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
  await expect.poll(async () => (await calls(page, 'enable_global_shortcut')).length).toBe(1);
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

test('confirmation dialog focuses its action and Escape cancels', async ({ page }) => {
  await page.goto('/');
  await page.getByRole('button', { name: 'Reset', exact: true }).click();
  const dialog = page.getByRole('alertdialog');
  await expect(dialog.getByRole('button', { name: 'Reset', exact: true })).toBeFocused();
  await dialog.getByRole('button', { name: 'Cancel', exact: true }).focus();
  await page.keyboard.press('Escape');
  await expect(dialog).toHaveCount(0);
  expect(await calls(page, 'update_settings')).toEqual([]);
});

test('migration dialog focuses Cancel and Escape cancels', async ({ page }) => {
  await page.goto('/');
  await page.getByRole('button', { name: 'Storage', exact: true }).click();
  await page.getByRole('button', { name: 'Change location', exact: true }).click();
  const dialog = page.getByRole('dialog', { name: 'Confirm data migration' });
  await expect(dialog.getByRole('button', { name: 'Cancel', exact: true })).toBeFocused();
  await dialog.getByRole('button', { name: 'Cancel', exact: true }).focus();
  await page.keyboard.press('Escape');
  await expect(dialog).toHaveCount(0);
  expect(
    await page.evaluate(() =>
      (window as any).calls.some((c: any) => c.cmd === 'migrate_data_location')
    )
  ).toBe(false);
});
