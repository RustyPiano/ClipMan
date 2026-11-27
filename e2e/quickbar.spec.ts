import { test, expect } from '@playwright/test';
import { calls, useTauriMock } from './tauri-mock';

useTauriMock('main', () => {
  const w = window as any;
  let initialHistory = true;
  const clips = Array.from({ length: 1001 }, (_, index) => ({
    id: `clip-${index}`,
    contentType: 'text',
    content: btoa(`Clipboard entry ${index}`),
    timestamp: Math.floor(Date.now() / 1000) - index * 60,
    isPinned: false,
    pinOrder: null as number | null,
    label: null as string | null,
    sourceApp: 'Editor',
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
  clips[1] = {
    ...clips[1],
    sourceApp: 'A Very Long Source Application Name That Forces Metadata Truncation',
  };
  clips[2] = {
    ...clips[2],
    contentType: 'image',
    content: canvas.toDataURL(),
    contentBytes: 2048,
  };
  clips[4] = {
    ...clips[4],
    contentType: 'files',
    content: btoa(JSON.stringify(['/tmp/report.pdf', '/tmp/budget.xlsx'])),
    fileCount: 2,
  };
  clips[6] = { ...clips[6], label: 'Build gate' };
  clips[7] = {
    ...clips[7],
    contentType: 'files',
    content: btoa(JSON.stringify(['/tmp/report.pdf', '/tmp/budget.xlsx'])),
    fileCount: 3,
  };
  clips[8] = {
    ...clips[8],
    contentType: 'files',
    content: btoa(JSON.stringify(['C:\\Users\\me\\report.docx'])),
    fileCount: 1,
  };
  const appIcon = canvas.toDataURL();
  let pasteFormat = 'original';
  w.__setPasteFormat = (value: string) => {
    pasteFormat = value;
  };
  return async (cmd: string, args: any) => {
    if (cmd === 'get_settings')
      return {
        autoPaste: true,
        maxHistoryItems: 10000,
        capturePaused: false,
        pasteFormat,
        locale: 'en',
      };
    if (cmd === 'check_accessibility_permission') return true;
    if (cmd === 'get_app_icon') return args.name === 'Editor' ? appIcon : null;
    if (cmd === 'get_pinned_clips') return structuredClone(clips.filter((item) => item.isPinned));
    const matchesFilter = (item: (typeof clips)[number], filter?: string) => {
      if (!filter || filter === 'all') return true;
      if (filter === 'image') return item.contentType === 'image';
      if (filter === 'files') return item.contentType === 'files';
      if (filter === 'text') return item.contentType === 'text';
      if (filter === 'link') {
        if (item.contentType !== 'text') return false;
        const decoded = new TextDecoder().decode(
          Uint8Array.from(atob(item.content), (c) => c.charCodeAt(0))
        );
        return /^\s*https?:\/\/\S+\s*$/.test(decoded);
      }
      return true;
    };
    if (cmd === 'get_recent_clips') {
      if (initialHistory) {
        initialHistory = false;
        setTimeout(() => w.emitTestEvent('quickbar-opened'), 5);
      }
      await new Promise((resolve) => setTimeout(resolve, 30));
      const pool = clips.filter((item) => !item.isPinned && matchesFilter(item, args.filter));
      const start = args.beforeId ? pool.findIndex((item) => item.id === args.beforeId) + 1 : 0;
      return structuredClone(pool.slice(start, start + args.limit));
    }
    if (cmd === 'search_clips') {
      await new Promise((resolve) => setTimeout(resolve, 90));
      return args.query === 'many'
        ? clips.filter((item) => matchesFilter(item, args.filter))
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
    if (cmd === 'batch_toggle_pin') {
      let changed = 0;
      for (const id of args.ids) {
        const item = clips.find((entry) => entry.id === id);
        if (item && item.isPinned !== args.isPinned) {
          item.isPinned = args.isPinned;
          changed += 1;
        }
      }
      return changed;
    }
    if (cmd === 'set_quickbar_panel') return null;
    if (cmd === 'update_clip_content') {
      const item = clips.find((entry) => entry.id === args.id);
      if (item) {
        item.content = btoa(String.fromCharCode(...new TextEncoder().encode(args.content)));
        item.contentBytes = args.content.length;
        item.hasHtml = false;
      }
      return null;
    }
    if (cmd === 'create_pinned_clip') {
      const created = {
        id: 'created-pinned',
        contentType: 'text',
        content: btoa(String.fromCharCode(...new TextEncoder().encode(args.content))),
        timestamp: Math.floor(Date.now() / 1000) + 10,
        isPinned: true,
        pinOrder: 99,
        label: args.label,
        sourceApp: 'ClipLeaf',
        hasHtml: false,
        contentBytes: args.content.length,
        fileCount: 0,
      };
      clips.unshift(created);
      return structuredClone(created);
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
  };
});

test.beforeEach(async ({ page }) => {
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
    .poll(() => calls(page, 'paste_clip'))
    .toEqual([{ cmd: 'paste_clip', args: { id: 'needle', mode: 'default', plain: false } }]);
});

test('takePlain paste format makes Enter plain and ⌥Enter rich', async ({ page }) => {
  await page.evaluate(() => (window as any).__setPasteFormat('takePlain'));
  await page.evaluate(() => (window as any).emitTestEvent('settings-changed'));
  await page.waitForTimeout(100);
  await page.getByRole('combobox').fill('plain-default');
  await page.keyboard.press('Enter');
  await expect
    .poll(() => calls(page, 'paste_clip'))
    .toEqual([{ cmd: 'paste_clip', args: { id: 'plain-default', mode: 'default', plain: true } }]);
  await page.getByRole('combobox').fill('plain-inverted');
  await page.keyboard.press('Alt+Enter');
  await expect
    .poll(async () => (await calls(page, 'paste_clip')).map((c) => c.args.plain))
    .toEqual([true, false]);
});

test('closing the session cancels Enter while a search is running', async ({ page }) => {
  await page.getByRole('combobox').evaluate((input: HTMLInputElement) => {
    input.value = 'cancelled';
    input.dispatchEvent(new Event('input', { bubbles: true }));
    for (const key of ['Enter', 'Escape'])
      input.dispatchEvent(new KeyboardEvent('keydown', { key, bubbles: true, cancelable: true }));
  });
  await page.waitForTimeout(150);
  const used = await calls(page, 'search_clips', 'paste_clip');
  expect(used.map((c) => c.cmd)).toEqual(['search_clips']);
  expect(used[0].args.query).toBe('cancelled');
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
  expect(await calls(page, 'search_clips', 'paste_clip')).toEqual([]);
  await input.dispatchEvent('compositionend');
  await expect.poll(async () => (await calls(page, 'search_clips')).length > 0).toBe(true);
});

test('IME confirmation after compositionend leaves Enter available for the next paste', async ({
  page,
}) => {
  const input = page.getByRole('combobox');
  const prevented = await input.evaluate((element: HTMLInputElement) => {
    element.dispatchEvent(new CompositionEvent('compositionstart', { bubbles: true }));
    element.value = '中文';
    element.dispatchEvent(new InputEvent('input', { bubbles: true, isComposing: true }));
    element.dispatchEvent(new CompositionEvent('compositionend', { bubbles: true }));
    const confirmation = new KeyboardEvent('keydown', {
      key: 'Enter',
      keyCode: 229,
      isComposing: false,
      bubbles: true,
      cancelable: true,
    });
    element.dispatchEvent(confirmation);
    return confirmation.defaultPrevented;
  });
  expect(prevented).toBe(false);
  await expect(page.getByRole('row')).toHaveCount(1);
  expect(await calls(page, 'paste_clip')).toEqual([]);
  await input.press('Enter');
  await expect
    .poll(() => calls(page, 'paste_clip'))
    .toEqual([{ cmd: 'paste_clip', args: { id: '中文', mode: 'default', plain: false } }]);
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
});

test('10000 loaded results keep the DOM bounded', async ({ page }) => {
  await page.evaluate(async () => {
    const path = '/src/lib/stores/clipboard.svelte.ts';
    const { clipboardStore } = await import(path);
    const item = clipboardStore.recentItems[0];
    clipboardStore.recentItems = Array.from({ length: 10000 }, (_, index) => ({
      ...item,
      id: `scale-${index}`,
      timestamp: item.timestamp - index,
    }));
    await new Promise<void>((resolve) =>
      requestAnimationFrame(() => requestAnimationFrame(() => resolve()))
    );
  });
  expect(await page.getByRole('row').count()).toBeLessThan(20);
});

test('normal images auto-load original preview while huge images require manual expand', async ({
  page,
}) => {
  await page.getByRole('combobox').focus();
  await page.keyboard.press('ArrowDown');
  await page.keyboard.press('ArrowDown');
  await expect
    .poll(async () => (await calls(page, 'get_clip')).some((call) => call.args.id === 'clip-2'))
    .toBe(true);
});

test('multi-selection is distinct from keyboard focus and exposes image skipping', async ({
  page,
}) => {
  await page.getByRole('combobox').focus();
  await page.keyboard.press('ArrowDown');
  await page.keyboard.press('ArrowDown');
  await page
    .locator('#clip-item-clip-0 [role=gridcell]')
    .first()
    .getByRole('button')
    .click({ modifiers: ['ControlOrMeta'] });
  await expect(page.locator('#clip-item-clip-2')).toHaveAttribute('aria-selected', 'false');
  await page
    .locator('#clip-item-clip-2')
    .getByRole('button', { name: 'Select / deselect' })
    .click();
  await expect(page.locator('.quickbar-panel > footer')).toContainText('Merge skips 1 image(s)');
});

test('multi-selection works from the keyboard and Enter pastes it after mouse toggles', async ({
  page,
}) => {
  const pasted = async () => (await calls(page, 'paste_clips')).map((c) => c.args.ids);
  await page.getByRole('combobox').focus();
  await page.keyboard.press('Shift+ArrowDown');
  await expect(page.locator('.quickbar-panel > footer')).toContainText('2 selected');
  await page.keyboard.press('Enter');
  await expect.poll(pasted).toEqual([['clip-0', 'clip-1']]);

  await page.keyboard.press('Escape');
  for (const id of ['clip-3', 'clip-4']) {
    const row = page.locator(`#clip-item-${id}`);
    await row.hover();
    await row.getByRole('button', { name: 'Select / deselect' }).click();
  }
  await expect(page.getByRole('combobox')).toBeFocused();
  await page.keyboard.press('Enter');
  await expect.poll(pasted).toEqual([
    ['clip-0', 'clip-1'],
    ['clip-3', 'clip-4'],
  ]);
});

test('shift-click selects a range and Enter uses a single checked row', async ({ page }) => {
  const footer = page.locator('.quickbar-panel > footer');
  const body = (id: string) =>
    page.locator(`#clip-item-${id} [role=gridcell]`).nth(1).getByRole('button');
  await body('clip-1').click({ modifiers: ['Shift'] });
  await expect(footer).toContainText('1 selected');
  await body('clip-3').click({ modifiers: ['Shift'] });
  await expect(footer).toContainText('3 selected');
  await page.keyboard.press('Escape');
  await body('clip-3').click({ modifiers: ['ControlOrMeta'] });
  await page.locator('#clip-item-clip-5').hover();
  await expect(page.locator('#clip-item-clip-5')).toHaveAttribute('data-active', 'true');
  await page.keyboard.press('Enter');
  const used = await calls(page, 'paste_clip', 'paste_clips');
  expect(used.map((c) => c.args.id ?? c.args.ids)).toEqual(['clip-3']);
});

test('deleting the cursor row keeps the cursor in place', async ({ page }) => {
  await page.getByRole('combobox').focus();
  for (let i = 0; i < 3; i++) await page.keyboard.press('ArrowDown');
  await expect(page.locator('#clip-item-clip-3')).toHaveAttribute('data-active', 'true');
  await page.keyboard.press('ControlOrMeta+Backspace');
  await expect(page.locator('#clip-item-clip-3')).toHaveCount(0);
  await expect(page.locator('#clip-item-clip-4')).toHaveAttribute('data-active', 'true');
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
  await expect(
    page.locator('aside').getByRole('button', { name: 'Edit content', exact: true })
  ).toBeDisabled();
});

test('row mouse actions show immediate pin badge, pinned count, and keep row height on edit', async ({
  page,
}) => {
  const row = page.locator('#clip-item-clip-1');
  await row.hover();
  await row.getByRole('button', { name: 'Copy', exact: true }).click();
  await expect(row.locator('button[aria-label="Copy"]')).toHaveClass(/is-copied/);
  await row.getByRole('button', { name: 'Favorite', exact: true }).click();
  await expect(row.getByRole('button', { name: 'Unfavorite', exact: true })).toBeVisible();
  await expect(row.locator('[data-pin-badge]')).toBeVisible();
  await expect(page.locator('[data-pinned-count]')).toHaveText('1');
  await row.getByRole('button', { name: 'Edit label', exact: true }).click();
  const input = row.getByRole('textbox', { name: 'Edit label' });
  await input.fill('Mouse label');
  expect((await row.boundingBox())?.height).toBe(60);
  await page.locator('#clip-item-clip-3').hover();
  await input.press('Enter');
  await expect(row).toContainText('Mouse label');
  await row.hover();
  await row.getByRole('button', { name: 'Delete', exact: true }).click();
  await expect(row).toHaveCount(0);
  const recorded = await page.evaluate(() => (window as any).calls);
  expect(recorded.find((call: any) => call.cmd === 'copy_to_system_clipboard').args).toEqual({
    clipId: 'clip-1',
  });
  expect(recorded.find((call: any) => call.cmd === 'set_clip_label').args).toEqual({
    id: 'clip-1',
    label: 'Mouse label',
  });
  expect(recorded.find((call: any) => call.cmd === 'delete_clip').args).toEqual({ id: 'clip-1' });
  expect(
    recorded.filter((call: any) => call.cmd === 'paste_clip' || call.cmd === 'paste_clips')
  ).toEqual([]);
});

test('category filters, creating pinned snippets, and editing preview content work end-to-end', async ({
  page,
}) => {
  await page.getByRole('button', { name: 'Links', exact: true }).click();
  await expect(page.getByRole('row')).toHaveCount(1);
  await expect(page.locator('#clip-item-clip-1')).toBeVisible();
  await page.getByRole('button', { name: 'All', exact: true }).click();

  await page.getByRole('button', { name: 'Favorites', exact: true }).click();
  await page.getByRole('button', { name: 'New', exact: true }).first().click();
  await page.getByPlaceholder('Label (optional)').fill('Standup');
  await page.getByPlaceholder('Type reusable text, snippet, or prompt...').fill('Daily update');
  await page.getByRole('button', { name: 'Save', exact: true }).click();
  await expect(page.locator('#clip-item-created-pinned')).toContainText('Standup');

  await page.locator('aside').getByRole('button', { name: 'Edit content', exact: true }).click();
  await page.locator('aside textarea').fill('Updated daily note');
  await page.locator('#clip-item-created-pinned').hover();
  await page.evaluate(() => {
    (window as any).emitTestEvent('quickbar-hidden');
    (window as any).emitTestEvent('quickbar-opened', { panel: 'pinned' });
  });
  await expect(page.locator('aside textarea')).toHaveValue('Updated daily note');
  await page.locator('aside').getByRole('button', { name: 'Save', exact: true }).click();
  await expect(page.locator('aside pre')).toHaveText('Updated daily note');
});

test('Escape cancels row editing without closing QuickBar or saving', async ({ page }) => {
  const row = page.locator('#clip-item-clip-0');
  await row.hover();
  await row.getByRole('button', { name: 'Edit label', exact: true }).click();
  await row.getByRole('textbox').fill('discard this');
  await row.getByRole('textbox').press('Escape');
  await expect(row.getByRole('textbox')).toHaveCount(0);
  await expect(page.getByRole('combobox')).toBeVisible();
  expect(await calls(page, 'hide_quickbar', 'set_clip_label', 'paste_clip')).toEqual([]);
});

test('row content preserves full width before and during hover', async ({ page }) => {
  const row = page.locator('#clip-item-clip-1');
  const content = row.locator('[role=gridcell]').nth(1);
  const rowBox = (await row.boundingBox())!;
  const before = (await content.boundingBox())!;
  expect(before.width).toBeGreaterThan(rowBox.width - 70);
  await row.hover();
  const after = (await content.boundingBox())!;
  expect(after.width).toBe(before.width);
  await expect(row.locator('.row-actions')).toBeVisible();
});

test('file rows show basenames with +N badge and Windows paths split correctly', async ({
  page,
}) => {
  const multi = page.locator('#clip-item-clip-7');
  await expect(multi).toContainText('report.pdf');
  await expect(multi).toContainText('budget.xlsx');
  await expect(multi.getByText('+1')).toBeVisible();
  await expect(multi).not.toContainText('/tmp/');
  expect(await multi.locator('.row-meta > span.truncate').textContent()).not.toMatch(/\s{2}/);

  const windows = page.locator('#clip-item-clip-8');
  await expect(windows).toContainText('report.docx');
  await expect(windows).toContainText('C:\\Users\\me');
});

test('duplicate file preview paths render both entries and keep the total count', async ({
  page,
}) => {
  await page.evaluate(() => {
    (window as any).emitTestEvent('clipboard-changed', {
      id: 'duplicate-files',
      contentType: 'files',
      content: btoa(JSON.stringify(['/qa/report.pdf', '/qa/report.pdf'])),
      timestamp: Date.now() / 1000,
      isPinned: false,
      pinOrder: null,
      label: null,
      sourceApp: 'Editor',
      hasHtml: false,
      contentBytes: 60,
      fileCount: 3,
    });
  });
  const row = page.locator('#clip-item-duplicate-files');
  const filenames = row.getByText('report.pdf', { exact: true });
  await expect(filenames).toHaveCount(2);
  await expect(row.getByText('+1', { exact: true })).toBeVisible();
  await filenames.first().click();
  await expect
    .poll(() => calls(page, 'paste_clip'))
    .toEqual([
      { cmd: 'paste_clip', args: { id: 'duplicate-files', mode: 'default', plain: false } },
    ]);
});

test('metadata starts with the source app icon and name', async ({ page }) => {
  const meta = page.locator('#clip-item-clip-0 .row-meta');
  await expect(meta.locator('img')).toHaveAttribute('src', /^data:image\/png;base64,/);
  await expect(meta.locator('span.truncate')).toHaveText(/^Editor\u00a0·\u00a0/);
  await expect(page.locator('#clip-item-clip-1 .row-meta img')).toHaveCount(0);
});

test('labeled rows keep a one-line preview next to the label', async ({ page }) => {
  const labeled = page.locator('#clip-item-clip-6');
  await expect(labeled).toContainText('Build gate');
  await expect(labeled).toContainText('bun run check');
});
