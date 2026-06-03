<script lang="ts">
  import { marked } from 'marked';

  let { content = '' } = $props<{ content: string }>();

  // 配置 marked
  marked.setOptions({
    breaks: true, // 支持换行
    gfm: true, // 启用 GitHub Flavored Markdown
  });

  // 渲染 Markdown
  let html = $derived(marked.parse(content) as string);
</script>

<div class="markdown-content">
  {@html html}
</div>

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
