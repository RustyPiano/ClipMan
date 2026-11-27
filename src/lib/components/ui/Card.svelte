<script lang="ts">
  import type { Snippet } from 'svelte';
  import type { HTMLAttributes } from 'svelte/elements';

  type Props = Omit<HTMLAttributes<HTMLDivElement>, 'class'> & {
    class?: string;
    children?: Snippet;
  };

  let { class: className = '', children, ...rest }: Props = $props();
</script>

<div class="rounded-lg border border-border bg-card text-card-foreground {className}" {...rest}>
  {@render children?.()}
</div>
