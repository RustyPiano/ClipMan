<script lang="ts">
  import { MoveHorizontal } from 'lucide-svelte';
  import { COPY, splitPhrases, type Locale, type PlainModeId } from '../lib/i18n';

  interface Props {
    locale: Locale;
  }

  let { locale }: Props = $props();

  let split = $state(62);
  let docKind = $state<'ppt' | 'word'>('ppt');
  let activeMode = $state<PlainModeId>('global');

  let copy = $derived(COPY[locale].plainPaste);

  function chooseMode(id: PlainModeId): void {
    activeMode = id;
    split = 0;
  }
</script>

<section id="plain-paste" class="mx-auto max-w-5xl px-4 py-20 sm:px-6 sm:py-28">
  <div class="flex flex-col justify-between gap-6 sm:flex-row sm:items-end">
    <h2
      class="max-w-2xl font-serif text-3xl font-semibold tracking-[-0.01em] text-[var(--ink)] sm:text-[44px] sm:leading-[1.15]"
    >
      {#each splitPhrases(copy.heading) as phrase, index (index)}
        <span class="inline-block">{phrase}</span>
      {/each}
    </h2>

    <div
      class="inline-flex self-start rounded-full bg-[var(--stage-bg)] p-1 text-xs sm:self-auto"
      role="group"
      aria-label="Target document"
    >
      <button
        type="button"
        onclick={() => (docKind = 'ppt')}
        aria-pressed={docKind === 'ppt'}
        class={[
          'rounded-full px-4 py-1.5 font-medium transition',
          docKind === 'ppt'
            ? 'bg-[var(--surface)] text-[var(--ink)] shadow-2xs'
            : 'text-[var(--ink-muted)] hover:text-[var(--ink)]',
        ]}
      >
        {copy.tabPpt}
      </button>
      <button
        type="button"
        onclick={() => (docKind = 'word')}
        aria-pressed={docKind === 'word'}
        class={[
          'rounded-full px-4 py-1.5 font-medium transition',
          docKind === 'word'
            ? 'bg-[var(--surface)] text-[var(--ink)] shadow-2xs'
            : 'text-[var(--ink-muted)] hover:text-[var(--ink)]',
        ]}
      >
        {copy.tabWord}
      </button>
    </div>
  </div>

  <div class="mt-10 rounded-[28px] bg-[var(--stage-bg)] p-4 sm:mt-12 sm:rounded-[36px] sm:p-10">
    <div class="space-y-4 sm:hidden">
      <div>
        <div class="mb-2 px-1 text-xs font-medium text-[var(--ink-faint)]">{copy.labelMessy}</div>
        <div class="qb-panel rounded-2xl p-4">
          <div
            class="rounded-sm bg-[#e5e7eb] px-3 py-2 text-[#1f2937]"
            style:font-family="Georgia, 'Times New Roman', serif"
          >
            <span class="text-[18px] font-bold text-[#0f172a]">{copy.messyLead}</span>
            <span class="bg-[#fef08a] px-1 text-[13px]">{copy.messyMid}</span>
            <span class="text-[12px] text-[#2563eb] underline">{copy.messyLink}</span>
            <span class="text-[15px] italic text-[#475569]">{copy.messyTail}</span>
          </div>
        </div>
      </div>
      <div>
        <div class="mb-2 px-1 text-xs font-medium text-[var(--accent)]">{copy.labelPlain}</div>
        <div class="qb-panel rounded-2xl p-4">
          <p class="text-sm leading-relaxed text-[var(--ink)]">{copy.sampleParagraph}</p>
        </div>
      </div>
    </div>

    <div class="mb-4 hidden items-center justify-between px-1 text-xs sm:flex">
      <button
        type="button"
        onclick={() => (split = 100)}
        class={[
          'font-medium transition',
          split > 55 ? 'text-[var(--ink)]' : 'text-[var(--ink-faint)] hover:text-[var(--ink)]',
        ]}
      >
        ← {copy.labelMessy}
      </button>
      <span class="hidden text-[var(--ink-faint)] sm:inline">{copy.dragHint}</span>
      <button
        type="button"
        onclick={() => (split = 0)}
        class={[
          'font-medium transition',
          split < 45 ? 'text-[var(--accent)]' : 'text-[var(--ink-faint)] hover:text-[var(--ink)]',
        ]}
      >
        {copy.labelPlain} →
      </button>
    </div>

    <div class="qb-panel relative hidden overflow-hidden rounded-[20px] select-none sm:block">
      {#if docKind === 'ppt'}
        <div class="p-7 sm:p-12">
          <div class="flex items-center justify-between font-mono text-xs text-[var(--ink-faint)]">
            <span>{copy.pptHeader}</span>
            <span>04 / 12</span>
          </div>
          <h3
            class="mt-5 max-w-lg text-2xl font-semibold tracking-tight text-[var(--ink)] sm:text-3xl"
          >
            {copy.pptTitle}
          </h3>
          <div class="mt-8 grid grid-cols-1 gap-8 sm:grid-cols-12">
            <div class="sm:col-span-4">
              <div class="text-5xl font-semibold tracking-tight text-[var(--ink)]">-72%</div>
              <div class="mt-2 text-sm text-[var(--ink-muted)]">{copy.pptMetric}</div>
            </div>
            <div class="sm:col-span-8">
              <div class="min-h-[112px]">
                <p class="text-base leading-relaxed text-[var(--ink)] sm:text-lg">
                  {copy.sampleParagraph}
                </p>
              </div>
            </div>
          </div>
        </div>

        <div
          aria-hidden="true"
          class="pointer-events-none absolute inset-0 bg-[var(--surface)] p-7 sm:p-12"
          style:clip-path={`inset(0 ${100 - split}% 0 0)`}
        >
          <div class="flex items-center justify-between font-mono text-xs text-[var(--ink-faint)]">
            <span>{copy.pptHeader}</span>
            <span>04 / 12</span>
          </div>
          <h3
            class="mt-5 max-w-lg text-2xl font-semibold tracking-tight text-[var(--ink)] sm:text-3xl"
          >
            {copy.pptTitle}
          </h3>
          <div class="mt-8 grid grid-cols-1 gap-8 sm:grid-cols-12">
            <div class="sm:col-span-4">
              <div class="text-5xl font-semibold tracking-tight text-[var(--ink)]">-72%</div>
              <div class="mt-2 text-sm text-[var(--ink-muted)]">{copy.pptMetric}</div>
            </div>
            <div class="sm:col-span-8">
              <div class="min-h-[112px]">
                <div
                  class="rounded-sm bg-[#e5e7eb] px-3 py-2 text-[#1f2937]"
                  style:font-family="Georgia, 'Times New Roman', serif"
                >
                  <span class="text-[20px] font-bold tracking-wide text-[#0f172a]">
                    {copy.messyLead}
                  </span>
                  <span class="bg-[#fef08a] px-1 text-[14px] text-[#334155]">
                    {copy.messyMid}
                  </span>
                  <span class="text-[13px] text-[#2563eb] underline">
                    {copy.messyLink}
                  </span>
                  <span class="text-[17px] italic text-[#475569]">
                    {copy.messyTail}
                  </span>
                </div>
              </div>
            </div>
          </div>
        </div>
      {:else}
        <div class="mx-auto max-w-2xl p-7 sm:p-12">
          <div class="flex items-center justify-between font-mono text-xs text-[var(--ink-faint)]">
            <span>{copy.wordDocName}</span>
            <span>10.5pt</span>
          </div>
          <h3 class="mt-6 text-lg font-semibold text-[var(--ink)]">{copy.wordHeading}</h3>
          <p class="mt-3 text-sm leading-relaxed text-[var(--ink-muted)]">
            {copy.wordBeforeText}
          </p>
          <div class="mt-4 flex min-h-[100px] items-center">
            <p class="text-sm leading-relaxed text-[var(--ink)]">
              {copy.sampleParagraph}
            </p>
          </div>
          <p class="mt-4 text-sm leading-relaxed text-[var(--ink-muted)]">
            {copy.wordAfterText}
          </p>
        </div>

        <div
          aria-hidden="true"
          class="pointer-events-none absolute inset-0 bg-[var(--surface)]"
          style:clip-path={`inset(0 ${100 - split}% 0 0)`}
        >
          <div class="mx-auto max-w-2xl p-7 sm:p-12">
            <div
              class="flex items-center justify-between font-mono text-xs text-[var(--ink-faint)]"
            >
              <span>{copy.wordDocName}</span>
              <span>10.5pt</span>
            </div>
            <h3 class="mt-6 text-lg font-semibold text-[var(--ink)]">{copy.wordHeading}</h3>
            <p class="mt-3 text-sm leading-relaxed text-[var(--ink-muted)]">
              {copy.wordBeforeText}
            </p>
            <div class="mt-4 flex min-h-[100px] items-center">
              <div
                class="w-full rounded-sm bg-[#e5e7eb] px-3 py-2 text-[#1f2937]"
                style:font-family="Georgia, 'Times New Roman', serif"
              >
                <span class="text-[18px] font-bold text-[#0f172a]">{copy.messyLead}</span>
                <span class="bg-[#fef08a] px-1 text-[13px]">{copy.messyMid}</span>
                <span class="text-[12px] text-[#2563eb] underline">{copy.messyLink}</span>
                <span class="text-[15px] italic text-[#475569]">{copy.messyTail}</span>
              </div>
            </div>
            <p class="mt-4 text-sm leading-relaxed text-[var(--ink-muted)]">
              {copy.wordAfterText}
            </p>
          </div>
        </div>
      {/if}

      <div
        aria-hidden="true"
        class="pointer-events-none absolute top-0 bottom-0 z-10 w-[2px] -translate-x-1/2 bg-[var(--accent)]"
        style:left={`${split}%`}
      >
        <div
          class="absolute top-1/2 left-1/2 flex h-9 w-9 -translate-x-1/2 -translate-y-1/2 items-center justify-center rounded-full bg-[var(--surface)] text-[var(--accent)] shadow-md ring-1 ring-[var(--hairline-strong)]"
        >
          <MoveHorizontal class="h-4 w-4" strokeWidth={1.8} />
        </div>
      </div>

      <input
        type="range"
        min="0"
        max="100"
        bind:value={split}
        aria-label={copy.dragHint}
        class="curtain-range absolute inset-0 z-20 h-full w-full"
      />
    </div>
  </div>

  <div class="mt-10 grid grid-cols-1 gap-8 sm:grid-cols-3">
    {#each copy.modes as mode (mode.id)}
      {@const selected = activeMode === mode.id && split < 50}
      <button
        type="button"
        onclick={() => chooseMode(mode.id)}
        aria-pressed={selected}
        class="group text-left transition"
      >
        <div class="flex items-center justify-between gap-2">
          <span
            class={[
              'text-base font-semibold transition-colors',
              selected
                ? 'text-[var(--ink)]'
                : 'text-[var(--ink-muted)] group-hover:text-[var(--ink)]',
            ]}
          >
            {mode.title}
          </span>
          <span
            class={[
              'font-mono text-xs transition-colors',
              selected ? 'text-[var(--accent)]' : 'text-[var(--ink-faint)]',
            ]}
          >
            {mode.badge}
          </span>
        </div>
        <p class="mt-2 text-sm leading-relaxed text-[var(--ink-muted)]">{mode.desc}</p>
      </button>
    {/each}
  </div>
</section>
