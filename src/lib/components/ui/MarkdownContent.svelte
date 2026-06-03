<script lang="ts">
  import type { Token, Tokens } from 'marked';
  import { markdownBlockTokens, safeMarkdownUrl } from '$lib/utils/markdown';

  let { content = '' } = $props<{ content: string }>();

  const tokens = $derived(markdownBlockTokens(content));

  function tokenKey(token: Token, index: number) {
    return `${index}:${token.type}:${token.raw.slice(0, 32)}`;
  }

  function isHeadingToken(token: Token): token is Tokens.Heading {
    return token.type === 'heading' && 'depth' in token;
  }

  function isListToken(token: Token): token is Tokens.List {
    return token.type === 'list' && 'items' in token;
  }

  function isTableToken(token: Token): token is Tokens.Table {
    return token.type === 'table' && 'header' in token && 'rows' in token;
  }

  function isLinkToken(token: Token): token is Tokens.Link {
    return token.type === 'link' && 'href' in token;
  }

  function isImageToken(token: Token): token is Tokens.Image {
    return token.type === 'image' && 'href' in token && 'text' in token;
  }

  function taskLabel(item: Tokens.ListItem) {
    return item.checked ? 'Checked task' : 'Unchecked task';
  }
</script>

<div class="markdown-content">
  {@render blockTokens(tokens)}
</div>

{#snippet blockTokens(items: Token[])}
  {#each items as token, index (tokenKey(token, index))}
    {@render blockToken(token)}
  {/each}
{/snippet}

{#snippet blockToken(token: Token)}
  {#if token.type === 'space' || token.type === 'def' || token.type === 'html'}
    <!-- Raw HTML and definitions are intentionally ignored. -->
  {:else if isHeadingToken(token)}
    {@render headingToken(token)}
  {:else if token.type === 'paragraph'}
    <p>{@render inlineTokens(token.tokens ?? [])}</p>
  {:else if token.type === 'blockquote'}
    <blockquote>{@render blockTokens(token.tokens ?? [])}</blockquote>
  {:else if isListToken(token)}
    {@render listToken(token)}
  {:else if token.type === 'code'}
    <pre><code>{token.text}</code></pre>
  {:else if token.type === 'hr'}
    <hr />
  {:else if isTableToken(token)}
    {@render tableToken(token)}
  {:else if token.type === 'text'}
    {#if token.tokens}
      <p>{@render inlineTokens(token.tokens)}</p>
    {:else}
      <p>{token.text}</p>
    {/if}
  {:else}
    <p>{token.raw}</p>
  {/if}
{/snippet}

{#snippet headingToken(token: Tokens.Heading)}
  {#if token.depth === 1}
    <h1>{@render inlineTokens(token.tokens)}</h1>
  {:else if token.depth === 2}
    <h2>{@render inlineTokens(token.tokens)}</h2>
  {:else if token.depth === 3}
    <h3>{@render inlineTokens(token.tokens)}</h3>
  {:else if token.depth === 4}
    <h4>{@render inlineTokens(token.tokens)}</h4>
  {:else if token.depth === 5}
    <h5>{@render inlineTokens(token.tokens)}</h5>
  {:else}
    <h6>{@render inlineTokens(token.tokens)}</h6>
  {/if}
{/snippet}

{#snippet listToken(token: Tokens.List)}
  {#if token.ordered}
    <ol start={typeof token.start === 'number' ? token.start : undefined}>
      {#each token.items as item, index (tokenKey(item, index))}
        {@render listItemToken(item)}
      {/each}
    </ol>
  {:else}
    <ul>
      {#each token.items as item, index (tokenKey(item, index))}
        {@render listItemToken(item)}
      {/each}
    </ul>
  {/if}
{/snippet}

{#snippet listItemToken(item: Tokens.ListItem)}
  {#if item.task}
    <li class="task-list-item">
      <input
        type="checkbox"
        checked={item.checked === true}
        disabled
        aria-label={taskLabel(item)}
      />
      <div class="task-list-content">
        {@render blockTokens(item.tokens)}
      </div>
    </li>
  {:else}
    <li>{@render blockTokens(item.tokens)}</li>
  {/if}
{/snippet}

{#snippet tableToken(token: Tokens.Table)}
  <table>
    <thead>
      <tr>
        {#each token.header as cell, index (`header:${index}:${cell.text}`)}
          <th>{@render inlineTokens(cell.tokens)}</th>
        {/each}
      </tr>
    </thead>
    <tbody>
      {#each token.rows as row, rowIndex (rowIndex)}
        <tr>
          {#each row as cell, cellIndex (`${rowIndex}:${cellIndex}:${cell.text}`)}
            <td>{@render inlineTokens(cell.tokens)}</td>
          {/each}
        </tr>
      {/each}
    </tbody>
  </table>
{/snippet}

{#snippet inlineTokens(items: Token[])}
  {#each items as token, index (tokenKey(token, index))}
    {@render inlineToken(token)}
  {/each}
{/snippet}

{#snippet inlineToken(token: Token)}
  {#if token.type === 'text' || token.type === 'escape'}
    {token.text}
  {:else if token.type === 'strong'}
    <strong>{@render inlineTokens(token.tokens ?? [])}</strong>
  {:else if token.type === 'em'}
    <em>{@render inlineTokens(token.tokens ?? [])}</em>
  {:else if token.type === 'codespan'}
    <code>{token.text}</code>
  {:else if token.type === 'br'}
    <br />
  {:else if token.type === 'del'}
    <del>{@render inlineTokens(token.tokens ?? [])}</del>
  {:else if isLinkToken(token)}
    {@render linkToken(token)}
  {:else if isImageToken(token)}
    {@render imageToken(token)}
  {:else if token.type === 'html'}
    <!-- Raw inline HTML is intentionally ignored. -->
  {:else}
    {token.raw}
  {/if}
{/snippet}

{#snippet linkToken(token: Tokens.Link)}
  {@const href = safeMarkdownUrl(token.href, 'href')}
  {#if href}
    <a {href} title={token.title ?? undefined} target="_blank" rel="noopener noreferrer">
      {@render inlineTokens(token.tokens)}
    </a>
  {:else}
    {@render inlineTokens(token.tokens)}
  {/if}
{/snippet}

{#snippet imageToken(token: Tokens.Image)}
  {@const src = safeMarkdownUrl(token.href, 'src')}
  {#if src}
    <img {src} alt={token.text} title={token.title ?? undefined} loading="lazy" />
  {:else}
    {token.text}
  {/if}
{/snippet}

<style>
  .markdown-content :global(h1),
  .markdown-content :global(h2),
  .markdown-content :global(h3),
  .markdown-content :global(h4),
  .markdown-content :global(h5),
  .markdown-content :global(h6) {
    font-weight: 700;
    margin-top: 1em;
    margin-bottom: 0.5em;
    line-height: 1.3;
  }

  .markdown-content :global(h1) {
    font-size: 1.5em;
    border-bottom: 1px solid hsl(var(--border));
    padding-bottom: 0.3em;
  }

  .markdown-content :global(h2) {
    font-size: 1.3em;
    border-bottom: 1px solid hsl(var(--border));
    padding-bottom: 0.3em;
  }

  .markdown-content :global(h3) {
    font-size: 1.15em;
  }

  .markdown-content :global(h4) {
    font-size: 1.05em;
  }

  .markdown-content :global(h5),
  .markdown-content :global(h6) {
    font-size: 1em;
  }

  .markdown-content :global(p) {
    margin: 0.75em 0;
    line-height: 1.6;
  }

  .markdown-content :global(ul),
  .markdown-content :global(ol) {
    margin: 0.75em 0;
    padding-left: 2em;
  }

  .markdown-content :global(li) {
    margin: 0.25em 0;
    line-height: 1.6;
  }

  .markdown-content :global(ul) {
    list-style-type: disc;
  }

  .markdown-content :global(ol) {
    list-style-type: decimal;
  }

  .markdown-content :global(.task-list-item) {
    list-style: none;
  }

  .task-list-item {
    display: flex;
    align-items: flex-start;
    gap: 0.5em;
  }

  .task-list-item input {
    margin-top: 0.45em;
  }

  .task-list-content {
    min-width: 0;
  }

  .task-list-content :global(p:first-child) {
    margin-top: 0;
  }

  .task-list-content :global(p:last-child) {
    margin-bottom: 0;
  }

  .markdown-content :global(code) {
    background-color: hsl(var(--muted));
    padding: 0.2em 0.4em;
    border-radius: 4px;
    font-family:
      'SF Mono', Monaco, 'Cascadia Code', 'Roboto Mono', Consolas, 'Courier New', monospace;
    font-size: 0.9em;
  }

  .markdown-content :global(pre) {
    background-color: hsl(var(--muted));
    padding: 1em;
    border-radius: 6px;
    overflow-x: auto;
    margin: 1em 0;
    border: 1px solid hsl(var(--border));
  }

  .markdown-content :global(pre code) {
    background-color: transparent;
    padding: 0;
    font-size: 0.875em;
    line-height: 1.5;
  }

  .markdown-content :global(blockquote) {
    border-left: 4px solid hsl(var(--primary));
    padding-left: 1em;
    margin: 1em 0;
    color: hsl(var(--muted-foreground));
    background-color: hsl(var(--muted) / 0.3);
    padding: 0.5em 1em;
    border-radius: 4px;
  }

  .markdown-content :global(a) {
    color: hsl(var(--primary));
    text-decoration: underline;
    text-decoration-color: hsl(var(--primary) / 0.3);
    transition: text-decoration-color 0.2s;
    word-break: break-all;
  }

  .markdown-content :global(a:hover) {
    text-decoration-color: hsl(var(--primary));
  }

  .markdown-content :global(strong) {
    font-weight: 700;
    color: hsl(var(--foreground));
  }

  .markdown-content :global(em) {
    font-style: italic;
  }

  .markdown-content :global(hr) {
    border: none;
    border-top: 1px solid hsl(var(--border));
    margin: 1.5em 0;
  }

  .markdown-content :global(table) {
    border-collapse: collapse;
    width: 100%;
    margin: 1em 0;
  }

  .markdown-content :global(th),
  .markdown-content :global(td) {
    border: 1px solid hsl(var(--border));
    padding: 0.5em;
    text-align: left;
  }

  .markdown-content :global(th) {
    background-color: hsl(var(--muted));
    font-weight: 700;
  }

  .markdown-content :global(img) {
    max-width: 100%;
    height: auto;
    border-radius: 4px;
    margin: 1em 0;
  }

  /* 首个元素和最后一个元素的边距优化 */
  .markdown-content :global(> *:first-child) {
    margin-top: 0;
  }

  .markdown-content :global(> *:last-child) {
    margin-bottom: 0;
  }
</style>
