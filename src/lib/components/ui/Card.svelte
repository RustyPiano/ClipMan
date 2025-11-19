<script lang="ts">
  import { type Snippet } from 'svelte';

  type Props = {
    class?: string;
    children?: Snippet;
    onclick?: (e: MouseEvent) => void;
  };

  let {
    class: className = '',
    children,
    onclick,
    ...rest
  }: Props = $props();
</script>

<!-- svelte-ignore a11y_click_events_have_key_events -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
  class="rounded-lg border bg-card text-card-foreground shadow-sm {className}"
  {onclick}
  {...rest}
>
  {@render children?.()}
</div>
