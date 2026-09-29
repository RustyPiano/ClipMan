<script lang="ts">
  import { onDestroy } from 'svelte';
  import type { ClipItem } from '$lib/types';
  import { i18n } from '$lib/i18n';
  import {
    decodeClipText,
    decodeFilePaths,
    fileBasename,
    fileDirname,
    formatClipTime,
    looksLikeCode,
    looksLikeDirectory,
  } from '$lib/utils/clip-items';
  import { ROW_HEIGHT_REM } from '$lib/constants';
  import { getNow } from '$lib/utils/now.svelte';
  import {
    CodeXml,
    Link2,
    Files,
    File as FileIcon,
    Folder,
    Image as ImageIcon,
    Pin,
    Check,
    Copy,
    Pencil,
    Trash2,
    X,
  } from 'lucide-svelte';
  import { clipboardStore } from '$lib/stores/clipboard.svelte';
  import { toastStore } from '$lib/stores/toast.svelte';
  import { appIconStore } from '$lib/stores/app-icons.svelte';
  import Button from './ui/Button.svelte';

  const GLYPHS = {
    image: ImageIcon,
    files: Files,
    folder: Folder,
    file: FileIcon,
    link: Link2,
    code: CodeXml,
  };

  type RowAction = {
    id: string;
    icon: typeof Copy;
    label: string;
    run: () => void | Promise<void>;
    pressed?: boolean;
    success?: boolean;
  };

  let {
    item,
    selected = false,
    multiSelected = false,
    multiSelecting = false,
    slotLabel = null,
    position,
    onSelect,
    onHover,
    onToggleSelect,
    onUse,
  }: {
    item: ClipItem;
    selected?: boolean;
    multiSelected?: boolean;
    multiSelecting?: boolean;
    slotLabel?: string | null;
    position: number;
    onSelect: () => void;
    onHover: () => void;
    onToggleSelect: () => void;
    onUse: () => void | Promise<void>;
  } = $props();
  const t = $derived(i18n.t);
  const paths = $derived(item.contentType === 'files' ? decodeFilePaths(item) : []);
  const text = $derived(
    item.contentType === 'text' ? decodeClipText(item, t.emptyContent, t.decodeFailed) : ''
  );
  // 固定结构：40px 内容区（两行文本 / 文件名 / 缩略图；有标签时标签加一行预览）加 24px
  // 元信息行。元信息在每行的位置相同，操作按钮与它等高对齐，不会盖住内容。
  const visiblePaths = $derived(item.label ? paths.slice(0, 1) : paths.slice(0, 2));
  const code = $derived(looksLikeCode(text));
  // 图标按内容种类区分，而不是所有文本都用同一个文档图标
  const kind = $derived(
    item.contentType === 'image'
      ? 'image'
      : item.contentType === 'files'
        ? paths.length > 1
          ? 'files'
          : looksLikeDirectory(paths[0] ?? '')
            ? 'folder'
            : 'file'
        : /^\s*https?:\/\/\S+\s*$/.test(text)
          ? 'link'
          : code
            ? 'code'
            : 'text'
  );
  const time = $derived(
    formatClipTime(item.timestamp * 1000, getNow(), i18n.locale, {
      justNow: t.justNow,
      minutesAgo: (n) => i18n.format(t.minutesAgo, { n }),
      yesterday: t.yesterday,
    })
  );

  // 用字符串拼接元信息：模板里的换行在 WebKit 下会多渲染一个空格。
  const meta = $derived(
    [
      item.sourceApp,
      time,
      item.contentType === 'files' ? i18n.format(t.fileCount, { n: item.fileCount }) : null,
    ]
      .filter(Boolean)
      .join('\u00a0·\u00a0')
  );

  // 来源应用在元信息行最前面，带上它的图标
  const appIcon = $derived(item.sourceApp ? appIconStore.get(item.sourceApp) : null);

  let editing = $state(false);
  let draft = $state('');
  let saving = $state(false);
  // Per-instance copy feedback: if the row scrolls out of the virtualized
  // window before the 1.5s timeout fires, the component unmounts and the
  // checkmark simply disappears early — acceptable for a transient hint.
  let isCopied = $state(false);
  let copyTimeout: ReturnType<typeof setTimeout>;
  const busy = $derived(clipboardStore.isUsing || clipboardStore.isSearchPending || saving);

  onDestroy(() => clearTimeout(copyTimeout));

  async function handleCopy() {
    if (await clipboardStore.copyToClipboard(item)) {
      isCopied = true;
      clearTimeout(copyTimeout);
      copyTimeout = setTimeout(() => (isCopied = false), 1500);
    }
  }

  // Stable ids keep the keyed each (and keyboard focus) intact when the pin
  // label/icon flips. 多选由左侧类型图标兼任的勾选框负责，不在操作栏里重复。
  const rowActions = $derived<RowAction[]>([
    {
      id: 'copy',
      icon: isCopied ? Check : Copy,
      label: t.copy,
      run: () => handleCopy(),
      success: isCopied,
    },
    {
      id: 'pin',
      icon: Pin,
      label: item.isPinned ? t.unpin : t.pin,
      run: () => clipboardStore.togglePin(item.id),
      pressed: item.isPinned,
    },
    {
      id: 'label',
      icon: Pencil,
      label: t.editLabel,
      run: () => {
        draft = item.label ?? '';
        editing = true;
      },
    },
    { id: 'delete', icon: Trash2, label: t.delete, run: () => clipboardStore.deleteItem(item.id) },
  ]);

  async function saveLabel(event: SubmitEvent) {
    event.preventDefault();
    if (busy) return;
    saving = true;
    try {
      await clipboardStore.setClipLabel(item.id, draft);
      editing = false;
    } catch (error) {
      toastStore.add(String(error), 'error');
    } finally {
      saving = false;
    }
  }

  function use(event: MouseEvent) {
    if (event.metaKey || event.ctrlKey) onToggleSelect();
    else {
      onSelect();
      void onUse();
    }
  }
</script>

<div
  role="row"
  tabindex="-1"
  id={`clip-item-${item.id}`}
  aria-selected={multiSelecting ? multiSelected : selected}
  aria-rowindex={position + 1}
  data-active={selected}
  data-multi={multiSelected}
  style:height={`${ROW_HEIGHT_REM}rem`}
  class="clip-row group relative flex w-full items-center gap-3 rounded-lg pl-3 pr-2"
  onmouseenter={() => {
    if (!editing) onHover();
  }}
>
  {#if selected}
    <!-- 光标行左侧强调条；纯装饰，不拦截点击 -->
    <div
      class="pointer-events-none absolute bottom-[25%] left-0 top-[25%] w-[3px] rounded-r-full bg-primary"
      aria-hidden="true"
    ></div>
  {/if}
  <!-- 类型图标兼任多选框：悬停或多选模式下换成圆形勾选框。图标列与 40px 内容区对齐（行内高 68px = 2 + 40 + 24 + 2） -->
  <div role="gridcell" class="flex flex-none self-stretch pb-[26px] pt-0.5">
    <button
      type="button"
      class="clip-glyph flex h-10 w-5 items-center justify-center"
      title={t.toggleSelection}
      aria-label={t.toggleSelection}
      aria-pressed={multiSelected}
      tabindex={selected ? 0 : -1}
      disabled={busy || editing}
      onmousedown={(event) => event.preventDefault()}
      onclick={() => {
        onSelect();
        onToggleSelect();
      }}
    >
      <span
        class="clip-check items-center justify-center rounded-full {multiSelected || multiSelecting
          ? 'flex'
          : 'hidden group-hover:flex'}"
        aria-hidden="true"
      >
        {#if multiSelected}<Check class="h-2.5 w-2.5" strokeWidth={3} />{/if}
      </span>
      <span class={multiSelected || multiSelecting ? 'hidden' : 'group-hover:hidden'}>
        {#if kind === 'text'}
          <!-- 多行文字：三条长短不一的线 -->
          <svg class="h-[15px] w-[15px]" viewBox="0 0 16 16" fill="none" aria-hidden="true">
            <path
              d="M3 4.5h10M3 8h10M3 11.5h6"
              stroke="currentColor"
              stroke-width="1.3"
              stroke-linecap="round"
            />
          </svg>
        {:else}
          {@const Glyph = GLYPHS[kind]}
          <Glyph class="h-[15px] w-[15px]" strokeWidth={1.4} />
        {/if}
      </span>
    </button>
  </div>
  {#if editing}
    <div role="gridcell" class="min-w-0 flex-1 self-stretch">
      <form data-row-editor class="flex h-full items-center gap-1" onsubmit={saveLabel}>
        <input
          aria-label={t.editLabel}
          class="h-8 min-w-0 flex-1 rounded border border-input bg-background px-2 text-sm"
          bind:value={draft}
          disabled={saving}
          {@attach (element) => {
            element.focus();
            element.select();
          }}
          onkeydown={(event) => {
            if (event.key === 'Escape') {
              event.preventDefault();
              editing = false;
            }
          }}
        />
        <Button
          type="submit"
          size="icon"
          variant="ghost"
          class="h-8 w-8"
          title={t.save}
          aria-label={t.save}
          disabled={busy}><Check class="h-4 w-4" /></Button
        >
        <Button
          size="icon"
          variant="ghost"
          class="h-8 w-8"
          title={t.cancel}
          aria-label={t.cancel}
          disabled={saving}
          onclick={() => (editing = false)}><X class="h-4 w-4" /></Button
        >
      </form>
    </div>
  {:else}
    <div role="gridcell" class="min-w-0 flex-1 self-stretch">
      <button
        type="button"
        tabindex="-1"
        class="flex h-full w-full min-w-0 flex-col justify-center pr-8 text-left"
        disabled={busy}
        onmousedown={(event) => event.preventDefault()}
        onclick={use}
      >
        <span class="flex h-10 min-w-0 flex-col justify-center">
          {#if item.label}
            <span class="truncate text-[13px] font-semibold leading-5 text-foreground"
              >{item.label}</span
            >
          {/if}
          {#if item.contentType === 'text'}
            <span
              class="text-foreground {code
                ? 'font-mono text-[12px] leading-5'
                : 'text-[13px] leading-5'} {item.label
                ? 'truncate'
                : 'line-clamp-2 [overflow-wrap:anywhere]'}">{text}</span
            >
          {:else if item.contentType === 'files'}
            {#each visiblePaths as path, index (path)}
              <span class="flex min-w-0 items-center gap-1.5">
                <span class="truncate text-[13px] leading-5 text-foreground"
                  >{fileBasename(path) || path}</span
                >
                {#if index === visiblePaths.length - 1 && paths.length > visiblePaths.length}
                  <span
                    class="flex-none rounded bg-muted/60 px-1.5 text-[10px] font-semibold leading-4 text-muted-foreground"
                    title={i18n.format(t.fileCount, { n: item.fileCount })}
                    >+{paths.length - visiblePaths.length}</span
                  >
                {/if}
              </span>
            {/each}
            {#if paths.length === 1 && fileDirname(paths[0])}
              <span class="truncate text-[11px] leading-4 text-muted-foreground"
                >{fileDirname(paths[0])}</span
              >
            {/if}
          {:else if item.content}
            <!-- 缩略图按自身宽度显示；有标签时缩小 -->
            <img
              src={item.content}
              alt={item.label || t.image}
              class="{item.label
                ? 'max-h-5'
                : 'max-h-10'} w-fit self-start rounded border border-border object-contain"
              loading="lazy"
            />
          {/if}
        </span>
        <span class="row-meta flex h-6 min-w-0 flex-none items-center gap-1 pr-[6.5rem]">
          {#if appIcon}<img src={appIcon} alt="" class="h-3.5 w-3.5 flex-none" />{/if}
          <span class="truncate text-[11px] leading-4 text-muted-foreground">{meta}</span>
          {#if item.hasHtml}
            <span
              class="flex-none rounded bg-muted/60 px-1 text-[9px] font-semibold leading-tight text-muted-foreground"
              title={t.richTextBadge}>Aa</span
            >
          {/if}
        </span>
      </button>
    </div>
    {#if slotLabel}
      <span
        class="slot-hint pointer-events-none absolute right-3 top-2 text-[10px] tabular-nums text-muted-foreground/60"
        aria-hidden="true">{slotLabel}</span
      >
    {/if}
    <div
      role="gridcell"
      class="row-actions absolute bottom-0.5 right-2 flex items-center gap-0.5 rounded-md px-0.5"
    >
      {#each rowActions as action (action.id)}
        <Button
          variant="ghost"
          size="icon"
          class="rounded-md {action.success
            ? 'text-emerald-500'
            : action.pressed
              ? 'text-primary'
              : 'text-muted-foreground'}"
          style="width: 1.5rem; height: 1.5rem"
          title={action.label}
          aria-label={action.label}
          aria-pressed={action.pressed}
          tabindex={selected ? 0 : -1}
          disabled={busy}
          onmousedown={(event) => event.preventDefault()}
          onclick={() => {
            onSelect();
            void action.run();
          }}><action.icon class="h-3.5 w-3.5" /></Button
        >
      {/each}
    </div>
  {/if}
</div>

<style>
  /* 上下透明边框配合 background-clip，让背景内缩 2px：相邻选中行之间留出缝隙，行高不变 */
  .clip-row {
    --row-surface: var(--qb-surface);
    background: var(--row-surface);
    border-block: 2px solid transparent;
    background-clip: padding-box;
  }
  .clip-row:hover {
    --row-surface: var(--qb-hover);
  }
  .clip-row[data-multi='true'] {
    --row-surface: var(--qb-multi);
  }
  .clip-row[data-active='true'] {
    --row-surface: var(--qb-selection);
  }
  .clip-glyph {
    color: color-mix(in srgb, var(--muted-foreground) 85%, transparent);
  }
  .clip-row[data-active='true'] .clip-glyph {
    color: var(--foreground);
  }
  /* 16px 圆形勾选框：未选中是细描边，选中填充主色 */
  .clip-check {
    width: 16px;
    height: 16px;
    box-shadow: inset 0 0 0 1.25px color-mix(in srgb, var(--muted-foreground) 70%, transparent);
  }
  .clip-row[data-multi='true'] .clip-check {
    color: var(--primary-foreground);
    background: var(--primary);
    box-shadow: none;
  }
  /* 操作按钮只在悬停或键盘焦点进入该行时出现，覆盖元信息预留的右侧空白
     （pr-[6.5rem] 对应 4 个 1.5rem 按钮加间距），不会盖住文字；出现时隐藏快捷键提示 */
  .row-actions {
    background: var(--row-surface);
    opacity: 0;
    pointer-events: none;
    transition: opacity 120ms ease-out;
  }
  .group:hover .row-actions,
  .group:focus-within .row-actions {
    opacity: 1;
    pointer-events: auto;
  }
  @media (hover: none) {
    .row-actions {
      opacity: 1;
      pointer-events: auto;
    }
  }
</style>
