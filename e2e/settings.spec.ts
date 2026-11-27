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
    onboardingCompleted: true,
    autoCheckUpdates: true,
  };
  w.update = {
    available: false,
    currentVersion: '2.5.3',
    latestVersion: null,
    body: null,
    date: null,
    readyToRestart: false,
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
    if (cmd === 'get_storage_usage') return { total: 3, pinned: 1, images: 0, databaseBytes: 4096 };
    if (cmd === 'list_old_backups') return [];
    if (cmd === 'get_update_status') return w.update;
    if (cmd === 'install_update') {
      w.update = { ...w.update, readyToRestart: true };
      return null;
    }
    if (cmd === 'update_settings') {
      if (w.holdSave) {
        w.holdSave = false;
        await new Promise<void>((resolve) => {
          w.finishSave = resolve;
        });
      }
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
  await expect(page.locator('#pinned-shortcut-input')).toHaveCount(0);
  await page.evaluate(() => {
    (window as any).failLoad = false;
  });
  await page.getByRole('button', { name: '重新检查', exact: true }).click();
  await expect(page.locator('#pinned-shortcut-input')).toBeVisible();
  await page.waitForTimeout(500);
  expect(await calls(page, 'update_settings')).toEqual([]);
});

test('changes save automatically and a rejected save reverts the control', async ({ page }) => {
  await page.goto('/');
  const autostart = page.locator('#enable-autostart');
  await autostart.check();
  await expect
    .poll(async () =>
      (await calls(page, 'update_settings')).map((c) => c.args.settings.enableAutostart)
    )
    .toEqual([true]);
  await expect(
    page.getByRole('status').filter({ hasText: 'Changes are saved automatically' })
  ).toBeVisible();

  await page.evaluate(() => {
    (window as any).failSave = true;
  });
  await autostart.uncheck();
  await expect(page.getByText('save unavailable')).toBeVisible();
  await expect(autostart).toBeChecked();
});

test('reset delegates defaults to backend and keeps the saved value on failure', async ({
  page,
}) => {
  await page.goto('/');
  const shortcut = page.locator('#pinned-shortcut-input');
  await shortcut.fill('CommandOrControl+Shift+P');
  await shortcut.press('Tab');
  await expect.poll(async () => (await calls(page, 'update_settings')).length).toBe(1);
  await page.evaluate(() => {
    (window as any).failSave = true;
  });
  await page.getByRole('button', { name: 'Reset', exact: true }).click();
  await page.getByRole('alertdialog').getByRole('button', { name: 'Reset', exact: true }).click();
  await expect(page.getByText('save unavailable')).toBeVisible();
  await expect(shortcut).toHaveValue('CommandOrControl+Shift+P');
  await page.evaluate(() => {
    (window as any).failSave = false;
  });
  await page.getByRole('button', { name: 'Reset', exact: true }).click();
  await page.getByRole('alertdialog').getByRole('button', { name: 'Reset', exact: true }).click();
  await expect(shortcut).toHaveValue('');
  expect(
    (await calls(page, 'update_settings')).map((c) => c.args.settings?.pinnedShortcut ?? null)
  ).toEqual(['CommandOrControl+Shift+P', null, null]);
  expect((await calls(page, 'update_settings')).slice(1).map((c) => c.args)).toEqual([
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
  await expect(page.getByText('folder unavailable')).toBeVisible();
});

test('size inputs keep typed values and invalid fields are not saved', async ({ page }) => {
  await page.goto('/');
  await page.getByRole('button', { name: 'Clipboard', exact: true }).click();
  const textSize = page.locator('#max-text-bytes');
  const imageSize = page.locator('#max-image-dimension');
  await textSize.fill('51');
  await textSize.press('Tab');
  await expect(textSize).toHaveValue('51');
  await page.waitForTimeout(500);
  expect(await calls(page, 'update_settings')).toEqual([]);
  await textSize.fill('1.5');
  await textSize.press('Tab');
  await imageSize.fill('');
  await imageSize.press('Tab');
  await expect
    .poll(async () =>
      (await calls(page, 'update_settings')).map((c) => c.args.settings.maxTextBytes)
    )
    .toEqual([1500000]);
  await imageSize.fill('0');
  await imageSize.press('Tab');
  await expect
    .poll(
      async () => (await calls(page, 'update_settings')).at(-1)?.args.settings.maxImageDimension
    )
    .toBe(0);
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
  await page.evaluate(async () => {
    await (window as any).__TAURI_INTERNALS__.invoke('plugin:event|emit', {
      event: 'settings-hidden',
    });
  });
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

test('settings-open-tab switches the visible page', async ({ page }) => {
  await page.goto('/');
  await expect(page.locator('#enable-autostart')).toBeVisible();
  await page.evaluate(async () => {
    await (window as any).__TAURI_INTERNALS__.invoke('plugin:event|emit', {
      event: 'settings-open-tab',
      payload: 'about',
    });
  });
  await expect(page.locator('#auto-check-updates')).toBeVisible();
});

test('history limit saves only when the slider is released and confirms deletions', async ({
  page,
}) => {
  await page.goto('/');
  await page.getByRole('button', { name: 'Clipboard', exact: true }).click();
  const slider = page.locator('#max-items');
  const drag = (event: string) =>
    slider.evaluate((input: HTMLInputElement, type) => {
      input.value = '1';
      input.dispatchEvent(new Event(type, { bubbles: true }));
    }, event);

  await drag('input');
  await expect(page.getByText('1', { exact: true })).toBeVisible();
  await page.waitForTimeout(500);
  expect(await calls(page, 'update_settings')).toEqual([]);

  await drag('change');
  const dialog = page.getByRole('alertdialog');
  await expect(dialog).toContainText('The oldest 1 non-favorite items will be deleted now.');
  await dialog.getByRole('button', { name: 'Cancel', exact: true }).click();
  await expect(slider).toHaveValue('321');
  await expect(page.getByText('321', { exact: true })).toBeVisible();
  await page.waitForTimeout(500);
  expect(await calls(page, 'update_settings')).toEqual([]);

  await drag('change');
  await dialog.getByRole('button', { name: 'Delete', exact: true }).click();
  await expect
    .poll(async () =>
      (await calls(page, 'update_settings')).map((c) => c.args.settings.maxHistoryItems)
    )
    .toEqual([1]);
});

test('a reset confirmed while a save is in flight runs after it', async ({ page }) => {
  await page.goto('/');
  await page.evaluate(() => {
    (window as any).holdSave = true;
  });
  await page.locator('#enable-autostart').check();
  await expect.poll(async () => (await calls(page, 'update_settings')).length).toBe(1);
  await page.getByRole('button', { name: 'Reset', exact: true }).click();
  await page.getByRole('alertdialog').getByRole('button', { name: 'Reset', exact: true }).click();
  await page.evaluate(() => (window as any).finishSave());
  await expect
    .poll(async () => (await calls(page, 'update_settings')).map((c) => c.args.settings === null))
    .toEqual([false, true]);
});

test('an installed update keeps the restart button after switching tabs', async ({ page }) => {
  await page.addInitScript(() => {
    const w = window as any;
    w.update = { ...w.update, available: true, latestVersion: '2.6.0' };
  });
  await page.goto('/');
  await page.getByRole('button', { name: 'About', exact: true }).click();
  await page.getByRole('button', { name: 'Install update', exact: true }).click();
  const restart = page.getByRole('button', { name: 'Restart now', exact: true });
  await expect(restart).toBeVisible();
  await page.getByRole('button', { name: 'General', exact: true }).click();
  await page.getByRole('button', { name: 'About', exact: true }).click();
  await expect(restart).toBeVisible();
  await expect(page.getByRole('button', { name: 'Install update', exact: true })).toHaveCount(0);
});
