<script lang="ts">
  import type { HTMLInputAttributes } from 'svelte/elements';

  type Props = Omit<HTMLInputAttributes, 'class' | 'checked' | 'type'> & {
    checked?: boolean;
    class?: string;
  };

  let { checked = $bindable(false), class: className = '', ...rest }: Props = $props();
</script>

<input
  type="checkbox"
  role="switch"
  aria-checked={checked}
  bind:checked
  class="w-11 h-6 appearance-none rounded-full relative cursor-pointer transition-colors disabled:cursor-not-allowed disabled:opacity-50
         before:content-[''] before:absolute before:top-1 before:left-1 before:w-4 before:h-4 before:bg-white checked:before:bg-primary-foreground before:rounded-full before:transition-transform
         checked:before:translate-x-5 {className}"
  style:background-color={checked ? 'var(--primary)' : 'var(--muted)'}
  {...rest}
/>
