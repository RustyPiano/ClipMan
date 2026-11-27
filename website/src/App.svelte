<script lang="ts">
  import { ArrowUpRight, Copy, Download, Globe, Moon, Sun } from 'lucide-svelte';
  import BrandMark from './components/BrandMark.svelte';
  import InstallDialog from './components/InstallDialog.svelte';
  import PlainPasteShowcase from './components/PlainPasteShowcase.svelte';
  import QuickBarSandbox from './components/QuickBarSandbox.svelte';
  import {
    COPY,
    DOWNLOADS,
    RELEASES_URL,
    REPO_URL,
    detectLocale,
    detectOs,
    type Locale,
    type OsKind,
  } from './lib/i18n';
  import { themeStore, type ResolvedTheme } from './lib/theme.svelte';

  let locale = $state<Locale>(detectLocale());
  let detectedOs = $state<OsKind | null>(detectOs());
  let dialogOpenSignal = $state(0);
  let swatchToast = $state<string | null>(null);
  let swatchTimer: ReturnType<typeof setTimeout> | null = null;

  let copy = $derived(COPY[locale]);
  let primaryDownload = $derived(detectedOs ? DOWNLOADS[detectedOs] : null);
  let osOrder: OsKind[] = ['mac-arm', 'mac-x64', 'win', 'linux'];

  const swatches = [
    { hex: '#181818', rgb: 'rgb(24, 24, 24)', label: 'Ink' },
    { hex: '#D97706', rgb: 'rgb(217, 119, 6)', label: 'Amber' },
    { hex: '#9F3C5C', rgb: 'rgb(159, 60, 92)', label: 'Petal' },
  ];

  $effect(() => {
    return themeStore.init();
  });

  $effect(() => {
    document.documentElement.lang = locale === 'zh' ? 'zh-CN' : 'en';
    document.title =
      locale === 'zh'
        ? 'ClipLeaf — 复制过的，都在手边'
        : 'ClipLeaf — Everything you copy. Kept light.';
  });

  function selectTheme(next: ResolvedTheme): void {
    themeStore.set(next);
  }

  function openMacGuide(): void {
    dialogOpenSignal += 1;
  }

  async function copySwatch(hex: string): Promise<void> {
    if (swatchTimer) clearTimeout(swatchTimer);
    if (!navigator.clipboard?.writeText) {
      swatchToast = copy.sandboxCopyFailedToast(hex);
    } else {
      try {
        await navigator.clipboard.writeText(hex);
        swatchToast = copy.sandboxCopiedToast(hex);
      } catch {
        swatchToast = copy.sandboxCopyFailedToast(hex);
      }
    }
    swatchTimer = setTimeout(() => {
      swatchToast = null;
    }, 2400);
  }
</script>

<div class="min-h-screen">
  <header class="sticky top-0 z-30 bg-[var(--page-bg)]/84 backdrop-blur-md">
    <div class="mx-auto flex h-16 max-w-6xl items-center justify-between gap-4 px-4 sm:px-6">
      <a href="#top" class="flex items-center gap-2.5 text-[var(--ink)]">
        <BrandMark size={24} />
        <span class="text-[15px] font-semibold tracking-tight">ClipLeaf</span>
      </a>

      <div class="flex items-center gap-2">
        <div
          class="flex items-center gap-0.5 rounded-full bg-[var(--stage-bg)] p-1 text-xs"
          role="group"
          aria-label="Theme"
        >
          <button
            type="button"
            onclick={() => selectTheme('paper')}
            aria-label={copy.themePaper}
            aria-pressed={themeStore.resolved === 'paper'}
            class={[
              'inline-flex items-center gap-1 rounded-full px-2.5 py-1 transition',
              themeStore.resolved === 'paper'
                ? 'bg-[var(--surface)] font-medium text-[var(--ink)] shadow-2xs'
                : 'text-[var(--ink-muted)] hover:text-[var(--ink)]',
            ]}
          >
            <Sun class="h-3.5 w-3.5" strokeWidth={1.6} aria-hidden="true" />
            <span class="hidden sm:inline" aria-hidden="true">{copy.themePaper}</span>
          </button>
          <button
            type="button"
            onclick={() => selectTheme('ink')}
            aria-label={copy.themeInk}
            aria-pressed={themeStore.resolved === 'ink'}
            class={[
              'inline-flex items-center gap-1 rounded-full px-2.5 py-1 transition',
              themeStore.resolved === 'ink'
                ? 'bg-[var(--surface)] font-medium text-[var(--ink)] shadow-2xs'
                : 'text-[var(--ink-muted)] hover:text-[var(--ink)]',
            ]}
          >
            <Moon class="h-3.5 w-3.5" strokeWidth={1.6} aria-hidden="true" />
            <span class="hidden sm:inline" aria-hidden="true">{copy.themeInk}</span>
          </button>
          <button
            type="button"
            onclick={() => selectTheme('petal')}
            aria-label={copy.themePetal}
            aria-pressed={themeStore.resolved === 'petal'}
            class={[
              'inline-flex items-center gap-1 rounded-full px-2.5 py-1 transition',
              themeStore.resolved === 'petal'
                ? 'bg-[var(--surface)] font-medium text-[var(--ink)] shadow-2xs'
                : 'text-[var(--ink-muted)] hover:text-[var(--ink)]',
            ]}
          >
            <span class="h-2.5 w-2.5 rounded-full bg-[#9f3c5c]" aria-hidden="true"></span>
            <span class="hidden sm:inline" aria-hidden="true">{copy.themePetal}</span>
          </button>
        </div>

        <button
          type="button"
          onclick={() => (locale = locale === 'zh' ? 'en' : 'zh')}
          aria-label="Switch language"
          class="inline-flex h-8 items-center gap-1 rounded-full bg-[var(--stage-bg)] px-3 text-xs font-medium text-[var(--ink-muted)] transition hover:text-[var(--ink)]"
        >
          <Globe class="h-3.5 w-3.5" strokeWidth={1.6} aria-hidden="true" />
          <span>{locale === 'zh' ? 'EN' : '中文'}</span>
        </button>

        <a
          href="#download"
          class="hidden h-8 items-center rounded-full bg-[var(--primary)] px-3.5 text-xs font-medium text-[var(--primary-fg)] transition hover:opacity-90 sm:inline-flex"
        >
          {copy.navDownload}
        </a>
      </div>
    </div>
  </header>

  <main id="top">
    <section class="mx-auto max-w-6xl px-4 pt-16 pb-12 text-center sm:px-6 sm:pt-24 sm:pb-16">
      <h1
        class="mx-auto max-w-4xl text-balance text-4xl font-semibold tracking-[-0.04em] text-[var(--ink)] sm:text-7xl sm:leading-[1.04]"
      >
        {copy.heroTitle}
      </h1>

      <p
        class="mx-auto mt-6 max-w-2xl text-balance text-base leading-relaxed text-[var(--ink-muted)] sm:text-lg"
      >
        {copy.heroSub}
      </p>

      <div class="mt-9 flex flex-wrap items-center justify-center gap-4">
        {#if primaryDownload}
          <a
            href={primaryDownload.url}
            class="inline-flex items-center gap-2 rounded-full bg-[var(--primary)] px-7 py-3.5 text-sm font-medium text-[var(--primary-fg)] transition hover:opacity-90"
          >
            <Download class="h-4 w-4" strokeWidth={1.7} aria-hidden="true" />
            <span>
              {copy.downloadFor}
              {locale === 'zh' ? ` ${primaryDownload.labelZh}` : ` ${primaryDownload.labelEn}`}
            </span>
          </a>
          <a
            href="#download"
            class="text-sm font-medium text-[var(--ink-muted)] transition hover:text-[var(--ink)]"
          >
            {copy.allPlatforms}
          </a>
        {:else}
          <a
            href="#download"
            class="inline-flex items-center gap-2 rounded-full bg-[var(--primary)] px-7 py-3.5 text-sm font-medium text-[var(--primary-fg)] transition hover:opacity-90"
          >
            <Download class="h-4 w-4" strokeWidth={1.7} aria-hidden="true" />
            <span>{copy.chooseInstaller}</span>
          </a>
        {/if}

        <button
          type="button"
          onclick={openMacGuide}
          class="text-sm text-[var(--ink-faint)] underline decoration-[var(--hairline-strong)] underline-offset-4 transition hover:text-[var(--ink)]"
        >
          {copy.macGuideTrigger}
        </button>
      </div>

      <div class="mt-14 sm:mt-18">
        <QuickBarSandbox {locale} />
      </div>
    </section>

    <PlainPasteShowcase {locale} />

    <section id="features" class="mx-auto max-w-5xl px-4 py-20 sm:px-6 sm:py-32">
      <h2
        class="text-balance text-3xl font-semibold tracking-[-0.03em] text-[var(--ink)] sm:text-5xl"
      >
        {copy.craftHeading}
      </h2>

      <div class="mt-16 space-y-20 sm:mt-20 sm:space-y-28">
        <div class="grid grid-cols-1 items-center gap-8 lg:grid-cols-12">
          <div class="lg:col-span-5">
            <h3 class="text-xl font-semibold tracking-tight text-[var(--ink)]">
              {copy.craftItems[0].title}
            </h3>
            <p class="mt-3 text-sm leading-relaxed text-[var(--ink-muted)] sm:text-base">
              {copy.craftItems[0].body}
            </p>
            <div class="mt-4 font-mono text-xs text-[var(--ink-faint)]">
              {copy.craftItems[0].meta}
            </div>
          </div>
          <div class="rounded-[28px] bg-[var(--stage-bg)] p-7 sm:p-10 lg:col-span-7">
            <div class="flex items-center justify-between font-mono text-sm">
              <div class="flex items-center gap-2 text-[var(--ink)]">
                <span class="text-[var(--ink-faint)]">/</span>
                <span>{locale === 'zh' ? '本地 剪贴' : 'sqlite trigram'}</span>
                <span class="inline-block h-4 w-[2px] bg-[var(--accent)]"></span>
              </div>
              <span class="text-xs text-[var(--accent)]">1.4 ms</span>
            </div>
            <div class="mt-6 space-y-3 text-sm text-[var(--ink-muted)]">
              <p>
                {locale === 'zh' ? 'ClipLeaf 是一款' : 'Sub-millisecond '}
                <mark class="qb-mark">{locale === 'zh' ? '本地' : 'sqlite'}</mark>
                {locale === 'zh' ? '优先的' : ' FTS5 '}
                <mark class="qb-mark">{locale === 'zh' ? '剪贴' : 'trigram'}</mark>
                {locale === 'zh' ? '板管理器' : ' full-text lookup'}
              </p>
              <p class="font-mono text-xs text-[var(--ink-faint)]">
                {locale === 'zh'
                  ? 'macOS 原生中文输入法候选窗无冲突'
                  : 'Zero conflict with macOS CJK IME candidate windows'}
              </p>
            </div>
          </div>
        </div>

        <div class="grid grid-cols-1 items-center gap-8 lg:grid-cols-12">
          <div class="lg:order-2 lg:col-span-5">
            <h3 class="text-xl font-semibold tracking-tight text-[var(--ink)]">
              {copy.craftItems[1].title}
            </h3>
            <p class="mt-3 text-sm leading-relaxed text-[var(--ink-muted)] sm:text-base">
              {copy.craftItems[1].body}
            </p>
            <div class="mt-4 font-mono text-xs text-[var(--ink-faint)]">
              {copy.craftItems[1].meta}
            </div>
          </div>
          <div class="lg:order-1 lg:col-span-7">
            <div class="grid grid-cols-3 gap-4">
              {#each swatches as sw (sw.hex)}
                <button
                  type="button"
                  onclick={() => copySwatch(sw.hex)}
                  class="group text-left transition"
                >
                  <div
                    class="flex h-32 items-end justify-between rounded-[22px] p-4 text-white transition group-hover:-translate-y-0.5 sm:h-40"
                    style:background={sw.hex}
                  >
                    <span class="font-mono text-xs font-medium">{sw.hex}</span>
                    <Copy
                      class="h-3.5 w-3.5 opacity-0 transition group-focus-visible:opacity-100 group-hover:opacity-90"
                      strokeWidth={1.6}
                      aria-hidden="true"
                    />
                  </div>
                  <div class="mt-2.5 flex items-center justify-between px-1 text-xs">
                    <span class="font-medium text-[var(--ink)]">{sw.label}</span>
                    <span class="hidden font-mono text-[11px] text-[var(--ink-faint)] sm:inline">
                      {sw.rgb}
                    </span>
                  </div>
                </button>
              {/each}
            </div>
            {#if swatchToast}
              <p role="status" aria-live="polite" class="mt-3 font-mono text-xs text-[var(--accent)]">
                {swatchToast}
              </p>
            {/if}
          </div>
        </div>

        <div class="grid grid-cols-1 items-center gap-8 lg:grid-cols-12">
          <div class="lg:col-span-5">
            <h3 class="text-xl font-semibold tracking-tight text-[var(--ink)]">
              {copy.craftItems[2].title}
            </h3>
            <p class="mt-3 text-sm leading-relaxed text-[var(--ink-muted)] sm:text-base">
              {copy.craftItems[2].body}
            </p>
            <div class="mt-4 font-mono text-xs text-[var(--ink-faint)]">
              {copy.craftItems[2].meta}
            </div>
          </div>
          <div class="rounded-[28px] bg-[var(--stage-bg)] p-7 sm:p-10 lg:col-span-7">
            <div class="divide-y divide-[var(--hairline)] text-sm">
              <div class="flex items-center justify-between py-3">
                <span class="font-medium text-[var(--ink)]">
                  {locale === 'zh' ? '★ 发版日志整理 Prompt' : '★ Release notes prompt'}
                </span>
                <span class="kbd-cap">↵</span>
              </div>
              <div class="flex items-center justify-between py-3">
                <span class="font-mono text-xs text-[var(--ink-muted)]">
                  git rebase -i origin/main
                </span>
                <span class="kbd-cap">⇧↓</span>
              </div>
              <div class="flex items-center justify-between py-3">
                <span class="text-[var(--ink-muted)]">
                  {locale === 'zh' ? '多选连续条目，回车一次合并粘贴' : 'Merge selected clips in one stroke'}
                </span>
                <span class="kbd-cap">⌘</span>
              </div>
            </div>
          </div>
        </div>
      </div>
    </section>

    <section id="privacy" class="mx-auto max-w-5xl px-4 py-24 sm:px-6 sm:py-36">
      <h2
        class="max-w-3xl text-balance text-3xl font-semibold tracking-[-0.03em] text-[var(--ink)] sm:text-5xl"
      >
        {copy.privacyHeading}
      </h2>
      <p class="mt-5 max-w-2xl text-base leading-relaxed text-[var(--ink-muted)] sm:text-lg">
        {copy.privacyLead}
      </p>

      <div class="mt-14 grid grid-cols-1 gap-10 border-t border-[var(--hairline)] pt-10 sm:grid-cols-2">
        {#each copy.privacyFacts as fact (fact.title)}
          <div>
            <h3 class="text-base font-semibold text-[var(--ink)]">{fact.title}</h3>
            <p class="mt-2.5 text-sm leading-relaxed text-[var(--ink-muted)]">{fact.body}</p>
          </div>
        {/each}
      </div>
    </section>

    <section id="themes" class="mx-auto max-w-5xl px-4 py-20 sm:px-6 sm:py-32">
      <h2
        class="text-balance text-3xl font-semibold tracking-[-0.03em] text-[var(--ink)] sm:text-5xl"
      >
        {copy.themesHeading}
      </h2>

      <div class="mt-12 grid grid-cols-1 gap-5 sm:grid-cols-3">
        {#each copy.themeCards as card (card.id)}
          {@const isActive = themeStore.resolved === card.id}
          <button
            type="button"
            onclick={() => selectTheme(card.id)}
            aria-pressed={isActive}
            class="group relative flex h-44 flex-col justify-between overflow-hidden rounded-[24px] p-6 text-left transition hover:-translate-y-1 sm:h-52"
            style:background={card.id === 'paper'
              ? '#f5f5f7'
              : card.id === 'ink'
                ? '#161619'
                : '#f5ebee'}
            style:color={card.id === 'ink' ? '#f2f2f4' : card.id === 'petal' ? '#24181d' : '#181818'}
            style:box-shadow={isActive ? 'inset 0 0 0 2px var(--accent)' : 'none'}
          >
            <div class="flex items-center justify-between">
              <span
                class="h-3 w-3 rounded-full"
                style:background={card.id === 'petal' ? '#9f3c5c' : '#d97706'}
              ></span>
              <span class="font-mono text-xs opacity-50">{card.sub}</span>
            </div>

            <div class="flex items-end justify-between">
              <span class="text-2xl font-semibold tracking-tight">{card.name}</span>
              {#if isActive}
                <span class="font-mono text-xs opacity-70">●</span>
              {/if}
            </div>
          </button>
        {/each}
      </div>
    </section>

    <section id="download" class="mx-auto max-w-5xl px-4 pt-20 pb-28 sm:px-6 sm:pt-32 sm:pb-36">
      <div class="flex flex-col justify-between gap-4 sm:flex-row sm:items-end">
        <div>
          <h2
            class="text-balance text-3xl font-semibold tracking-[-0.03em] text-[var(--ink)] sm:text-5xl"
          >
            {copy.downloadHeading}
          </h2>
          <p class="mt-3 text-sm text-[var(--ink-muted)]">{copy.downloadNote}</p>
        </div>
        <button
          type="button"
          onclick={openMacGuide}
          class="self-start text-sm text-[var(--ink-muted)] underline decoration-[var(--hairline-strong)] underline-offset-4 transition hover:text-[var(--ink)] sm:self-auto"
        >
          {copy.downloadGuideBtn}
        </button>
      </div>

      <div class="mt-12 divide-y divide-[var(--hairline)] border-y border-[var(--hairline)]">
        {#each osOrder as osId (osId)}
          {@const item = DOWNLOADS[osId]}
          {@const isCurrent = detectedOs !== null && osId === detectedOs}
          <a
            href={item.url}
            class="group flex flex-col justify-between gap-2 py-5 transition sm:flex-row sm:items-center"
          >
            <div class="flex items-center gap-3">
              <span
                class={[
                  'text-base font-semibold transition-colors group-hover:text-[var(--accent)]',
                  isCurrent ? 'text-[var(--accent)]' : 'text-[var(--ink)]',
                ]}
              >
                {locale === 'zh' ? item.labelZh : item.labelEn}
              </span>
              <span class="text-xs text-[var(--ink-muted)]">
                {locale === 'zh' ? item.metaZh : item.metaEn}
              </span>
            </div>

            <div class="flex items-center gap-3">
              <span class="font-mono text-xs text-[var(--ink-faint)]">{item.file}</span>
              <Download
                class="h-4 w-4 text-[var(--ink-muted)] transition group-hover:translate-y-0.5 group-hover:text-[var(--ink)]"
                strokeWidth={1.6}
                aria-hidden="true"
              />
            </div>
          </a>
        {/each}
      </div>
    </section>
  </main>

  <footer class="mx-auto max-w-5xl border-t border-[var(--hairline)] px-4 py-10 sm:px-6">
    <div
      class="flex flex-col items-start justify-between gap-4 text-xs text-[var(--ink-faint)] sm:flex-row sm:items-center"
    >
      <div class="flex items-center gap-2.5">
        <BrandMark size={20} />
        <span class="font-medium text-[var(--ink)]">ClipLeaf</span>
        <span>·</span>
        <span>{copy.footerLicense}</span>
      </div>
      <div class="flex items-center gap-5">
        <a
          href={REPO_URL}
          target="_blank"
          rel="noopener noreferrer"
          class="inline-flex items-center gap-1 transition hover:text-[var(--ink)]"
        >
          <span>GitHub</span>
          <ArrowUpRight class="h-3 w-3" aria-hidden="true" />
        </a>
        <a
          href={RELEASES_URL}
          target="_blank"
          rel="noopener noreferrer"
          class="transition hover:text-[var(--ink)]"
        >
          {copy.footerReleases}
        </a>
        <a
          href={`${REPO_URL}/issues`}
          target="_blank"
          rel="noopener noreferrer"
          class="transition hover:text-[var(--ink)]"
        >
          {copy.footerIssues}
        </a>
        <span class="font-mono">clipleaf.app</span>
      </div>
    </div>
  </footer>

  <InstallDialog {locale} openSignal={dialogOpenSignal} />
</div>
