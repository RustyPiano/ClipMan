import { expect, test } from '@playwright/test';
import { calls, useTauriMock } from './tauri-mock';

useTauriMock('main', () => {
  const w = window as any;
  w.onboardingCompleted = false;
  w.accessibility = false;
  return async (cmd: string) => {
    if (cmd === 'get_settings')
      return {
        autoPaste: true,
        maxHistoryItems: 100,
        capturePaused: false,
        pasteFormat: 'original',
        locale: 'en',
        onboardingCompleted: w.onboardingCompleted,
        globalShortcut: 'CommandOrControl+Shift+V',
      };
    if (cmd === 'get_update_status')
      return {
        available: true,
        currentVersion: '2.5.3',
        latestVersion: '2.6.0',
        body: null,
        date: null,
      };
    if (cmd === 'check_accessibility_permission') return w.accessibility;
    if (cmd === 'complete_onboarding') {
      w.onboardingCompleted = true;
      return null;
    }
    if (cmd === 'get_recent_clips' || cmd === 'get_pinned_clips') return [];
    return null;
  };
});

test('first launch shows the welcome page until the user starts', async ({ page }) => {
  await page.goto('/');
  const welcome = page.getByRole('region', { name: 'Welcome to ClipLeaf' });
  await expect(welcome).toBeVisible();
  const start = welcome.getByRole('button', { name: 'Get started', exact: true });
  await expect(start).toBeFocused();
  await expect(page.getByText('Accessibility permission needed')).toHaveCount(0);

  await page.keyboard.press('Enter');
  await expect(welcome).toHaveCount(0);
  expect((await calls(page, 'complete_onboarding')).length).toBe(1);
  await expect(page.locator('#quickbar-search')).toBeFocused();
  await expect(page.getByText('Nothing copied yet')).toBeVisible();
  await expect(page.getByText('Copy any text, image, or file and it shows up here.')).toBeVisible();
});

test('keyboard focus cannot reach QuickBar controls behind the welcome page', async ({ page }) => {
  await page.goto('/');
  const welcome = page.getByRole('region', { name: 'Welcome to ClipLeaf' });
  await expect(welcome.getByRole('button', { name: 'Get started', exact: true })).toBeFocused();
  for (let step = 0; step < 4; step++) {
    await page.keyboard.press('Tab');
    expect(
      await page.evaluate(
        () =>
          document.activeElement === document.body ||
          document.activeElement?.closest('[aria-labelledby="onboarding-title"]') !== null
      )
    ).toBe(true);
  }
  await expect(page.locator('#quickbar-search')).not.toBeFocused();
});

test('Escape hides QuickBar without finishing the welcome page', async ({ page }) => {
  await page.goto('/');
  await expect(page.getByRole('button', { name: 'Get started', exact: true })).toBeFocused();
  await page.keyboard.press('Escape');
  await expect.poll(async () => (await calls(page, 'hide_quickbar')).length).toBe(1);
  expect(await calls(page, 'complete_onboarding')).toEqual([]);
});

test('an available update shows in the footer and opens the About page', async ({ page }) => {
  await page.goto('/');
  await page.getByRole('button', { name: 'Get started', exact: true }).click();
  await page.getByRole('button', { name: 'Version 2.6.0 available', exact: true }).click();
  expect((await calls(page, 'open_settings_window')).map((c) => c.args)).toEqual([
    { tab: 'about' },
  ]);
});
