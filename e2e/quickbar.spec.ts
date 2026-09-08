import { test, expect, type Page } from '@playwright/test';

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
    const callbacks = new Map<number, (event: unknown) => void>();
    const listeners = new Map<string, number[]>();
    let nextId = 0;
    let initialHistory = true;
    const clips = Array.from({ length: 10000 }, (_, index) => ({
      id: `clip-${index}`,
      contentType: 'text',
      content: btoa(`Clipboard entry ${index}`),
      timestamp: Math.floor(Date.now() / 1000) - index * 60,
      isPinned: false,
      pinOrder: null,
      label: null,
      sourceApp: 'Editor',
      groupName: null,
      hasHtml: false,
      contentBytes: 20,
      fileCount: 0,
    }));
    const examples = [
      'Keep common actions close to the content.\n\nThe list should be easy to scan, with room for text and a consistent visual rhythm.',
      'https://developer.apple.com/design/human-interface-guidelines/',
      '',
      '{ "theme": "system", "autoPaste": true }',
      '',
      '周会纪要：确认搜索体验、鼠标操作与发布验收。',
      'bun run check && bun run test:ui',
      'Release notes — clipboard history and search improvements',
    ];
    examples.forEach((text, index) => {
      if (text) clips[index].content = btoa(String.fromCharCode(...new TextEncoder().encode(text)));
    });
    const canvas = document.createElement('canvas');
    canvas.width = 80;
    canvas.height = 40;
    const context = canvas.getContext('2d')!;
    context.fillStyle = '#476ac7';
    context.fillRect(0, 0, 80, 40);
    context.fillStyle = '#d9e5ff';
    context.fillRect(12, 12, 56, 16);
    clips[2] = {
      ...clips[2],
      contentType: 'image',
      content: canvas.toDataURL(),
      contentBytes: 2048,
    };
    clips[4] = {
      ...clips[4],
      contentType: 'files',
      content: btoa('/tmp/report.pdf\n/tmp/budget.xlsx'),
      fileCount: 2,
    };
    w.calls = [];
    w.emitTestEvent = (event: string, payload: unknown = {}) => {
      for (const handler of listeners.get(event) ?? [])
        callbacks.get(handler)?.({ event, payload, id: handler });
    };
    w.__TAURI_INTERNALS__ = {
      metadata: { currentWindow: { label: 'main' }, currentWebview: { label: 'main' } },
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
        if (cmd === 'get_settings')
          return { autoPaste: true, maxHistoryItems: 10000, capturePaused: false };
        if (cmd === 'check_clipboard_permission') return 'granted';
        if (cmd === 'check_accessibility_permission') return true;
        if (cmd === 'get_pinned_clips')
          return structuredClone(clips.filter((item) => item.isPinned));
        if (cmd === 'get_recent_clips') {
          // Exercise a native open event while first-page IPC is still outstanding.
          if (initialHistory) {
            initialHistory = false;
            setTimeout(() => w.emitTestEvent('quickbar-opened'), 5);
          }
          await new Promise((resolve) => setTimeout(resolve, 30));
          const start = args.beforeId ? Number(args.beforeId.split('-')[1]) + 1 : 0;
          return structuredClone(clips.slice(start, start + args.limit));
        }
        if (cmd === 'search_clips') {
          await new Promise((resolve) => setTimeout(resolve, 90));
          return args.query === 'many'
            ? clips.slice(0, 1001)
            : [
                {
                  ...clips[0],
                  id: args.query,
                  contentBytes: args.query === 'large' ? 2_000_000 : 100,
                  content: btoa(String.fromCharCode(...new TextEncoder().encode(args.query))),
                },
              ];
        }
        if (cmd === 'toggle_pin' || cmd === 'set_clip_label') {
          const item = clips.find((item) => item.id === args.id);
          if (item) {
            if (cmd === 'toggle_pin') item.isPinned = args.isPinned;
            else item.label = args.label;
          }
          return null;
        }
        if (cmd === 'get_clip' && args.id === 'clip-2')
          return { id: args.id, contentType: 'image', text: '', imageUrl: clips[2].content };
        if (cmd === 'get_clip')
          return {
            id: args.id,
            contentType: 'text',
            text: clips.find((item) => item.id === args.id)
              ? new TextDecoder().decode(
                  Uint8Array.from(atob(clips.find((item) => item.id === args.id)!.content), (c) =>
                    c.charCodeAt(0)
                  )
                )
              : `Detail for ${args.id}`,
            truncated: args.id === 'large',
            imageUrl: null,
          };
        if (cmd === 'paste_clip' || cmd === 'paste_clips' || cmd === 'hide_quickbar') {
          w.emitTestEvent('quickbar-hidden');
          return 'pasteRequested';
        }
        return null;
      },
    };
  });
  await page.goto('/');
  await expect(page.getByRole('row').first()).toBeVisible();
});

test('renders a bounded list and keyboard navigation reveals the selected row', async ({
  page,
}) => {
  expect(await page.getByRole('row').count()).toBeLessThan(20);
  await page.getByRole('combobox').focus();
  for (let i = 0; i < 30; i++) await page.keyboard.press('ArrowDown');
  await expect(page.getByRole('combobox')).toHaveAttribute(
    'aria-activedescendant',
    'clip-item-clip-30'
  );
  await expect(page.locator('#clip-item-clip-30')).toBeInViewport();
  expect(await page.getByRole('row').count()).toBeLessThan(20);
});

test('Enter submits the current query once without waiting for debounce', async ({ page }) => {
  await page.getByRole('combobox').fill('needle');
  await page.keyboard.press('Enter');
  await expect
    .poll(() =>
      page.evaluate(() => (window as any).calls.filter((c: any) => c.cmd === 'paste_clip'))
    )
    .toEqual([{ cmd: 'paste_clip', args: { id: 'needle', mode: 'default', plain: false } }]);
});

test('closing the session cancels Enter while a search is running', async ({ page }) => {
  await page.getByRole('combobox').fill('cancelled');
  await page.keyboard.press('Enter');
  await page.keyboard.press('Escape');
  await page.waitForTimeout(150);
  expect(
    await page.evaluate(() => (window as any).calls.filter((c: any) => c.cmd === 'paste_clip'))
  ).toEqual([]);
});

test('large searches show a truncation notice and reset the scroller', async ({ page }) => {
  await page.locator('#clipboard-results').evaluate((el) => {
    el.scrollTop = 1200;
  });
  await page.getByRole('combobox').fill('many');
  await expect(page.getByText('Showing the first 1000 results. Refine your search.')).toBeVisible();
  await expect(page.getByRole('combobox')).toHaveAttribute(
    'aria-activedescendant',
    'clip-item-clip-0'
  );
  expect(await page.getByRole('row').count()).toBeLessThan(20);
});

test('IME composition does not search or paste before composition ends', async ({ page }) => {
  const input = page.getByRole('combobox');
  await input.dispatchEvent('compositionstart');
  await input.fill('中');
  await input.dispatchEvent('keydown', { key: 'Enter', isComposing: true, bubbles: true });
  await page.waitForTimeout(60);
  expect(
    await page.evaluate(() =>
      (window as any).calls.filter((c: any) => ['search_clips', 'paste_clip'].includes(c.cmd))
    )
  ).toEqual([]);
  await input.dispatchEvent('compositionend');
  await expect
    .poll(() =>
      page.evaluate(() => (window as any).calls.some((c: any) => c.cmd === 'search_clips'))
    )
    .toBe(true);
});

test('Tab reaches controls and screenshots cover both panel sizes', async ({
  page,
  browserName,
}, testInfo) => {
  await page.getByRole('combobox').focus();
  await page.keyboard.press(browserName === 'webkit' ? 'Alt+Tab' : 'Tab');
  await expect(page.getByRole('button', { name: 'History', exact: true })).toBeFocused();
  await page.screenshot({ path: testInfo.outputPath('quickbar-wide.png') });
  await page.evaluate(() => {
    localStorage.setItem('theme', 'dark');
    window.dispatchEvent(new StorageEvent('storage', { key: 'theme', newValue: 'dark' }));
  });
  await expect(page.locator('html')).toHaveClass(/dark/);
  await page.screenshot({ path: testInfo.outputPath('quickbar-dark.png') });
  await page.setViewportSize({ width: 560, height: 480 });
  await expect(page.locator('aside')).toHaveCount(0);
  await page.screenshot({ path: testInfo.outputPath('quickbar-narrow.png') });
  await page.setViewportSize({ width: 820, height: 600 });
  await page.evaluate(() => {
    window.dispatchEvent(new StorageEvent('storage', { key: 'theme', newValue: 'light-pink' }));
  });
  await expect(page.locator('html')).toHaveClass(/light-pink/);
  await page.screenshot({ path: testInfo.outputPath('quickbar-pink.png') });
  await page.evaluate(async () => {
    const path = '/src/lib/i18n/index.svelte.ts';
    const { i18n } = await import(path);
    i18n.setLocale('zh-CN');
    window.dispatchEvent(new StorageEvent('storage', { key: 'theme', newValue: 'light' }));
  });
  await page.getByRole('combobox').focus();
  await page.screenshot({ path: testInfo.outputPath('quickbar-zh.png') });
});

test('100, 1000 and 10000 loaded results keep the same DOM bound', async ({ page }, testInfo) => {
  const samples = [];
  for (const count of [100, 1000, 10000]) {
    const elapsed = await page.evaluate(async (count) => {
      const path = '/src/lib/stores/clipboard.svelte.ts';
      const { clipboardStore } = await import(path);
      const item = clipboardStore.recentItems[0];
      const start = performance.now();
      clipboardStore.recentItems = Array.from({ length: count }, (_, index) => ({
        ...item,
        id: `scale-${index}`,
        timestamp: item.timestamp - index,
      }));
      await new Promise<void>((resolve) =>
        requestAnimationFrame(() => requestAnimationFrame(() => resolve()))
      );
      return performance.now() - start;
    }, count);
    const mounted = await page.getByRole('row').count();
    expect(mounted).toBeLessThan(20);
    samples.push({ count, mounted, stateToTwoAnimationFramesMs: elapsed });
  }
  await testInfo.attach('renderer-scaling', {
    body: JSON.stringify(samples, null, 2),
    contentType: 'application/json',
  });
});

test('image originals are loaded only when explicitly requested', async ({ page }) => {
  await page.getByRole('combobox').focus();
  await page.keyboard.press('ArrowDown');
  await page.keyboard.press('ArrowDown');
  const load = page.getByRole('button', { name: 'Expand preview' });
  await expect(load).toBeVisible();
  expect(
    await page.evaluate(() =>
      (window as any).calls.filter(
        (call: any) => call.cmd === 'get_clip' && call.args.id === 'clip-2'
      )
    )
  ).toEqual([]);
  await load.click();
  await expect(load).toHaveCount(0);
});

test('multi-selection is distinct from keyboard focus and exposes image skipping', async ({
  page,
}) => {
  await page
    .locator('#clip-item-clip-0 [role=gridcell]')
    .first()
    .getByRole('button')
    .click({ modifiers: ['ControlOrMeta'] });
  await page.getByRole('combobox').focus();
  await page.keyboard.press('ArrowDown');
  await page.keyboard.press('ArrowDown');
  await expect(page.locator('#clip-item-clip-2')).toHaveAttribute('aria-selected', 'false');
  await page
    .locator('#clip-item-clip-2')
    .getByRole('button', { name: 'Select / deselect' })
    .click();
  await expect(page.locator('.quickbar-panel > footer')).toContainText('Merge skips 1 image(s)');
});

test('limited text preview explains that copy still uses complete content', async ({ page }) => {
  await page.getByRole('combobox').fill('large');
  const expand = page.getByRole('button', { name: 'Expand preview' });
  await expect(expand).toBeVisible();
  await expand.click();
  await expect(
    page.getByText('Preview limited to 1 MiB. Copy and paste use the full content.')
  ).toBeVisible();
  await expect(expand).toHaveCount(0);
  await expect(page.locator('pre')).toHaveText('Detail for large');
});

test('row mouse actions target their own item without pasting, and editing keeps row height', async ({
  page,
}) => {
  const row = page.locator('#clip-item-clip-1');
  await row.hover();
  await row.getByRole('button', { name: 'Copy', exact: true }).click();
  await row.getByRole('button', { name: 'Pin', exact: true }).click();
  await expect(row.getByRole('button', { name: 'Unpin', exact: true })).toBeVisible();
  await row.getByRole('button', { name: 'Edit label', exact: true }).click();
  const input = row.getByRole('textbox', { name: 'Edit label' });
  await input.fill('Mouse label');
  expect((await row.boundingBox())?.height).toBe(64);
  await page.locator('#clip-item-clip-3').hover();
  await input.press('Enter');
  await expect(row).toContainText('Mouse label');
  await row.hover();
  await row.getByRole('button', { name: 'Delete', exact: true }).click();
  await expect(row).toHaveCount(0);
  const calls = await page.evaluate(() => (window as any).calls);
  expect(calls.find((call: any) => call.cmd === 'copy_to_system_clipboard').args).toEqual({
    clipId: 'clip-1',
  });
  expect(calls.find((call: any) => call.cmd === 'set_clip_label').args).toEqual({
    id: 'clip-1',
    label: 'Mouse label',
  });
  expect(calls.find((call: any) => call.cmd === 'delete_clip').args).toEqual({ id: 'clip-1' });
  expect(
    calls.filter((call: any) => call.cmd === 'paste_clip' || call.cmd === 'paste_clips')
  ).toEqual([]);
});

test('Escape cancels row editing without closing QuickBar or saving', async ({ page }) => {
  const row = page.locator('#clip-item-clip-0');
  await row.getByRole('button', { name: 'Edit label', exact: true }).click();
  await row.getByRole('textbox').fill('discard this');
  await row.getByRole('textbox').press('Escape');
  await expect(row.getByRole('textbox')).toHaveCount(0);
  await expect(page.getByRole('combobox')).toBeVisible();
  expect(
    await page.evaluate(() =>
      (window as any).calls.filter((call: any) =>
        ['hide_quickbar', 'set_clip_label', 'paste_clip'].includes(call.cmd)
      )
    )
  ).toEqual([]);
});

test('row actions never reserve text width or cover the content line', async ({ page }) => {
  const row = page.locator('#clip-item-clip-1');
  const content = row.locator('[role=gridcell]').first();
  const title = content.locator('.text-foreground');
  const rowBox = (await row.boundingBox())!;
  const before = (await content.boundingBox())!;
  expect(before.width).toBeGreaterThan(rowBox.width - 20);
  await row.hover();
  const after = (await content.boundingBox())!;
  expect(after.width).toBe(before.width);
  const actions = (await row.locator('.row-actions').boundingBox())!;
  const titleBox = (await title.boundingBox())!;
  expect(actions.y).toBeGreaterThanOrEqual(titleBox.y + titleBox.height);
  const slot = (await content.locator('.tabular-nums').boundingBox())!;
  expect(actions.y).toBeGreaterThanOrEqual(slot.y + slot.height);
  await row.getByRole('button', { name: 'Copy', exact: true }).click();
  expect(
    await page.evaluate(() =>
      (window as any).calls.some(
        (call: any) => call.cmd === 'copy_to_system_clipboard' && call.args.clipId === 'clip-1'
      )
    )
  ).toBe(true);
});
